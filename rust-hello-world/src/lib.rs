//! Chapter 04: the Part 1 terminal assistant's model-independent behavior, moved to Rust.
//!
//! Model calls are not here yet; chapter 05 adds the message and tool protocol. The candidate
//! arrives as a `proposal.json` written by the Python `diagnose` command.

pub mod apply;
pub mod check;
pub mod error;
pub mod process;
pub mod review;
pub mod tools;
pub mod workspace;

pub use apply::{apply_candidate, ApplyOutcome, Decision};
pub use check::{check_cpp, CheckResult};
pub use error::{ErrorKind, HelloError, HelloResult};
pub use review::{render_review, unified_diff, Color};
pub use tools::{dispatch_tool, ToolCall};
pub use workspace::{safe_read, ReadResult};

pub const MAX_CANDIDATE_BYTES: usize = 16384;

/// A candidate read from `proposal.json`, checked the same way Python checks model output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub code: String,
    pub reason: String,
    pub source_digest: String,
}

impl Proposal {
    pub fn from_json(text: &str) -> HelloResult<Proposal> {
        let invalid = |detail: &str| HelloError::new(ErrorKind::ResponseInvalid, detail);
        let value: serde_json::Value = serde_json::from_str(text).map_err(|_| invalid("proposal is not JSON"))?;
        let field = |key: &str| value.get(key).and_then(serde_json::Value::as_str).unwrap_or_default().to_string();
        let proposal = Proposal { code: field("code"), reason: field("reason"), source_digest: field("source_digest") };
        if proposal.code.trim().is_empty() || proposal.reason.trim().is_empty() {
            return Err(invalid("candidate code and reason must be non-empty strings"));
        }
        if proposal.code.len() > MAX_CANDIDATE_BYTES {
            return Err(invalid("candidate code is too large"));
        }
        if proposal.source_digest.len() != 64 {
            return Err(invalid("proposal must carry the source digest"));
        }
        Ok(proposal)
    }
}

/// Where one edit run stopped. Errors such as `source_changed` travel in `HelloResult` instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOutcome {
    Rejected,
    Verified { backup: std::path::PathBuf, check: CheckResult },
}
