use rein_ch01_helloworld::rein::{
    run_agent_loop_with_adapter, run_agent_loop_with_options, ContextConfig, ControlSignal,
    LoopEvent, LoopOptions, LoopState, Message, ModelAdapter, ModelTurn, StopReason, ToolCall,
    ToolDefinition, ToolError, Workspace,
};
use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
    time::Duration,
};

fn turn(content: &str, calls: Vec<ToolCall>) -> ModelTurn {
    ModelTurn {
        message: Message {
            role: "assistant".into(),
            content: content.into(),
            tool_call_id: None,
            tool_calls: calls.clone(),
        },
        tool_calls: calls,
    }
}

struct DroppablePending(Arc<AtomicBool>);
impl Future for DroppablePending {
    type Output = Result<ModelTurn, ToolError>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}
impl Drop for DroppablePending {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

struct DropTrackingAdapter {
    dropped: Arc<AtomicBool>,
}
impl ModelAdapter for DropTrackingAdapter {
    fn complete<'a>(
        &'a self,
        _: &'a [Message],
        _: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        Box::pin(async { Ok(turn("unexpected", vec![])) })
    }
    fn complete_controlled<'a>(
        &'a self,
        _: &'a [Message],
        _: &'a [ToolDefinition],
        _: &'a ControlSignal,
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        Box::pin(DroppablePending(self.dropped.clone()))
    }
}
fn call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: name.into(),
        arguments: args,
    }
}
struct Replay(Arc<Mutex<VecDeque<Result<ModelTurn, ToolError>>>>);
impl ModelAdapter for Replay {
    fn complete<'a>(
        &'a self,
        _: &'a [Message],
        _: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        let value = self.0.lock().unwrap().pop_front().unwrap_or_else(|| {
            Err(ToolError {
                code: "replay_exhausted".into(),
                message: "replay exhausted".into(),
            })
        });
        Box::pin(async move { value })
    }
}

struct SearchReplay {
    requests: Arc<Mutex<Vec<Vec<Message>>>>,
}
impl ModelAdapter for SearchReplay {
    fn complete<'a>(
        &'a self,
        messages: &'a [Message],
        _: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        self.requests.lock().unwrap().push(messages.to_vec());
        let tool_messages: Vec<&Message> = messages.iter().filter(|m| m.role == "tool").collect();
        let response = if tool_messages.is_empty() {
            turn(
                "",
                vec![call(
                    "search",
                    "search_files",
                    serde_json::json!({"needle":"marker:"}),
                )],
            )
        } else if tool_messages.len() == 1 {
            let calls = tool_messages[0]
                .content
                .lines()
                .enumerate()
                .map(|(i, path)| {
                    call(
                        &format!("read-{i}"),
                        "read_file",
                        serde_json::json!({"path":path}),
                    )
                })
                .collect();
            turn("", calls)
        } else {
            turn(
                &tool_messages[1..]
                    .iter()
                    .map(|m| m.content.trim())
                    .collect::<Vec<_>>()
                    .join(" | "),
                vec![],
            )
        };
        Box::pin(async move { Ok(response) })
    }
}
fn workspace(label: &str, files: &[(&str, &str)]) -> Workspace {
    let root = std::env::temp_dir().join(format!("rein-ch05-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    for (path, content) in files {
        std::fs::write(root.join(path), content).unwrap();
    }
    Workspace { root }
}

struct ControlledReplay;
impl ModelAdapter for ControlledReplay {
    fn complete<'a>(
        &'a self,
        _: &'a [Message],
        _: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        Box::pin(async { Ok(turn("late", vec![])) })
    }
    fn complete_controlled<'a>(
        &'a self,
        _: &'a [Message],
        _: &'a [ToolDefinition],
        signal: &'a ControlSignal,
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        let signal = signal.clone();
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(40)).await;
            if signal.is_cancelled() {
                Ok(turn("late", vec![]))
            } else {
                Ok(turn("done", vec![]))
            }
        })
    }
}

