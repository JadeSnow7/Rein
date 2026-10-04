//! The user's decision and the only place that writes hello.cpp.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{ErrorKind, HelloError, HelloResult};
use crate::workspace::safe_read;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Accept,
    Reject,
}

impl Decision {
    /// Only an explicit yes accepts; empty input, EOF and anything unexpected reject.
    pub fn from_answer(answer: &str) -> Decision {
        match answer.trim().to_lowercase().as_str() {
            "y" | "yes" | "接受" | "a" | "accept" => Decision::Accept,
            _ => Decision::Reject,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyOutcome {
    Rejected,
    Applied { backup: PathBuf },
}

pub fn apply_candidate(
    workspace: &Path,
    candidate: &str,
    expected_digest: &str,
    decision: Decision,
) -> HelloResult<ApplyOutcome> {
    if decision == Decision::Reject {
        return Ok(ApplyOutcome::Rejected); // rejecting never touches the workspace
    }
    let source = safe_read(workspace, "hello.cpp")?;
    if source.digest != expected_digest {
        return Err(HelloError::new(ErrorKind::SourceChanged, "hello.cpp changed while waiting for approval"));
    }
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or_default();
    let backup = workspace.join(format!("hello.cpp.bak-{nanos}"));
    let write_failed = |e: std::io::Error| HelloError::new(ErrorKind::WriteFailed, e.to_string());
    fs::write(&backup, source.text.as_bytes()).map_err(write_failed)?; // the bytes that were just checked
    fs::write(workspace.join("hello.cpp"), candidate).map_err(write_failed)?;
    Ok(ApplyOutcome::Applied { backup })
}
