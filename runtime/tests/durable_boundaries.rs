use std::fs;
use std::path::PathBuf;

use rein_core::ObservationKind;
use rein_runtime::model::OfflineModel;
use rein_runtime::verify::{FixedVerifier, VerificationStatus};
use rein_runtime::{ReadFileTool, Runtime, RuntimeConfig};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/runtime-r1a")
        .join(name)
}

fn open(state: &std::path::Path, source: &std::path::Path, verifier: FixedVerifier) -> Runtime {
    Runtime::open(
        RuntimeConfig::for_fixture(state, source),
        OfflineModel::reads_named_fixture(),
        ReadFileTool::rooted_read_only(),
        verifier,
    )
    .unwrap()
}

#[test]
fn verification_plan_survives_restart_and_source_mutation() {
    let source = tempfile::NamedTempFile::new().unwrap();
    fs::write(source.path(), b"alpha\n").unwrap();

    let wrong_state = tempfile::tempdir().unwrap();
    let mut prepared_wrong = open(
        wrong_state.path(),
        source.path(),
        FixedVerifier::expected_bytes(b"wrong\n"),
    );
    prepared_wrong.start().unwrap();
    drop(prepared_wrong);
    fs::write(source.path(), b"beta\n").unwrap();
    let mut restarted_wrong = open(
        wrong_state.path(),
        source.path(),
        FixedVerifier::expected_bytes(b"alpha\n"),
    );
    let wrong_result = restarted_wrong.run_to_acceptance().unwrap();
    assert_eq!(wrong_result.acceptance, VerificationStatus::Failed);
    assert_eq!(wrong_result.answer, "alpha\n");

    let right_state = tempfile::tempdir().unwrap();
    let mut prepared_right = open(
        right_state.path(),
        source.path(),
        FixedVerifier::expected_bytes(b"beta\n"),
    );
    prepared_right.start().unwrap();
    drop(prepared_right);
    fs::remove_file(source.path()).unwrap();
    let mut restarted_right = open(
        right_state.path(),
        source.path(),
        FixedVerifier::expected_bytes(b"wrong\n"),
    );
    let right_result = restarted_right.run_to_acceptance().unwrap();
    assert_eq!(right_result.acceptance, VerificationStatus::Passed);
    assert_eq!(right_result.answer, "beta\n");
}

#[test]
fn verifier_event_trigger_rolls_back_and_recovery_quarantines_claim() {
    let state = tempfile::tempdir().unwrap();
    let mut runtime = open(
        state.path(),
        &fixture("alpha.txt"),
        FixedVerifier::expected_file(fixture("alpha.txt")),
    );
    runtime.start().unwrap();
    runtime.drive_one().unwrap();
    runtime.drive_one().unwrap();
    runtime.drive_one().unwrap();
    let before = runtime.store().counts().unwrap();
    runtime.store().connection().execute_batch("CREATE TRIGGER reject_verify BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'reject verify'); END;").unwrap();
    assert!(runtime.drive_one().is_err());
    let after = runtime.store().counts().unwrap();
    assert_eq!(after, before);
    assert_eq!(runtime.store().acceptance().unwrap(), None);
    let effect: String = runtime
        .store()
        .connection()
        .query_row(
            "SELECT effect_id FROM outbox WHERE state='claimed'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        runtime.store().effect_state(&effect).unwrap(),
        rein_runtime::store::EffectState::Claimed
    );
    assert_eq!(runtime.store().dispatch_count(&effect).unwrap(), 1);
    runtime
        .store()
        .connection()
        .execute_batch("DROP TRIGGER reject_verify")
        .unwrap();
    drop(runtime);
    let mut reopened = Runtime::reopen(state.path()).unwrap();
    let dispatches = reopened.store().dispatch_count_total().unwrap();
    reopened.recover_explicitly().unwrap();
    let session = reopened.store().load_session().unwrap();
    assert_eq!(session.status, rein_core::SessionStatus::OutcomeUnknown);
    reopened.drive_to_completion().unwrap();
    assert_eq!(reopened.store().dispatch_count_total().unwrap(), dispatches);
}

#[test]
fn invalid_utf8_tool_input_never_passes_verification() {
    let state = tempfile::tempdir().unwrap();
    let mut runtime = open(
        state.path(),
        &fixture("invalid-utf8.bin"),
        FixedVerifier::expected_bytes(b"passed"),
    );
    let _ = runtime.run_to_acceptance();
    assert_ne!(
        runtime.store().acceptance().unwrap(),
        Some(VerificationStatus::Passed)
    );
    let observations = runtime
        .store()
        .connection()
        .prepare("SELECT body FROM observations ORDER BY rowid")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(|row| serde_json::from_str::<rein_core::Observation>(&row.unwrap()).unwrap())
        .collect::<Vec<_>>();
    let failures = observations
        .iter()
        .filter_map(|observation| match &observation.kind {
            ObservationKind::ToolFailed { error_ref, .. } => Some(error_ref.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(failures.len(), 1);
    let error_ref = failures.into_iter().next().unwrap();
    let error_body = runtime.artifacts().read(&error_ref).unwrap();
    let error_body = String::from_utf8(error_body).unwrap();
    assert!(
        error_body.contains("utf8"),
        "unexpected tool error: {error_body}"
    );
    assert_eq!(
        runtime
            .store()
            .effect_dispatch_count("ExecuteTool")
            .unwrap(),
        1
    );
}
