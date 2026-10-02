//! Bounded reads of the two files the exercise allows.

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{ErrorKind, HelloError, HelloResult};

pub const MAX_READ_BYTES: usize = 4096;
pub const ALLOWED_FILES: [&str; 2] = ["hello.cpp", "compiler.log"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadResult {
    pub path: String,
    pub text: String,
    pub digest: String,
    pub size: usize,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).map(|meta| meta.file_type().is_symlink()).unwrap_or(false)
}

/// Read `name` from the workspace only if it is one of the allowed names and a regular file.
pub fn safe_read(workspace: &Path, name: &str) -> HelloResult<ReadResult> {
    if !ALLOWED_FILES.contains(&name) {
        return Err(HelloError::new(ErrorKind::PathInvalid, "only hello.cpp and compiler.log are allowed"));
    }
    let target = workspace.join(name);
    if is_symlink(workspace) || is_symlink(&target) || !target.is_file() {
        return Err(HelloError::new(ErrorKind::PathInvalid, "target must be a regular file in workspace"));
    }
    let mut raw = Vec::new();
    let opened = File::open(&target).map_err(|e| HelloError::new(ErrorKind::PathInvalid, e.to_string()))?;
    opened
        .take(MAX_READ_BYTES as u64 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| HelloError::new(ErrorKind::PathInvalid, e.to_string()))?;
    if raw.len() > MAX_READ_BYTES {
        return Err(HelloError::new(ErrorKind::FileTooLarge, format!("limit is {MAX_READ_BYTES} bytes")));
    }
    let digest = sha256_hex(&raw);
    let size = raw.len();
    let text = String::from_utf8(raw).map_err(|e| HelloError::new(ErrorKind::InvalidUtf8, e.to_string()))?;
    Ok(ReadResult { path: name.to_string(), text, digest, size })
}
