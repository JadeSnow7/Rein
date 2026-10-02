use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

pub mod context_methods;
pub mod extension02;
pub mod extension02_executor;
pub mod r#loop;
pub mod maintenance;
pub mod maintenance_session;
pub use maintenance_session::{
    run_maintenance_session, Observation, SessionReport, StopReason as SessionStopReason,
};
pub mod stdio_executor;
pub use extension02_executor::Extension02Executor;
pub use r#loop::{
    estimated_units, run_agent_loop, run_agent_loop_with_adapter, run_agent_loop_with_executor,
    run_agent_loop_with_options, ContextConfig, ContextMode, ControlSignal, ExecutorResult,
    LoopEvent, LoopOptions, LoopResult, LoopState, ModelAdapter, OpenAiModelAdapter, StopReason,
    ToolExecutor, ToolObserver,
};
pub use stdio_executor::StdioExecutor;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(rename = "toolCallId", skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(rename = "toolCalls", default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolResult {
    #[serde(rename = "toolCallId")]
    pub tool_call_id: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ToolError>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    pub messages: Vec<Message>,
    pub tool_calls: Vec<ToolCall>,
    pub tool_results: Vec<ToolResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelTurn {
    pub message: Message,
    #[serde(rename = "toolCalls")]
    pub tool_calls: Vec<ToolCall>,
}

pub trait OpenAiHttp: Send + Sync {
    fn post<'a>(
        &'a self,
        url: &'a str,
        api_key: &'a str,
        body: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<String, ToolError>> + Send + 'a>>;
}

pub struct ReqwestHttp {
    client: reqwest::Client,
    timeout: Duration,
}

impl ReqwestHttp {
    pub fn new(timeout: Duration) -> Result<Self, ToolError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|_| ToolError {
                code: "transport_config".into(),
                message: "could not configure HTTP client".into(),
            })?;
        Ok(Self { client, timeout })
    }
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl OpenAiHttp for ReqwestHttp {
    fn post<'a>(
        &'a self,
        url: &'a str,
        api_key: &'a str,
        body: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<String, ToolError>> + Send + 'a>> {
        Box::pin(async move {
            let response = self
                .client
                .post(url)
                .bearer_auth(api_key)
                .json(&body)
                .send()
                .await
                .map_err(|error| ToolError {
                    code: if error.is_timeout() {
                        "timeout"
                    } else {
                        "network"
                    }
                    .into(),
                    message: "HTTP request failed".into(),
                })?;
            let status = response.status();
            let text = response.text().await.map_err(|_| ToolError {
                code: "network".into(),
                message: "could not read HTTP response".into(),
            })?;
            if !status.is_success() {
                return Err(ToolError {
                    code: format!("http_{}", status.as_u16()),
                    message: "OpenAI HTTP request failed".into(),
                });
            }
            Ok(text)
        })
    }
}

pub async fn openai_complete<H: OpenAiHttp>(
    http: &H,
    base_url: &str,
    api_key: &str,
    model: &str,
    messages: &[Message],
    tools: &[ToolDefinition],
) -> Result<ModelTurn, ToolError> {
    let wire_messages: Vec<_> = messages.iter().map(|message| {
        if message.role == "tool" { serde_json::json!({"role":"tool","tool_call_id":message.tool_call_id,"content":message.content}) }
        else if message.role == "assistant" && !message.tool_calls.is_empty() { serde_json::json!({"role":"assistant","content":message.content,"tool_calls":message.tool_calls.iter().map(|call| serde_json::json!({"id":call.id,"type":"function","function":{"name":call.name,"arguments":call.arguments.to_string()}})).collect::<Vec<_>>()}) }
        else { serde_json::json!({"role":message.role,"content":message.content}) }
    }).collect();
    let body = serde_json::json!({ "model": model, "messages": wire_messages, "tools": tools.iter().map(|tool| serde_json::json!({"type":"function","function":{"name":tool.name,"description":tool.description,"parameters":tool.input_schema}})).collect::<Vec<_>>() });
    let response = http
        .post(
            &format!("{}/chat/completions", base_url.trim_end_matches('/')),
            api_key,
            body,
        )
        .await?;
    parse_openai_turn(&response)
}

pub fn parse_openai_turn(body: &str) -> Result<ModelTurn, ToolError> {
    let root: serde_json::Value = serde_json::from_str(body).map_err(|_| ToolError {
        code: "response_format".into(),
        message: "invalid JSON".into(),
    })?;
    let message = root
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.get("message"))
        .ok_or_else(|| ToolError {
            code: "response_format".into(),
            message: "missing message".into(),
        })?;
    let content = match message.get("content") {
        None | Some(serde_json::Value::Null) => String::new(),
        Some(value) => value
            .as_str()
            .ok_or_else(|| ToolError {
                code: "response_format".into(),
                message: "message content must be string or null".into(),
            })?
            .to_owned(),
    };
    if let Some(value) = message.get("tool_calls") {
        if !value.is_array() {
            return Err(ToolError {
                code: "response_format".into(),
                message: "tool_calls must be an array".into(),
            });
        }
    }
    let mut calls = Vec::new();
    for call in message
        .get("tool_calls")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        let id = call
            .get("id")
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ToolError {
                code: "response_format".into(),
                message: "tool call lacks id".into(),
            })?;
        let function = call.get("function").ok_or_else(|| ToolError {
            code: "response_format".into(),
            message: "missing tool function".into(),
        })?;
        let arguments: serde_json::Value = serde_json::from_str(
            function
                .get("arguments")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError {
                    code: "arguments_invalid".into(),
                    message: "tool arguments must be JSON text".into(),
                })?,
        )
        .map_err(|_| ToolError {
            code: "arguments_invalid".into(),
            message: "tool arguments are not JSON".into(),
        })?;
        if !arguments.is_object() {
            return Err(ToolError {
                code: "arguments_invalid".into(),
                message: "tool arguments must be an object".into(),
            });
        }
        let name = function
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| ToolError {
                code: "response_format".into(),
                message: "tool call lacks name".into(),
            })?;
        calls.push(ToolCall {
            id: id.into(),
            name: function
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(name)
                .into(),
            arguments,
        });
    }
    let message = Message {
        role: "assistant".into(),
        content,
        tool_call_id: None,
        tool_calls: calls.clone(),
    };
    Ok(ModelTurn {
        message,
        tool_calls: calls,
    })
}

