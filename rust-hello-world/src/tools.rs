//! The three read-only tools a model may request. Every request is checked here before it runs.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::{json, Map, Value};

use crate::error::{ErrorKind, HelloError, HelloResult};
use crate::process::{returncode, run_with_timeout};
use crate::workspace::{is_symlink, safe_read};

pub const MAX_BASH_OUTPUT: usize = 4096;
pub const BASH_TIMEOUT: Duration = Duration::from_secs(2);
/// Fixed inspection commands: the text the model sends, and the program actually started.
pub const BASH_COMMANDS: [(&str, &[&str]); 2] = [("pwd", &["/bin/pwd"]), ("ls -1", &["/bin/ls", "-1"])];

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

impl ToolCall {
    /// Accept the loose shape `{"id", "name", "arguments"}`; missing parts are rejected later.
    pub fn from_value(value: &Value) -> ToolCall {
        let text = |key: &str| value.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
        let arguments = value.get("arguments").cloned().unwrap_or_else(|| json!({}));
        ToolCall { id: text("id"), name: text("name"), arguments }
    }
}

fn invalid(detail: &str) -> HelloError {
    HelloError::new(ErrorKind::ToolInvalid, detail)
}

fn has_exactly(arguments: &Map<String, Value>, key: &str) -> bool {
    arguments.len() == 1 && arguments.contains_key(key)
}

pub fn dispatch_tool(workspace: &Path, call: &ToolCall) -> HelloResult<Value> {
    if !matches!(call.name.as_str(), "bash" | "read_file" | "read_environment") {
        return Err(HelloError::new(ErrorKind::ToolUnknown, "only bash, read_file and read_environment are available"));
    }
    if call.id.is_empty() {
        return Err(invalid("tool call id must be a non-empty string"));
    }
    let arguments = call.arguments.as_object().ok_or_else(|| invalid("tool arguments must be an object"))?;
    match call.name.as_str() {
        "read_environment" if arguments.is_empty() => Ok(environment_snapshot()),
        "read_environment" => Err(invalid("read_environment takes no arguments")),
        "bash" if has_exactly(arguments, "command") => run_bash(workspace, &arguments["command"]),
        "bash" => Err(invalid("bash requires exactly command")),
        _ if has_exactly(arguments, "path") => {
            let path = arguments["path"]
                .as_str()
                .ok_or_else(|| HelloError::new(ErrorKind::PathInvalid, "path must be a string"))?;
            let read = safe_read(workspace, path)?;
            Ok(json!({"path": read.path, "text": read.text, "digest": read.digest, "size": read.size}))
        }
        _ => Err(invalid("read_file requires exactly path")),
    }
}

pub fn run_bash(workspace: &Path, command: &Value) -> HelloResult<Value> {
    let text = command.as_str().unwrap_or_default();
    let program = BASH_COMMANDS
        .iter()
        .find(|(allowed, _)| *allowed == text)
        .map(|(_, program)| *program)
        .ok_or_else(|| HelloError::new(ErrorKind::BashCommandInvalid, "allowed commands are pwd and ls -1"))?;
    if is_symlink(workspace) || !workspace.is_dir() {
        return Err(HelloError::new(ErrorKind::PathInvalid, "workspace must be a regular directory"));
    }
    let mut process = Command::new(program[0]);
    process.args(&program[1..]).current_dir(workspace);
    let finished = run_with_timeout(&mut process, BASH_TIMEOUT)
        .map_err(|e| HelloError::new(ErrorKind::BashFailed, e.to_string()))?;
    let status = finished.status.ok_or_else(|| {
        HelloError::new(ErrorKind::BashTimeout, format!("command timed out after {}s", BASH_TIMEOUT.as_secs()))
    })?;
    if finished.stdout.len() > MAX_BASH_OUTPUT {
        return Err(HelloError::new(ErrorKind::BashOutputTooLarge, format!("limit is {MAX_BASH_OUTPUT} bytes")));
    }
    let utf8 =
        |bytes: Vec<u8>| String::from_utf8(bytes).map_err(|e| HelloError::new(ErrorKind::InvalidUtf8, e.to_string()));
    let stdout = utf8(finished.stdout)?;
    let stderr = utf8(finished.stderr)?;
    let code = returncode(status);
    if code != 0 {
        let detail = if stderr.trim().is_empty() { format!("exit code {code}") } else { stderr.trim().to_string() };
        return Err(HelloError::new(ErrorKind::BashFailed, detail));
    }
    Ok(json!({"command": text, "stdout": stdout, "stderr": stderr, "returncode": code}))
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?).map(|dir| dir.join(name)).find(|candidate| candidate.is_file())
}

/// Operating system, compiler path and compiler version. No environment variables or secrets.
pub fn environment_snapshot() -> Value {
    let compiler = find_on_path("c++");
    let version = compiler
        .as_ref()
        .and_then(|path| run_with_timeout(Command::new(path).arg("--version"), Duration::from_secs(5)).ok())
        .filter(|done| done.status.map(returncode) == Some(0))
        .and_then(|done| {
            let text = if done.stdout.is_empty() { done.stderr } else { done.stdout };
            String::from_utf8_lossy(&text).lines().next().map(str::to_string)
        })
        .unwrap_or_else(|| "unavailable".to_string());
    let compiler = compiler.map(|p| p.display().to_string()).unwrap_or_else(|| "unavailable".to_string());
    json!({
        "os": format!("{}-{}", env::consts::OS, env::consts::ARCH),
        "harness": "rust",
        "compiler": compiler,
        "compiler_version": version,
    })
}
