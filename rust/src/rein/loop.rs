//! Chapter 05's typed, provider-independent agent loop.
use super::{
    dispatch_readonly, openai_complete, Message, ModelTurn, OpenAiHttp, ToolCall, ToolDefinition,
    ToolError, ToolResult, Workspace,
};
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

pub trait ModelAdapter {
    fn complete<'a>(
        &'a self,
        messages: &'a [Message],
        tools: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>>;
    fn complete_controlled<'a>(
        &'a self,
        messages: &'a [Message],
        tools: &'a [ToolDefinition],
        _signal: &'a ControlSignal,
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        self.complete(messages, tools)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ControlSignal(Arc<AtomicBool>);
impl ControlSignal {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// A provided trusted local observer after a real tool has completed.
///
/// This example only uses it to cancel a run; it provides no permission
/// isolation, so a caller-supplied closure remains able to mutate its own
/// captured state (including the workspace).
pub type ToolObserver = Arc<dyn Fn(&ToolCall, &ToolResult, &ControlSignal) + Send + Sync>;

/// Executes one tool call. The core owns sequencing and budgets; hosts only
/// provide this narrow capability boundary.
pub trait ToolExecutor: Send + Sync {
    fn execute<'a>(
        &'a self,
        call: &'a ToolCall,
        signal: &'a ControlSignal,
        deadline: Option<Instant>,
    ) -> Pin<Box<dyn Future<Output = ExecutorResult> + Send + 'a>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutorResult {
    Completed(ToolResult),
    Cancelled,
    OutcomeUnknown,
    NotDispatched,
}

struct LocalReadonlyExecutor<'a> {
    workspace: &'a Workspace,
}
impl ToolExecutor for LocalReadonlyExecutor<'_> {
    fn execute<'a>(
        &'a self,
        call: &'a ToolCall,
        _signal: &'a ControlSignal,
        _deadline: Option<Instant>,
    ) -> Pin<Box<dyn Future<Output = ExecutorResult> + Send + 'a>> {
        Box::pin(async move { ExecutorResult::Completed(dispatch_readonly(call, self.workspace)) })
    }
}