pub fn anthropic_request(messages: &[Message], model: &str) -> serde_json::Value {
    let system = messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.clone());
    let mut filtered: Vec<serde_json::Value> = Vec::new();
    for m in messages.iter().filter(|m| m.role != "system") {
        if m.role == "tool" {
            let block = serde_json::json!({"type":"tool_result","tool_use_id":m.tool_call_id,"content":m.content});
            if filtered
                .last()
                .and_then(|v| v.get("role"))
                .and_then(|v| v.as_str())
                == Some("user")
                && filtered
                    .last()
                    .and_then(|v| v.get("content"))
                    .and_then(|v| v.as_array())
                    .is_some()
            {
                filtered.last_mut().unwrap()["content"]
                    .as_array_mut()
                    .unwrap()
                    .push(block);
            } else {
                filtered.push(serde_json::json!({"role":"user","content":[block]}));
            }
        } else if m.role == "assistant" && !m.tool_calls.is_empty() {
            let mut content = Vec::new();
            if !m.content.is_empty() {
                content.push(serde_json::json!({"type":"text","text":m.content}));
            }
            content.extend(m.tool_calls.iter().map(|c| serde_json::json!({"type":"tool_use","id":c.id,"name":c.name,"input":c.arguments})));
            filtered.push(serde_json::json!({"role":"assistant","content":content}));
        } else {
            filtered.push(serde_json::json!({"role": if m.role == "assistant" { "assistant" } else { "user" }, "content": m.content}));
        }
    }
    let mut request = serde_json::json!({"model": model, "max_tokens": 1024, "messages": filtered});
    if let Some(system) = system {
        request["system"] = system.into();
    }
    request
}

