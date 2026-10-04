use rein_ch01_helloworld::rein::{
    run_agent_loop_with_executor, ControlSignal, LoopOptions, Message, ModelAdapter, ModelTurn,
    StdioExecutor, ToolCall, ToolDefinition, ToolError, Workspace,
};
use std::{
    env,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
struct FixtureModel {
    calls: Arc<AtomicUsize>,
}
impl ModelAdapter for FixtureModel {
    fn complete<'a>(
        &'a self,
        messages: &'a [Message],
        _: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let has_tool = messages.iter().any(|m| m.role == "tool");
        Box::pin(async move {
            if n == 1 && !has_tool {
                let call = ToolCall {
                    id: "call-1".into(),
                    name: "read_file".into(),
                    arguments: serde_json::json!({"path":"fixtures/hybrid-marker.txt"}),
                };
                return Ok(ModelTurn {
                    message: Message {
                        role: "assistant".into(),
                        content: String::new(),
                        tool_call_id: None,
                        tool_calls: vec![call.clone()],
                    },
                    tool_calls: vec![call],
                });
            }
            if has_tool {
                let content = messages
                    .iter()
                    .rev()
                    .find(|m| m.role == "tool" && m.tool_call_id.as_deref() == Some("call-1"))
                    .map(|m| m.content.clone())
                    .unwrap_or_default();
                return Ok(ModelTurn {
                    message: Message {
                        role: "assistant".into(),
                        content: format!("fixture final: {content}"),
                        tool_call_id: None,
                        tool_calls: vec![],
                    },
                    tool_calls: vec![],
                });
            }
            Err(ToolError {
                code: "fixture_state".into(),
                message: "unexpected model state".into(),
            })
        })
    }
}
#[tokio::main]
async fn main() {
    let scenario = env::args().nth(1).unwrap_or_else(|| "normal".into());
    if !matches!(
        scenario.as_str(),
        "normal" | "invalid" | "cancel" | "crash" | "disconnect-after-effect"
    ) {
        eprintln!("unknown scenario: {scenario}");
        std::process::exit(2);
    }
    let configured_root = env::var("REIN_HYBRID_ROOT").unwrap_or_else(|_| ".".into());
    let root = if scenario == "normal" {
        configured_root.into()
    } else {
        let temp = env::temp_dir().join(format!("rein-hybrid-cli-{}", std::process::id()));
        tokio::fs::create_dir_all(temp.join("fixtures"))
            .await
            .unwrap();
        tokio::fs::write(
            temp.join("fixtures/hybrid-marker.txt"),
            format!("marker: {scenario}\n"),
        )
        .await
        .unwrap();
        temp
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let signal = ControlSignal::new();
    let mut executor = StdioExecutor::new(&root);
    if scenario != "normal" {
        executor.host_script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("ts/tests/fixtures/hybrid-fault-host.ts");
        executor.env.push((
            "REIN_HYBRID_FAULT_MODE".into(),
            match scenario.as_str() {
                "invalid" => "wrong_id",
                "cancel" => "cancellation_wait",
                "crash" => "crash_after_invoke",
                "disconnect-after-effect" => "disconnect_after_effect",
                _ => "normal",
            }
            .into(),
        ));
    }
    if scenario == "cancel" {
        let cancel = signal.clone();
        executor.on_started = Some(Arc::new(move || {
            let cancel = cancel.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                cancel.cancel();
            });
        }));
    }
    if scenario == "disconnect-after-effect" {
        executor.env.push((
            "REIN_HYBRID_LEDGER".into(),
            root.join("ledger").to_string_lossy().into_owned(),
        ));
    }
    let result = run_agent_loop_with_executor(
        &FixtureModel {
            calls: calls.clone(),
        },
        &Workspace {
            root: root.clone().into(),
        },
        "read fixture",
        LoopOptions {
            max_turns: 3,
            max_tool_calls: 1,
            signal: Some(signal),
            ..LoopOptions::default()
        },
        &executor,
    )
    .await;
    println!(
        "{}",
        serde_json::json!({"status":result.state,"reason":result.reason,"modelCalls":calls.load(Ordering::SeqCst),"answer":result.answer,"ledgerPath":if scenario == "disconnect-after-effect" { Some(root.join("ledger")) } else { None::<PathBuf> },"events":result.events,"records":executor.records()})
    );
    if result.answer.is_none() {
        std::process::exit(1);
    }
}
