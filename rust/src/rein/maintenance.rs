use ring::digest::{digest, SHA256};
use std::fs::{self, OpenOptions};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub(crate) path: PathBuf,
    pub(crate) baseline_sha256: String,
    pub(crate) replacement: String,
    pub(crate) digest: String,
    pub(crate) diff: String,
}
pub struct Approval(String);
impl Approval {
    pub(crate) fn matches(&self, digest: &str) -> bool {
        self.0 == digest
    }
}
impl Candidate {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn replacement(&self) -> &str {
        &self.replacement
    }
    pub fn diff(&self) -> &str {
        &self.diff
    }
    pub fn baseline_sha256(&self) -> &str {
        &self.baseline_sha256
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn approve(&self) -> Approval {
        Approval(self.digest.clone())
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum MaintenanceError {
    PathEscape,
    NotRegularFile,
    NoChange,
    BaselineMismatch,
    ApprovalMismatch,
    Io,
}
fn hash(text: &[u8]) -> String {
    digest(&SHA256, text)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn safe(root: &Path, path: &Path) -> Result<PathBuf, MaintenanceError> {
    if path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(MaintenanceError::PathEscape);
    }
    let canonical_root = fs::canonicalize(root).map_err(|_| MaintenanceError::Io)?;
    let mut cursor = canonical_root.clone();
    for component in path.components() {
        if let Component::Normal(name) = component {
            cursor.push(name);
            let meta = fs::symlink_metadata(&cursor).map_err(|_| MaintenanceError::Io)?;
            if meta.file_type().is_symlink() {
                return Err(MaintenanceError::PathEscape);
            }
        }
    }
    let target = canonical_root.join(path);
    let meta = fs::symlink_metadata(&target).map_err(|_| MaintenanceError::Io)?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(MaintenanceError::NotRegularFile);
    }
    Ok(target)
}
pub fn propose(
    root: &Path,
    path: impl AsRef<Path>,
    replacement: String,
) -> Result<Candidate, MaintenanceError> {
    let rel = path.as_ref();
    let target = safe(root, rel)?;
    let old = fs::read_to_string(&target).map_err(|_| MaintenanceError::Io)?;
    if old == replacement {
        return Err(MaintenanceError::NoChange);
    }
    let baseline_sha256 = hash(old.as_bytes());
    let digest =
        hash(format!("{}\0{}\0{}", rel.display(), baseline_sha256, replacement).as_bytes());
    let old_lines: Vec<_> = old.lines().collect();
    let new_lines: Vec<_> = replacement.lines().collect();
    let mut diff = format!("--- a/{0}\n+++ b/{0}\n", rel.display());
    for line in old_lines {
        diff.push('-');
        diff.push_str(line);
        diff.push('\n');
    }
    for line in new_lines {
        diff.push('+');
        diff.push_str(line);
        diff.push('\n');
    }
    Ok(Candidate {
        path: rel.to_owned(),
        baseline_sha256,
        replacement: replacement.clone(),
        digest,
        diff,
    })
}
pub fn apply(
    root: &Path,
    candidate: &Candidate,
    approval: &Approval,
) -> Result<String, MaintenanceError> {
    if approval.0 != candidate.digest {
        return Err(MaintenanceError::ApprovalMismatch);
    }
    let target = safe(root, &candidate.path)?;
    let old = fs::read_to_string(&target).map_err(|_| MaintenanceError::Io)?;
    if hash(old.as_bytes()) != candidate.baseline_sha256 {
        return Err(MaintenanceError::BaselineMismatch);
    }
    let metadata = fs::metadata(&target).map_err(|_| MaintenanceError::Io)?;
    let tmp = target.with_file_name(format!(
        ".rein-{}-{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let mut created = false;
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|_| MaintenanceError::Io)?;
        created = true;
        use std::io::Write;
        file.write_all(candidate.replacement.as_bytes())
            .map_err(|_| MaintenanceError::Io)?;
        fs::set_permissions(&tmp, metadata.permissions()).map_err(|_| MaintenanceError::Io)?;
        fs::rename(&tmp, &target).map_err(|_| MaintenanceError::Io)
    })();
    if result.is_err() {
        if created {
            let _ = fs::remove_file(&tmp);
        }
        return Err(MaintenanceError::Io);
    }
    Ok(hash(candidate.replacement.as_bytes()))
}
