//! Frozen R1a B01-B10 acceptance tests.
//!
//! API choice for this baseline: `rein_runtime::Runtime` owns the public driver
//! boundary; `rein_runtime::SqliteStore` and `rein_runtime::ArtifactStore` expose
//! only durable operations and references; `rein_core::HarnessStep` is the pure
//! transition boundary. Tests inspect durable rows and bytes after invoking those
//! APIs. No test-only report flags or self-reported outcome fields are accepted.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;

use rein_core::harness::Observation;
use rein_core::{ArtifactRef, ObservationKind, VerificationStatus as CoreVerificationStatus};
use rein_runtime::model::OfflineModel;
use rein_runtime::store::{EffectState, SqliteStore};
use rein_runtime::verify::{FixedVerifier, VerificationStatus};
use rein_runtime::{ReadFileTool, Runtime, RuntimeConfig};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/runtime-r1a")
        .join(name)
}

fn runtime(name: &str) -> (tempfile::TempDir, Runtime) {
    let dir = tempfile::tempdir().expect("temporary R1a state directory");
    let config = RuntimeConfig::for_fixture(dir.path(), fixture(name));
    let runtime = Runtime::open(
        config,
        OfflineModel::reads_named_fixture(),
        ReadFileTool::rooted_read_only(),
        FixedVerifier::expected_file(fixture(name)),
    )
    .expect("runtime opens");
    (dir, runtime)
}

fn runtime_with_verifier(name: &str, verifier: FixedVerifier) -> (tempfile::TempDir, Runtime) {
    let dir = tempfile::tempdir().expect("temporary R1a state directory");
    let config = RuntimeConfig::for_fixture(dir.path(), fixture(name));
    let runtime = Runtime::open(
        config,
        OfflineModel::reads_named_fixture(),
        ReadFileTool::rooted_read_only(),
        verifier,
    )
    .expect("runtime opens");
    (dir, runtime)
}

#[test]
fn b01_feedback_comes_from_actual_alpha_and_beta_artifacts() {
    let (_alpha_dir, mut alpha) = runtime("alpha.txt");
    let (_beta_dir, mut beta) = runtime("beta.txt");
    let alpha_result = alpha.run_to_acceptance().expect("alpha run");
    let beta_result = beta.run_to_acceptance().expect("beta run");
    assert_eq!(alpha_result.answer, "alpha\n");
    assert_eq!(beta_result.answer, "beta\n");
    assert_ne!(alpha_result.answer, beta_result.answer);
    assert_eq!(alpha_result.acceptance, VerificationStatus::Passed);
    for (store, expected) in [
        (&alpha, b"alpha\n".as_slice()),
        (&beta, b"beta\n".as_slice()),
    ] {
        let persisted = store.store().load_session().unwrap();
        let final_bytes = store
            .artifacts()
            .read(persisted.final_ref.as_ref().unwrap())
            .unwrap();
        assert_eq!(final_bytes, expected);
        assert!(persisted
            .tool_result_refs
            .iter()
            .map(|reference| store.artifacts().read(reference).unwrap())
            .any(|bytes| bytes == expected));
    }
    assert_eq!(
        alpha.store().effect_kinds(),
        vec!["CallModel", "ExecuteTool", "CallModel", "Verify"]
    );
    let body: String = alpha
        .store()
        .connection()
        .query_row("SELECT body FROM sessions", [], |r| r.get(0))
        .unwrap();
    let persisted: rein_core::HarnessSession = serde_json::from_str(&body).unwrap();
    assert_eq!(persisted.status, rein_core::SessionStatus::Accepted);
    assert!(persisted.final_ref.as_ref().is_some_and(|r| r.len == 6));
    assert!(!body.contains("alpha\\n"));
}

#[test]
fn b02_replayed_observation_and_same_effect_are_durable_noops() {
    let (_dir, mut runtime) = runtime("alpha.txt");
    runtime.start().expect("start");
    let before = runtime.store().counts().expect("before counts");
    runtime.drive_one().expect("first observation");
    let after_first = runtime.store().counts().expect("first counts");
    let observation = runtime
        .store()
        .last_observation()
        .expect("stored observation");
    runtime
        .observe(observation.clone())
        .expect("same observation replay");
    let after_same = runtime.store().counts().expect("same replay counts");
    let different_id_same_effect = observation.with_observation_id("replay-id");
    runtime
        .observe(different_id_same_effect)
        .expect("same effect replay");
    let after_effect = runtime.store().counts().expect("effect replay counts");
    assert_eq!(after_same.session_revision, after_first.session_revision);
    assert_eq!(after_same.events, after_first.events);
    assert_eq!(after_same.outbox, after_first.outbox);
    assert_eq!(after_effect, after_same);
    assert!(after_first.session_revision > before.session_revision);
}

