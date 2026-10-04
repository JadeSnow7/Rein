//! Error categories shared with the Python version. The `code()` strings are the contract.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    PathInvalid,
    FileTooLarge,
    InvalidUtf8,
    ToolUnknown,
    ToolInvalid,
    BashCommandInvalid,
    BashTimeout,
    BashFailed,
    BashOutputTooLarge,
    ResponseInvalid,
    SourceChanged,
    WriteFailed,
    CompileTimeout,
    CompilerMissing,
}

impl ErrorKind {
    pub fn code(self) -> &'static str {
        match self {
            ErrorKind::PathInvalid => "path_invalid",
            ErrorKind::FileTooLarge => "file_too_large",
            ErrorKind::InvalidUtf8 => "invalid_utf8",
            ErrorKind::ToolUnknown => "tool_unknown",
            ErrorKind::ToolInvalid => "tool_invalid",
            ErrorKind::BashCommandInvalid => "bash_command_invalid",
            ErrorKind::BashTimeout => "bash_timeout",
            ErrorKind::BashFailed => "bash_failed",
            ErrorKind::BashOutputTooLarge => "bash_output_too_large",
            ErrorKind::ResponseInvalid => "response_invalid",
            ErrorKind::SourceChanged => "source_changed",
            ErrorKind::WriteFailed => "write_failed",
            ErrorKind::CompileTimeout => "compile_timeout",
            ErrorKind::CompilerMissing => "compiler_missing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelloError {
    pub kind: ErrorKind,
    pub detail: String,
}

impl HelloError {
    pub fn new(kind: ErrorKind, detail: impl Into<String>) -> Self {
        HelloError { kind, detail: detail.into() }
    }
}

impl fmt::Display for HelloError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.code(), self.detail)
    }
}

impl std::error::Error for HelloError {}

pub type HelloResult<T> = Result<T, HelloError>;
