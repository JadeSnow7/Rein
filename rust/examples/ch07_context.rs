use rein_ch01_helloworld::rein::{
    estimated_units, run_agent_loop_with_options, ContextConfig, LoopOptions, Message,
    ModelAdapter, ModelTurn, ToolDefinition, ToolError, Workspace,
};
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

#[derive(Deserialize)]
struct Input {
    unit: String,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    id: String,
    rules: Vec<String>,
    goal: String,
    history: Vec<Message>,
    budget: usize,
    managed: bool,
}
#[derive(Serialize)]
struct Output {
    unit: String,
    cases: Vec<CaseOutput>,
}
#[derive(Serialize)]
struct CaseOutput {
    id: String,
    result: rein_ch01_helloworld::rein::LoopResult,
    requests: usize,
    #[serde(rename = "sentMessages")]
    sent_messages: Vec<Vec<Message>>,
}

struct Offline {
    requests: Arc<Mutex<Vec<Vec<Message>>>>,
    budget: usize,
}
impl ModelAdapter for Offline {
    fn complete<'a>(
        &'a self,
        messages: &'a [Message],
        _: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        let snapshot = messages.to_vec();
        let requests = self.requests.clone();
        let budget = self.budget;
        Box::pin(async move {
            requests.lock().unwrap().push(snapshot.clone());
            if estimated_units(&snapshot) > budget {
                return Err(ToolError {
                    code: "context_budget_exhausted".into(),
                    message: "offline adapter budget exceeded".into(),
                });
            }
            let tool_messages: Vec<&Message> = snapshot
                .iter()
                .rev()
                .find(|m| m.role == "assistant" && !m.tool_calls.is_empty())
                .map(|assistant| {
                    assistant
                        .tool_calls
                        .iter()
                        .filter_map(|call| {
                            snapshot.iter().find(|m| {
                                m.role == "tool"
                                    && m.tool_call_id.as_deref() == Some(call.id.as_str())
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let answer = if tool_messages.is_empty() {
                snapshot
                    .iter()
                    .rev()
                    .find(|m| m.role == "user")
                    .map(|m| m.content.clone())
                    .unwrap_or_default()
            } else {
                tool_messages
                    .iter()
                    .map(|m| m.content.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            Ok(ModelTurn {
                message: Message {
                    role: "assistant".into(),
                    content: answer,
                    tool_call_id: None,
                    tool_calls: vec![],
                },
                tool_calls: vec![],
            })
        })
    }
}

#[tokio::main]
async fn main() {
    let path = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../fixtures/cases/ch07-context.json")
        });
    let input: Input =
        serde_json::from_str(&std::fs::read_to_string(path).expect("read case file"))
            .expect("valid case schema");
    let mut cases = Vec::new();
    for case in input.cases {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let adapter = Offline {
            requests: requests.clone(),
            budget: case.budget,
        };
        let result = run_agent_loop_with_options(
            &adapter,
            &Workspace {
                root: std::env::temp_dir(),
            },
            &case.goal,
            LoopOptions {
                max_turns: 2,
                context: Some(ContextConfig {
                    rules: case.rules,
                    history: case.history,
                    budget: case.budget,
                    manage: case.managed,
                }),
                ..LoopOptions::default()
            },
        )
        .await;
        let sent = requests.lock().unwrap().clone();
        cases.push(CaseOutput {
            id: case.id,
            result,
            requests: sent.len(),
            sent_messages: sent,
        });
    }
    println!(
        "{}",
        serde_json::to_string(&Output {
            unit: input.unit,
            cases
        })
        .unwrap()
    );
}
