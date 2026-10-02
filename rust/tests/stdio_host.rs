use rein_ch01_helloworld::rein::{
    run_agent_loop_with_executor, ControlSignal, ExecutorResult, LoopOptions, LoopState, Message,
    ModelAdapter, ModelTurn, StdioExecutor, StopReason, ToolCall, ToolDefinition, ToolError,
    ToolExecutor, Workspace,
};
use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

struct Fixture {
    calls: Arc<AtomicUsize>,
    path: String,
}
impl ModelAdapter for Fixture {
    fn complete<'a>(
        &'a self,
        messages: &'a [Message],
        _: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let path = self.path.clone();
        let has_tool = messages.iter().any(|m| m.role == "tool");
        let tool_content = messages
            .iter()
            .find(|m| m.role == "tool" && m.tool_call_id.as_deref() == Some("model-call"))
            .map(|m| m.content.clone());
        Box::pin(async move {
            if n == 1 {
                let call = ToolCall {
                    id: "model-call".into(),
                    name: "read_file".into(),
                    arguments: serde_json::json!({"path":path}),
                };
                return Ok(ModelTurn {
                    message: Message {
                        role: "assistant".into(),
                        content: String::new(),
                        tool_call_id: None,
                        tool_calls: vec![call.clone()],
                    },
                    tool_calls: vec![call],
                });
            }
            Ok(ModelTurn {
                message: Message {
                    role: "assistant".into(),
                    content: if has_tool && tool_content.as_deref() == Some("fixture-output\n") {
                        tool_content.unwrap()
                    } else {
                        String::new()
                    },
                    tool_call_id: None,
                    tool_calls: vec![],
                },
                tool_calls: vec![],
            })
        })
    }
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
async fn run(
    mode: Option<&str>,
    signal: ControlSignal,
) -> (
    rein_ch01_helloworld::rein::LoopResult,
    Arc<AtomicUsize>,
    PathBuf,
    Vec<rein_ch01_helloworld::rein::stdio_executor::CallRecord>,
) {
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::temp_dir().join(format!(
        "rein-hybrid-test-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    tokio::fs::create_dir(&base).await.unwrap();
    let path = base.join("marker.txt");
    tokio::fs::write(&path, "fixture-output\n").await.unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let script = root().join("ts/tests/fixtures/hybrid-fault-host.ts");
    let mut executor = StdioExecutor::new(&base);
    executor.host_script = if mode.is_some() {
        script
    } else {
        root().join("ts/src/hybrid-host.ts")
    };
    executor.code_root = root();
    if let Some(mode) = mode {
        executor
            .env
            .push(("REIN_HYBRID_FAULT_MODE".into(), mode.into()));
        if mode == "disconnect_after_effect" {
            executor.env.push((
                "REIN_HYBRID_LEDGER".into(),
                base.join("ledger").to_string_lossy().into_owned(),
            ));
        }
        if mode == "stale_target" {
            executor.env.push((
                "REIN_HYBRID_FAULT_MARKER".into(),
                base.join("marker.txt").to_string_lossy().into_owned(),
            ));
        }
        if [
            "cancellation_wait",
            "ignore_cancel",
            "wrong_cancel_ack_ids",
            "cancel_ack_trailing_frame",
            "cancel_ack_nonzero_exit",
            "partial_frame_cancel",
        ]
        .contains(&mode)
        {
            let cancel = signal.clone();
            executor.on_started = Some(Arc::new(move || {
                let cancel = cancel.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    cancel.cancel();
                });
            }));
        }
    }
    let result = run_agent_loop_with_executor(
        &Fixture {
            calls: calls.clone(),
            path: "marker.txt".into(),
        },
        &Workspace { root: base.clone() },
        "read",
        LoopOptions {
            max_turns: 3,
            max_tool_calls: 1,
            signal: Some(signal),
            ..LoopOptions::default()
        },
        &executor,
    )
    .await;
    let records = executor.records();
    (result, calls, base, records)
}
#[tokio::test]
async fn production_host_reads_real_file_and_reaches_fixture_final() {
    let (result, calls, _, records) = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        run(None, ControlSignal::new()),
    )
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(result.state, LoopState::Completed);
    assert_eq!(result.reason, StopReason::FinalAnswer);
    assert_eq!(result.answer.as_deref(), Some("fixture-output\n"));
    assert_eq!(records.len(), 1);
    assert!(records[0].dispatched);
    assert!(records[0].reaped);
    assert_eq!(records[0].exit_code, Some(0));
}
#[tokio::test]
async fn fault_host_normal_control_reaches_fixture_final() {
    let (result, calls, _, _) = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        run(Some("normal"), ControlSignal::new()),
    )
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(result.reason, StopReason::FinalAnswer);
    assert_eq!(result.answer.as_deref(), Some("fixture-output\n"));
}
#[tokio::test]
async fn malformed_and_identity_faults_stop_without_second_model_call() {
    for mode in [
        "invalid_json",
        "wrong_id",
        "wrong_version",
        "contradictory",
        "duplicate",
        "wrong_session",
        "wrong_started",
        "wrong_terminal_session",
    ] {
        let (result, calls, _, _) = tokio::time::timeout(
            std::time::Duration::from_secs(8),
            run(Some(mode), ControlSignal::new()),
        )
        .await
        .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1, "{mode}");
        assert!(result.answer.is_none(), "{mode}");
        assert_eq!(result.reason, StopReason::OutcomeUnknown, "{mode}");
    }
}
#[tokio::test]
async fn cancellation_wait_is_observed_after_started() {
    let signal = ControlSignal::new();
    let (result, calls, _, _) = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        run(Some("cancellation_wait"), signal.clone()),
    )
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(result.reason, StopReason::Cancelled);
}
#[tokio::test]
async fn ignored_cancel_becomes_unknown() {
    let signal = ControlSignal::new();
    let (result, calls, _, _) = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        run(Some("ignore_cancel"), signal.clone()),
    )
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(result.answer.is_none());
    assert_eq!(result.reason, StopReason::OutcomeUnknown);
}

