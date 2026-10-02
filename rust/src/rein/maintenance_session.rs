//! A bounded, evidence carrying document-maintenance session.
use super::{
    maintenance, ControlSignal, ExecutorResult, Extension02Executor, ToolCall, ToolExecutor,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceVersion {
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug)]
pub struct Observation {
    pub target: String,
    pub rule: String,
    pub original: String,
    pub sources: Vec<String>,
    pub source_contents: Vec<(String, String)>,
    pub feedback: Option<String>,
    pub attempt: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionEvent {
    Read {
        attempt: usize,
        paths: Vec<String>,
        source_versions: Vec<SourceVersion>,
    },
    Proposed {
        attempt: usize,
        path: String,
        baseline_sha256: String,
        candidate_digest: String,
        diff: String,
    },
    Approval {
        attempt: usize,
        approved: bool,
        candidate_digest: Option<String>,
    },
    Applied {
        attempt: usize,
        post_sha256: String,
    },
    Verified {
        attempt: usize,
        ok: bool,
        output: String,
    },
    ToolFailed {
        attempt: usize,
        tool: String,
        code: String,
    },
    StoppedUnknown {
        attempt: usize,
    },
    StoppedCancelled {
        attempt: usize,
    },
    Stopped {
        reason: StopReason,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum StopReason {
    Completed,
    Rejected,
    ToolFailure,
    Cancelled,
    Unknown,
    MaxRepairs,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionReport {
    pub events: Vec<SessionEvent>,
    pub stop_reason: StopReason,
}

#[derive(Debug)]
pub enum SessionError {
    InvalidInput,
}

#[derive(Debug)]
enum ReadFailure {
    Cancelled,
    Unknown,
    Tool(String),
}

fn source_paths(rule: &str) -> Vec<&'static str> {
    match rule {
        "command-v1" | "document-maintenance-v1" => vec!["package.json"],
        "parameter-v1" => vec!["help.txt"],
        "relative-link-v1" => vec![],
        _ => vec![],
    }
}

fn sha256(text: &str) -> String {
    ring::digest::digest(&ring::digest::SHA256, text.as_bytes())
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

async fn read_file(
    executor: &Extension02Executor,
    signal: &ControlSignal,
    path: &str,
    id: &str,
) -> Result<String, ReadFailure> {
    match executor
        .execute(
            &ToolCall {
                id: id.into(),
                name: "read_file".into(),
                arguments: serde_json::json!({"path":path}),
            },
            signal,
            None,
        )
        .await
    {
        ExecutorResult::Completed(result) if result.ok => Ok(result.output.unwrap_or_default()),
        ExecutorResult::Cancelled => Err(ReadFailure::Cancelled),
        ExecutorResult::OutcomeUnknown => Err(ReadFailure::Unknown),
        ExecutorResult::Completed(result) => Err(ReadFailure::Tool(
            result
                .error
                .map(|e| e.code)
                .unwrap_or_else(|| "tool_failed".into()),
        )),
        ExecutorResult::NotDispatched if signal.is_cancelled() => Err(ReadFailure::Cancelled),
        ExecutorResult::NotDispatched => Err(ReadFailure::Tool("not_dispatched".into())),
    }
}

fn stop(events: &mut Vec<SessionEvent>, reason: StopReason, attempt: usize) -> SessionReport {
    events.push(match reason {
        StopReason::Unknown => SessionEvent::StoppedUnknown { attempt },
        StopReason::Cancelled => SessionEvent::StoppedCancelled { attempt },
        _ => SessionEvent::Stopped {
            reason: reason.clone(),
        },
    });
    SessionReport {
        events: events.clone(),
        stop_reason: reason,
    }
}

/// Run one candidate and at most two newly generated repair candidates.
/// Every attempt rereads the document and its rule-specific sources before
/// invoking the generator. Expected fixture answers never enter this API.
pub async fn run_maintenance_session<G, A>(
    executor: &Extension02Executor,
    signal: &ControlSignal,
    target: &str,
    rule: &str,
    mut generate: G,
    mut approve: A,
) -> Result<SessionReport, SessionError>
where
    G: FnMut(&Observation) -> Option<String>,
    A: FnMut(&maintenance::Candidate) -> Option<maintenance::Approval>,
{
    if target.is_empty() || rule.is_empty() {
        return Err(SessionError::InvalidInput);
    }
    executor.enable_maintenance(target.to_owned(), rule.to_owned());
    let mut events = Vec::new();
    let mut feedback = None;
    for attempt in 0..=2 {
        let original = match read_file(
            executor,
            signal,
            target,
            &format!("maintenance-target-{attempt}"),
        )
        .await
        {
            Ok(value) => value,
            Err(ReadFailure::Cancelled) => {
                return Ok(stop(&mut events, StopReason::Cancelled, attempt))
            }
            Err(ReadFailure::Unknown) => {
                return Ok(stop(&mut events, StopReason::Unknown, attempt))
            }
            Err(ReadFailure::Tool(code)) => {
                events.push(SessionEvent::ToolFailed {
                    attempt,
                    tool: "read_file".into(),
                    code,
                });
                return Ok(stop(&mut events, StopReason::ToolFailure, attempt));
            }
        };
        let mut sources: Vec<String> = Vec::new();
        let mut source_contents: Vec<(String, String)> = Vec::new();
        for (index, source) in source_paths(rule).into_iter().enumerate() {
            match read_file(
                executor,
                signal,
                source,
                &format!("maintenance-source-{attempt}-{index}"),
            )
            .await
            {
                Ok(value) => {
                    sources.push(source.into());
                    source_contents.push((source.into(), value));
                }
                Err(ReadFailure::Cancelled) => {
                    return Ok(stop(&mut events, StopReason::Cancelled, attempt))
                }
                Err(ReadFailure::Unknown) => {
                    return Ok(stop(&mut events, StopReason::Unknown, attempt))
                }
                Err(ReadFailure::Tool(code)) => {
                    events.push(SessionEvent::ToolFailed {
                        attempt,
                        tool: "read_file".into(),
                        code,
                    });
                    return Ok(stop(&mut events, StopReason::ToolFailure, attempt));
                }
            }
        }
        let paths: Vec<String> = std::iter::once(target.to_owned())
            .chain(sources.iter().cloned())
            .collect();
        let source_versions: Vec<SourceVersion> = std::iter::once(SourceVersion {
            path: target.into(),
            sha256: sha256(&original),
        })
        .chain(source_contents.iter().map(|(path, text)| SourceVersion {
            path: path.clone(),
            sha256: sha256(text),
        }))
        .collect();
        events.push(SessionEvent::Read {
            attempt,
            paths,
            source_versions,
        });
        let observation = Observation {
            target: target.into(),
            rule: rule.into(),
            original: original.clone(),
            sources,
            source_contents,
            feedback: feedback.clone(),
            attempt,
        };
        let replacement = generate(&observation).unwrap_or_else(|| original.clone());
        if replacement == original {
            let call = ToolCall {
                id: format!("maintenance-verify-nochange-{attempt}"),
                name: "run_verification".into(),
                arguments: serde_json::json!({"target":target,"rule":rule}),
            };
            match executor.execute(&call, signal, None).await {
                ExecutorResult::Completed(result) if result.ok => {
                    let output = result.output.unwrap_or_default();
                    let ok = serde_json::from_str::<Value>(&output)
                        .ok()
                        .and_then(|v| v["ok"].as_bool())
                        .unwrap_or(false);
                    events.push(SessionEvent::Verified {
                        attempt,
                        ok,
                        output: output.clone(),
                    });
                    if ok {
                        return Ok(stop(&mut events, StopReason::Completed, attempt));
                    }
                    feedback = Some(output);
                }
                ExecutorResult::Cancelled => {
                    return Ok(stop(&mut events, StopReason::Cancelled, attempt))
                }
                ExecutorResult::OutcomeUnknown => {
                    return Ok(stop(&mut events, StopReason::Unknown, attempt))
                }
                ExecutorResult::Completed(result) => {
                    events.push(SessionEvent::ToolFailed {
                        attempt,
                        tool: "run_verification".into(),
                        code: result
                            .error
                            .map(|e| e.code)
                            .unwrap_or_else(|| "tool_failed".into()),
                    });
                    return Ok(stop(&mut events, StopReason::ToolFailure, attempt));
                }
                ExecutorResult::NotDispatched => {
                    return Ok(stop(&mut events, StopReason::ToolFailure, attempt))
                }
            }
            continue;
        }
        let candidate = match maintenance::propose(&executor.workspace, target, replacement) {
            Ok(candidate) => candidate,
            Err(_) => {
                events.push(SessionEvent::ToolFailed {
                    attempt,
                    tool: "candidate".into(),
                    code: "candidate_invalid".into(),
                });
                return Ok(stop(&mut events, StopReason::ToolFailure, attempt));
            }
        };
        if candidate.baseline_sha256() != sha256(&observation.original) {
            events.push(SessionEvent::ToolFailed {
                attempt,
                tool: "candidate".into(),
                code: "baseline_changed".into(),
            });
            return Ok(stop(&mut events, StopReason::ToolFailure, attempt));
        }
        events.push(SessionEvent::Proposed {
            attempt,
            path: candidate.path().display().to_string(),
            baseline_sha256: candidate.baseline_sha256().into(),
            candidate_digest: candidate.digest().into(),
            diff: candidate.diff().into(),
        });
        let Some(approval) = approve(&candidate) else {
            events.push(SessionEvent::Approval {
                attempt,
                approved: false,
                candidate_digest: Some(candidate.digest().into()),
            });
            return Ok(stop(&mut events, StopReason::Rejected, attempt));
        };
        events.push(SessionEvent::Approval {
            attempt,
            approved: true,
            candidate_digest: Some(candidate.digest().into()),
        });
        if !executor.authorize_candidate(&candidate, &approval) {
            return Ok(stop(&mut events, StopReason::Rejected, attempt));
        }
        let call = ToolCall {
            id: format!("maintenance-apply-{attempt}"),
            name: "apply_patch".into(),
            arguments: serde_json::json!({"path":target,"baselineSha256":candidate.baseline_sha256(),"replacement":candidate.replacement(),"digest":candidate.digest()}),
        };
        match executor.execute(&call, signal, None).await {
            ExecutorResult::Completed(result) if result.ok => {
                let output = result.output.unwrap_or_default();
                let Some(post_sha256) = serde_json::from_str::<Value>(&output)
                    .ok()
                    .and_then(|value| value["postSha256"].as_str().map(str::to_owned))
                else {
                    events.push(SessionEvent::ToolFailed {
                        attempt,
                        tool: "apply_patch".into(),
                        code: "result_invalid".into(),
                    });
                    return Ok(stop(&mut events, StopReason::ToolFailure, attempt));
                };
                events.push(SessionEvent::Applied {
                    attempt,
                    post_sha256,
                })
            }
            ExecutorResult::Cancelled => {
                return Ok(stop(&mut events, StopReason::Cancelled, attempt))
            }
            ExecutorResult::OutcomeUnknown => {
                return Ok(stop(&mut events, StopReason::Unknown, attempt))
            }
            ExecutorResult::Completed(result) => {
                events.push(SessionEvent::ToolFailed {
                    attempt,
                    tool: "apply_patch".into(),
                    code: result
                        .error
                        .map(|e| e.code)
                        .unwrap_or_else(|| "tool_failed".into()),
                });
                return Ok(stop(&mut events, StopReason::ToolFailure, attempt));
            }
            ExecutorResult::NotDispatched => {
                return Ok(stop(&mut events, StopReason::ToolFailure, attempt))
            }
        }
        let verify = ToolCall {
            id: format!("maintenance-verify-{attempt}"),
            name: "run_verification".into(),
            arguments: serde_json::json!({"target":target,"rule":rule}),
        };
        match executor.execute(&verify, signal, None).await {
            ExecutorResult::Completed(result) if result.ok => {
                let output = result.output.unwrap_or_default();
                let value = match serde_json::from_str::<Value>(&output) {
                    Ok(value) => value,
                    Err(_) => {
                        events.push(SessionEvent::ToolFailed {
                            attempt,
                            tool: "run_verification".into(),
                            code: "result_invalid".into(),
                        });
                        return Ok(stop(&mut events, StopReason::ToolFailure, attempt));
                    }
                };
                let ok = value["ok"].as_bool().unwrap_or(false);
                events.push(SessionEvent::Verified {
                    attempt,
                    ok,
                    output: output.clone(),
                });
                if ok {
                    return Ok(stop(&mut events, StopReason::Completed, attempt));
                }
                feedback = Some(output);
            }
            ExecutorResult::Cancelled => {
                return Ok(stop(&mut events, StopReason::Cancelled, attempt))
            }
            ExecutorResult::OutcomeUnknown => {
                return Ok(stop(&mut events, StopReason::Unknown, attempt))
            }
            ExecutorResult::Completed(result) => {
                events.push(SessionEvent::ToolFailed {
                    attempt,
                    tool: "run_verification".into(),
                    code: result
                        .error
                        .map(|e| e.code)
                        .unwrap_or_else(|| "tool_failed".into()),
                });
                return Ok(stop(&mut events, StopReason::ToolFailure, attempt));
            }
            ExecutorResult::NotDispatched => {
                return Ok(stop(&mut events, StopReason::ToolFailure, attempt))
            }
        }
    }
    Ok(stop(&mut events, StopReason::MaxRepairs, 2))
}