pub fn anthropic_response_to_message(body: &str) -> Result<Message, ToolError> {
    Ok(parse_anthropic_turn(body)?.message)
}

pub fn parse_anthropic_turn(body: &str) -> Result<ModelTurn, ToolError> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|_| ToolError {
        code: "response_format".into(),
        message: "invalid JSON".into(),
    })?;
    let blocks = value
        .get("content")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ToolError {
            code: "response_format".into(),
            message: "missing content".into(),
        })?;
    let content = blocks
        .iter()
        .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
        .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
        .collect::<Vec<_>>()
        .join("");
    let mut calls = Vec::new();
    for block in blocks
        .iter()
        .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
    {
        let id = block
            .get("id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError {
                code: "response_format".into(),
                message: "tool_use lacks id".into(),
            })?;
        let name = block
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError {
                code: "response_format".into(),
                message: "tool_use lacks name".into(),
            })?;
        let arguments = block
            .get("input")
            .filter(|v| v.is_object())
            .cloned()
            .ok_or_else(|| ToolError {
                code: "arguments_invalid".into(),
                message: "tool input must be an object".into(),
            })?;
        calls.push(ToolCall {
            id: id.into(),
            name: name.into(),
            arguments,
        });
    }
    for block in blocks {
        if block.get("type").and_then(|v| v.as_str()) == Some("text")
            && block.get("text").and_then(|v| v.as_str()).is_none()
        {
            return Err(ToolError {
                code: "response_format".into(),
                message: "text block is invalid".into(),
            });
        }
        if !matches!(
            block.get("type").and_then(|v| v.as_str()),
            Some("text") | Some("tool_use")
        ) {
            return Err(ToolError {
                code: "response_format".into(),
                message: "content block is invalid".into(),
            });
        }
    }
    if calls.is_empty() && content.is_empty() {
        return Err(ToolError {
            code: "response_format".into(),
            message: "Anthropic response lacks text content".into(),
        });
    }
    Ok(ModelTurn {
        message: Message {
            role: "assistant".into(),
            content,
            tool_call_id: None,
            tool_calls: calls.clone(),
        },
        tool_calls: calls,
    })
}

pub fn replay(responses: &[String], index: &mut usize) -> Result<ModelTurn, ToolError> {
    let body = responses.get(*index).ok_or_else(|| ToolError {
        code: "replay_exhausted".into(),
        message: "replay exhausted".into(),
    })?;
    *index += 1;
    parse_anthropic_turn(body)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub root: PathBuf,
}

fn safe_path(workspace: &Workspace, input: &str) -> Result<PathBuf, ToolError> {
    let root = workspace.root.canonicalize().map_err(|_| ToolError {
        code: "workspace_invalid".into(),
        message: "workspace unavailable".into(),
    })?;
    let mut components = Vec::new();
    for component in Path::new(input).components() {
        match component {
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err(ToolError {
                    code: "path_escape".into(),
                    message: "path escapes workspace".into(),
                });
            }
            std::path::Component::ParentDir => {
                if components.pop().is_none() {
                    return Err(ToolError {
                        code: "path_escape".into(),
                        message: "path escapes workspace".into(),
                    });
                }
            }
            std::path::Component::Normal(value) => components.push(value),
            std::path::Component::CurDir => {}
        }
    }
    let requested = root.join(input);
    let candidate = requested.canonicalize().map_err(|_| ToolError {
        code: "path_invalid".into(),
        message: "path unavailable".into(),
    })?;
    if !candidate.starts_with(&root) {
        return Err(ToolError {
            code: "path_escape".into(),
            message: "path escapes workspace".into(),
        });
    }
    Ok(candidate)
}