#[tokio::test]
async fn cancel_ack_variants_are_classified_after_started() {
    for mode in [
        "cancellation_wait",
        "ignore_cancel",
        "wrong_cancel_ack_ids",
        "cancel_ack_trailing_frame",
        "cancel_ack_nonzero_exit",
        "partial_frame_cancel",
    ] {
        let signal = ControlSignal::new();
        let (result, calls, _, records) =
            tokio::time::timeout(std::time::Duration::from_secs(8), run(Some(mode), signal))
                .await
                .unwrap();
        let expected = if mode == "cancellation_wait" || mode == "partial_frame_cancel" {
            StopReason::Cancelled
        } else {
            StopReason::OutcomeUnknown
        };
        assert_eq!(result.reason, expected, "{mode}");
        assert_eq!(calls.load(Ordering::SeqCst), 1, "{mode}");
        assert!(result.answer.is_none(), "{mode}");
        assert_eq!(records.len(), 1, "{mode}");
        assert!(records[0].dispatched, "{mode}");
        assert!(records[0].reaped, "{mode}");
        if mode == "cancellation_wait" || mode == "partial_frame_cancel" {
            assert_eq!(records[0].exit_code, Some(0), "{mode}");
        }
        if mode == "cancel_ack_nonzero_exit" {
            assert_eq!(records[0].exit_code, Some(17), "{mode}");
        }
    }
}

#[tokio::test]
async fn crash_modes_have_no_followup_model_call() {
    for mode in ["crash_before_ready", "crash_after_invoke"] {
        let (result, calls, _, _) = tokio::time::timeout(
            std::time::Duration::from_secs(8),
            run(Some(mode), ControlSignal::new()),
        )
        .await
        .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1, "{mode}");
        assert!(result.answer.is_none(), "{mode}");
        assert_eq!(
            result.reason,
            if mode == "crash_before_ready" {
                StopReason::NotDispatched
            } else {
                StopReason::OutcomeUnknown
            },
            "{mode}"
        );
    }
}

#[tokio::test]
async fn evidence_and_stale_target_faults_are_unknown() {
    for mode in [
        "wrong_evidence_task",
        "wrong_evidence_call",
        "wrong_evidence_path",
        "wrong_evidence_rule",
        "wrong_evidence_version",
        "stale_target",
    ] {
        let (result, calls, base, _) = tokio::time::timeout(
            std::time::Duration::from_secs(8),
            run(Some(mode), ControlSignal::new()),
        )
        .await
        .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1, "{mode}");
        assert!(result.answer.is_none(), "{mode}");
        assert_eq!(result.reason, StopReason::OutcomeUnknown, "{mode}");
        if mode == "stale_target" {
            assert_eq!(
                tokio::fs::read_to_string(base.join("marker.txt"))
                    .await
                    .unwrap(),
                "stale-target\n"
            );
        }
    }
}

