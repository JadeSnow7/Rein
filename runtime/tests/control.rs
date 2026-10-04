use rein_core::{Observation, ObservationKind, VerificationStatus as CoreVerificationStatus};
use rein_runtime::model::OfflineModel;
use rein_runtime::owner::DriverOwner;
use rein_runtime::verify::FixedVerifier;
use rein_runtime::{ReadFileTool, Runtime, RuntimeConfig};
use std::fs;
use std::path::Path;

fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/runtime-r1a/alpha.txt")
}

fn open(dir: &Path) -> Runtime {
    Runtime::open(
        RuntimeConfig::for_fixture(dir, fixture()),
        OfflineModel::reads_named_fixture(),
        ReadFileTool::rooted_read_only(),
        FixedVerifier::expected_file(fixture()),
    )
    .unwrap()
}

#[test]
fn owner_is_exclusive_but_inspect_and_cancel_at_are_available() {
    let dir = tempfile::tempdir().unwrap();
    let mut runtime = open(dir.path());
    assert!(Runtime::open(
        RuntimeConfig::for_fixture(dir.path(), fixture()),
        OfflineModel::reads_named_fixture(),
        ReadFileTool::rooted_read_only(),
        FixedVerifier::expected_file(fixture()),
    )
    .is_err());
    runtime.start().unwrap();
    let inspected = Runtime::inspect(dir.path()).unwrap();
    assert_eq!(inspected.session.revision, 1);
    let cancelled = Runtime::cancel_at(dir.path()).unwrap();
    assert_eq!(
        cancelled.session.status,
        rein_core::SessionStatus::Cancelled
    );
}

#[test]
fn pending_recovery_remains_runnable_and_claimed_recovery_is_unknown() {
    let pending_dir = tempfile::tempdir().unwrap();
    let mut pending = open(pending_dir.path());
    pending.start().unwrap();
    drop(pending);
    let mut resumed = Runtime::reopen(pending_dir.path()).unwrap();
    resumed.recover_explicitly().unwrap();
    resumed.drive_to_completion().unwrap();
    assert_eq!(resumed.store().state().unwrap(), "accepted");

    let claimed_dir = tempfile::tempdir().unwrap();
    let mut claimed = open(claimed_dir.path());
    claimed.start().unwrap();
    let effect = claimed.claim_next_effect().unwrap().unwrap();
    drop(claimed);
    let mut recovered = Runtime::reopen(claimed_dir.path()).unwrap();
    recovered.recover_explicitly().unwrap();
    assert_eq!(recovered.store().state().unwrap(), "unknown");
    assert_eq!(
        recovered.store().effect_state(effect.id()).unwrap(),
        rein_runtime::store::EffectState::OutcomeUnknown
    );
    recovered.recover_explicitly().unwrap();
}

#[test]
fn missing_state_control_operations_have_no_filesystem_side_effects() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    assert!(Runtime::inspect(&missing).is_err());
    assert!(Runtime::cancel_at(&missing).is_err());
    assert!(Runtime::reopen(&missing).is_err());
    assert!(!missing.exists());
    let _ = DriverOwner::acquire(dir.path()).unwrap();
}

#[test]
fn existing_fixture_target_is_rejected_without_mutating_source() {
    let dir = tempfile::tempdir().unwrap();
    let workdir = dir.path().join("workdir");
    fs::create_dir_all(&workdir).unwrap();
    let target = workdir.join("fixture.txt");
    fs::hard_link(fixture(), &target).unwrap();
    let before = fs::read(fixture()).unwrap();
    assert!(Runtime::open(
        RuntimeConfig::for_fixture(dir.path(), fixture()),
        OfflineModel::reads_named_fixture(),
        ReadFileTool::rooted_read_only(),
        FixedVerifier::expected_file(fixture()),
    )
    .is_err());
    assert_eq!(fs::read(fixture()).unwrap(), before);
}

#[test]
fn cancel_after_claim_blocks_dispatch_without_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let mut runtime = open(dir.path());
    runtime.start().unwrap();
    let effect = runtime.claim_next_effect().unwrap().unwrap();
    let inspected = Runtime::inspect(dir.path()).unwrap();
    assert_eq!(inspected.session.status, rein_core::SessionStatus::Running);
    assert_eq!(
        inspected.effects[0].state,
        rein_runtime::store::EffectState::Claimed
    );
    let cancelled = Runtime::cancel_at(dir.path()).unwrap();
    assert_eq!(
        cancelled.session.status,
        rein_core::SessionStatus::Cancelled
    );
    assert!(runtime.store().dispatch_claimed(effect.id()).is_err());
    assert_eq!(runtime.store().dispatch_count(effect.id()).unwrap(), 0);
    assert!(runtime.drive_one().is_err());
    assert_eq!(runtime.store().dispatch_count(effect.id()).unwrap(), 0);
}

