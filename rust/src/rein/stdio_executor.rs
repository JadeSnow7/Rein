use super::{ControlSignal, ExecutorResult, ToolCall, ToolError, ToolExecutor, ToolResult};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    pin::Pin,
    process::Stdio,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
};

const VERSION: &str = "rein-extension/0.1";
const RULE: &str = "read-file-content-v1";
const MAX_LINE: usize = 256 * 1024;
static TASK_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

enum WaitControl {
    Cancelled,
    TimedOut,
}

struct FrameReader {
    reader: BufReader<tokio::process::ChildStdout>,
    pending: Vec<u8>,
}
impl FrameReader {
    fn new(stdout: tokio::process::ChildStdout) -> Self {
        Self {
            reader: BufReader::new(stdout),
            pending: Vec::new(),
        }
    }
    async fn next(&mut self) -> Result<Option<Wire>, ()> {
        loop {
            if let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
                if end > MAX_LINE {
                    return Err(());
                }
                let frame = self.pending.drain(..=end).collect::<Vec<_>>();
                let text = std::str::from_utf8(&frame[..end]).map_err(|_| ())?;
                return serde_json::from_str(text).map(Some).map_err(|_| ());
            }
            if self.pending.len() >= MAX_LINE {
                return Err(());
            }
            let chunk = self.reader.fill_buf().await.map_err(|_| ())?;
            if chunk.is_empty() {
                return if self.pending.is_empty() {
                    Ok(None)
                } else {
                    Err(())
                };
            }
            let room = MAX_LINE - self.pending.len();
            let take = chunk.len().min(room);
            self.pending.extend_from_slice(&chunk[..take]);
            self.reader.consume(take);
        }
    }
}