#[tokio::test]
async fn bounded_host_failures_are_reaped() {
    for mode in ["ready_hang", "oversize_no_newline", "terminal_hang"] {
        let started = std::time::Instant::now();
        let (result, calls, _, records) = tokio::time::timeout(
            std::time::Duration::from_secs(8),
            run(Some(mode), ControlSignal::new()),
        )
        .await
        .unwrap();
        assert!(
            started.elapsed() < std::time::Duration::from_secs(7),
            "{mode}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1, "{mode}");
        assert!(result.answer.is_none(), "{mode}");
        assert_eq!(records.len(), 1, "{mode}");
        assert!(
            records[0].child_pid.is_some() && records[0].reaped,
            "{mode}"
        );
        if mode == "ready_hang" {
            assert_eq!(result.reason, StopReason::NotDispatched);
            assert!(!records[0].dispatched);
        } else {
            assert_eq!(result.reason, StopReason::OutcomeUnknown);
            assert!(records[0].dispatched);
        }
    }
}

#[tokio::test]
async fn predispatched_calls_have_no_child() {
    let base = std::env::temp_dir().join(format!("rein-pre-{}", std::process::id()));
    tokio::fs::create_dir_all(&base).await.unwrap();
    tokio::fs::write(base.join("marker.txt"), "x\n")
        .await
        .unwrap();
    let make = |name: &str, args: serde_json::Value| ToolCall {
        id: name.into(),
        name: name.into(),
        arguments: args,
    };
    let signal = ControlSignal::new();
    signal.cancel();
    let ex = StdioExecutor::new(&base);
    let r = ex
        .execute(
            &make("read_file", serde_json::json!({"path":"marker.txt"})),
            &signal,
            None,
        )
        .await;
    assert!(matches!(r, ExecutorResult::NotDispatched));
    let r = ex
        .execute(
            &make("write_file", serde_json::json!({"path":"marker.txt"})),
            &ControlSignal::new(),
            None,
        )
        .await;
    assert!(matches!(r, ExecutorResult::Completed(v) if !v.ok));
    let r = ex
        .execute(
            &ToolCall {
                id: "escape".into(),
                name: "read_file".into(),
                arguments: serde_json::json!({"path":"../outside"}),
            },
            &ControlSignal::new(),
            None,
        )
        .await;
    assert!(matches!(r, ExecutorResult::Completed(v) if !v.ok));
    assert_eq!(ex.records().len(), 3);
    for record in ex.records() {
        assert!(!record.dispatched);
        assert!(record.child_pid.is_none());
        assert!(!record.reaped);
        assert!(record.exit_code.is_none());
    }
    let a = StdioExecutor::new(&base);
    let b = StdioExecutor::new(&base);
    let signal_a = ControlSignal::new();
    signal_a.cancel();
    let signal_b = ControlSignal::new();
    signal_b.cancel();
    let call = make("read_file", serde_json::json!({"path":"marker.txt"}));
    assert!(matches!(
        a.execute(&call, &signal_a, None).await,
        ExecutorResult::NotDispatched
    ));
    assert!(matches!(
        b.execute(&call, &signal_b, None).await,
        ExecutorResult::NotDispatched
    ));
    assert_ne!(a.records()[0].task_id, b.records()[0].task_id);
}

#[tokio::test]
async fn zero_tool_budget_never_starts_host() {
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::temp_dir().join(format!(
        "rein-zero-budget-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    tokio::fs::create_dir(&base).await.unwrap();
    tokio::fs::write(base.join("marker.txt"), "fixture-output\n")
        .await
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = StdioExecutor::new(&base);
    let result = run_agent_loop_with_executor(
        &Fixture {
            calls: calls.clone(),
            path: "marker.txt".into(),
        },
        &Workspace { root: base },
        "read",
        LoopOptions {
            max_turns: 3,
            max_tool_calls: 0,
            ..LoopOptions::default()
        },
        &executor,
    )
    .await;
    assert_eq!(result.reason, StopReason::ToolBudgetExhausted);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(executor.records().is_empty());
}

#[tokio::test]
async fn disconnect_after_effect_records_one_ledger_line_and_is_unknown() {
    let (result, calls, base, _) = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        run(Some("disconnect_after_effect"), ControlSignal::new()),
    )
    .await
    .unwrap();
    let ledger = tokio::fs::read_to_string(base.join("ledger"))
        .await
        .unwrap_or_default();
    assert_eq!(
        ledger
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
        1
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(result.answer.is_none());
    assert_eq!(result.reason, StopReason::OutcomeUnknown);
}