#[test]
fn b03_zero_budget_has_no_execute_tool_effect_and_preserves_fixture() {
    let (_dir, mut runtime) = runtime("alpha.txt");
    runtime.set_tool_budget(0).expect("set budget");
    let before = fs::read(fixture("alpha.txt")).expect("fixture bytes");
    let result = runtime.run_to_acceptance().expect("budgeted run");
    let after = fs::read(fixture("alpha.txt")).expect("fixture bytes");
    assert_eq!(before, after);
    assert_eq!(
        runtime
            .store()
            .effect_dispatch_count("ExecuteTool")
            .unwrap(),
        0
    );
    assert_ne!(result.acceptance, VerificationStatus::Passed);
}

#[test]
fn b04_cancel_at_each_pending_stage_rejects_late_verifier() {
    for stage in ["CallModel", "ExecuteTool", "Verify"] {
        let (_dir, mut runtime) = runtime("alpha.txt");
        runtime.start().expect("start");
        while runtime.store().pending_effect().unwrap().kind != stage {
            runtime
                .drive_one()
                .expect("advance to requested pending stage");
        }
        let effect_id = runtime.store().pending_effect().unwrap().id().to_owned();
        runtime.cancel().expect("cancel");
        let dispatches_before = runtime.store().dispatch_count_total().unwrap();
        runtime.drive_to_completion().expect("cancelled drive");
        let session = runtime.store().load_session().unwrap();
        assert!(runtime
            .observe(Observation {
                observation_id: format!("late-{stage}"),
                session_id: session.session_id,
                run_id: session.run_id,
                attempt_id: session.attempt_id,
                revision: session.revision,
                effect_id: Some(effect_id),
                kind: ObservationKind::VerifierResult {
                    status: CoreVerificationStatus::Passed,
                    output_ref: ArtifactRef {
                        hash: "missing".into(),
                        len: 7
                    },
                },
            })
            .is_err());
        assert_eq!(runtime.store().state().unwrap(), "cancelled");
        assert_ne!(
            runtime.store().acceptance().unwrap(),
            Some(VerificationStatus::Passed)
        );
        assert_eq!(
            runtime.store().dispatch_count_total().unwrap(),
            dispatches_before
        );
    }
}

