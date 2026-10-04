use rein_ch01_helloworld::rein::{
    run_agent_loop_with_options, ControlSignal, LoopEvent, LoopOptions, Message, ModelAdapter,
    ModelTurn, ToolCall, ToolDefinition, ToolError, ToolObserver, ToolResult, Workspace,
};
use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

fn call(id: &str, name: &str, arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: name.into(),
        arguments,
    }
}
fn turn(content: &str, calls: Vec<ToolCall>) -> ModelTurn {
    ModelTurn {
        message: Message {
            role: "assistant".into(),
            content: content.into(),
            tool_call_id: None,
            tool_calls: calls.clone(),
        },
        tool_calls: calls,
    }
}

struct Replay {
    mode: String,
    requests: Arc<AtomicUsize>,
}
impl ModelAdapter for Replay {
    fn complete<'a>(
        &'a self,
        messages: &'a [Message],
        _: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        let _ = messages;
        Box::pin(async { Ok(turn("完成", vec![])) })
    }
    fn complete_controlled<'a>(
        &'a self,
        messages: &'a [Message],
        _: &'a [ToolDefinition],
        signal: &'a ControlSignal,
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        let mode = self.mode.clone();
        let requests = self.requests.clone();
        let signal = signal.clone();
        let tools: Vec<&Message> = messages.iter().filter(|m| m.role == "tool").collect();
        requests.fetch_add(1, Ordering::SeqCst);
        if mode == "cancel" {
            let cancel = signal.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                cancel.cancel();
            });
        }
        Box::pin(async move {
            if matches!(mode.as_str(), "cancel" | "timeout") {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            if mode == "cancel-at-return" {
                signal.cancel();
                return Ok(turn("迟到答案", vec![]));
            }
            if mode == "cancel" && signal.is_cancelled() {
                return Ok(turn("迟到答案", vec![]));
            }
            let calls = match mode.as_str() {
                "normal" if tools.is_empty() => vec![call(
                    "search-1",
                    "search_files",
                    serde_json::json!({"needle":"marker: ch06"}),
                )],
                "normal" if tools.len() == 1 => {
                    if tools[0].content.trim().is_empty() {
                        return Err(ToolError {
                            code: "no_matches".into(),
                            message: "no files match marker: ch06".into(),
                        });
                    }
                    if let Some(error) =
                        serde_json::from_str::<serde_json::Value>(&tools[0].content)
                            .ok()
                            .filter(|value| {
                                value.get("ok") == Some(&serde_json::Value::Bool(false))
                            })
                            .and_then(|value| value.get("error").cloned())
                    {
                        let _ = error;
                        return Err(ToolError {
                            code: "search_failed".into(),
                            message: "search failed".into(),
                        });
                    }
                    tools[0]
                        .content
                        .lines()
                        .enumerate()
                        .map(|(i, p)| {
                            call(
                                &format!("read-{}", i + 1),
                                "read_file",
                                serde_json::json!({"path":p}),
                            )
                        })
                        .collect()
                }
                "normal" => vec![],
                "budget" | "tool-zero" | "cancel-between-tools" => vec![
                    call("one", "read_file", serde_json::json!({"path":"README.md"})),
                    call("two", "read_file", serde_json::json!({"path":"guide.md"})),
                ],
                "duplicate" => vec![call(
                    if tools.is_empty() {
                        "duplicate-1"
                    } else {
                        "duplicate-2"
                    },
                    "read_file",
                    serde_json::json!({"path":"README.md"}),
                )],
                _ if tools.is_empty() && !matches!(mode.as_str(), "cancel" | "timeout") => {
                    vec![call(
                        "one",
                        "read_file",
                        serde_json::json!({"path":"README.md"}),
                    )]
                }
                _ => vec![],
            };
            let content = if mode == "normal" && tools.len() >= 2 {
                let values: Vec<String> = tools
                    .iter()
                    .skip(1)
                    .map(|m| m.content.trim().to_owned())
                    .collect();
                format!("完成：{}", values.join(" | "))
            } else if calls.is_empty() {
                "完成".into()
            } else {
                String::new()
            };
            Ok(turn(&content, calls))
        })
    }
}

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "normal".into());
    let allowed = [
        "normal",
        "budget",
        "duplicate",
        "zero",
        "tool-zero",
        "cancel-before",
        "cancel",
        "cancel-at-return",
        "cancel-between-tools",
        "timeout",
    ];
    if !allowed.contains(&mode.as_str()) {
        eprintln!("unknown ch06 mode: {mode}");
        std::process::exit(2);
    }
    let workspace_arg = args.next();
    let cleanup = workspace_arg.is_none();
    let root = workspace_arg.map(PathBuf::from).unwrap_or_else(|| {
        let p = std::env::temp_dir().join(format!("rein-ch06-rust-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("README.md"), "marker: ch06 alpha\n").unwrap();
        std::fs::write(p.join("guide.md"), "marker: ch06 beta\n").unwrap();
        p
    });
    let ws = Workspace { root: root.clone() };
    let requests = Arc::new(AtomicUsize::new(0));
    let signal = ControlSignal::new();
    if mode == "cancel-before" {
        signal.cancel();
    }
    let observer: Option<ToolObserver> = (mode == "cancel-between-tools").then(|| {
        Arc::new(|_: &ToolCall, _: &ToolResult, s: &ControlSignal| s.cancel()) as ToolObserver
    });
    let options = LoopOptions {
        max_turns: if mode == "zero" { 0 } else { 3 },
        max_tool_calls: match mode.as_str() {
            "budget" => 1,
            "tool-zero" => 0,
            _ => usize::MAX,
        },
        duplicate_limit: if mode == "duplicate" { 1 } else { usize::MAX },
        timeout: (mode == "timeout").then(|| Duration::from_millis(30)),
        signal: matches!(
            mode.as_str(),
            "cancel-before" | "cancel" | "cancel-at-return"
        )
        .then_some(signal.clone()),
        tool_observer: observer,
        context: None,
    };
    let result = run_agent_loop_with_options(
        &Replay {
            mode: mode.clone(),
            requests: requests.clone(),
        },
        &ws,
        "查找 marker: ch06 并读取两个文件",
        options,
    )
    .await;
    let requests_at_stop = requests.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(130));
    let settled = requests.load(Ordering::SeqCst);
    let (calls, skipped) = result
        .events
        .iter()
        .fold((Vec::new(), Vec::new()), |mut a, e| {
            match e {
                LoopEvent::ToolResult { call, .. } => a.0.push(call.id.clone()),
                LoopEvent::ActionSkipped {
                    call_id: Some(id),
                    reason,
                    ..
                } => a.1.push(serde_json::json!({"id":id,"reason":reason})),
                _ => {}
            }
            a
        });
    println!("{}", serde_json::to_string(&serde_json::json!({"mode":mode,"requests":requests_at_stop,"settledRequests":settled,"toolCalls":calls,"skipped":skipped,"result":result})).unwrap());
    if cleanup {
        let _ = std::fs::remove_dir_all(root);
    }
}
