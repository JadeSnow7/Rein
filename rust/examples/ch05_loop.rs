use rein_ch01_helloworld::{
    load_config,
    rein::{
        run_agent_loop, run_agent_loop_with_adapter, LoopState, Message, ModelAdapter, ModelTurn,
        ReqwestHttp, StopReason, ToolCall, ToolDefinition, ToolError, Workspace,
    },
};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

struct Replay {
    calls: Arc<Mutex<usize>>,
    single: bool,
    mode: String,
}
impl ModelAdapter for Replay {
    fn complete<'a>(
        &'a self,
        messages: &'a [Message],
        _: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        let mut turn = self.calls.lock().unwrap();
        *turn += 1;
        let n = *turn;
        let tool_messages: Vec<String> = messages
            .iter()
            .filter(|m| m.role == "tool")
            .map(|m| m.content.clone())
            .collect();
        let response = if self.mode == "empty" {
            ModelTurn {
                message: Message {
                    role: "assistant".into(),
                    content: "   ".into(),
                    tool_call_id: None,
                    tool_calls: vec![],
                },
                tool_calls: vec![],
            }
        } else if self.mode == "exhausted" && !tool_messages.is_empty() {
            return Box::pin(async {
                Err(ToolError {
                    code: "replay_exhausted".into(),
                    message: "replay exhausted".into(),
                })
            });
        } else if self.mode == "limit" {
            ModelTurn {
                message: Message {
                    role: "assistant".into(),
                    content: String::new(),
                    tool_call_id: None,
                    tool_calls: vec![ToolCall {
                        id: "limit-1".into(),
                        name: "unknown".into(),
                        arguments: serde_json::json!({}),
                    }],
                },
                tool_calls: vec![ToolCall {
                    id: "limit-1".into(),
                    name: "unknown".into(),
                    arguments: serde_json::json!({}),
                }],
            }
        } else if self.mode == "recovery" && tool_messages.is_empty() {
            ModelTurn {
                message: Message {
                    role: "assistant".into(),
                    content: String::new(),
                    tool_call_id: None,
                    tool_calls: vec![ToolCall {
                        id: "missing-1".into(),
                        name: "read_file".into(),
                        arguments: serde_json::json!({"path":"missing.md"}),
                    }],
                },
                tool_calls: vec![ToolCall {
                    id: "missing-1".into(),
                    name: "read_file".into(),
                    arguments: serde_json::json!({"path":"missing.md"}),
                }],
            }
        } else if self.mode == "recovery"
            && tool_messages
                .last()
                .is_some_and(|text| text.contains("path_invalid"))
        {
            ModelTurn {
                message: Message {
                    role: "assistant".into(),
                    content: String::new(),
                    tool_call_id: None,
                    tool_calls: vec![ToolCall {
                        id: "ok-1".into(),
                        name: "read_file".into(),
                        arguments: serde_json::json!({"path":"README.md"}),
                    }],
                },
                tool_calls: vec![ToolCall {
                    id: "ok-1".into(),
                    name: "read_file".into(),
                    arguments: serde_json::json!({"path":"README.md"}),
                }],
            }
        } else if self.mode == "recovery" {
            let text = tool_messages.last().cloned().unwrap_or_default();
            ModelTurn {
                message: Message {
                    role: "assistant".into(),
                    content: format!("恢复内容：{}", text.trim()),
                    tool_call_id: None,
                    tool_calls: vec![],
                },
                tool_calls: vec![],
            }
        } else if self.single && n == 1 {
            ModelTurn {
                message: Message {
                    role: "assistant".into(),
                    content: String::new(),
                    tool_call_id: None,
                    tool_calls: vec![ToolCall {
                        id: "read-1".into(),
                        name: "read_file".into(),
                        arguments: serde_json::json!({"path":"README.md"}),
                    }],
                },
                tool_calls: vec![ToolCall {
                    id: "read-1".into(),
                    name: "read_file".into(),
                    arguments: serde_json::json!({"path":"README.md"}),
                }],
            }
        } else if !self.single && n == 1 {
            ModelTurn {
                message: Message {
                    role: "assistant".into(),
                    content: String::new(),
                    tool_call_id: None,
                    tool_calls: vec![ToolCall {
                        id: "search-1".into(),
                        name: "search_files".into(),
                        arguments: serde_json::json!({"needle":"marker:"}),
                    }],
                },
                tool_calls: vec![ToolCall {
                    id: "search-1".into(),
                    name: "search_files".into(),
                    arguments: serde_json::json!({"needle":"marker:"}),
                }],
            }
        } else if !self.single && n == 2 {
            let paths: Vec<&str> = tool_messages
                .first()
                .map(|s| s.lines().take(2).collect())
                .unwrap_or_default();
            let calls: Vec<_> = paths
                .iter()
                .enumerate()
                .map(|(i, path)| ToolCall {
                    id: format!("read-{}", i + 1),
                    name: "read_file".into(),
                    arguments: serde_json::json!({"path": path}),
                })
                .collect();
            ModelTurn {
                message: Message {
                    role: "assistant".into(),
                    content: String::new(),
                    tool_call_id: None,
                    tool_calls: calls.clone(),
                },
                tool_calls: calls,
            }
        } else {
            let answer = if self.single {
                format!(
                    "单文件内容：{}",
                    tool_messages
                        .first()
                        .map(String::as_str)
                        .unwrap_or("")
                        .trim()
                )
            } else {
                format!(
                    "多文件摘要：{}",
                    tool_messages
                        .iter()
                        .skip(1)
                        .map(|s| s.trim())
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
            };
            ModelTurn {
                message: Message {
                    role: "assistant".into(),
                    content: answer,
                    tool_call_id: None,
                    tool_calls: vec![],
                },
                tool_calls: vec![],
            }
        };
        Box::pin(async move { Ok(response) })
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let live = args.iter().any(|arg| arg == "--live");
    let workspace_at = args
        .iter()
        .position(|arg| arg == "workspace")
        .map(|index| index + 1);
    let root = workspace_at
        .and_then(|index| args.get(index))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../fixtures/workspaces/prerequisites")
        });
    let single = workspace_at
        .and_then(|index| args.get(index + 1))
        .map(|mode| mode == "single")
        .unwrap_or_else(|| args.first().map(|mode| mode == "single").unwrap_or(false));
    let mode = workspace_at
        .and_then(|index| args.get(index + 1))
        .cloned()
        .unwrap_or_else(|| args.first().cloned().unwrap_or_else(|| "multi".into()));
    if live {
        let config = load_config().expect("REIN_BASE_URL/REIN_API_KEY/REIN_MODEL are required");
        let http = ReqwestHttp::new(std::time::Duration::from_secs(30)).expect("HTTP client");
        let prompt = args
            .iter()
            .skip(workspace_at.unwrap_or(0) + 1)
            .filter(|arg| *arg != "--live" && *arg != "single" && *arg != "multi")
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        let prompt = if prompt.trim().is_empty() {
            "请搜索 marker: 并读取命中文件"
        } else {
            prompt.as_str()
        };
        let result = run_agent_loop(
            &http,
            &config.base_url,
            &config.api_key,
            &config.model,
            &Workspace { root },
            prompt,
            4,
        )
        .await;
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
        std::process::exit(
            if result.state == LoopState::Completed && result.reason == StopReason::FinalAnswer {
                0
            } else {
                1
            },
        );
    }
    let result = run_agent_loop_with_adapter(
        &Replay {
            calls: Arc::new(Mutex::new(0)),
            single,
            mode: mode.clone(),
        },
        &Workspace { root },
        if single {
            "读取 README.md"
        } else {
            "先搜索 marker:，再读取命中的两个文件"
        },
        if mode == "limit" { 2 } else { 32 },
    )
    .await;
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