#[test]
fn b05_real_sqlite_rollback_and_artifact_revalidation_leave_no_dangling_ref() {
    let (_dir, mut runtime) = runtime("alpha.txt");
    runtime.start().expect("start");
    let session = runtime.store().load_session().unwrap();
    let effect = session.active_effect.clone().unwrap();
    let args = runtime.artifacts().put(b"fixture.txt").unwrap();
    let observation = Observation {
        observation_id: "model-with-trigger".into(),
        session_id: session.session_id.clone(),
        run_id: session.run_id.clone(),
        attempt_id: session.attempt_id.clone(),
        revision: session.revision,
        effect_id: Some(effect.clone()),
        kind: rein_core::ObservationKind::ModelTurn {
            tool_call: Some(rein_core::ToolCall {
                call_id: "call".into(),
                name: "read_file".into(),
                args_ref: args,
            }),
            final_ref: None,
        },
    };
    runtime
        .store()
        .connection()
        .execute_batch("CREATE TRIGGER fail_events BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'b05'); END;")
        .unwrap();
    let before = runtime.store().counts().unwrap();
    let old_effect_state = runtime.store().effect_state(&effect).unwrap();
    assert!(runtime.observe(observation.clone()).is_err());
    assert_eq!(runtime.store().counts().unwrap(), before);
    assert_eq!(
        runtime.store().effect_state(&effect).unwrap(),
        old_effect_state
    );
    runtime
        .store()
        .connection()
        .execute_batch("DROP TRIGGER fail_events;")
        .unwrap();
    runtime.observe(observation).expect("exact retry succeeds");
    let session = runtime.store().load_session().unwrap();
    let tool = match session.active_intent.unwrap() {
        rein_core::ActiveIntent::Tool { call_id, .. } => call_id,
        other => panic!("expected active tool, got {other:?}"),
    };
    let missing = Observation {
        observation_id: "missing-artifact".into(),
        session_id: session.session_id.clone(),
        run_id: session.run_id.clone(),
        attempt_id: session.attempt_id.clone(),
        revision: session.revision,
        effect_id: session.active_effect.clone(),
        kind: rein_core::ObservationKind::ToolResult {
            call_id: tool,
            result_ref: ArtifactRef {
                hash: "0".repeat(64),
                len: 1,
            },
        },
    };
    let after_success = runtime.store().counts().unwrap();
    assert!(runtime.observe(missing).is_err());
    assert_eq!(runtime.store().counts().unwrap(), after_success);
    let corrupt_ref = runtime.artifacts().put(b"valid-result").unwrap();
    fs::write(
        runtime.artifacts().root().join(&corrupt_ref.hash),
        b"corrupt-result",
    )
    .unwrap();
    let current = runtime.store().load_session().unwrap();
    let corrupt_call = match current.active_intent.as_ref().unwrap() {
        rein_core::ActiveIntent::Tool { call_id, .. } => call_id.clone(),
        other => panic!("expected active tool, got {other:?}"),
    };
    let before_corrupt = runtime.store().counts().unwrap();
    assert!(runtime
        .observe(Observation {
            observation_id: "corrupt-artifact".into(),
            session_id: current.session_id,
            run_id: current.run_id,
            attempt_id: current.attempt_id,
            revision: current.revision,
            effect_id: current.active_effect,
            kind: rein_core::ObservationKind::ToolResult {
                call_id: corrupt_call,
                result_ref: corrupt_ref,
            },
        })
        .is_err());
    assert_eq!(runtime.store().counts().unwrap(), before_corrupt);
    let stale = runtime.store().load_session().unwrap();
    assert!(runtime
        .observe(Observation {
            observation_id: "stale".into(),
            session_id: stale.session_id,
            run_id: stale.run_id,
            attempt_id: stale.attempt_id,
            revision: stale.revision.saturating_sub(1),
            effect_id: None,
            kind: rein_core::ObservationKind::Start,
        })
        .is_err());
}

#[test]
fn b06_reopen_requires_explicit_recovery_and_quarantines_claimed_effect() {
    let dir = tempfile::tempdir().expect("state dir");
    let config = RuntimeConfig::for_fixture(dir.path(), fixture("alpha.txt"));
    let mut first = Runtime::open(
        config.clone(),
        OfflineModel::reads_named_fixture(),
        ReadFileTool::rooted_read_only(),
        FixedVerifier::expected_file(fixture("alpha.txt")),
    )
    .unwrap();
    first.start().unwrap();
    let pending = first.store().pending_effect().unwrap();
    drop(first);
    let mut reopened = Runtime::open(
        config,
        OfflineModel::reads_named_fixture(),
        ReadFileTool::rooted_read_only(),
        FixedVerifier::expected_file(fixture("alpha.txt")),
    )
    .unwrap();
    assert_eq!(
        reopened.store().effect_state(pending.id()).unwrap(),
        EffectState::Pending
    );
    reopened.drive_to_completion().unwrap();
    assert_eq!(
        reopened.store().acceptance().unwrap(),
        Some(VerificationStatus::Passed)
    );
    drop(reopened);
    let claimed_dir = tempfile::tempdir().expect("claimed state dir");
    let mut claimed = Runtime::open(
        RuntimeConfig::for_fixture(claimed_dir.path(), fixture("alpha.txt")),
        OfflineModel::reads_named_fixture(),
        ReadFileTool::rooted_read_only(),
        FixedVerifier::expected_file(fixture("alpha.txt")),
    )
    .unwrap();
    claimed.start().unwrap();
    let claimed_effect = claimed.store().pending_effect().unwrap();
    claimed.store().claim_next_effect().unwrap();
    drop(claimed);
    let mut recovered = Runtime::open(
        RuntimeConfig::for_fixture(claimed_dir.path(), fixture("alpha.txt")),
        OfflineModel::reads_named_fixture(),
        ReadFileTool::rooted_read_only(),
        FixedVerifier::expected_file(fixture("alpha.txt")),
    )
    .unwrap();
    recovered.recover_explicitly().unwrap();
    assert_eq!(
        recovered.store().effect_state(claimed_effect.id()).unwrap(),
        EffectState::OutcomeUnknown
    );
    let dispatches = recovered
        .store()
        .dispatch_count(claimed_effect.id())
        .unwrap();
    recovered.drive_to_completion().unwrap();
    assert_eq!(
        recovered
            .store()
            .dispatch_count(claimed_effect.id())
            .unwrap(),
        dispatches
    );
}