pub struct OpenAiModelAdapter<'a, H: OpenAiHttp> {
    http: &'a H,
    base_url: &'a str,
    api_key: &'a str,
    model: &'a str,
}
impl<'a, H: OpenAiHttp> OpenAiModelAdapter<'a, H> {
    pub fn new(http: &'a H, base_url: &'a str, api_key: &'a str, model: &'a str) -> Self {
        Self {
            http,
            base_url,
            api_key,
            model,
        }
    }
}
impl<'a, H: OpenAiHttp> ModelAdapter for OpenAiModelAdapter<'a, H> {
    fn complete<'b>(
        &'b self,
        messages: &'b [Message],
        tools: &'b [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'b>> {
        Box::pin(openai_complete(
            self.http,
            self.base_url,
            self.api_key,
            self.model,
            messages,
            tools,
        ))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoopState {
    Running,
    Completed,
    Failed,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    FinalAnswer,
    EmptyFinal,
    ModelError,
    MaxTurns,
    ToolBudgetExhausted,
    DuplicateAction,
    Cancelled,
    Timeout,
    ContextBudgetExhausted,
    InvalidContextHistory,
    NotDispatched,
    OutcomeUnknown,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LoopEvent {
    ContextPrepared {
        turn: usize,
        unit: String,
        budget: usize,
        #[serde(rename = "beforeUnits")]
        before_units: usize,
        #[serde(rename = "afterUnits")]
        after_units: usize,
        #[serde(rename = "requiredUnits")]
        required_units: usize,
        #[serde(rename = "keptGroups")]
        kept_groups: Vec<String>,
        #[serde(rename = "removedGroups")]
        removed_groups: Vec<String>,
        #[serde(rename = "requiredGroups")]
        required_groups: Vec<String>,
        mode: ContextMode,
    },
    ContextRejected {
        turn: usize,
        reason: String,
        unit: String,
        mode: ContextMode,
        #[serde(rename = "errorCode", skip_serializing_if = "Option::is_none")]
        error_code: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        budget: Option<usize>,
        #[serde(rename = "beforeUnits")]
        #[serde(skip_serializing_if = "Option::is_none")]
        before_units: Option<usize>,
        #[serde(rename = "requiredUnits")]
        #[serde(skip_serializing_if = "Option::is_none")]
        required_units: Option<usize>,
    },
    ModelRequested {
        turn: usize,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
    },
    ModelReceived {
        turn: usize,
        message: Message,
        #[serde(rename = "toolCallIds")]
        tool_call_ids: Vec<String>,
        text: String,
    },
    ToolResult {
        turn: usize,
        call: ToolCall,
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        result: ToolResult,
    },
    ActionSkipped {
        turn: usize,
        action: String,
        #[serde(rename = "callId", skip_serializing_if = "Option::is_none")]
        call_id: Option<String>,
        reason: StopReason,
    },
    Stopped {
        state: LoopState,
        reason: StopReason,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContextMode {
    Managed,
    Unmanaged,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextConfig {
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub history: Vec<Message>,
    pub budget: usize,
    #[serde(default = "default_manage")]
    pub manage: bool,
}

fn default_manage() -> bool {
    true
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopResult {
    pub state: LoopState,
    pub reason: StopReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    pub messages: Vec<Message>,
    pub events: Vec<LoopEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
pub const READONLY_TOOLS: &[(&str, &str)] = &[
    ("search_files", "Search text in workspace files."),
    ("read_file", "Read one UTF-8 file in the workspace."),
];

pub async fn run_agent_loop<H: OpenAiHttp>(
    http: &H,
    base_url: &str,
    api_key: &str,
    model: &str,
    workspace: &Workspace,
    prompt: &str,
    max_turns: usize,
) -> LoopResult {
    run_agent_loop_with_adapter(
        &OpenAiModelAdapter::new(http, base_url, api_key, model),
        workspace,
        prompt,
        max_turns,
    )
    .await
}
pub async fn run_agent_loop_with_adapter<A: ModelAdapter>(
    adapter: &A,
    workspace: &Workspace,
    prompt: &str,
    max_turns: usize,
) -> LoopResult {
    run_agent_loop_with_options(
        adapter,
        workspace,
        prompt,
        LoopOptions {
            max_turns,
            ..LoopOptions::default()
        },
    )
    .await
}

#[derive(Clone)]
pub struct LoopOptions {
    pub max_turns: usize,
    pub max_tool_calls: usize,
    pub duplicate_limit: usize,
    pub timeout: Option<Duration>,
    pub signal: Option<ControlSignal>,
    pub tool_observer: Option<ToolObserver>,
    pub context: Option<ContextConfig>,
}
impl Default for LoopOptions {
    fn default() -> Self {
        Self {
            max_turns: 32,
            max_tool_calls: usize::MAX,
            duplicate_limit: usize::MAX,
            timeout: None,
            signal: None,
            tool_observer: None,
            context: None,
        }
    }
}

pub async fn run_agent_loop_with_options<A: ModelAdapter>(
    adapter: &A,
    workspace: &Workspace,
    prompt: &str,
    options: LoopOptions,
) -> LoopResult {
    run_agent_loop_with_executor(
        adapter,
        workspace,
        prompt,
        options,
        &LocalReadonlyExecutor { workspace },
    )
    .await
}

pub async fn run_agent_loop_with_executor<A: ModelAdapter, E: ToolExecutor>(
    adapter: &A,
    _workspace: &Workspace,
    prompt: &str,
    options: LoopOptions,
    executor: &E,
) -> LoopResult {
    let context = options.context.clone();
    let history_len = context.as_ref().map(|c| c.history.len()).unwrap_or(0);
    let mut messages = context
        .as_ref()
        .map(|c| c.history.clone())
        .unwrap_or_default();
    let mut events = Vec::new();
    messages.push(Message {
        role: "user".into(),
        content: prompt.into(),
        tool_call_id: None,
        tool_calls: vec![],
    });
    if let Some(config) = &context {
        if config.budget > 9_007_199_254_740_991 {
            events.push(LoopEvent::ContextRejected {
                turn: 0,
                reason: "invalid_context_history".into(),
                error_code: Some("invalid_context_config".into()),
                unit: CONTEXT_UNIT.into(),
                budget: None,
                before_units: None,
                required_units: None,
                mode: if config.manage {
                    ContextMode::Managed
                } else {
                    ContextMode::Unmanaged
                },
            });
            return finish_loop(
                messages,
                events,
                LoopState::Failed,
                StopReason::InvalidContextHistory,
                None,
                Some("invalid_context_config".into()),
            );
        }
        if let Err(code) = validate_context_history(&config.history) {
            events.push(LoopEvent::ContextRejected {
                turn: 0,
                reason: "invalid_context_history".into(),
                error_code: Some(code.into()),
                unit: CONTEXT_UNIT.into(),
                budget: None,
                before_units: None,
                required_units: None,
                mode: if config.manage {
                    ContextMode::Managed
                } else {
                    ContextMode::Unmanaged
                },
            });
            return finish_loop(
                messages,
                events,
                LoopState::Failed,
                StopReason::InvalidContextHistory,
                None,
                Some(code.into()),
            );
        }
    }
    let definitions = READONLY_TOOLS.iter().map(|(name, description)| ToolDefinition { name: (*name).into(), description: (*description).into(), input_schema: match *name {
        "read_file" => serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}),
        "search_files" => serde_json::json!({"type":"object","properties":{"needle":{"type":"string"}},"required":["needle"],"additionalProperties":false}), _ => serde_json::json!({"type":"object"}),
    }}).collect::<Vec<_>>();
    let signal = options.signal.unwrap_or_default();
    let deadline = options.timeout.map(|d| Instant::now() + d);
    if options.max_turns == 0 {
        return finish_loop(
            messages,
            events,
            LoopState::Failed,
            StopReason::MaxTurns,
            None,
            None,
        );
    }
    if let Some(reason) = control_reason(&signal, deadline) {
        return finish_loop(messages, events, LoopState::Failed, reason, None, None);
    }
    let mut tool_started = 0usize;
    let mut seen = std::collections::HashMap::<String, usize>::new();
    for turn in 1..=options.max_turns {
        if let Some(reason) = control_reason(&signal, deadline) {
            return finish_loop(messages, events, LoopState::Failed, reason, None, None);
        }
        if context.is_some() && validate_context_history(&messages).is_err() {
            let config = context.as_ref().unwrap();
            events.push(LoopEvent::ContextRejected {
                turn,
                reason: "invalid_context_history".into(),
                error_code: Some("invalid_context_history".into()),
                unit: CONTEXT_UNIT.into(),
                budget: None,
                before_units: None,
                required_units: None,
                mode: if config.manage {
                    ContextMode::Managed
                } else {
                    ContextMode::Unmanaged
                },
            });
            return finish_loop(
                messages,
                events,
                LoopState::Failed,
                StopReason::InvalidContextHistory,
                None,
                Some("invalid_context_history".into()),
            );
        }
        let sent_messages =
            match prepare_context(&messages, history_len, context.as_ref(), turn, &mut events) {
                Ok(value) => value,
                Err(reason) => {
                    return finish_loop(messages, events, LoopState::Failed, reason, None, None)
                }
            };
        events.push(LoopEvent::ModelRequested {
            turn,
            messages: sent_messages.clone(),
            tools: definitions.clone(),
        });
        let remaining = deadline.map(|d| d.saturating_duration_since(Instant::now()));
        let response = match await_control(
            adapter.complete_controlled(&sent_messages, &definitions, &signal),
            &signal,
            remaining,
        )
        .await
        {
            Ok(value) => value,
            Err(error) => {
                if let Some(reason) = control_reason(&signal, deadline) {
                    return finish_loop(messages, events, LoopState::Failed, reason, None, None);
                }
                return finish_loop(
                    messages,
                    events,
                    LoopState::Failed,
                    StopReason::ModelError,
                    None,
                    Some(error.message),
                );
            }
        };
        if let Some(reason) = control_reason(&signal, deadline) {
            return finish_loop(messages, events, LoopState::Failed, reason, None, None);
        }
        let ids = response
            .tool_calls
            .iter()
            .map(|call| call.id.clone())
            .collect();
        events.push(LoopEvent::ModelReceived {
            turn,
            message: response.message.clone(),
            tool_call_ids: ids,
            text: response.message.content.clone(),
        });
        messages.push(response.message.clone());
        if response.tool_calls.is_empty() {
            if response.message.content.trim().is_empty() {
                return finish_loop(
                    messages,
                    events,
                    LoopState::Failed,
                    StopReason::EmptyFinal,
                    None,
                    None,
                );
            }
            return finish_loop(
                messages,
                events,
                LoopState::Completed,
                StopReason::FinalAnswer,
                Some(response.message.content),
                None,
            );
        }
        if !response.tool_calls.is_empty() && tool_started >= options.max_tool_calls {
            for call in &response.tool_calls {
                events.push(LoopEvent::ActionSkipped {
                    turn,
                    action: "tool".into(),
                    call_id: Some(call.id.clone()),
                    reason: StopReason::ToolBudgetExhausted,
                });
            }
            return finish_loop(
                messages,
                events,
                LoopState::Failed,
                StopReason::ToolBudgetExhausted,
                None,
                None,
            );
        }
        let calls = response.tool_calls.clone();
        for (index, call) in calls.into_iter().enumerate() {
            if let Some(reason) = control_reason(&signal, deadline) {
                for skipped in response.tool_calls.iter().skip(index) {
                    events.push(LoopEvent::ActionSkipped {
                        turn,
                        action: "tool".into(),
                        call_id: Some(skipped.id.clone()),
                        reason: reason.clone(),
                    });
                }
                return finish_loop(messages, events, LoopState::Failed, reason, None, None);
            }
            if tool_started >= options.max_tool_calls {
                for skipped in response.tool_calls.iter().skip(index) {
                    events.push(LoopEvent::ActionSkipped {
                        turn,
                        action: "tool".into(),
                        call_id: Some(skipped.id.clone()),
                        reason: StopReason::ToolBudgetExhausted,
                    });
                }
                return finish_loop(
                    messages,
                    events,
                    LoopState::Failed,
                    StopReason::ToolBudgetExhausted,
                    None,
                    None,
                );
            }
            let identity = format!("{}\0{}", call.name, canonical_json(&call.arguments));
            let occurrence = seen.get(&identity).copied().unwrap_or(0) + 1;
            if occurrence > options.duplicate_limit {
                for skipped in response.tool_calls.iter().skip(index) {
                    events.push(LoopEvent::ActionSkipped {
                        turn,
                        action: "tool".into(),
                        call_id: Some(skipped.id.clone()),
                        reason: StopReason::DuplicateAction,
                    });
                }
                return finish_loop(
                    messages,
                    events,
                    LoopState::Failed,
                    StopReason::DuplicateAction,
                    None,
                    None,
                );
            }
            seen.insert(identity, occurrence);
            tool_started += 1;
            let result = executor.execute(&call, &signal, deadline).await;
            let result = match result {
                ExecutorResult::Completed(result) => {
                    if result.tool_call_id != call.id
                        || (result.ok && (result.output.is_none() || result.error.is_some()))
                        || (!result.ok && (result.error.is_none() || result.output.is_some()))
                    {
                        for skipped in response.tool_calls.iter().skip(index + 1) {
                            events.push(LoopEvent::ActionSkipped {
                                turn,
                                action: "tool".into(),
                                call_id: Some(skipped.id.clone()),
                                reason: StopReason::OutcomeUnknown,
                            });
                        }
                        return finish_loop(
                            messages,
                            events,
                            LoopState::Failed,
                            StopReason::OutcomeUnknown,
                            None,
                            None,
                        );
                    }
                    result
                }
                other => {
                    let (reason, first_skipped) = match other {
                        ExecutorResult::Cancelled => (StopReason::Cancelled, index + 1),
                        ExecutorResult::NotDispatched => (StopReason::NotDispatched, index),
                        _ => (StopReason::OutcomeUnknown, index + 1),
                    };
                    for skipped in response.tool_calls.iter().skip(first_skipped) {
                        events.push(LoopEvent::ActionSkipped {
                            turn,
                            action: "tool".into(),
                            call_id: Some(skipped.id.clone()),
                            reason: reason.clone(),
                        });
                    }
                    return finish_loop(messages, events, LoopState::Failed, reason, None, None);
                }
            };
            events.push(LoopEvent::ToolResult {
                turn,
                call: call.clone(),
                tool_call_id: call.id.clone(),
                result: result.clone(),
            });
            let content = if result.ok {
                result.output.clone().unwrap_or_default()
            } else {
                serde_json::json!({"ok":false,"error":result.error}).to_string()
            };
            messages.push(Message {
                role: "tool".into(),
                content,
                tool_call_id: Some(result.tool_call_id.clone()),
                tool_calls: vec![],
            });
            if let Some(observer) = &options.tool_observer {
                observer(&call, &result, &signal);
            }
            if let Some(reason) = control_reason(&signal, deadline) {
                for skipped in response.tool_calls.iter().skip(index + 1) {
                    events.push(LoopEvent::ActionSkipped {
                        turn,
                        action: "tool".into(),
                        call_id: Some(skipped.id.clone()),
                        reason: reason.clone(),
                    });
                }
                return finish_loop(messages, events, LoopState::Failed, reason, None, None);
            }
        }
    }
    let reason = control_reason(&signal, deadline).unwrap_or(StopReason::MaxTurns);
    finish_loop(messages, events, LoopState::Failed, reason, None, None)
}

fn control_reason(signal: &ControlSignal, deadline: Option<Instant>) -> Option<StopReason> {
    if signal.is_cancelled() {
        Some(StopReason::Cancelled)
    } else if deadline.is_some_and(|d| Instant::now() >= d) {
        Some(StopReason::Timeout)
    } else {
        None
    }
}

async fn await_control<T>(
    work: impl Future<Output = Result<T, ToolError>>,
    signal: &ControlSignal,
    timeout: Option<Duration>,
) -> Result<T, ToolError> {
    let watch = async {
        let start = Instant::now();
        loop {
            if signal.is_cancelled() {
                return Err(ToolError {
                    code: "cancelled".into(),
                    message: "cancelled".into(),
                });
            }
            if timeout.is_some_and(|d| start.elapsed() >= d) {
                return Err(ToolError {
                    code: "timeout".into(),
                    message: "deadline exceeded".into(),
                });
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    };
    tokio::select! { value = work => value, value = watch => value }
}

fn canonical_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            format!(
                "{{{}}}",
                keys.into_iter()
                    .map(|k| format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap(),
                        canonical_json(&map[k])
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        serde_json::Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        other => other.to_string(),
    }
}

const CONTEXT_UNIT: &str = "estimated-bytes-v1";

#[derive(Clone)]
struct ContextGroup {
    id: String,
    start: usize,
    end: usize,
    has_tool_result: bool,
}

fn validate_context_history(history: &[Message]) -> Result<(), &'static str> {
    let mut used = std::collections::HashSet::new();
    let mut index = 0;
    while index < history.len() {
        let message = &history[index];
        match message.role.as_str() {
            "user" => {
                if message.tool_call_id.is_some() || !message.tool_calls.is_empty() {
                    return Err("invalid_context_history");
                }
                index += 1;
            }
            "assistant" => {
                if message.tool_call_id.is_some() {
                    return Err("invalid_context_history");
                }
                if message.tool_calls.is_empty() {
                    index += 1;
                    continue;
                }
                let mut ids = std::collections::HashSet::new();
                for call in &message.tool_calls {
                    if call.id.is_empty()
                        || call.name.is_empty()
                        || !ids.insert(call.id.clone())
                        || !used.insert(call.id.clone())
                    {
                        return Err("invalid_context_history");
                    }
                }
                index += 1;
                for _ in 0..message.tool_calls.len() {
                    let result = history.get(index).ok_or("invalid_context_history")?;
                    if result.role != "tool" || result.tool_calls.len() != 0 {
                        return Err("invalid_context_history");
                    }
                    let id = result
                        .tool_call_id
                        .as_ref()
                        .ok_or("invalid_context_history")?;
                    if !ids.remove(id) {
                        return Err("invalid_context_history");
                    }
                    index += 1;
                }
                if !ids.is_empty() {
                    return Err("invalid_context_history");
                }
            }
            "tool" => return Err("invalid_context_history"),
            _ => return Err("invalid_context_history"),
        }
    }
    Ok(())
}

fn context_groups(messages: &[Message]) -> Vec<ContextGroup> {
    let mut groups = Vec::new();
    let mut index = 0;
    while index < messages.len() {
        let start = index;
        let mut has_tool_result = false;
        index += 1;
        if messages[start].role == "assistant" && !messages[start].tool_calls.is_empty() {
            while index < messages.len() && messages[index].role == "tool" {
                has_tool_result = true;
                index += 1;
            }
        }
        groups.push(ContextGroup {
            id: format!("g{start}"),
            start,
            end: index,
            has_tool_result,
        });
    }
    groups
}

fn estimated_json(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Null => 4,
        serde_json::Value::Bool(v) => {
            if *v {
                4
            } else {
                5
            }
        }
        serde_json::Value::Number(_) => 8,
        serde_json::Value::String(v) => 2 + v.as_bytes().len(),
        serde_json::Value::Array(values) => {
            2 + values.iter().map(estimated_json).sum::<usize>() + values.len().saturating_sub(1)
        }
        serde_json::Value::Object(values) => {
            2 + values
                .iter()
                .map(|(key, value)| 2 + key.as_bytes().len() + 1 + estimated_json(value))
                .sum::<usize>()
                + values.len().saturating_sub(1)
        }
    }
}

fn estimated_message(message: &Message) -> usize {
    8 + message.role.as_bytes().len()
        + message.content.as_bytes().len()
        + message
            .tool_call_id
            .as_ref()
            .map_or(0, |id| id.as_bytes().len())
        + message
            .tool_calls
            .iter()
            .map(|call| {
                8 + call.id.as_bytes().len()
                    + call.name.as_bytes().len()
                    + estimated_json(&call.arguments)
            })
            .sum::<usize>()
}

/// The deterministic teaching estimate used by the chapter's offline adapter.
pub fn estimated_units(messages: &[Message]) -> usize {
    messages.iter().map(estimated_message).sum()
}

fn prepare_context(
    messages: &[Message],
    goal_index: usize,
    config: Option<&ContextConfig>,
    turn: usize,
    events: &mut Vec<LoopEvent>,
) -> Result<Vec<Message>, StopReason> {
    let Some(config) = config else {
        return Ok(messages.to_vec());
    };
    let groups = context_groups(messages);
    let rules_units = config
        .rules
        .iter()
        .map(|rule| {
            estimated_message(&Message {
                role: "system".into(),
                content: rule.clone(),
                tool_call_id: None,
                tool_calls: vec![],
            })
        })
        .sum::<usize>();
    let before_units = messages.iter().map(estimated_message).sum::<usize>() + rules_units;
    let goal_id = format!("g{goal_index}");
    let latest_tool = groups
        .iter()
        .rev()
        .find(|group| group.has_tool_result)
        .map(|group| group.id.clone());
    let mut required = vec![goal_id];
    if let Some(id) = latest_tool {
        if !required.contains(&id) {
            required.push(id);
        }
    }
    required.sort_by_key(|id| id[1..].parse::<usize>().unwrap_or(usize::MAX));
    let required_units = groups
        .iter()
        .filter(|group| required.contains(&group.id))
        .flat_map(|group| messages[group.start..group.end].iter())
        .map(estimated_message)
        .sum::<usize>()
        + rules_units;
    let mode = if config.manage {
        ContextMode::Managed
    } else {
        ContextMode::Unmanaged
    };
    if !config.manage {
        events.push(LoopEvent::ContextPrepared {
            turn,
            unit: CONTEXT_UNIT.into(),
            budget: config.budget,
            before_units,
            after_units: before_units,
            required_units,
            kept_groups: groups.iter().map(|g| g.id.clone()).collect(),
            removed_groups: vec![],
            required_groups: required,
            mode,
        });
        return Ok(with_rules(messages, &config.rules));
    }
    if required_units > config.budget {
        events.push(LoopEvent::ContextRejected {
            turn,
            reason: "context_budget_exhausted".into(),
            error_code: None,
            unit: CONTEXT_UNIT.into(),
            budget: Some(config.budget),
            before_units: Some(before_units),
            required_units: Some(required_units),
            mode,
        });
        return Err(StopReason::ContextBudgetExhausted);
    }
    let mut kept = vec![true; groups.len()];
    let mut after_units = before_units;
    let mut removed = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        if after_units <= config.budget {
            break;
        }
        if required.contains(&group.id) {
            continue;
        }
        kept[index] = false;
        let units = messages[group.start..group.end]
            .iter()
            .map(estimated_message)
            .sum::<usize>();
        after_units -= units;
        removed.push(group.id.clone());
    }
    let kept_groups: Vec<String> = groups
        .iter()
        .enumerate()
        .filter(|(i, _)| kept[*i])
        .map(|(_, g)| g.id.clone())
        .collect();
    let mut sent = Vec::new();
    sent.extend(with_rules_prefix(&config.rules));
    for (index, group) in groups.iter().enumerate() {
        if kept[index] {
            sent.extend(messages[group.start..group.end].iter().cloned());
        }
    }
    events.push(LoopEvent::ContextPrepared {
        turn,
        unit: CONTEXT_UNIT.into(),
        budget: config.budget,
        before_units,
        after_units,
        required_units,
        kept_groups,
        removed_groups: removed,
        required_groups: required,
        mode,
    });
    Ok(sent)
}

fn with_rules_prefix(rules: &[String]) -> Vec<Message> {
    rules
        .iter()
        .map(|rule| Message {
            role: "system".into(),
            content: rule.clone(),
            tool_call_id: None,
            tool_calls: vec![],
        })
        .collect()
}
fn with_rules(messages: &[Message], rules: &[String]) -> Vec<Message> {
    let mut output = with_rules_prefix(rules);
    output.extend(messages.iter().cloned());
    output
}
fn finish_loop(
    messages: Vec<Message>,
    mut events: Vec<LoopEvent>,
    state: LoopState,
    reason: StopReason,
    answer: Option<String>,
    error: Option<String>,
) -> LoopResult {
    events.push(LoopEvent::Stopped {
        state: state.clone(),
        reason: reason.clone(),
    });
    LoopResult {
        state,
        reason,
        answer,
        messages,
        events,
        error,
    }
}