pub fn read_file(workspace: &Workspace, path: &str) -> ToolResult {
    match safe_path(workspace, path).and_then(|path| {
        std::fs::read_to_string(path).map_err(|_| ToolError {
            code: "read_failed".into(),
            message: "read failed".into(),
        })
    }) {
        Ok(output) => ToolResult {
            tool_call_id: "read_file".into(),
            ok: true,
            output: Some(output),
            error: None,
        },
        Err(error) => ToolResult {
            tool_call_id: "read_file".into(),
            ok: false,
            output: None,
            error: Some(error),
        },
    }
}

pub fn search_files(workspace: &Workspace, needle: &str) -> ToolResult {
    fn walk(root: &Path, dir: &Path, needle: &str, found: &mut Vec<String>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            if matches!(name.to_str(), Some(".git" | "target" | "node_modules")) {
                continue;
            }
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                walk(root, &path, needle, found)?;
            } else if path.is_file()
                && std::fs::read_to_string(&path)
                    .map(|s| s.contains(needle))
                    .unwrap_or(false)
            {
                found.push(
                    path.strip_prefix(root)
                        .unwrap_or(&path)
                        .display()
                        .to_string(),
                );
            }
        }
        Ok(())
    }
    let root = match workspace.root.canonicalize() {
        Ok(root) => root,
        Err(_) => {
            return ToolResult {
                tool_call_id: "search_files".into(),
                ok: false,
                output: None,
                error: Some(ToolError {
                    code: "workspace_invalid".into(),
                    message: "workspace unavailable".into(),
                }),
            }
        }
    };
    let mut found = Vec::new();
    match walk(&root, &root, needle, &mut found) {
        Ok(()) => {
            found.sort();
            ToolResult {
                tool_call_id: "search_files".into(),
                ok: true,
                output: Some(found.join("\n")),
                error: None,
            }
        }
        Err(_) => ToolResult {
            tool_call_id: "search_files".into(),
            ok: false,
            output: None,
            error: Some(ToolError {
                code: "search_failed".into(),
                message: "search failed".into(),
            }),
        },
    }
}

pub fn dispatch_readonly(call: &ToolCall, workspace: &Workspace) -> ToolResult {
    match call.name.as_str() {
        "read_file" => match call.arguments.get("path").and_then(|v| v.as_str()) {
            Some(path) => {
                let mut result = read_file(workspace, path);
                result.tool_call_id = call.id.clone();
                result
            }
            None => invalid_result(&call.id, "read_file requires string path"),
        },
        "search_files" => match call.arguments.get("needle").and_then(|v| v.as_str()) {
            Some(needle) => {
                let mut result = search_files(workspace, needle);
                result.tool_call_id = call.id.clone();
                result
            }
            None => invalid_result(&call.id, "search_files requires string needle"),
        },
        _ => ToolResult {
            tool_call_id: call.id.clone(),
            ok: false,
            output: None,
            error: Some(ToolError {
                code: "unknown_tool".into(),
                message: format!("unknown tool: {}", call.name),
            }),
        },
    }
}

pub use dispatch_readonly as execute_tool;
pub use dispatch_readonly as execute_tool_call;

fn invalid_result(id: &str, message: &str) -> ToolResult {
    ToolResult {
        tool_call_id: id.into(),
        ok: false,
        output: None,
        error: Some(ToolError {
            code: "arguments_invalid".into(),
            message: message.into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anthropic_round_trip_shape() {
        let input = vec![Message {
            role: "user".into(),
            content: "hi".into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }];
        assert_eq!(
            anthropic_request(&input, "claude-test")["model"],
            "claude-test"
        );
        assert_eq!(
            anthropic_response_to_message(r#"{"content":[{"type":"text","text":"hello"}]}"#)
                .unwrap()
                .content,
            "hello"
        );
    }
    #[test]
    fn read_file_rejects_escape() {
        let result = read_file(
            &Workspace {
                root: std::env::current_dir().unwrap(),
            },
            "../../outside",
        );
        assert!(!result.ok);
        assert!(result.error.is_some());
    }
}
