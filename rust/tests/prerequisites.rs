use rein_ch01_helloworld::rein::{
    anthropic_request, anthropic_response_to_message, dispatch_readonly, openai_complete, replay,
    Message, ReqwestHttp, ToolCall, Workspace,
};
use serde_json::Value;
use std::time::Duration;

const FIXTURE: &str = include_str!("../../fixtures/cases/prerequisites.json");

#[test]
fn shared_fixture_and_replay_are_structured() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(fixture["expected"]["assistant_text"], "hello");
    let mut index = 0;
    let responses = vec![fixture["anthropic_response"].to_string()];
    let turn = replay(&responses, &mut index).unwrap();
    assert_eq!(turn.message.content, "hello");
    assert_eq!(
        turn.tool_calls
            .iter()
            .map(|call| call.id.as_str())
            .collect::<Vec<_>>(),
        vec!["anthropic-1", "anthropic-2"]
    );
    assert_eq!(
        replay(&responses, &mut index).unwrap_err().code,
        "replay_exhausted"
    );
    assert!(anthropic_response_to_message("{}").is_err());
    let request = anthropic_request(
        &[
            Message {
                role: "system".into(),
                content: "rules".into(),
                tool_call_id: None,
                tool_calls: vec![],
            },
            Message {
                role: "assistant".into(),
                content: "hello".into(),
                tool_call_id: None,
                tool_calls: vec![
                    ToolCall {
                        id: "anthropic-1".into(),
                        name: "read_file".into(),
                        arguments: serde_json::json!({"path":"README.md"}),
                    },
                    ToolCall {
                        id: "anthropic-2".into(),
                        name: "search_files".into(),
                        arguments: serde_json::json!({"needle":"前置"}),
                    },
                ],
            },
            Message {
                role: "tool".into(),
                content: "one".into(),
                tool_call_id: Some("anthropic-1".into()),
                tool_calls: vec![],
            },
            Message {
                role: "tool".into(),
                content: "two".into(),
                tool_call_id: Some("anthropic-2".into()),
                tool_calls: vec![],
            },
        ],
        "claude",
    );
    assert_eq!(request["system"], "rules");
    assert_eq!(
        request["messages"][0]["content"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|block| block["type"] == "tool_use")
            .map(|block| block["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["anthropic-1", "anthropic-2"]
    );
    assert_eq!(
        request["messages"][1]["content"]
            .as_array()
            .unwrap()
            .iter()
            .map(|block| block["tool_use_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["anthropic-1", "anthropic-2"]
    );
}

#[test]
fn malformed_shared_fixture_responses_are_rejected_and_optional_json_is_canonical() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    for sample in fixture["malformed_responses"].as_array().unwrap() {
        let body = sample["body"].to_string();
        let result = if sample["provider"] == "openai" {
            rein_ch01_helloworld::rein::parse_openai_turn(&body).map(|_| ())
        } else {
            rein_ch01_helloworld::rein::parse_anthropic_turn(&body).map(|_| ())
        };
        assert!(
            result.is_err(),
            "malformed sample unexpectedly accepted: {sample}"
        );
    }
    let canonical = rein_ch01_helloworld::rein::Message {
        role: "user".into(),
        content: "canonical".into(),
        tool_call_id: None,
        tool_calls: vec![],
    };
    assert_eq!(
        serde_json::to_value(canonical).unwrap(),
        fixture["canonical_sample"]
    );
    let pure_tools =
        rein_ch01_helloworld::rein::parse_openai_turn(&fixture["openai_response"].to_string())
            .unwrap();
    assert_eq!(pure_tools.tool_calls.len(), 2);
}

#[test]
fn tools_use_real_fixture_and_bind_actual_ids() {
    let workspace = Workspace {
        root: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/workspaces/prerequisites"),
    };
    let result = dispatch_readonly(
        &ToolCall {
            id: "call-real".into(),
            name: "read_file".into(),
            arguments: serde_json::json!({"path":"README.md"}),
        },
        &workspace,
    );
    assert_eq!(result.tool_call_id, "call-real");
    assert!(result.output.unwrap().contains("前置 workspace"));
    let escaped = dispatch_readonly(
        &ToolCall {
            id: "escape".into(),
            name: "read_file".into(),
            arguments: serde_json::json!({"path":"../README.md"}),
        },
        &workspace,
    );
    assert_eq!(escaped.error.unwrap().code, "path_escape");
    let unknown = dispatch_readonly(
        &ToolCall {
            id: "unknown-id".into(),
            name: "nope".into(),
            arguments: serde_json::json!({}),
        },
        &workspace,
    );
    assert_eq!(unknown.tool_call_id, "unknown-id");
    assert_eq!(unknown.error.unwrap().code, "unknown_tool");
}

#[test]
fn symlink_escape_is_rejected() {
    let root = std::env::temp_dir().join(format!("rein-prereq-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir(&root).unwrap();
    std::os::unix::fs::symlink(
        std::env::current_dir().unwrap().join("README.md"),
        root.join("outside.txt"),
    )
    .unwrap();
    let result = dispatch_readonly(
        &ToolCall {
            id: "symlink".into(),
            name: "read_file".into(),
            arguments: serde_json::json!({"path":"outside.txt"}),
        },
        &Workspace { root: root.clone() },
    );
    assert_eq!(result.error.unwrap().code, "path_escape");
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn concrete_reqwest_captures_two_openai_turns_and_tool_results() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    let first_body = fixture["openai_response"].to_string();
    let second_body = fixture["openai_followup_response"].to_string();
    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/workspaces/prerequisites");
    let expected_read = std::fs::read_to_string(workspace_root.join("README.md")).unwrap();
    let expected_search = "README.md".to_string();
    let server = tokio::spawn(async move {
        for n in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0u8; 2048];
            loop {
                let count = socket.read(&mut chunk).await.unwrap();
                assert!(count > 0);
                request.extend_from_slice(&chunk[..count]);
                if request.windows(b"\r\n\r\n".len()).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let header_end = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|v| v.trim().parse::<usize>().ok())
                })
                .unwrap();
            while request.len() < header_end + length {
                let count = socket.read(&mut chunk).await.unwrap();
                request.extend_from_slice(&chunk[..count]);
            }
            let body = if n == 0 {
                first_body.as_str()
            } else {
                second_body.as_str()
            };
            if n == 1 {
                let sent: Value =
                    serde_json::from_slice(&request[header_end..header_end + length]).unwrap();
                let assistant = sent["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|m| m["role"] == "assistant")
                    .unwrap();
                assert_eq!(
                    assistant["tool_calls"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|c| c["id"].as_str().unwrap())
                        .collect::<Vec<_>>(),
                    vec!["call-1", "call-2"]
                );
                let tools: Vec<_> = sent["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|m| m["role"] == "tool")
                    .collect();
                assert_eq!(
                    tools
                        .iter()
                        .map(|m| m["tool_call_id"].as_str().unwrap())
                        .collect::<Vec<_>>(),
                    vec!["call-1", "call-2"]
                );
                assert_eq!(tools[0]["content"], expected_read);
                assert_eq!(tools[1]["content"], expected_search);
            }
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });
    let http = ReqwestHttp::new(Duration::from_secs(2)).unwrap();
    let first = openai_complete(
        &http,
        &format!("http://{address}"),
        "key",
        "model",
        &[Message {
            role: "user".into(),
            content: "inspect".into(),
            tool_call_id: None,
            tool_calls: vec![],
        }],
        &[],
    )
    .await
    .unwrap();
    assert_eq!(
        first
            .tool_calls
            .iter()
            .map(|call| call.id.as_str())
            .collect::<Vec<_>>(),
        vec!["call-1", "call-2"]
    );
    let workspace = Workspace {
        root: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/workspaces/prerequisites"),
    };
    let results: Vec<_> = first
        .tool_calls
        .iter()
        .map(|call| dispatch_readonly(call, &workspace))
        .collect();
    let messages = vec![
        Message {
            role: "user".into(),
            content: "inspect".into(),
            tool_call_id: None,
            tool_calls: vec![],
        },
        first.message.clone(),
        Message {
            role: "tool".into(),
            content: results[0].output.clone().unwrap(),
            tool_call_id: Some(results[0].tool_call_id.clone()),
            tool_calls: vec![],
        },
        Message {
            role: "tool".into(),
            content: results[1].output.clone().unwrap(),
            tool_call_id: Some(results[1].tool_call_id.clone()),
            tool_calls: vec![],
        },
    ];
    let second = openai_complete(
        &http,
        &format!("http://{address}"),
        "key",
        "model",
        &messages,
        &[],
    )
    .await
    .unwrap();
    assert_eq!(second.message.content, "done");
    server.await.unwrap();
}
