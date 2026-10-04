//! Stable, generic `rein-extension/0.2` wire contract.
//! The 0.1 read-file envelope remains in `stdio_executor` for historical examples.
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL: &str = "rein-extension/0.2";
pub const MAX_MESSAGE_BYTES: usize = 256 * 1024;
pub const READ_FILE: &str = "read_file";
pub const SEARCH_FILES: &str = "search_files";
pub const RUN_VERIFICATION: &str = "run_verification";
pub const APPLY_PATCH: &str = "apply_patch";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolDeclaration {
    pub name: String,
    pub description: String,
    pub arguments_schema: Value,
    pub result_schema: Value,
}

pub fn readonly_declarations() -> Vec<ToolDeclaration> {
    vec![
        ToolDeclaration {
            name: READ_FILE.into(),
            description: "Read one UTF-8 file in the workspace.".into(),
            arguments_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}),
            result_schema: result_schema(),
        },
        ToolDeclaration {
            name: SEARCH_FILES.into(),
            description: "Search text in workspace files.".into(),
            arguments_schema: serde_json::json!({"type":"object","properties":{"needle":{"type":"string"}},"required":["needle"],"additionalProperties":false}),
            result_schema: result_schema(),
        },
    ]
}

pub fn maintenance_declarations() -> Vec<ToolDeclaration> {
    let mut tools = readonly_declarations();
    tools.push(ToolDeclaration {
        name: RUN_VERIFICATION.into(),
        description: "Run one trusted document verification rule.".into(),
        arguments_schema: serde_json::json!({"type":"object","properties":{"target":{"type":"string"},"rule":{"type":"string"}},"required":["target","rule"],"additionalProperties":false}),
        result_schema: serde_json::json!({"type":"object","required":["ok","rule","target","output"],"properties":{"ok":{"type":"boolean"},"rule":{"type":"string"},"target":{"type":"string"},"output":{"type":"string"},"error":{"type":"object"}}}),
    });
    tools.push(ToolDeclaration {
        name: APPLY_PATCH.into(),
        description: "Apply one core-approved replacement to an existing UTF-8 file.".into(),
        arguments_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"baselineSha256":{"type":"string"},"replacement":{"type":"string"},"digest":{"type":"string"}},"required":["path","baselineSha256","replacement","digest"],"additionalProperties":false}),
        result_schema: serde_json::json!({"type":"object","required":["ok","path"],"properties":{"ok":{"type":"boolean"},"path":{"type":"string"},"postSha256":{"type":"string"},"error":{"type":"object"}}}),
    });
    tools
}

fn result_schema() -> Value {
    serde_json::json!({"type":"object","required":["ok"],"properties":{"ok":{"type":"boolean"},"output":{"type":"string"},"error":{"type":"object"}}})
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Invoke {
    pub protocol: String,
    pub r#type: String,
    pub session_id: String,
    pub request_id: String,
    pub task_id: String,
    pub call_id: String,
    pub tool: String,
    pub arguments: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_target: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Cancel {
    pub protocol: String,
    pub r#type: String,
    pub session_id: String,
    pub request_id: String,
    pub task_id: String,
    pub call_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Ready {
    pub protocol: String,
    pub r#type: String,
    pub tools: Vec<ToolDeclaration>,
    pub capabilities: Vec<String>,
    pub max_message_bytes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evidence {
    pub task_id: String,
    pub call_id: String,
    pub target: Option<String>,
}

/// Core-side validation keeps identity and protocol policy outside the host.
/// It deliberately does not validate tool-specific arguments: handlers own that schema.
pub fn validate_invoke(invoke: &Invoke) -> Result<(), ToolError> {
    if invoke.protocol != PROTOCOL || invoke.r#type != "invoke" {
        return Err(ToolError::new(
            "protocol_version",
            "unsupported extension protocol",
        ));
    }
    for (name, value) in [
        ("session_id", &invoke.session_id),
        ("request_id", &invoke.request_id),
        ("task_id", &invoke.task_id),
        ("call_id", &invoke.call_id),
        ("tool", &invoke.tool),
    ] {
        if value.is_empty() {
            return Err(ToolError::new("invalid_identity", name));
        }
    }
    if !invoke.arguments.is_object() {
        return Err(ToolError::new(
            "arguments_invalid",
            "arguments must be an object",
        ));
    }
    if !matches!(
        invoke.tool.as_str(),
        READ_FILE | SEARCH_FILES | RUN_VERIFICATION | APPLY_PATCH
    ) {
        return Err(ToolError::new("unknown_tool", "tool is not declared"));
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolError {
    pub code: String,
    pub message: String,
}
impl ToolError {
    fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declarations_only_expose_implemented_read_tools() {
        let tools = readonly_declarations();
        assert_eq!(
            tools
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>(),
            vec![READ_FILE, SEARCH_FILES]
        );
        assert!(tools
            .iter()
            .all(|tool| tool.arguments_schema.is_object() && tool.result_schema.is_object()));
    }
    #[test]
    fn validation_binds_generic_identity_and_version() {
        let invoke = Invoke {
            protocol: PROTOCOL.into(),
            r#type: "invoke".into(),
            session_id: "s".into(),
            request_id: "r".into(),
            task_id: "t".into(),
            call_id: "c".into(),
            tool: READ_FILE.into(),
            arguments: serde_json::json!({"path":"README.md"}),
            evidence_target: Some("sha256:x".into()),
        };
        assert!(validate_invoke(&invoke).is_ok());
        let mut invalid = invoke.clone();
        invalid.protocol = "rein-extension/0.1".into();
        assert_eq!(
            validate_invoke(&invalid).unwrap_err().code,
            "protocol_version"
        );
    }
}