#[test]
fn b07_failed_or_undetermined_verifier_never_creates_passed_acceptance() {
    let (_dir, mut failed) =
        runtime_with_verifier("alpha.txt", FixedVerifier::expected_bytes(b"wrong"));
    let failed_result = failed.run_to_acceptance().unwrap();
    assert_eq!(failed_result.acceptance, VerificationStatus::Failed);
    assert_ne!(
        failed.store().acceptance().unwrap(),
        Some(VerificationStatus::Passed)
    );
    let missing = tempfile::tempdir().unwrap();
    let (_dir, mut undetermined) = runtime_with_verifier(
        "alpha.txt",
        FixedVerifier::expected_file(missing.path().join("missing-expected")),
    );
    let undetermined_result = undetermined.run_to_acceptance().unwrap();
    assert_eq!(
        undetermined_result.acceptance,
        VerificationStatus::Undetermined
    );
    assert_ne!(
        undetermined.store().acceptance().unwrap(),
        Some(VerificationStatus::Passed)
    );
}

#[test]
fn b08_read_tool_rejects_escape_symlink_invalid_utf8_and_oversize_without_canary_read() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let outside = dir.path().join("outside");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("canary"), b"outside-secret").unwrap();
    std::os::unix::fs::symlink(outside.join("canary"), root.join("escape-link")).unwrap();
    fs::write(root.join("invalid"), [0xff, 0xfe]).unwrap();
    fs::write(root.join("oversized"), b"12345").unwrap();
    let tool = ReadFileTool::new(root.clone(), 4);
    for path in [
        "../outside/canary",
        "/etc/hosts",
        "escape-link",
        "invalid",
        "oversized",
    ] {
        let error = tool.read(path).expect_err("path should be rejected");
        assert!(
            !format!("{error:?}").contains("outside-secret"),
            "canary escaped through {path}"
        );
    }
    assert!(tool.read("escape-link").is_err());
}

#[test]
fn b09_schema_check_detects_real_generated_file_drift() {
    let dir = tempfile::tempdir().unwrap();
    rein_runtime::schema::generate(dir.path()).unwrap();
    rein_runtime::schema::check(dir.path()).unwrap();
    fs::write(dir.path().join("session.json"), b"drift").unwrap();
    assert!(rein_runtime::schema::check(dir.path()).is_err());
    rein_runtime::schema::generate(dir.path()).unwrap();
    rein_runtime::schema::check(dir.path()).unwrap();
}

#[test]
fn b10_two_sqlite_connections_claim_one_effect() {
    let (dir, mut setup) = runtime("alpha.txt");
    setup.start().unwrap();
    let db = dir.path().join("state.sqlite");
    let barrier = Arc::new(Barrier::new(2));
    let dispatches = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    drop(setup);
    let a = {
        let db = db.clone();
        let barrier = barrier.clone();
        let dispatches = dispatches.clone();
        thread::spawn(move || {
            let store = SqliteStore::open(db).unwrap();
            barrier.wait();
            if let Some(effect) = store.claim_next_effect().unwrap() {
                if store.dispatch_claimed(effect.id()).is_ok() {
                    dispatches.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            }
        })
    };
    let b = {
        let db = db.clone();
        let barrier = barrier.clone();
        let dispatches = dispatches.clone();
        thread::spawn(move || {
            let store = SqliteStore::open(db).unwrap();
            barrier.wait();
            if let Some(effect) = store.claim_next_effect().unwrap() {
                if store.dispatch_claimed(effect.id()).is_ok() {
                    dispatches.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            }
        })
    };
    a.join().unwrap();
    b.join().unwrap();
    assert_eq!(dispatches.load(std::sync::atomic::Ordering::SeqCst), 1);
    let check = SqliteStore::open(db).unwrap();
    assert_eq!(check.dispatch_count_total().unwrap(), 1);
    let revisions: i64 = check
        .connection()
        .query_row("SELECT COUNT(DISTINCT revision) FROM sessions", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(revisions, 1);
}