fn advance_to_verify(runtime: &mut Runtime) {
    runtime.start().unwrap();
    runtime.drive_one().unwrap();
    runtime.drive_one().unwrap();
    runtime.drive_one().unwrap();
}

#[test]
fn cancelled_verify_rejects_late_valid_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let mut runtime = open(dir.path());
    advance_to_verify(&mut runtime);
    let before_cancel = runtime.store().load_session().unwrap();
    let final_ref = before_cancel.final_ref.clone().unwrap();
    let final_bytes = runtime.artifacts().read(&final_ref).unwrap();
    let verifier = FixedVerifier::from_plan_bytes(
        &runtime
            .artifacts()
            .read(&before_cancel.verification_plan_ref)
            .unwrap(),
    )
    .unwrap();
    let receipt = verifier
        .receipt(
            &before_cancel.verification_plan_ref,
            &final_ref,
            &final_bytes,
        )
        .unwrap();
    let receipt_ref = runtime
        .artifacts()
        .put(&serde_json::to_vec(&receipt).unwrap())
        .unwrap();
    let old_effect = before_cancel.active_effect.clone().unwrap();
    Runtime::cancel_at(dir.path()).unwrap();
    let cancelled = runtime.store().load_session().unwrap();
    let before = runtime.store().counts().unwrap();
    assert!(runtime
        .observe(Observation {
            observation_id: "late-valid-verifier".into(),
            session_id: cancelled.session_id,
            run_id: cancelled.run_id,
            attempt_id: cancelled.attempt_id,
            revision: cancelled.revision,
            effect_id: Some(old_effect),
            kind: ObservationKind::VerifierResult {
                status: CoreVerificationStatus::Passed,
                output_ref: receipt_ref,
            },
        })
        .is_err());
    assert_eq!(runtime.store().counts().unwrap(), before);
    assert_ne!(
        runtime.store().acceptance().unwrap(),
        Some(rein_runtime::verify::VerificationStatus::Passed)
    );
}

#[test]
fn forged_passed_receipt_is_rejected_without_commit() {
    let dir = tempfile::tempdir().unwrap();
    let mut runtime = Runtime::open(
        RuntimeConfig::for_fixture(dir.path(), fixture()),
        OfflineModel::reads_named_fixture(),
        ReadFileTool::rooted_read_only(),
        FixedVerifier::expected_bytes(b"wrong"),
    )
    .unwrap();
    advance_to_verify(&mut runtime);
    let session = runtime.store().load_session().unwrap();
    let final_ref = session.final_ref.clone().unwrap();
    let final_bytes = runtime.artifacts().read(&final_ref).unwrap();
    let verifier = FixedVerifier::from_plan_bytes(
        &runtime
            .artifacts()
            .read(&session.verification_plan_ref)
            .unwrap(),
    )
    .unwrap();
    let mut receipt = verifier
        .receipt(&session.verification_plan_ref, &final_ref, &final_bytes)
        .unwrap();
    assert_eq!(
        receipt.status,
        rein_runtime::verify::VerificationStatus::Failed
    );
    receipt.status = rein_runtime::verify::VerificationStatus::Passed;
    let output_ref = runtime
        .artifacts()
        .put(&serde_json::to_vec(&receipt).unwrap())
        .unwrap();
    let before = runtime.store().counts().unwrap();
    assert!(runtime
        .observe(Observation {
            observation_id: "forged-passed".into(),
            session_id: session.session_id,
            run_id: session.run_id,
            attempt_id: session.attempt_id,
            revision: session.revision,
            effect_id: session.active_effect,
            kind: ObservationKind::VerifierResult {
                status: CoreVerificationStatus::Passed,
                output_ref,
            },
        })
        .is_err());
    assert_eq!(runtime.store().counts().unwrap(), before);
    assert_eq!(runtime.store().acceptance().unwrap(), None);
}