#[tokio::test]
async fn controls_stop_pending_model_and_zero_budget_without_dispatch() {
    let ws = workspace("controls", &[]);
    let signal = ControlSignal::new();
    let to_cancel = signal.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(5)).await;
        to_cancel.cancel();
    });
    let cancelled = run_agent_loop_with_options(
        &ControlledReplay,
        &ws,
        "cancel",
        LoopOptions {
            signal: Some(signal),
            ..LoopOptions::default()
        },
    )
    .await;
    assert_eq!(cancelled.reason, StopReason::Cancelled);
    assert_eq!(
        cancelled
            .events
            .iter()
            .filter(|event| matches!(event, LoopEvent::ModelRequested { .. }))
            .count(),
        1
    );
    let zero = run_agent_loop_with_options(
        &ControlledReplay,
        &ws,
        "zero",
        LoopOptions {
            max_turns: 0,
            ..LoopOptions::default()
        },
    )
    .await;
    assert_eq!(zero.reason, StopReason::MaxTurns);
    assert_eq!(
        zero.events
            .iter()
            .filter(|event| matches!(event, LoopEvent::ModelRequested { .. }))
            .count(),
        0
    );
    let timed = run_agent_loop_with_options(
        &ControlledReplay,
        &ws,
        "timeout",
        LoopOptions {
            timeout: Some(Duration::from_millis(5)),
            ..LoopOptions::default()
        },
    )
    .await;
    assert_eq!(timed.reason, StopReason::Timeout);
    let _ = std::fs::remove_dir_all(ws.root);
}

#[tokio::test]
async fn observer_cancels_between_tools_and_pending_model_is_dropped() {
    let ws = workspace(
        "observer",
        &[("README.md", "marker: alpha"), ("guide.md", "marker: beta")],
    );
    let signal = ControlSignal::new();
    let observed = Arc::new(AtomicUsize::new(0));
    let observed_copy = observed.clone();
    let replay = Replay(Arc::new(Mutex::new(VecDeque::from([Ok(turn(
        "",
        vec![
            call(
                "first",
                "read_file",
                serde_json::json!({"path":"README.md"}),
            ),
            call(
                "second",
                "read_file",
                serde_json::json!({"path":"guide.md"}),
            ),
        ],
    ))]))));
    let result = run_agent_loop_with_options(
        &replay,
        &ws,
        "observe",
        LoopOptions {
            signal: Some(signal.clone()),
            tool_observer: Some(Arc::new(move |_, _, signal| {
                observed_copy.fetch_add(1, Ordering::SeqCst);
                signal.cancel();
            })),
            ..LoopOptions::default()
        },
    )
    .await;
    assert_eq!(result.reason, StopReason::Cancelled);
    assert_eq!(observed.load(Ordering::SeqCst), 1);
    assert_eq!(
        result
            .events
            .iter()
            .filter(|e| matches!(e, LoopEvent::ToolResult { .. }))
            .count(),
        1
    );
    assert!(result.events.iter().any(|e| matches!(e, LoopEvent::ActionSkipped { call_id: Some(id), reason: StopReason::Cancelled, .. } if id == "second")));

    let dropped = Arc::new(AtomicBool::new(false));
    let cancel = ControlSignal::new();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(5)).await;
        trigger.cancel();
    });
    let pending = run_agent_loop_with_options(
        &DropTrackingAdapter {
            dropped: dropped.clone(),
        },
        &ws,
        "pending",
        LoopOptions {
            signal: Some(cancel),
            ..LoopOptions::default()
        },
    )
    .await;
    assert_eq!(pending.reason, StopReason::Cancelled);
    assert!(
        dropped.load(Ordering::SeqCst),
        "cancellation must drop the pending adapter future"
    );
    let _ = std::fs::remove_dir_all(ws.root);
}

#[tokio::test]
async fn nested_arguments_are_canonical_for_duplicate_detection() {
    let ws = workspace("nested-duplicate", &[("README.md", "alpha")]);
    let replay = Replay(Arc::new(Mutex::new(VecDeque::from([
        Ok(turn(
            "",
            vec![call(
                "first",
                "read_file",
                serde_json::json!({"path":"README.md", "nested":{"b":2,"a":1}}),
            )],
        )),
        Ok(turn(
            "",
            vec![call(
                "second",
                "read_file",
                serde_json::json!({"nested":{"a":1,"b":2}, "path":"README.md"}),
            )],
        )),
    ]))));
    let result = run_agent_loop_with_options(
        &replay,
        &ws,
        "duplicate",
        LoopOptions {
            duplicate_limit: 1,
            ..LoopOptions::default()
        },
    )
    .await;
    assert_eq!(result.reason, StopReason::DuplicateAction);
    assert!(result.events.iter().any(|event| matches!(event, LoopEvent::ActionSkipped { call_id: Some(id), reason: StopReason::DuplicateAction, .. } if id == "second")));
    let _ = std::fs::remove_dir_all(ws.root);
}

