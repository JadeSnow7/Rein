use rein_ch01_helloworld::rein::{
    maintenance_session::{run_maintenance_session, SessionEvent, StopReason},
    ControlSignal, Extension02Executor,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new(readme: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "rein-maint-session-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("README.md"), readme).unwrap();
        fs::write(
            root.join("package.json"),
            r#"{"scripts":{"dev":"node -e \"process.exit(0)\""}}"#,
        )
        .unwrap();
        Self { root }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn real_host_applies_and_verifies_one_candidate() {
    let fixture = Fixture::new("启动：`npm run start`。\n");
    let executor = Extension02Executor::new(&fixture.root);
    let saw_source = std::sync::Arc::new(std::sync::Mutex::new(false));
    let seen = saw_source.clone();
    let report = run_maintenance_session(
        &executor,
        &ControlSignal::new(),
        "README.md",
        "command-v1",
        move |observation| {
            assert_eq!(observation.rule, "command-v1");
            *seen.lock().unwrap() = observation
                .source_contents
                .iter()
                .any(|(path, text)| path == "package.json" && text.contains("\"dev\""));
            Some("启动：`npm run dev`。\n".into())
        },
        |candidate| Some(candidate.approve()),
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert!(*saw_source.lock().unwrap());
    assert_eq!(
        fs::read_to_string(fixture.root.join("README.md")).unwrap(),
        "启动：`npm run dev`。\n"
    );
    assert!(report
        .events
        .iter()
        .any(|event| matches!(event, SessionEvent::Applied { attempt: 0, .. })));
    assert!(executor.records().len() >= 4);
}

#[tokio::test]
async fn rejected_approval_keeps_document_and_history() {
    let fixture = Fixture::new("启动：`npm run start`。\n");
    let executor = Extension02Executor::new(&fixture.root);
    let report = run_maintenance_session(
        &executor,
        &ControlSignal::new(),
        "README.md",
        "command-v1",
        |_| Some("启动：`npm run dev`。\n".into()),
        |_| None,
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Rejected);
    assert_eq!(
        fs::read_to_string(fixture.root.join("README.md")).unwrap(),
        "启动：`npm run start`。\n"
    );
    assert!(report.events.iter().any(|event| matches!(
        event,
        SessionEvent::Approval {
            approved: false,
            ..
        }
    )));
}

#[tokio::test]
async fn no_change_still_runs_real_verification() {
    let fixture = Fixture::new("启动：`npm run dev`。\n");
    let executor = Extension02Executor::new(&fixture.root);
    let report = run_maintenance_session(
        &executor,
        &ControlSignal::new(),
        "README.md",
        "command-v1",
        |_| None,
        |_| panic!("no approval for no-change"),
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    assert!(report.events.iter().any(|event| matches!(
        event,
        SessionEvent::Verified {
            attempt: 0,
            ok: true,
            ..
        }
    )));
    assert_eq!(executor.records().len(), 3);
}

#[tokio::test]
async fn changed_baseline_is_a_known_tool_failure() {
    let fixture = Fixture::new("启动：`npm run start`。\n");
    let executor = Extension02Executor::new(&fixture.root);
    let path = fixture.root.join("README.md");
    let approved = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let approved_for_callback = approved.clone();
    let report = run_maintenance_session(
        &executor,
        &ControlSignal::new(),
        "README.md",
        "command-v1",
        move |_| {
            fs::write(&path, "启动：`npm run changed`。\n").unwrap();
            Some("启动：`npm run dev`。\n".into())
        },
        move |candidate| {
            approved_for_callback.store(true, Ordering::Relaxed);
            Some(candidate.approve())
        },
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::ToolFailure);
    assert!(report.events.iter().any(
        |event| matches!(event, SessionEvent::ToolFailed { tool, code, .. } if tool == "candidate" && code == "baseline_changed")
    ));
    assert!(!approved.load(Ordering::Relaxed));
    assert_eq!(
        fs::read_to_string(fixture.root.join("README.md")).unwrap(),
        "启动：`npm run changed`。\n"
    );
}

#[tokio::test]
async fn one_failed_verification_is_repaired_with_fresh_observation_and_approval() {
    let fixture = Fixture::new("启动：`npm run start`。\n");
    let executor = Extension02Executor::new(&fixture.root);
    let report = run_maintenance_session(
        &executor,
        &ControlSignal::new(),
        "README.md",
        "command-v1",
        |observation| {
            assert_eq!(observation.source_contents.len(), 1);
            if observation.attempt == 0 {
                Some("启动：`npm run missing`。\n".into())
            } else {
                assert!(observation.feedback.is_some());
                Some("启动：`npm run dev`。\n".into())
            }
        },
        |candidate| Some(candidate.approve()),
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::Completed);
    let failed_output = report
        .events
        .iter()
        .find_map(|event| match event {
            SessionEvent::Verified {
                attempt: 0,
                ok: false,
                output,
            } => Some(output.clone()),
            _ => None,
        })
        .unwrap();
    let success_output = report
        .events
        .iter()
        .find_map(|event| match event {
            SessionEvent::Verified {
                attempt: 1,
                ok: true,
                output,
            } => Some(output.clone()),
            _ => None,
        })
        .unwrap();
    assert!(failed_output.contains("\"ok\":false"));
    assert!(success_output.contains("\"ok\":true"));
    let digests: Vec<_> = report
        .events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::Proposed {
                candidate_digest, ..
            } => Some(candidate_digest.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(digests.len(), 2);
    assert_ne!(digests[0], digests[1]);
    assert!(report.events.iter().any(|event| matches!(
        event,
        SessionEvent::Verified {
            attempt: 0,
            ok: false,
            ..
        }
    )));
    assert!(report
        .events
        .iter()
        .any(|event| matches!(event, SessionEvent::Applied { attempt: 1, .. })));
    assert_eq!(
        fs::read_to_string(fixture.root.join("README.md")).unwrap(),
        "启动：`npm run dev`。\n"
    );
}

#[tokio::test]
async fn repeated_failures_stop_after_two_repairs_and_preserve_history() {
    let fixture = Fixture::new("启动：`npm run start`。\n");
    let executor = Extension02Executor::new(&fixture.root);
    let report = run_maintenance_session(
        &executor,
        &ControlSignal::new(),
        "README.md",
        "command-v1",
        |observation| {
            Some(format!(
                "启动：`npm run missing-{}`。\n",
                observation.attempt
            ))
        },
        |candidate| Some(candidate.approve()),
    )
    .await
    .unwrap();
    assert_eq!(report.stop_reason, StopReason::MaxRepairs);
    assert!(report.events.iter().any(|event| matches!(
        event,
        SessionEvent::Verified {
            attempt: 0,
            ok: false,
            ..
        }
    )));
    assert!(report.events.iter().any(|event| matches!(
        event,
        SessionEvent::Verified {
            attempt: 1,
            ok: false,
            ..
        }
    )));
    assert!(report.events.iter().any(|event| matches!(
        event,
        SessionEvent::Verified {
            attempt: 2,
            ok: false,
            ..
        }
    )));
    assert_eq!(
        report
            .events
            .iter()
            .filter(|event| matches!(event, SessionEvent::Applied { .. }))
            .count(),
        3
    );
}
