use std::fs;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_rein-runtime")
}
fn fixture() -> String {
    format!(
        "{}/../fixtures/runtime-r1a/alpha.txt",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(bin()).args(args).output().unwrap()
}

#[test]
fn demo_show_duplicate_lifecycle_uses_durable_json() {
    let dir = tempfile::tempdir().unwrap();
    let first = run(&[
        "demo",
        "--state-dir",
        dir.path().to_str().unwrap(),
        "--fixture",
        &fixture(),
    ]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(json["session"]["status"], "Accepted");
    assert_eq!(json["acceptance"], "Passed");
    let show = run(&["show", "--state-dir", dir.path().to_str().unwrap()]);
    assert!(show.status.success());
    let before_db = fs::read(dir.path().join("state.sqlite")).unwrap();
    let before_workdir = fs::read(dir.path().join("workdir/fixture.txt")).unwrap();
    let duplicate = run(&[
        "demo",
        "--state-dir",
        dir.path().to_str().unwrap(),
        "--fixture",
        &fixture(),
    ]);
    assert!(!duplicate.status.success());
    assert_eq!(
        fs::read(dir.path().join("state.sqlite")).unwrap(),
        before_db
    );
    assert_eq!(
        fs::read(dir.path().join("workdir/fixture.txt")).unwrap(),
        before_workdir
    );
}

#[test]
fn prepare_then_cancel_changes_persisted_state() {
    let dir = tempfile::tempdir().unwrap();
    let prepared = run(&[
        "demo",
        "--state-dir",
        dir.path().to_str().unwrap(),
        "--fixture",
        &fixture(),
        "--prepare-only",
    ]);
    assert!(prepared.status.success());
    let cancelled = run(&["cancel", "--state-dir", dir.path().to_str().unwrap()]);
    assert!(cancelled.status.success());
    let value: serde_json::Value = serde_json::from_slice(&cancelled.stdout).unwrap();
    assert_eq!(value["session"]["status"], "Cancelled");
}

#[test]
fn prepare_resume_and_wrong_expected_are_reported_by_cli() {
    for (expected, acceptance) in [("alpha.txt", "Passed"), ("beta.txt", "Failed")] {
        let dir = tempfile::tempdir().unwrap();
        let prepared = run(&[
            "demo",
            "--state-dir",
            dir.path().to_str().unwrap(),
            "--fixture",
            &fixture(),
            "--expected",
            &format!(
                "{}/../fixtures/runtime-r1a/{expected}",
                env!("CARGO_MANIFEST_DIR")
            ),
            "--prepare-only",
        ]);
        assert!(
            prepared.status.success(),
            "{}",
            String::from_utf8_lossy(&prepared.stderr)
        );
        let resumed = run(&["resume", "--state-dir", dir.path().to_str().unwrap()]);
        assert!(
            resumed.status.success(),
            "{}",
            String::from_utf8_lossy(&resumed.stderr)
        );
        let json: serde_json::Value = serde_json::from_slice(&resumed.stdout).unwrap();
        assert_eq!(json["acceptance"], acceptance);
    }
}

#[test]
fn read_only_commands_do_not_create_missing_state() {
    for command in ["show", "cancel", "resume"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing");
        let output = run(&[command, "--state-dir", path.to_str().unwrap()]);
        assert!(!output.status.success(), "{command} unexpectedly succeeded");
        assert!(!path.exists(), "{command} created state directory");
    }
}

#[test]
fn invalid_budget_fails_before_state_creation() {
    let dir = tempfile::tempdir().unwrap();
    let output = run(&[
        "demo",
        "--state-dir",
        dir.path().join("state").to_str().unwrap(),
        "--fixture",
        &fixture(),
        "--budget",
        "-1",
    ]);
    assert!(!output.status.success());
    assert!(!dir.path().join("state").exists());
}

#[test]
fn unknown_and_trailing_flags_are_rejected_without_side_effects() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let output = run(&[
        "demo",
        "--state-dir",
        state.to_str().unwrap(),
        "--fixture",
        &fixture(),
        "--unknown",
    ]);
    assert!(!output.status.success());
    assert!(!state.exists());
}