#[derive(Clone)]
pub struct StdioExecutor {
    pub workspace: PathBuf,
    pub node: String,
    task_id: Arc<String>,
    sequence: Arc<AtomicUsize>,
    pub host_script: PathBuf,
    pub code_root: PathBuf,
    pub env: Vec<(String, String)>,
    pub on_started: Option<Arc<dyn Fn() + Send + Sync>>,
    records: Arc<Mutex<Vec<CallRecord>>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CallRecord {
    pub task_id: String,
    pub request_id: String,
    pub call_id: String,
    pub prepared: bool,
    pub dispatched: bool,
    pub terminal: Option<String>,
    pub reason: Option<String>,
    pub child_pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub reaped: bool,
}
impl StdioExecutor {
    async fn wait_line(
        reader: &mut FrameReader,
        signal: &ControlSignal,
        deadline: Option<Instant>,
    ) -> Result<Option<Wire>, WaitControl> {
        let mut pending = Box::pin(reader.next());
        let mut tick = tokio::time::interval(Duration::from_millis(8));
        loop {
            tokio::select! {
                result = &mut pending => return result.map_err(|_| WaitControl::TimedOut),
                _ = tick.tick() => {
                    if signal.is_cancelled() { return Err(WaitControl::Cancelled); }
                    if deadline.is_some_and(|d| Instant::now() >= d) { return Err(WaitControl::TimedOut); }
                }
            }
        }
    }
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        let code_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        Self {
            workspace: workspace.into(),
            node: "node".into(),
            task_id: Arc::new(format!(
                "task-{}-{}-{}",
                std::process::id(),
                TASK_SEQUENCE.fetch_add(1, Ordering::Relaxed),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            )),
            sequence: Arc::new(AtomicUsize::new(0)),
            host_script: code_root.join("ts/src/hybrid-host.ts"),
            code_root,
            env: vec![],
            on_started: None,
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn records(&self) -> Vec<CallRecord> {
        self.records
            .lock()
            .map(|records| records.clone())
            .unwrap_or_default()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Invoke<'a> {
    protocol: &'static str,
    r#type: &'static str,
    session_id: &'a str,
    request_id: &'a str,
    task_id: &'a str,
    call_id: &'a str,
    tool: &'static str,
    path: &'a str,
    rule_id: &'static str,
    target_version: &'a str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Cancel<'a> {
    protocol: &'a str,
    r#type: &'a str,
    session_id: &'a str,
    request_id: &'a str,
    task_id: &'a str,
    call_id: &'a str,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
struct Wire {
    protocol: String,
    r#type: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    call_id: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    evidence: Option<Evidence>,
    #[serde(default)]
    error: Option<ToolError>,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    max_message_bytes: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
struct Evidence {
    task_id: String,
    call_id: String,
    path: String,
    rule_id: String,
    target_version: String,
}

impl ToolExecutor for StdioExecutor {
    fn execute<'a>(
        &'a self,
        call: &'a ToolCall,
        signal: &'a ControlSignal,
        deadline: Option<Instant>,
    ) -> Pin<Box<dyn std::future::Future<Output = ExecutorResult> + Send + 'a>> {
        Box::pin(async move { self.execute_one(call, signal, deadline).await })
    }
}
impl StdioExecutor {
    async fn wait_record(
        &self,
        child: &mut Child,
        index: usize,
        timeout: Duration,
    ) -> Option<std::process::ExitStatus> {
        let status = tokio::time::timeout(timeout, child.wait())
            .await
            .ok()?
            .ok()?;
        if let Ok(mut records) = self.records.lock() {
            if let Some(record) = records.get_mut(index) {
                record.reaped = true;
                record.exit_code = status.code();
            }
        }
        Some(status)
    }
    async fn cleanup(&self, child: &mut Child, index: usize) {
        if let Ok(Some(status)) = child.try_wait() {
            if let Ok(mut records) = self.records.lock() {
                if let Some(record) = records.get_mut(index) {
                    record.reaped = true;
                    record.exit_code = status.code();
                }
            }
            return;
        }
        let _ = child.start_kill();
        let _ = self.wait_record(child, index, Duration::from_secs(1)).await;
    }
    async fn write_bounded(
        stdin: &mut tokio::process::ChildStdin,
        data: &[u8],
        deadline: Instant,
    ) -> bool {
        tokio::time::timeout_at(
            tokio::time::Instant::from_std(deadline),
            stdin.write_all(data),
        )
        .await
        .is_ok_and(|result| result.is_ok())
    }
    async fn execute_one(
        &self,
        call: &ToolCall,
        signal: &ControlSignal,
        deadline: Option<Instant>,
    ) -> ExecutorResult {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let session_id = format!("session-{sequence}");
        let request_id = format!("request-{sequence}");
        let record_index = {
            let mut records = self.records.lock().expect("records mutex");
            records.push(CallRecord {
                task_id: self.task_id.as_ref().clone(),
                request_id: request_id.clone(),
                call_id: call.id.clone(),
                prepared: true,
                dispatched: false,
                terminal: None,
                reason: None,
                child_pid: None,
                exit_code: None,
                reaped: false,
            });
            records.len() - 1
        };
        let effective_deadline = deadline.or_else(|| Some(Instant::now() + Duration::from_secs(5)));
        let result = self
            .execute_inner(
                call,
                signal,
                effective_deadline,
                record_index,
                &session_id,
                &request_id,
            )
            .await;
        if let Ok(mut records) = self.records.lock() {
            if let Some(record) = records.get_mut(record_index) {
                record.terminal = Some(
                    match result {
                        ExecutorResult::Completed(ref r) if r.ok => "succeeded",
                        ExecutorResult::Completed(_) => "tool_failed",
                        ExecutorResult::Cancelled => "cancelled",
                        ExecutorResult::OutcomeUnknown => "unknown",
                        ExecutorResult::NotDispatched => "not_dispatched",
                    }
                    .into(),
                );
                record.reason = record.terminal.clone();
            }
        }
        result
    }
    async fn execute_inner(
        &self,
        call: &ToolCall,
        signal: &ControlSignal,
        deadline: Option<Instant>,
        record_index: usize,
        session_id: &str,
        request_id: &str,
    ) -> ExecutorResult {
        if signal.is_cancelled() {
            return ExecutorResult::NotDispatched;
        }
        if call.name != "read_file"
            || !call
                .arguments
                .get("path")
                .and_then(|v| v.as_str())
                .is_some()
        {
            return ExecutorResult::Completed(ToolResult {
                tool_call_id: call.id.clone(),
                ok: false,
                output: None,
                error: Some(ToolError {
                    code: "permission_denied".into(),
                    message: "read_file only".into(),
                }),
            });
        }
        let path = call.arguments["path"].as_str().unwrap();
        if path.is_empty()
            || std::path::Path::new(path).is_absolute()
            || path.split('/').any(|part| part == "..")
        {
            return ExecutorResult::Completed(ToolResult {
                tool_call_id: call.id.clone(),
                ok: false,
                output: None,
                error: Some(ToolError {
                    code: "path_escape".into(),
                    message: "path must be workspace relative".into(),
                }),
            });
        }
        let workspace = match tokio::fs::canonicalize(&self.workspace).await {
            Ok(v) => v,
            Err(_) => return ExecutorResult::NotDispatched,
        };
        let target = match tokio::fs::canonicalize(workspace.join(path)).await {
            Ok(v) if v.starts_with(&workspace) => v,
            _ => {
                return ExecutorResult::Completed(ToolResult {
                    tool_call_id: call.id.clone(),
                    ok: false,
                    output: None,
                    error: Some(ToolError {
                        code: "path_escape".into(),
                        message: "path escapes workspace".into(),
                    }),
                })
            }
        };
        let bytes = match tokio::fs::read(&target).await {
            Ok(v) => v,
            Err(_) => return ExecutorResult::NotDispatched,
        };
        let version = ring::digest::digest(&ring::digest::SHA256, &bytes)
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if deadline.is_some_and(|d| Instant::now() >= d) {
            return ExecutorResult::NotDispatched;
        }
        let mut command = Command::new(&self.node);
        command
            .args([
                "--import",
                "tsx",
                self.host_script.to_str().unwrap_or("ts/src/hybrid-host.ts"),
            ])
            .current_dir(&self.code_root);
        for (key, value) in &self.env {
            command.env(key, value);
        }
        let mut child = match command
            .env("REIN_HYBRID_WORKSPACE", &workspace)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(v) => v,
            Err(_) => return ExecutorResult::NotDispatched,
        };
        if let Ok(mut records) = self.records.lock() {
            if let Some(record) = records.get_mut(record_index) {
                record.child_pid = child.id();
            }
        }
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut reader = FrameReader::new(stdout);
        let ready = match Self::wait_line(&mut reader, signal, deadline).await {
            Ok(Some(v)) => match v {
                v if v.protocol == VERSION
                    && v.r#type == "ready"
                    && v.capabilities.iter().any(|x| x == "read_file") =>
                {
                    v
                }
                _ => {
                    self.cleanup(&mut child, record_index).await;
                    return ExecutorResult::NotDispatched;
                }
            },
            Err(WaitControl::Cancelled) => {
                self.cleanup(&mut child, record_index).await;
                return ExecutorResult::NotDispatched;
            }
            Err(WaitControl::TimedOut) | Ok(None) => {
                self.cleanup(&mut child, record_index).await;
                return ExecutorResult::NotDispatched;
            }
        };
        if ready.protocol != VERSION
            || ready.r#type != "ready"
            || !ready.capabilities.iter().any(|x| x == "read_file")
            || ready.session_id.is_some()
            || ready.request_id.is_some()
            || ready.task_id.is_some()
            || ready.call_id.is_some()
            || ready.status.is_some()
            || ready.output.is_some()
            || ready.evidence.is_some()
            || ready.error.is_some()
            || !ready
                .capabilities
                .iter()
                .all(|capability| capability == "read_file")
            || ready.max_message_bytes.is_none()
            || ready.max_message_bytes == Some(0)
        {
            self.cleanup(&mut child, record_index).await;
            return ExecutorResult::NotDispatched;
        }
        let task_id = self.task_id.as_str();
        let invoke = serde_json::to_string(&Invoke {
            protocol: VERSION,
            r#type: "invoke",
            session_id: &session_id,
            request_id: &request_id,
            task_id: &task_id,
            call_id: &call.id,
            tool: "read_file",
            path,
            rule_id: RULE,
            target_version: &version,
        })
        .unwrap()
            + "\n";
        if signal.is_cancelled() || deadline.is_some_and(|d| Instant::now() >= d) {
            self.cleanup(&mut child, record_index).await;
            return ExecutorResult::NotDispatched;
        }
        if invoke.len() > MAX_LINE
            || ready
                .max_message_bytes
                .is_some_and(|limit| invoke.len() > limit)
        {
            self.cleanup(&mut child, record_index).await;
            return ExecutorResult::NotDispatched;
        }
        if let Ok(mut records) = self.records.lock() {
            if let Some(record) = records.get_mut(record_index) {
                record.dispatched = true;
                record.reason = Some("invoke_write_started".into());
            }
        }
        if !Self::write_bounded(
            &mut stdin,
            invoke.as_bytes(),
            deadline.unwrap_or_else(|| Instant::now() + Duration::from_secs(5)),
        )
        .await
        {
            self.cleanup(&mut child, record_index).await;
            return ExecutorResult::OutcomeUnknown;
        }
        let mut started_seen = false;
        let mut cancel_sent = false;
        let mut grace = false;
        let mut grace_deadline = None;
        let neutral = ControlSignal::new();
        let response = loop {
            let wait_signal = if grace { &neutral } else { signal };
            let wait_deadline = if grace { grace_deadline } else { deadline };
            match Self::wait_line(&mut reader, wait_signal, wait_deadline).await {
                Ok(Some(message)) if message.r#type == "started" => {
                    if started_seen
                        || message.protocol != VERSION
                        || message.session_id.as_deref() != Some(session_id)
                        || message.request_id.as_deref() != Some(request_id)
                        || message.task_id.as_deref() != Some(task_id)
                        || message.call_id.as_deref() != Some(call.id.as_str())
                        || message.status.is_some()
                        || message.output.is_some()
                        || message.evidence.is_some()
                        || message.error.is_some()
                        || !message.capabilities.is_empty()
                        || message.max_message_bytes.is_some()
                    {
                        self.cleanup(&mut child, record_index).await;
                        return ExecutorResult::OutcomeUnknown;
                    }
                    started_seen = true;
                    if let Some(callback) = &self.on_started {
                        callback();
                    }
                    continue;
                }
                Ok(Some(message)) if message.r#type == "terminal" => break message,
                Ok(Some(_)) => {
                    self.cleanup(&mut child, record_index).await;
                    return ExecutorResult::OutcomeUnknown;
                }
                Err(WaitControl::Cancelled) if !cancel_sent => {
                    let cancel = serde_json::to_string(&Cancel {
                        protocol: VERSION,
                        r#type: "cancel",
                        session_id: &session_id,
                        request_id: &request_id,
                        task_id: &task_id,
                        call_id: &call.id,
                    })
                    .unwrap()
                        + "\n";
                    if !Self::write_bounded(
                        &mut stdin,
                        cancel.as_bytes(),
                        Instant::now() + Duration::from_secs(1),
                    )
                    .await
                    {
                        self.cleanup(&mut child, record_index).await;
                        return ExecutorResult::OutcomeUnknown;
                    }
                    cancel_sent = true;
                    grace = true;
                    grace_deadline = Some(Instant::now() + Duration::from_secs(1));
                }
                Err(_) | Ok(None) => {
                    self.cleanup(&mut child, record_index).await;
                    return ExecutorResult::OutcomeUnknown;
                }
            }
        };
        if !started_seen
            || response.protocol != VERSION
            || response.r#type != "terminal"
            || response.session_id.as_deref() != Some(session_id)
            || response.request_id.as_deref() != Some(request_id)
            || response.task_id.as_deref() != Some(task_id)
            || response.call_id.as_deref() != Some(call.id.as_str())
            || !response.capabilities.is_empty()
            || response.max_message_bytes.is_some()
        {
            self.cleanup(&mut child, record_index).await;
            return ExecutorResult::OutcomeUnknown;
        }
        let duplicate = match tokio::time::timeout(Duration::from_secs(1), reader.next()).await {
            Ok(Ok(None)) => false,
            _ => true,
        };
        let exited = self
            .wait_record(&mut child, record_index, Duration::from_secs(1))
            .await
            .map(|s| s.success())
            .unwrap_or(false);
        if duplicate || !exited || response.session_id.as_deref() != Some(session_id) {
            self.cleanup(&mut child, record_index).await;
            return ExecutorResult::OutcomeUnknown;
        }
        if response.status.as_deref() == Some("cancelled") {
            if response.output.is_some() || response.error.is_some() || response.evidence.is_some()
            {
                return ExecutorResult::OutcomeUnknown;
            }
            if cancel_sent {
                return ExecutorResult::Cancelled;
            }
            return ExecutorResult::OutcomeUnknown;
        }
        if response.status.as_deref() == Some("tool_failed") {
            if response.error.is_none() || response.output.is_some() || response.evidence.is_some()
            {
                return ExecutorResult::OutcomeUnknown;
            }
            return ExecutorResult::Completed(ToolResult {
                tool_call_id: call.id.clone(),
                ok: false,
                output: None,
                error: response.error,
            });
        }
        if response.status.as_deref() != Some("succeeded") || response.error.is_some() {
            return ExecutorResult::OutcomeUnknown;
        }
        let Some(evidence) = response.evidence else {
            return ExecutorResult::OutcomeUnknown;
        };
        if evidence.task_id != task_id
            || evidence.call_id != call.id
            || evidence.path != path
            || evidence.rule_id != RULE
            || evidence.target_version != version
        {
            return ExecutorResult::OutcomeUnknown;
        }
        if response.output.is_none()
            || response.output.as_deref() != std::str::from_utf8(&bytes).ok()
        {
            return ExecutorResult::OutcomeUnknown;
        }
        let current_target = match tokio::fs::canonicalize(workspace.join(path)).await {
            Ok(v) if v.starts_with(&workspace) => v,
            _ => return ExecutorResult::OutcomeUnknown,
        };
        let current = match tokio::fs::read(current_target).await {
            Ok(v) => v,
            Err(_) => return ExecutorResult::OutcomeUnknown,
        };
        let current_version = ring::digest::digest(&ring::digest::SHA256, &current)
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        if current_version != version
            || response.output.as_deref() != std::str::from_utf8(&current).ok()
        {
            return ExecutorResult::OutcomeUnknown;
        }
        ExecutorResult::Completed(ToolResult {
            tool_call_id: call.id.clone(),
            ok: true,
            output: response.output,
            error: None,
        })
    }
}
