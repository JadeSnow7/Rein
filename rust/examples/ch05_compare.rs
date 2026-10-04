use rein_ch01_helloworld::rein::{
    run_agent_loop_with_adapter, Message, ModelAdapter, ModelTurn, ToolCall, ToolDefinition,
    ToolError, Workspace,
};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

struct Replay {
    turns: Arc<Mutex<Vec<ModelTurn>>>,
}
impl ModelAdapter for Replay {
    fn complete<'a>(
        &'a self,
        _: &'a [Message],
        _: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        let value = self.turns.lock().unwrap().remove(0);
        Box::pin(async move { Ok(value) })
    }
}
#[tokio::main]
async fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../fixtures/cases/ch05-loop.json".into());
    let source: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let fixture = source
        .get("cases")
        .and_then(|cases| {
            cases.get(
                std::env::args()
                    .nth(2)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0),
            )
        })
        .unwrap_or(&source);
    let root = std::env::temp_dir().join(format!("rein-ch05-compare-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    for (name, content) in fixture["workspaceFiles"].as_object().unwrap() {
        std::fs::write(root.join(name), content.as_str().unwrap()).unwrap();
    }
    let turns = fixture["modelTurns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|turn| {
            let calls: Vec<ToolCall> = turn["toolCalls"]
                .as_array()
                .unwrap()
                .iter()
                .map(|call| ToolCall {
                    id: call["id"].as_str().unwrap().into(),
                    name: call["name"].as_str().unwrap().into(),
                    arguments: call["arguments"].clone(),
                })
                .collect();
            ModelTurn {
                message: Message {
                    role: "assistant".into(),
                    content: turn["content"].as_str().unwrap().into(),
                    tool_call_id: None,
                    tool_calls: calls.clone(),
                },
                tool_calls: calls,
            }
        })
        .collect();
    let result = run_agent_loop_with_adapter(
        &Replay {
            turns: Arc::new(Mutex::new(turns)),
        },
        &Workspace { root: root.clone() },
        fixture["prompt"].as_str().unwrap(),
        32,
    )
    .await;
    let expected = &fixture["expected"];
    assert_eq!(
        serde_json::to_value(&result.state).unwrap(),
        expected["state"]
    );
    assert_eq!(
        serde_json::to_value(&result.reason).unwrap(),
        expected["reason"]
    );
    assert_eq!(result.answer.as_deref(), expected["answer"].as_str());
    assert!(result.events.iter().any(|event| matches!(
        event,
        rein_ch01_helloworld::rein::LoopEvent::ToolResult { .. }
    )));
    println!("{}", serde_json::to_string(&result).unwrap());
    let _ = std::fs::remove_dir_all(root);
}
