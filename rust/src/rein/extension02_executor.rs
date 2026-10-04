//! A disposable 0.2 host. The core owns sequencing, identity and adoption.
use super::maintenance::{Approval, Candidate};
use super::stdio_executor::CallRecord;
use super::{
    dispatch_readonly, extension02, ControlSignal, ExecutorResult, ToolCall, ToolError,
    ToolExecutor, ToolResult, Workspace,
};
use serde_json::{json, Value};
use std::{
    path::{Component, Path, PathBuf},
    pin::Pin,
    process::Stdio,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
};
const LIMIT: usize = extension02::MAX_MESSAGE_BYTES;
static SEQUENCE: AtomicUsize = AtomicUsize::new(1);
#[derive(Clone)]
pub struct Extension02Executor {
    pub workspace: PathBuf,
    pub node: String,
    pub host_script: PathBuf,
    pub code_root: PathBuf,
    pub grace: Duration,
    pub env: Vec<(String, String)>,
    pub on_started: Option<Arc<dyn Fn() + Send + Sync>>,
    records: Arc<Mutex<Vec<CallRecord>>>,
    busy: Arc<AtomicBool>,
    task_id: String,
    maintenance: Arc<Mutex<Option<(String, String)>>>,
    approvals: Arc<Mutex<std::collections::BTreeMap<String, Candidate>>>,
}
struct Lease(Arc<AtomicBool>);
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release)
    }
}
struct Frames {
    reader: BufReader<ChildStdout>,
    pending: Vec<u8>,
}
impl Frames {
    fn new(stdout: ChildStdout) -> Self {
        Self {
            reader: BufReader::new(stdout),
            pending: vec![],
        }
    }
    async fn next(&mut self) -> Result<Option<Value>, ()> {
        loop {
            if let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
                let frame = self.pending.drain(..=end).collect::<Vec<_>>();
                let text = std::str::from_utf8(&frame[..end]).map_err(|_| ())?;
                return serde_json::from_str(text).map(Some).map_err(|_| ());
            }
            if self.pending.len() >= LIMIT {
                return Err(());
            }
            let bytes = self.reader.fill_buf().await.map_err(|_| ())?;
            if bytes.is_empty() {
                return if self.pending.is_empty() {
                    Ok(None)
                } else {
                    Err(())
                };
            }
            let n = bytes.len().min(LIMIT - self.pending.len());
            self.pending.extend_from_slice(&bytes[..n]);
            self.reader.consume(n);
        }
    }
    async fn controlled(
        &mut self,
        signal: &ControlSignal,
        deadline: Instant,
    ) -> Result<Option<Value>, &'static str> {
        let mut tick = tokio::time::interval(Duration::from_millis(5));
        let mut read = Box::pin(self.next());
        loop {
            tokio::select! {v=&mut read=>return v.map_err(|_|"invalid_frame"),_ = tick.tick()=>{
             if signal.is_cancelled(){return Err("cancelled")}
             if Instant::now()>=deadline{return Err("deadline")}
            }}
        }
    }
}
fn keys(v: &Value, expected: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == expected.len() && expected.iter().all(|k| o.contains_key(*k)))
}
fn hash(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn workspace_snapshot(root: &Path) -> Option<Value> {
    fn visit(root: &Path, dir: &Path, out: &mut Vec<Value>) -> Option<()> {
        let mut entries = std::fs::read_dir(dir)
            .ok()?
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path).ok()?;
            if meta.file_type().is_symlink() {
                return None;
            }
            if meta.is_dir() {
                visit(root, &path, out)?;
            } else if meta.is_file() {
                let bytes = std::fs::read(&path).ok()?;
                out.push(json!({"path": path.strip_prefix(root).ok()?.to_string_lossy().replace('\\',"/"), "sha256": hash(&bytes)}));
            }
        }
        Some(())
    }
    let mut out = Vec::new();
    visit(root, root, &mut out)?;
    Some(json!(out))
}
fn safe_file(root: &Path, path: &str) -> Option<PathBuf> {
    let rel = Path::new(path);
    if path.is_empty() || rel.is_absolute() {
        return None;
    }
    let mut p = root.to_path_buf();
    for component in rel.components() {
        match component {
            Component::Normal(c) => p.push(c),
            _ => return None,
        };
        if std::fs::symlink_metadata(&p).ok()?.file_type().is_symlink() {
            return None;
        }
    }
    let actual = p.canonicalize().ok()?;
    if !actual.starts_with(root) || !actual.is_file() {
        return None;
    }
    Some(actual)
}
/// Teaching evidence checker: repeat the bounded readonly observation before adoption.
/// This costs another read/search; it is not a production performance optimization.
fn observe(root: &Path, call: &ToolCall) -> Option<(String, Value)> {
    if call.name == "read_file" {
        safe_file(root, call.arguments["path"].as_str()?)?;
    }
    let result = dispatch_readonly(
        call,
        &Workspace {
            root: root.to_path_buf(),
        },
    );
    if !result.ok {
        return None;
    }
    let output = result.output?;
    let paths = if call.name == "read_file" {
        vec![call.arguments["path"].as_str()?.to_owned()]
    } else {
        output.lines().map(str::to_owned).collect()
    };
    let mut sources = vec![];
    for path in paths {
        let actual = safe_file(root, &path)?;
        let bytes = std::fs::read(actual).ok()?;
        sources.push(json!({"path":path,"sha256":hash(&bytes)}));
    }
    Some((output, json!(sources)))
}
impl Extension02Executor {
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        let code_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        Self {
            workspace: workspace.into(),
            node: "node".into(),
            host_script: code_root.join("ts/src/rein/extension02-host.ts"),
            code_root,
            grace: Duration::from_secs(1),
            env: vec![],
            on_started: None,
            records: Arc::new(Mutex::new(vec![])),
            busy: Arc::new(AtomicBool::new(false)),
            task_id: format!(
                "task-{}-{}",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ),
            maintenance: Arc::new(Mutex::new(None)),
            approvals: Arc::new(Mutex::new(std::collections::BTreeMap::new())),
        }
    }
    pub fn enable_maintenance(&self, target: impl Into<String>, rule: impl Into<String>) {
        *self.maintenance.lock().unwrap() = Some((target.into(), rule.into()));
    }
    pub fn authorize_candidate(&self, candidate: &Candidate, approval: &Approval) -> bool {
        if !approval.matches(candidate.digest()) {
            return false;
        }
        self.approvals
            .lock()
            .unwrap()
            .insert(candidate.digest().into(), candidate.clone());
        true
    }
    pub fn records(&self) -> Vec<CallRecord> {
        self.records.lock().unwrap().clone()
    }
    async fn write(input: &mut ChildStdin, value: &Value, deadline: Instant) -> bool {
        let mut bytes = serde_json::to_vec(value).unwrap();
        bytes.push(b'\n');
        bytes.len() <= LIMIT
            && tokio::time::timeout_at(
                tokio::time::Instant::from_std(deadline),
                input.write_all(&bytes),
            )
            .await
            .is_ok_and(|r| r.is_ok())
    }
    async fn reap(&self, child: &mut Child, record: &mut CallRecord, kill: bool) {
        if kill {
            let _ = child.start_kill();
        }
        if let Ok(Ok(status)) = tokio::time::timeout(self.grace, child.wait()).await {
            record.reaped = true;
            record.exit_code = status.code();
            return;
        }
        let _ = child.start_kill();
        if let Ok(Ok(status)) = tokio::time::timeout(self.grace, child.wait()).await {
            record.reaped = true;
            record.exit_code = status.code();
        }
    }
    async fn exchange(
        &self,
        call: &ToolCall,
        signal: &ControlSignal,
        deadline: Instant,
        root: &Path,
        record: &mut CallRecord,
    ) -> ExecutorResult {
        let before = observe(root, call);
        let mut command = Command::new(&self.node);
        if self.host_script.extension().and_then(|x| x.to_str()) == Some("mjs") {
            command.arg(&self.host_script);
        } else {
            command.args(["--import", "tsx"]).arg(&self.host_script);
        }
        command
            .current_dir(&self.code_root)
            .env("REIN_HYBRID_WORKSPACE", root)
            .envs(self.env.iter().cloned())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if self.maintenance.lock().unwrap().is_some() {
            command.env("REIN_EXTENSION_PROFILE", "maintenance");
        }
        if call.name == extension02::APPLY_PATCH {
            if let Some(digest) = call.arguments.get("digest").and_then(Value::as_str) {
                command.env("REIN_APPROVED_PATCH_DIGEST", digest);
            }
        }
        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(_) => return ExecutorResult::NotDispatched,
        };
        record.child_pid = child.id();
        let mut input = child.stdin.take().unwrap();
        let mut frames = Frames::new(child.stdout.take().unwrap());
        let outcome = self
            .conversation(
                call,
                signal,
                deadline,
                root,
                record,
                &before,
                &if call.name == extension02::RUN_VERIFICATION {
                    workspace_snapshot(root)
                } else {
                    None
                },
                &mut input,
                &mut frames,
            )
            .await;
        drop(input);
        // No extra frame, truncated tail, nonzero exit or unreaped child is accepted.
        if matches!(
            outcome,
            ExecutorResult::Completed(_) | ExecutorResult::Cancelled
        ) {
            let eof = tokio::time::timeout(self.grace, frames.next()).await;
            if !matches!(eof, Ok(Ok(None))) {
                self.reap(&mut child, record, true).await;
                return ExecutorResult::OutcomeUnknown;
            }
            self.reap(&mut child, record, false).await;
            if !record.reaped || record.exit_code != Some(0) {
                return ExecutorResult::OutcomeUnknown;
            }
            outcome
        } else {
            self.reap(&mut child, record, true).await;
            outcome
        }
    }
    #[allow(clippy::too_many_arguments)]
    async fn conversation(
        &self,
        call: &ToolCall,
        signal: &ControlSignal,
        deadline: Instant,
        root: &Path,
        record: &mut CallRecord,
        before: &Option<(String, Value)>,
        snapshot_before: &Option<Value>,
        input: &mut ChildStdin,
        frames: &mut Frames,
    ) -> ExecutorResult {
        let ready = match frames.controlled(signal, deadline).await {
            Ok(Some(v)) => v,
            _ => return ExecutorResult::NotDispatched,
        };
        let maintenance = self.maintenance.lock().unwrap().clone();
        let declarations = serde_json::to_value(if maintenance.is_some() {
            extension02::maintenance_declarations()
        } else {
            extension02::readonly_declarations()
        })
        .unwrap();
        let capabilities = if maintenance.is_some() {
            json!([
                "read_file",
                "search_files",
                "run_verification",
                "apply_patch"
            ])
        } else {
            json!(["read_file", "search_files"])
        };
        if !keys(
            &ready,
            &[
                "protocol",
                "type",
                "tools",
                "capabilities",
                "maxMessageBytes",
            ],
        ) || ready["protocol"] != extension02::PROTOCOL
            || ready["type"] != "ready"
            || ready["tools"] != declarations
            || ready["capabilities"] != capabilities
            || ready["maxMessageBytes"]
                .as_u64()
                .is_none_or(|x| x == 0 || x > LIMIT as u64)
        {
            return ExecutorResult::NotDispatched;
        }
        if signal.is_cancelled() || Instant::now() >= deadline {
            return ExecutorResult::NotDispatched;
        }
        let session = format!("session-{}", record.request_id);
        let mut invoke = json!({"protocol":extension02::PROTOCOL,"type":"invoke","sessionId":session,"requestId":record.request_id,"taskId":record.task_id,"callId":call.id,"tool":call.name,"arguments":call.arguments});
        if let Some((_, sources)) = before {
            if call.name == "read_file" {
                invoke["evidenceTarget"] =
                    json!(format!("sha256:{}", sources[0]["sha256"].as_str().unwrap()));
            }
        }
        if call.name == extension02::APPLY_PATCH {
            let digest = call
                .arguments
                .get("digest")
                .and_then(Value::as_str)
                .unwrap_or("");
            let allowed = maintenance.is_some()
                && self.approvals.lock().unwrap().get(digest).is_some_and(|c| {
                    c.path.to_string_lossy() == call.arguments["path"].as_str().unwrap_or("")
                        && c.baseline_sha256 == call.arguments["baselineSha256"]
                        && c.replacement == call.arguments["replacement"]
                });
            if !allowed {
                return ExecutorResult::Completed(ToolResult {
                    tool_call_id: call.id.clone(),
                    ok: false,
                    output: None,
                    error: Some(ToolError {
                        code: "approval_required".into(),
                        message: "core approval is required".into(),
                    }),
                });
            }
            self.approvals.lock().unwrap().remove(digest);
        }
        if serde_json::to_vec(&invoke).unwrap().len() + 1
            > ready["maxMessageBytes"].as_u64().unwrap() as usize
        {
            return ExecutorResult::NotDispatched;
        }
        record.dispatched = true; // A partial write can already reach the child.
        if !Self::write(input, &invoke, deadline).await {
            return ExecutorResult::OutcomeUnknown;
        }
        let identity = |v: &Value| {
            v["protocol"] == extension02::PROTOCOL
                && ["sessionId", "requestId", "taskId", "callId"]
                    .iter()
                    .all(|k| v[*k] == invoke[*k])
        };
        let mut started = false;
        let mut cancelled = false;
        let mut end = deadline;
        let neutral = ControlSignal::new();
        let terminal = loop {
            let next = frames
                .controlled(if cancelled { &neutral } else { signal }, end)
                .await;
            match next {
                Err("cancelled") if !cancelled => {
                    cancelled = true;
                    end = Instant::now() + self.grace;
                    let cancel = json!({"protocol":extension02::PROTOCOL,"type":"cancel","sessionId":session,"requestId":record.request_id,"taskId":record.task_id,"callId":call.id});
                    if !Self::write(input, &cancel, end).await {
                        return ExecutorResult::OutcomeUnknown;
                    }
                }
                Ok(Some(v)) if v["type"] == "started" => {
                    if started
                        || !identity(&v)
                        || !keys(
                            &v,
                            &[
                                "protocol",
                                "type",
                                "sessionId",
                                "requestId",
                                "taskId",
                                "callId",
                            ],
                        )
                    {
                        return ExecutorResult::OutcomeUnknown;
                    }
                    started = true;
                    if let Some(callback) = &self.on_started {
                        callback()
                    }
                }
                Ok(Some(v)) if v["type"] == "terminal" => {
                    if !started || !identity(&v) {
                        return ExecutorResult::OutcomeUnknown;
                    }
                    break v;
                }
                _ => return ExecutorResult::OutcomeUnknown,
            }
        };
        let base = [
            "protocol",
            "type",
            "sessionId",
            "requestId",
            "taskId",
            "callId",
            "status",
        ];
        if terminal["status"] == "cancelled" {
            return if cancelled && keys(&terminal, &base) {
                ExecutorResult::Cancelled
            } else {
                ExecutorResult::OutcomeUnknown
            };
        }
        if cancelled || signal.is_cancelled() || Instant::now() >= deadline {
            return ExecutorResult::OutcomeUnknown;
        }
        if terminal["status"] == "tool_failed" {
            let mut fields = base.to_vec();
            fields.push("result");
            let r = &terminal["result"];
            if !keys(&terminal, &fields)
                || !keys(r, &["ok", "error"])
                || r["ok"] != false
                || !keys(&r["error"], &["code", "message"])
            {
                return ExecutorResult::OutcomeUnknown;
            }
            let Ok(error) = serde_json::from_value::<ToolError>(r["error"].clone()) else {
                return ExecutorResult::OutcomeUnknown;
            };
            if error.code.is_empty() {
                return ExecutorResult::OutcomeUnknown;
            }
            return ExecutorResult::Completed(ToolResult {
                tool_call_id: call.id.clone(),
                ok: false,
                output: None,
                error: Some(error),
            });
        }
        let mut fields = base.to_vec();
        fields.extend(["result", "evidence"]);
        if terminal["status"] != "succeeded"
            || !keys(&terminal, &fields)
            || !keys(&terminal["result"], &["ok", "output"])
            || terminal["result"]["ok"] != true
        {
            return ExecutorResult::OutcomeUnknown;
        }
        if call.name == extension02::RUN_VERIFICATION || call.name == extension02::APPLY_PATCH {
            let output = terminal["result"]["output"].as_str().unwrap_or("");
            if call.name == extension02::RUN_VERIFICATION {
                let Ok(v) = serde_json::from_str::<Value>(output) else {
                    return ExecutorResult::OutcomeUnknown;
                };
                if v["rule"] != call.arguments["rule"]
                    || v["target"] != call.arguments["target"]
                    || !v["ok"].is_boolean()
                {
                    return ExecutorResult::OutcomeUnknown;
                }
            }
            let sources = if call.name == extension02::RUN_VERIFICATION {
                workspace_snapshot(root).unwrap_or_else(|| json!([]))
            } else {
                serde_json::from_str::<Value>(output)
                    .ok()
                    .and_then(|v| {
                        v["postSha256"]
                            .as_str()
                            .map(|h| json!([{"path":call.arguments["path"],"sha256":h}]))
                    })
                    .unwrap_or_else(|| json!([]))
            };
            let evidence = json!({"taskId":record.task_id,"callId":call.id,"tool":call.name,"arguments":call.arguments,"sources":sources});
            if terminal["evidence"] != evidence || terminal["result"]["output"].as_str().is_none() {
                return ExecutorResult::OutcomeUnknown;
            }
            if call.name == extension02::RUN_VERIFICATION
                && (snapshot_before.as_ref() != Some(&sources)
                    || workspace_snapshot(root).as_ref() != Some(&sources))
            {
                return ExecutorResult::OutcomeUnknown;
            }
            if call.name == extension02::APPLY_PATCH {
                let value: Value = serde_json::from_str(output).unwrap_or(Value::Null);
                let path = call.arguments["path"].as_str().unwrap_or("");
                let replacement = call.arguments["replacement"].as_str().unwrap_or("");
                let actual = safe_file(root, path).and_then(|p| std::fs::read(p).ok());
                let actual_hash = actual.as_ref().map(|b| hash(b));
                if value["ok"] != true
                    || value["path"] != path
                    || actual.as_deref() != Some(replacement.as_bytes())
                    || value["postSha256"].as_str() != actual_hash.as_deref()
                {
                    return ExecutorResult::OutcomeUnknown;
                }
            }
            return ExecutorResult::Completed(ToolResult {
                tool_call_id: call.id.clone(),
                ok: true,
                output: terminal["result"]["output"].as_str().map(str::to_owned),
                error: None,
            });
        }
        let Some((expected, sources)) = before else {
            return ExecutorResult::OutcomeUnknown;
        };
        let evidence = json!({"taskId":record.task_id,"callId":call.id,"tool":call.name,"arguments":call.arguments,"sources":sources});
        if terminal["evidence"] != evidence
            || terminal["result"]["output"] != *expected
            || observe(root, call).as_ref() != before.as_ref()
        {
            return ExecutorResult::OutcomeUnknown;
        }
        ExecutorResult::Completed(ToolResult {
            tool_call_id: call.id.clone(),
            ok: true,
            output: Some(expected.clone()),
            error: None,
        })
    }
    async fn run(
        &self,
        call: &ToolCall,
        signal: &ControlSignal,
        deadline: Option<Instant>,
    ) -> ExecutorResult {
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return ExecutorResult::NotDispatched;
        }
        let _lease = Lease(self.busy.clone());
        let request_id = format!("request-{}", SEQUENCE.fetch_add(1, Ordering::Relaxed));
        let mut record = CallRecord {
            task_id: self.task_id.clone(),
            request_id,
            call_id: call.id.clone(),
            prepared: false,
            dispatched: false,
            terminal: None,
            reason: None,
            child_pid: None,
            exit_code: None,
            reaped: false,
        };
        let end = deadline.unwrap_or_else(|| Instant::now() + Duration::from_secs(5));
        let outcome = if signal.is_cancelled() || Instant::now() >= end {
            ExecutorResult::NotDispatched
        } else if let Some(root) = self.workspace.canonicalize().ok().filter(|p| p.is_dir()) {
            let maintenance_cfg = self.maintenance.lock().unwrap().clone();
            let valid = match call.name.as_str() {
                "read_file" => {
                    keys(&call.arguments, &["path"])
                        && call.arguments["path"]
                            .as_str()
                            .is_some_and(|s| !s.is_empty())
                }
                "search_files" => {
                    keys(&call.arguments, &["needle"])
                        && call.arguments["needle"]
                            .as_str()
                            .is_some_and(|s| !s.is_empty())
                }
                "run_verification" => {
                    keys(&call.arguments, &["target", "rule"])
                        && call.arguments["target"]
                            .as_str()
                            .is_some_and(|s| !s.is_empty())
                        && call.arguments["rule"]
                            .as_str()
                            .is_some_and(|s| !s.is_empty())
                }
                "apply_patch" => {
                    keys(
                        &call.arguments,
                        &["path", "baselineSha256", "replacement", "digest"],
                    ) && ["path", "baselineSha256", "replacement", "digest"]
                        .iter()
                        .all(|k| call.arguments[*k].as_str().is_some())
                }
                _ => false,
            } && !call.id.is_empty()
                && (call.name == "read_file"
                    || call.name == "search_files"
                    || maintenance_cfg.is_some());
            let valid = valid
                && match call.name.as_str() {
                    "run_verification" => maintenance_cfg.as_ref().is_some_and(|(target, rule)| {
                        call.arguments["rule"] == *rule && call.arguments["target"] == *target
                    }),
                    "apply_patch" => {
                        maintenance_cfg.as_ref().is_some_and(|(target, _)| {
                            target == call.arguments["path"].as_str().unwrap_or("")
                        }) && self
                            .approvals
                            .lock()
                            .unwrap()
                            .get(call.arguments["digest"].as_str().unwrap_or(""))
                            .is_some_and(|c| {
                                c.path.to_string_lossy()
                                    == call.arguments["path"].as_str().unwrap_or("")
                                    && c.baseline_sha256 == call.arguments["baselineSha256"]
                                    && c.replacement == call.arguments["replacement"]
                            })
                    }
                    _ => true,
                };
            if !valid {
                ExecutorResult::Completed(ToolResult {
                    tool_call_id: call.id.clone(),
                    ok: false,
                    output: None,
                    error: Some(ToolError {
                        code: if !matches!(
                            call.name.as_str(),
                            "read_file" | "search_files" | "run_verification" | "apply_patch"
                        ) {
                            "unknown_tool"
                        } else {
                            "arguments_invalid"
                        }
                        .into(),
                        message: "tool or arguments rejected before dispatch".into(),
                    }),
                })
            } else if call.name == "read_file"
                && safe_file(&root, call.arguments["path"].as_str().unwrap()).is_none()
            {
                ExecutorResult::Completed(ToolResult {
                    tool_call_id: call.id.clone(),
                    ok: false,
                    output: None,
                    error: Some(ToolError {
                        code: "path_invalid".into(),
                        message: "expected an existing in-workspace regular file without symlinks"
                            .into(),
                    }),
                })
            } else if call.name == extension02::APPLY_PATCH && {
                let approvals = self.approvals.lock().unwrap();
                let candidate = approvals.get(call.arguments["digest"].as_str().unwrap_or(""));
                let current = safe_file(&root, call.arguments["path"].as_str().unwrap_or(""))
                    .and_then(|p| std::fs::read(p).ok());
                candidate
                    .zip(current)
                    .is_some_and(|(c, bytes)| hash(&bytes) == c.baseline_sha256)
            } {
                record.prepared = true;
                self.exchange(call, signal, end, &root, &mut record).await
            } else if call.name == extension02::APPLY_PATCH {
                ExecutorResult::Completed(ToolResult {
                    tool_call_id: call.id.clone(),
                    ok: false,
                    output: None,
                    error: Some(ToolError {
                        code: "baseline_mismatch".into(),
                        message: "target changed before dispatch".into(),
                    }),
                })
            } else {
                record.prepared = true;
                self.exchange(call, signal, end, &root, &mut record).await
            }
        } else {
            ExecutorResult::NotDispatched
        };
        record.terminal = Some(
            match &outcome {
                ExecutorResult::Completed(r) if r.ok => "succeeded",
                ExecutorResult::Completed(_) => "tool_failed",
                ExecutorResult::Cancelled => "cancelled",
                ExecutorResult::NotDispatched => "not_dispatched",
                ExecutorResult::OutcomeUnknown => "outcome_unknown",
            }
            .into(),
        );
        record.reason = record.terminal.clone();
        self.records.lock().unwrap().push(record);
        outcome
    }
}
impl ToolExecutor for Extension02Executor {
    fn execute<'a>(
        &'a self,
        call: &'a ToolCall,
        signal: &'a ControlSignal,
        deadline: Option<Instant>,
    ) -> Pin<Box<dyn std::future::Future<Output = ExecutorResult> + Send + 'a>> {
        Box::pin(self.run(call, signal, deadline))
    }
}