#[tokio::test]
async fn two_workspaces_search_then_read_and_pair_multiple_calls() {
    for (label, files, expected) in [
        (
            "blue",
            vec![("a.txt", "marker: blue"), ("b.txt", "marker: sea")],
            "marker: blue | marker: sea",
        ),
        (
            "gold",
            vec![("x.txt", "marker: gold"), ("y.txt", "marker: wind")],
            "marker: gold | marker: wind",
        ),
    ] {
        let ws = workspace(label, &files);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let result = run_agent_loop_with_adapter(
            &SearchReplay {
                requests: requests.clone(),
            },
            &ws,
            "inspect",
            32,
        )
        .await;
        assert_eq!(result.reason, StopReason::FinalAnswer);
        assert_eq!(result.answer.as_deref(), Some(expected));
        assert_eq!(
            result
                .events
                .iter()
                .filter_map(|event| match event {
                    LoopEvent::ToolResult { tool_call_id, .. } => Some(tool_call_id.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            vec!["search", "read-0", "read-1"]
        );
        assert!(result.messages.iter().any(|m| m.content == expected));
        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 3);
        let third_tools: Vec<_> = captured[2]
            .iter()
            .filter(|m| m.role == "tool")
            .skip(1)
            .collect();
        assert_eq!(
            third_tools
                .iter()
                .map(|m| m.tool_call_id.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("read-0"), Some("read-1")]
        );
        assert_eq!(
            third_tools
                .iter()
                .map(|m| m.content.as_str())
                .collect::<Vec<_>>(),
            files
                .iter()
                .map(|(_, content)| *content)
                .collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(ws.root);
    }
}

#[tokio::test]
async fn unknown_badargs_missing_file_are_feedback_and_can_recover() {
    let ws = workspace("recover", &[("ok.txt", "recovered")]);
    let replay = Replay(Arc::new(Mutex::new(VecDeque::from([
        Ok(turn(
            "",
            vec![
                call("u", "unknown", serde_json::json!({})),
                call("a", "read_file", serde_json::json!({"path": 3})),
                call("m", "read_file", serde_json::json!({"path":"missing"})),
            ],
        )),
        Ok(turn(
            "recovered",
            vec![call("r", "read_file", serde_json::json!({"path":"ok.txt"}))],
        )),
        Ok(turn("done", vec![])),
    ]))));
    let result = run_agent_loop_with_adapter(&replay, &ws, "recover", 32).await;
    assert_eq!(result.reason, StopReason::FinalAnswer);
    assert!(result.events.iter().any(|event| matches!(event, LoopEvent::ToolResult { result, .. } if result.error.as_ref().map(|e| e.code.as_str()) == Some("unknown_tool"))));
    assert!(result.events.iter().any(|event| matches!(event, LoopEvent::ToolResult { result, .. } if result.error.as_ref().map(|e| e.code.as_str()) == Some("arguments_invalid"))));
    assert!(result.events.iter().any(|event| matches!(event, LoopEvent::ToolResult { result, .. } if result.error.as_ref().map(|e| e.code.as_str()) == Some("path_invalid"))));
    let _ = std::fs::remove_dir_all(ws.root);
}

#[tokio::test]
async fn model_error_empty_final_max_turns_and_replay_exhaustion_are_structured() {
    let ws = workspace("boundaries", &[]);
    let failed = run_agent_loop_with_adapter(
        &Replay(Arc::new(Mutex::new(VecDeque::from([Err(ToolError {
            code: "model_error".into(),
            message: "offline".into(),
        })])))),
        &ws,
        "x",
        32,
    )
    .await;
    assert_eq!(failed.reason, StopReason::ModelError);
    assert_eq!(failed.state, LoopState::Failed);
    let empty = run_agent_loop_with_adapter(
        &Replay(Arc::new(Mutex::new(VecDeque::from([Ok(turn(
            "  ",
            vec![],
        ))])))),
        &ws,
        "x",
        32,
    )
    .await;
    assert_eq!(empty.reason, StopReason::EmptyFinal);
    let max_responses = (0..32)
        .map(|_| Ok(turn("", vec![call("u", "unknown", serde_json::json!({}))])))
        .collect();
    let max =
        run_agent_loop_with_adapter(&Replay(Arc::new(Mutex::new(max_responses))), &ws, "x", 32)
            .await;
    assert_eq!(max.reason, StopReason::MaxTurns);
    assert_eq!(
        max.events
            .iter()
            .filter(|e| matches!(e, LoopEvent::ModelRequested { .. }))
            .count(),
        32
    );
    let exhausted = run_agent_loop_with_adapter(
        &Replay(Arc::new(Mutex::new(VecDeque::from([Ok(turn(
            "",
            vec![call("s", "search_files", serde_json::json!({"needle":"x"}))],
        ))])))),
        &ws,
        "x",
        2,
    )
    .await;
    assert_eq!(exhausted.reason, StopReason::ModelError);
    assert_eq!(exhausted.error.as_deref(), Some("replay exhausted"));
    let _ = std::fs::remove_dir_all(ws.root);
}

#[test]
fn loop_wire_values_are_snake_case_and_fields_camel_case() {
    let value = serde_json::to_value(StopReason::MaxTurns).unwrap();
    assert_eq!(value, "max_turns");
    let value = serde_json::to_value(LoopEvent::ModelReceived {
        turn: 1,
        message: Message {
            role: "assistant".into(),
            content: "x".into(),
            tool_call_id: None,
            tool_calls: vec![],
        },
        tool_call_ids: vec!["a".into()],
        text: "x".into(),
    })
    .unwrap();
    assert_eq!(value["type"], "model_received");
    assert!(value.get("toolCallIds").is_some());
}

struct ContextCapture(Arc<Mutex<Vec<Vec<Message>>>>);
impl ModelAdapter for ContextCapture {
    fn complete<'a>(
        &'a self,
        messages: &'a [Message],
        _: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        self.0.lock().unwrap().push(messages.to_vec());
        Box::pin(async { Ok(turn("答案", vec![])) })
    }
}

#[tokio::test]
async fn ch07_context_keeps_goal_and_latest_tool_group_but_audits_all_history() {
    let ws = workspace("ch07-context", &[]);
    let captured = Arc::new(Mutex::new(Vec::new()));
    let history = vec![
        Message {
            role: "user".into(),
            content: "old".into(),
            tool_call_id: None,
            tool_calls: vec![],
        },
        Message {
            role: "assistant".into(),
            content: "".into(),
            tool_call_id: None,
            tool_calls: vec![call(
                "old-call",
                "read_file",
                serde_json::json!({"path":"a"}),
            )],
        },
        Message {
            role: "tool".into(),
            content: "old-result".into(),
            tool_call_id: Some("old-call".into()),
            tool_calls: vec![],
        },
    ];
    let result = run_agent_loop_with_options(
        &ContextCapture(captured.clone()),
        &ws,
        "goal",
        LoopOptions {
            max_turns: 1,
            context: Some(ContextConfig {
                rules: vec!["rule".into()],
                history,
                budget: 125,
                manage: true,
            }),
            ..LoopOptions::default()
        },
    )
    .await;
    assert_eq!(result.answer.as_deref(), Some("答案"));
    assert_eq!(result.messages.len(), 5);
    let sent = &captured.lock().unwrap()[0];
    assert!(sent
        .iter()
        .any(|m| m.role == "system" && m.content == "rule"));
    assert!(sent.iter().any(|m| m.content == "goal"));
    let prepared = result
        .events
        .iter()
        .find_map(|e| {
            if let LoopEvent::ContextPrepared {
                kept_groups,
                removed_groups,
                ..
            } = e
            {
                Some((kept_groups, removed_groups))
            } else {
                None
            }
        })
        .unwrap();
    assert!(prepared.0.contains(&"g3".into()));
    assert!(prepared.1.contains(&"g0".into()));
}

#[tokio::test]
async fn ch07_context_rejects_invalid_history_and_required_overflow_without_dispatch() {
    let ws = workspace("ch07-invalid", &[]);
    let captured = Arc::new(Mutex::new(Vec::new()));
    let orphan = Message {
        role: "tool".into(),
        content: "orphan".into(),
        tool_call_id: Some("x".into()),
        tool_calls: vec![],
    };
    let invalid = run_agent_loop_with_options(
        &ContextCapture(captured.clone()),
        &ws,
        "goal",
        LoopOptions {
            context: Some(ContextConfig {
                rules: vec![],
                history: vec![orphan],
                budget: 100,
                manage: true,
            }),
            ..LoopOptions::default()
        },
    )
    .await;
    assert_eq!(invalid.reason, StopReason::InvalidContextHistory);
    assert!(captured.lock().unwrap().is_empty());
    let overflow = run_agent_loop_with_options(
        &ContextCapture(captured.clone()),
        &ws,
        "long goal",
        LoopOptions {
            context: Some(ContextConfig {
                rules: vec!["large rule".repeat(20)],
                history: vec![],
                budget: 1,
                manage: true,
            }),
            ..LoopOptions::default()
        },
    )
    .await;
    assert_eq!(overflow.reason, StopReason::ContextBudgetExhausted);
    assert!(captured.lock().unwrap().is_empty());
}

#[tokio::test]
async fn ch07_context_rejects_unsafe_integer_budget_as_invalid_context_config() {
    let ws = workspace("ch07-unsafe-budget", &[]);
    let captured = Arc::new(Mutex::new(Vec::new()));
    let result = run_agent_loop_with_options(
        &ContextCapture(captured.clone()),
        &ws,
        "goal",
        LoopOptions {
            context: Some(ContextConfig {
                rules: vec![],
                history: vec![],
                budget: 9_007_199_254_740_992,
                manage: true,
            }),
            ..LoopOptions::default()
        },
    )
    .await;

    assert_eq!(result.reason, StopReason::InvalidContextHistory);
    assert_eq!(result.error.as_deref(), Some("invalid_context_config"));
    assert!(captured.lock().unwrap().is_empty());
    assert!(result.events.iter().any(|event| matches!(
        event,
        LoopEvent::ContextRejected {
            reason,
            error_code: Some(error_code),
            budget: None,
            before_units: None,
            required_units: None,
            ..
        } if reason == "invalid_context_history" && error_code == "invalid_context_config"
    )));
}

struct Ch07RoundAdapter {
    requests: Arc<Mutex<Vec<Vec<Message>>>>,
    duplicate_id: bool,
}
impl ModelAdapter for Ch07RoundAdapter {
    fn complete<'a>(
        &'a self,
        messages: &'a [Message],
        _: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        let requests = self.requests.clone();
        let round = requests.lock().unwrap().len();
        requests.lock().unwrap().push(messages.to_vec());
        let tool_count = messages.iter().filter(|m| m.role == "tool").count();
        let id = if self.duplicate_id {
            "same-id"
        } else if tool_count == 0 {
            "first-id"
        } else {
            "second-id"
        };
        let response = if tool_count == 0 {
            turn(
                "",
                vec![call(id, "read_file", serde_json::json!({"path":"a.txt"}))],
            )
        } else if tool_count == 1 && round < 2 {
            turn(
                "",
                vec![call(id, "read_file", serde_json::json!({"path":"b.txt"}))],
            )
        } else {
            turn(
                &messages
                    .iter()
                    .rev()
                    .find(|m| m.role == "tool")
                    .map(|m| m.content.clone())
                    .unwrap_or_default(),
                vec![],
            )
        };
        Box::pin(async move { Ok(response) })
    }
}

#[tokio::test]
async fn ch07_context_tracks_latest_group_across_real_tool_rounds() {
    let ws = workspace(
        "ch07-rounds",
        &[("a.txt", "first fact"), ("b.txt", "second fact")],
    );
    let requests = Arc::new(Mutex::new(Vec::new()));
    let result = run_agent_loop_with_options(
        &Ch07RoundAdapter {
            requests: requests.clone(),
            duplicate_id: false,
        },
        &ws,
        "read both",
        LoopOptions {
            max_turns: 4,
            context: Some(ContextConfig {
                rules: vec!["keep facts".into()],
                history: vec![],
                budget: 180,
                manage: true,
            }),
            ..LoopOptions::default()
        },
    )
    .await;
    assert_eq!(result.reason, StopReason::FinalAnswer);
    assert!(result.answer.as_deref().unwrap().contains("second fact"));
    let prepared: Vec<(Vec<String>, Vec<String>)> = result
        .events
        .iter()
        .filter_map(|event| match event {
            LoopEvent::ContextPrepared {
                required_groups,
                removed_groups,
                ..
            } => Some((required_groups.clone(), removed_groups.clone())),
            _ => None,
        })
        .collect();
    assert!(prepared.len() >= 3);
    assert!(prepared[1].0.iter().any(|id| id == "g1"));
    assert!(prepared[2].0.iter().any(|id| id == "g3"));
    assert!(prepared[2].1.iter().any(|id| id == "g1"));
    assert_eq!(
        result.messages.iter().filter(|m| m.role == "tool").count(),
        2
    );
}

#[tokio::test]
async fn ch07_context_stops_when_new_latest_group_does_not_fit() {
    let ws = workspace(
        "ch07-budget",
        &[("a.txt", &"x".repeat(400)), ("b.txt", "unused")],
    );
    let requests = Arc::new(Mutex::new(Vec::new()));
    let result = run_agent_loop_with_options(
        &Ch07RoundAdapter {
            requests: requests.clone(),
            duplicate_id: false,
        },
        &ws,
        "read",
        LoopOptions {
            max_turns: 4,
            context: Some(ContextConfig {
                rules: vec![],
                history: vec![],
                budget: 80,
                manage: true,
            }),
            ..LoopOptions::default()
        },
    )
    .await;
    assert_eq!(result.reason, StopReason::ContextBudgetExhausted);
    assert_eq!(requests.lock().unwrap().len(), 1);
    assert!(result.events.iter().any(|event| matches!(event, LoopEvent::ContextRejected { reason, .. } if reason == "context_budget_exhausted")));
}

#[tokio::test]
async fn ch07_context_allows_adjacent_seeded_groups_but_rejects_new_duplicate_id() {
    let seeded = vec![
        Message {
            role: "assistant".into(),
            content: "".into(),
            tool_call_id: None,
            tool_calls: vec![call("a", "read_file", serde_json::json!({"path":"a.txt"}))],
        },
        Message {
            role: "tool".into(),
            content: "A".into(),
            tool_call_id: Some("a".into()),
            tool_calls: vec![],
        },
        Message {
            role: "assistant".into(),
            content: "".into(),
            tool_call_id: None,
            tool_calls: vec![call("b", "read_file", serde_json::json!({"path":"b.txt"}))],
        },
        Message {
            role: "tool".into(),
            content: "B".into(),
            tool_call_id: Some("b".into()),
            tool_calls: vec![],
        },
    ];
    let ws = workspace("ch07-adjacent", &[("a.txt", "A"), ("b.txt", "B")]);
    let ok_requests = Arc::new(Mutex::new(Vec::new()));
    let ok = run_agent_loop_with_options(
        &Ch07RoundAdapter {
            requests: ok_requests.clone(),
            duplicate_id: false,
        },
        &ws,
        "goal",
        LoopOptions {
            max_turns: 1,
            context: Some(ContextConfig {
                rules: vec![],
                history: seeded.clone(),
                budget: 300,
                manage: true,
            }),
            ..LoopOptions::default()
        },
    )
    .await;
    assert_eq!(ok.reason, StopReason::FinalAnswer);
    let duplicate_requests = Arc::new(Mutex::new(Vec::new()));
    let duplicate = run_agent_loop_with_options(
        &Ch07RoundAdapter {
            requests: duplicate_requests.clone(),
            duplicate_id: true,
        },
        &ws,
        "goal",
        LoopOptions {
            max_turns: 3,
            context: Some(ContextConfig {
                rules: vec![],
                history: vec![],
                budget: 300,
                manage: true,
            }),
            ..LoopOptions::default()
        },
    )
    .await;
    assert_eq!(duplicate.reason, StopReason::InvalidContextHistory);
    assert_eq!(duplicate_requests.lock().unwrap().len(), 2);
}