#[test]
fn recovered_unknown_effect_can_be_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    let mut runtime = open(dir.path());
    runtime.start().unwrap();
    let effect = runtime.claim_next_effect().unwrap().unwrap();
    drop(runtime);
    let mut recovered = Runtime::reopen(dir.path()).unwrap();
    recovered.recover_explicitly().unwrap();
    assert_eq!(
        recovered.store().effect_state(effect.id()).unwrap(),
        rein_runtime::store::EffectState::OutcomeUnknown
    );
    let cancelled = Runtime::cancel_at(dir.path()).unwrap();
    assert_eq!(
        cancelled.session.status,
        rein_core::SessionStatus::Cancelled
    );
    assert_eq!(
        cancelled.effects[0].state,
        rein_runtime::store::EffectState::Cancelled
    );
}

#[test]
fn late_outbox_write_failure_rolls_back_model_transition() {
    let dir = tempfile::tempdir().unwrap();
    let mut runtime = open(dir.path());
    runtime.start().unwrap();
    let session = runtime.store().load_session().unwrap();
    let args = runtime.artifacts().put(b"fixture.txt").unwrap();
    let observation = Observation {
        observation_id: "late-outbox-model".into(),
        session_id: session.session_id.clone(),
        run_id: session.run_id.clone(),
        attempt_id: session.attempt_id.clone(),
        revision: session.revision,
        effect_id: session.active_effect.clone(),
        kind: ObservationKind::ModelTurn {
            tool_call: Some(rein_core::ToolCall {
                call_id: "read-1".into(),
                name: "read_file".into(),
                args_ref: args,
            }),
            final_ref: None,
        },
    };
    runtime
        .store()
        .connection()
        .execute_batch("CREATE TRIGGER fail_late_outbox BEFORE INSERT ON outbox BEGIN SELECT RAISE(ABORT, 'late outbox'); END;")
        .unwrap();
    let before = runtime.store().counts().unwrap();
    let old_effect = runtime.store().pending_effect().unwrap().id().to_owned();
    assert!(runtime.observe(observation.clone()).is_err());
    assert_eq!(runtime.store().counts().unwrap(), before);
    assert_eq!(
        runtime.store().effect_state(&old_effect).unwrap(),
        rein_runtime::store::EffectState::Pending
    );
    runtime
        .store()
        .connection()
        .execute_batch("DROP TRIGGER fail_late_outbox;")
        .unwrap();
    runtime.observe(observation).unwrap();
}

#[test]
fn late_acceptance_write_failure_rolls_back_verifier_transition() {
    let dir = tempfile::tempdir().unwrap();
    let mut runtime = open(dir.path());
    advance_to_verify(&mut runtime);
    let session = runtime.store().load_session().unwrap();
    let final_ref = session.final_ref.clone().unwrap();
    let final_bytes = runtime.artifacts().read(&final_ref).unwrap();
    let verifier = FixedVerifier::from_plan_bytes(
        &runtime
            .artifacts()
            .read(&session.verification_plan_ref)
            .unwrap(),
    )
    .unwrap();
    let receipt = verifier
        .receipt(&session.verification_plan_ref, &final_ref, &final_bytes)
        .unwrap();
    let output_ref = runtime
        .artifacts()
        .put(&serde_json::to_vec(&receipt).unwrap())
        .unwrap();
    let observation = Observation {
        observation_id: "late-acceptance".into(),
        session_id: session.session_id.clone(),
        run_id: session.run_id.clone(),
        attempt_id: session.attempt_id.clone(),
        revision: session.revision,
        effect_id: session.active_effect.clone(),
        kind: ObservationKind::VerifierResult {
            status: CoreVerificationStatus::Passed,
            output_ref,
        },
    };
    runtime
        .store()
        .connection()
        .execute_batch("CREATE TRIGGER fail_late_acceptance BEFORE INSERT ON acceptance BEGIN SELECT RAISE(ABORT, 'late acceptance'); END;")
        .unwrap();
    let before = runtime.store().counts().unwrap();
    let old_effect = session.active_effect.unwrap();
    assert!(runtime.observe(observation.clone()).is_err());
    assert_eq!(runtime.store().counts().unwrap(), before);
    assert_eq!(
        runtime.store().effect_state(&old_effect).unwrap(),
        rein_runtime::store::EffectState::Pending
    );
    assert_eq!(runtime.store().acceptance().unwrap(), None);
    runtime
        .store()
        .connection()
        .execute_batch("DROP TRIGGER fail_late_acceptance;")
        .unwrap();
    runtime.observe(observation).unwrap();
    assert_eq!(
        runtime.store().acceptance().unwrap(),
        Some(rein_runtime::verify::VerificationStatus::Passed)
    );
}
