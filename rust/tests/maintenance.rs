#[path = "../src/rein/maintenance.rs"]
mod maintenance;
use maintenance::{apply, propose, MaintenanceError};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
static TEMP_SEQ: AtomicUsize = AtomicUsize::new(0);
#[test]
fn candidate_requires_fresh_approval_and_current_baseline() {
    let dir = tempfile_dir();
    fs::write(dir.join("README.md"), "old\n").unwrap();
    let c = propose(&dir, "README.md", "new\n".into()).unwrap();
    let forged = propose(&dir, "README.md", "forged\n".into()).unwrap();
    assert_eq!(
        apply(&dir, &c, &forged.approve()),
        Err(MaintenanceError::ApprovalMismatch)
    );
    fs::write(dir.join("README.md"), "changed\n").unwrap();
    assert_eq!(
        apply(&dir, &c, &c.approve()),
        Err(MaintenanceError::BaselineMismatch)
    );
}
#[test]
fn applies_exact_candidate_and_second_candidate_needs_approval() {
    let dir = tempfile_dir();
    fs::write(dir.join("README.md"), "old\n").unwrap();
    let first = propose(&dir, "README.md", "new\n".into()).unwrap();
    assert!(apply(&dir, &first, &first.approve()).is_ok());
    let second = propose(&dir, "README.md", "newer\n".into()).unwrap();
    assert_eq!(
        apply(&dir, &second, &first.approve()),
        Err(MaintenanceError::ApprovalMismatch)
    );
    assert!(apply(&dir, &second, &second.approve()).is_ok());
}
#[cfg(unix)]
#[test]
fn rejects_symlink_parent_and_preserves_unrelated_temp_file() {
    use std::os::unix::fs::symlink;
    let dir = tempfile_dir();
    let outside = tempfile_dir().join("outside");
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, dir.join("link")).unwrap();
    assert_eq!(
        propose(&dir, "link/file.md", "x".into()),
        Err(MaintenanceError::PathEscape)
    );
    fs::write(dir.join("README.md"), "old\n").unwrap();
    fs::write(dir.join(".rein-victim.tmp"), "victim").unwrap();
    let c = propose(&dir, "README.md", "new\n".into()).unwrap();
    assert!(apply(&dir, &c, &c.approve()).is_ok());
    assert_eq!(
        fs::read_to_string(dir.join(".rein-victim.tmp")).unwrap(),
        "victim"
    );
}
fn tempfile_dir() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "rein-maint-{}-{}-{}",
        std::process::id(),
        TEMP_SEQ.fetch_add(1, Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&p).expect("unique maintenance fixture directory");
    p
}
