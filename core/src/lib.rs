//! Pure R1a domain state machine. No I/O, time, randomness, async, or storage.
#![forbid(unsafe_code)]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub type SessionId = String;
pub type RunId = String;
pub type AttemptId = String;
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ArtifactRef {
    pub hash: String,
    pub len: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ToolCall {
    pub call_id: String,
    pub name: String,
    pub args_ref: ArtifactRef,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum SessionStatus {
    Running,
    Cancelled,
    Completed,
    Accepted,
    OutcomeUnknown,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum VerificationStatus {
    Passed,
    Failed,
    Undetermined,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum ActiveIntent {
    Model,
    Tool {
        call_id: String,
        name: String,
        args_ref: ArtifactRef,
    },
    Verify,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum ObservationKind {
    Start,
    ModelTurn {
        tool_call: Option<ToolCall>,
        final_ref: Option<ArtifactRef>,
    },
    ToolResult {
        call_id: String,
        result_ref: ArtifactRef,
    },
    ToolFailed {
        call_id: String,
        error_ref: ArtifactRef,
    },
    VerifierResult {
        status: VerificationStatus,
        output_ref: ArtifactRef,
    },
    Cancel,
    OutcomeUnknown,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Observation {
    pub observation_id: String,
    pub session_id: SessionId,
    pub run_id: RunId,
    pub attempt_id: AttemptId,
    pub revision: u64,
    pub effect_id: Option<String>,
    pub kind: ObservationKind,
}
impl Observation {
    pub fn with_observation_id(&self, id: impl Into<String>) -> Self {
        let mut v = self.clone();
        v.observation_id = id.into();
        v
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct HarnessSession {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub attempt_id: AttemptId,
    pub revision: u64,
    pub status: SessionStatus,
    pub tool_budget: u32,
    pub active_effect: Option<String>,
    pub active_intent: Option<ActiveIntent>,
    pub history: Vec<String>,
    pub completed_effects: Vec<String>,
    pub tool_result_refs: Vec<ArtifactRef>,
    pub final_ref: Option<ArtifactRef>,
    pub verification_plan_ref: ArtifactRef,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum Intent {
    CallModel {
        effect_id: String,
    },
    ExecuteTool {
        effect_id: String,
        call: ToolCall,
    },
    Verify {
        effect_id: String,
        plan_ref: ArtifactRef,
        final_ref: ArtifactRef,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum DomainEvent {
    ObservationAccepted {
        observation_id: String,
        revision: u64,
    },
    EffectScheduled {
        effect_id: String,
    },
    Cancelled,
    ToolBudgetExhausted,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum RejectCode {
    SessionMismatch,
    RunMismatch,
    AttemptMismatch,
    RevisionMismatch,
    InvalidEffect,
    Terminal,
    InvalidSequence,
    BudgetExhausted,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum StepResult {
    Transition {
        expected_revision: u64,
        next_revision: u64,
        session: HarnessSession,
        next_intent: Option<Intent>,
        events: Vec<DomainEvent>,
    },
    Duplicate {
        revision: u64,
        observation_id: String,
    },
    Rejected {
        code: RejectCode,
        expected_revision: u64,
    },
}

pub struct HarnessStep;
impl HarnessStep {
    pub fn effect_id(s: &HarnessSession, k: &str) -> String {
        format!("{}/{}/{}/{}", s.session_id, s.attempt_id, s.revision, k)
    }
    pub fn advance(s: &HarnessSession, o: &Observation) -> StepResult {
        if s.session_id != o.session_id {
            return Self::rej(s, RejectCode::SessionMismatch);
        }
        if s.run_id != o.run_id {
            return Self::rej(s, RejectCode::RunMismatch);
        }
        if s.attempt_id != o.attempt_id {
            return Self::rej(s, RejectCode::AttemptMismatch);
        }
        if s.active_effect.is_some() != s.active_intent.is_some() {
            return Self::rej(s, RejectCode::InvalidSequence);
        }
        if s.history.iter().any(|x| x == &o.observation_id)
            || o.effect_id
                .as_ref()
                .is_some_and(|x| s.completed_effects.iter().any(|d| d == x))
        {
            return StepResult::Duplicate {
                revision: s.revision,
                observation_id: o.observation_id.clone(),
            };
        }
        if o.revision != s.revision {
            return Self::rej(s, RejectCode::RevisionMismatch);
        }
        if matches!(o.kind, ObservationKind::Cancel) {
            if matches!(
                s.status,
                SessionStatus::Completed | SessionStatus::Accepted | SessionStatus::Cancelled
            ) {
                return Self::rej(s, RejectCode::Terminal);
            }
            let mut n = s.clone();
            let Some(next) = n.revision.checked_add(1) else {
                return Self::rej(s, RejectCode::InvalidSequence);
            };
            n.revision = next;
            n.history.push(o.observation_id.clone());
            n.status = SessionStatus::Cancelled;
            n.active_effect = None;
            n.active_intent = None;
            return StepResult::Transition {
                expected_revision: s.revision,
                next_revision: n.revision,
                session: n,
                next_intent: None,
                events: vec![
                    DomainEvent::ObservationAccepted {
                        observation_id: o.observation_id.clone(),
                        revision: s.revision + 1,
                    },
                    DomainEvent::Cancelled,
                ],
            };
        }
        if s.status != SessionStatus::Running {
            return Self::rej(s, RejectCode::Terminal);
        }
        if let Some(active) = &s.active_effect {
            if o.effect_id.as_ref() != Some(active) {
                return Self::rej(s, RejectCode::InvalidEffect);
            }
        }
        if matches!(o.kind, ObservationKind::OutcomeUnknown) {
            if s.active_effect.is_none() {
                return Self::rej(s, RejectCode::InvalidEffect);
            }
            let mut n = s.clone();
            let Some(next) = n.revision.checked_add(1) else {
                return Self::rej(s, RejectCode::InvalidSequence);
            };
            n.revision = next;
            n.history.push(o.observation_id.clone());
            n.status = SessionStatus::OutcomeUnknown;
            return StepResult::Transition {
                expected_revision: s.revision,
                next_revision: n.revision,
                session: n,
                next_intent: None,
                events: vec![DomainEvent::ObservationAccepted {
                    observation_id: o.observation_id.clone(),
                    revision: s.revision + 1,
                }],
            };
        }
        let expected = s.revision;
        let mut n = s.clone();
        let Some(next) = n.revision.checked_add(1) else {
            return Self::rej(s, RejectCode::InvalidSequence);
        };
        n.revision = next;
        n.history.push(o.observation_id.clone());
        if let Some(e) = &o.effect_id {
            n.completed_effects.push(e.clone())
        }
        n.active_effect = None;
        n.active_intent = None;
        let mut events = vec![DomainEvent::ObservationAccepted {
            observation_id: o.observation_id.clone(),
            revision: n.revision,
        }];
        let intent = match &o.kind {
            ObservationKind::Start => {
                if expected != 0 || !s.history.is_empty() || o.effect_id.is_some() {
                    return Self::rej(s, RejectCode::InvalidSequence);
                }
                let id = Self::effect_id(&n, "model");
                n.active_effect = Some(id.clone());
                n.active_intent = Some(ActiveIntent::Model);
                events.push(DomainEvent::EffectScheduled {
                    effect_id: id.clone(),
                });
                Some(Intent::CallModel { effect_id: id })
            }
            ObservationKind::ModelTurn {
                tool_call,
                final_ref,
            } => {
                if s.active_intent != Some(ActiveIntent::Model)
                    || (tool_call.is_some() == final_ref.is_some())
                {
                    return Self::rej(s, RejectCode::InvalidSequence);
                }
                if let Some(r) = final_ref {
                    n.final_ref = Some(r.clone());
                    let id = Self::effect_id(&n, "verify");
                    n.active_effect = Some(id.clone());
                    n.active_intent = Some(ActiveIntent::Verify);
                    events.push(DomainEvent::EffectScheduled {
                        effect_id: id.clone(),
                    });
                    Some(Intent::Verify {
                        effect_id: id,
                        plan_ref: n.verification_plan_ref.clone(),
                        final_ref: r.clone(),
                    })
                } else {
                    let call = tool_call.as_ref().unwrap();
                    if n.tool_budget == 0 {
                        n.status = SessionStatus::Completed;
                        events.push(DomainEvent::ToolBudgetExhausted);
                        None
                    } else {
                        n.tool_budget -= 1;
                        let id = Self::effect_id(&n, "tool");
                        n.active_effect = Some(id.clone());
                        n.active_intent = Some(ActiveIntent::Tool {
                            call_id: call.call_id.clone(),
                            name: call.name.clone(),
                            args_ref: call.args_ref.clone(),
                        });
                        events.push(DomainEvent::EffectScheduled {
                            effect_id: id.clone(),
                        });
                        Some(Intent::ExecuteTool {
                            effect_id: id,
                            call: call.clone(),
                        })
                    }
                }
            }
            ObservationKind::ToolResult {
                call_id,
                result_ref,
            } => {
                if !matches!(&s.active_intent,Some(ActiveIntent::Tool{call_id:expected,..}) if expected==call_id)
                {
                    return Self::rej(s, RejectCode::InvalidSequence);
                }
                n.tool_result_refs.push(result_ref.clone());
                let id = Self::effect_id(&n, "model");
                n.active_effect = Some(id.clone());
                n.active_intent = Some(ActiveIntent::Model);
                events.push(DomainEvent::EffectScheduled {
                    effect_id: id.clone(),
                });
                Some(Intent::CallModel { effect_id: id })
            }
            ObservationKind::ToolFailed { call_id, .. } => {
                if !matches!(&s.active_intent,Some(ActiveIntent::Tool{call_id:expected,..}) if expected==call_id)
                {
                    return Self::rej(s, RejectCode::InvalidSequence);
                }
                n.status = SessionStatus::Completed;
                None
            }
            ObservationKind::VerifierResult { status, .. } => {
                if s.active_intent != Some(ActiveIntent::Verify) || s.final_ref.is_none() {
                    return Self::rej(s, RejectCode::InvalidSequence);
                }
                n.status = match status {
                    VerificationStatus::Passed => SessionStatus::Accepted,
                    _ => SessionStatus::Completed,
                };
                None
            }
            ObservationKind::OutcomeUnknown => {
                n.status = SessionStatus::OutcomeUnknown;
                None
            }
            ObservationKind::Cancel => unreachable!(),
        };
        StepResult::Transition {
            expected_revision: expected,
            next_revision: n.revision,
            session: n,
            next_intent: intent,
            events,
        }
    }
    fn rej(s: &HarnessSession, c: RejectCode) -> StepResult {
        StepResult::Rejected {
            code: c,
            expected_revision: s.revision,
        }
    }
}
pub mod harness {
    pub use super::{HarnessSession, HarnessStep, Observation, ObservationKind, StepResult};
}
#[cfg(test)]
mod tests {
    use super::*;
    fn s() -> HarnessSession {
        HarnessSession {
            session_id: "s".into(),
            run_id: "r".into(),
            attempt_id: "a".into(),
            revision: 0,
            status: SessionStatus::Running,
            tool_budget: 1,
            active_effect: None,
            active_intent: None,
            history: vec![],
            completed_effects: vec![],
            tool_result_refs: vec![],
            final_ref: None,
            verification_plan_ref: ArtifactRef {
                hash: "plan".into(),
                len: 4,
            },
        }
    }
    fn start() -> Observation {
        Observation {
            observation_id: "start".into(),
            session_id: "s".into(),
            run_id: "r".into(),
            attempt_id: "a".into(),
            revision: 0,
            effect_id: None,
            kind: ObservationKind::Start,
        }
    }
    #[test]
    fn start_model() {
        assert!(matches!(
            HarnessStep::advance(&s(), &start()),
            StepResult::Transition {
                next_intent: Some(Intent::CallModel { .. }),
                ..
            }
        ))
    }
    #[test]
    fn duplicate() {
        let mut x = s();
        x.history.push("start".into());
        assert!(matches!(
            HarnessStep::advance(&x, &start()),
            StepResult::Duplicate { .. }
        ))
    }
    #[test]
    fn budget() {
        let mut x = s();
        let o = Observation {
            observation_id: "m".into(),
            session_id: "s".into(),
            run_id: "r".into(),
            attempt_id: "a".into(),
            revision: 0,
            effect_id: None,
            kind: ObservationKind::ModelTurn {
                tool_call: Some(ToolCall {
                    call_id: "c".into(),
                    name: "read_file".into(),
                    args_ref: ArtifactRef {
                        hash: "x".into(),
                        len: 1,
                    },
                }),
                final_ref: None,
            },
        };
        assert!(matches!(
            HarnessStep::advance(&x, &o),
            StepResult::Rejected {
                code: RejectCode::InvalidSequence,
                ..
            }
        ));
        x.active_intent = Some(ActiveIntent::Model);
        x.active_effect = Some("s/a/0/model".into());
        let mut o = o;
        o.effect_id = x.active_effect.clone();
        x.tool_budget = 0;
        assert!(matches!(
            HarnessStep::advance(&x, &o),
            StepResult::Transition {
                next_intent: None,
                ..
            }
        ))
    }
    #[test]
    fn cancel() {
        let mut x = s();
        x.active_effect = Some("e".into());
        x.active_intent = Some(ActiveIntent::Model);
        assert!(matches!(
            HarnessStep::advance(
                &x,
                &Observation {
                    kind: ObservationKind::Cancel,
                    ..start()
                }
            ),
            StepResult::Transition {
                session: HarnessSession {
                    status: SessionStatus::Cancelled,
                    ..
                },
                ..
            }
        ))
    }
    #[test]
    fn model_stage_cannot_submit_verifier_or_wrong_call() {
        let mut x = s();
        x.active_intent = Some(ActiveIntent::Model);
        x.active_effect = Some("s/a/0/model".into());
        let mut fake = start();
        fake.effect_id = Some("s/a/0/model".into());
        fake.kind = ObservationKind::VerifierResult {
            status: VerificationStatus::Passed,
            output_ref: ArtifactRef {
                hash: "r".into(),
                len: 1,
            },
        };
        assert!(matches!(
            HarnessStep::advance(&x, &fake),
            StepResult::Rejected {
                code: RejectCode::InvalidSequence,
                ..
            }
        ));
        x.active_intent = Some(ActiveIntent::Tool {
            call_id: "expected".into(),
            name: "read_file".into(),
            args_ref: ArtifactRef {
                hash: "x".into(),
                len: 1,
            },
        });
        let wrong = Observation {
            observation_id: "wrong".into(),
            session_id: "s".into(),
            run_id: "r".into(),
            attempt_id: "a".into(),
            revision: 0,
            effect_id: x.active_effect.clone(),
            kind: ObservationKind::ToolResult {
                call_id: "actual".into(),
                result_ref: ArtifactRef {
                    hash: "x".into(),
                    len: 1,
                },
            },
        };
        assert!(matches!(
            HarnessStep::advance(&x, &wrong),
            StepResult::Rejected {
                code: RejectCode::InvalidSequence,
                ..
            }
        ));
    }
    #[test]
    fn start_replay_and_completed_effect_replay_are_rejected_or_duplicate() {
        let mut x = s();
        x.revision = 1;
        x.active_intent = Some(ActiveIntent::Model);
        x.active_effect = Some("s/a/1/model".into());
        let mut replay = start();
        replay.revision = 1;
        assert!(matches!(
            HarnessStep::advance(&x, &replay),
            StepResult::Rejected { .. }
        ));
        x.completed_effects.push("s/a/0/tool".into());
        let mut done = start();
        done.observation_id = "new".into();
        done.effect_id = Some("s/a/0/tool".into());
        assert!(matches!(
            HarnessStep::advance(&x, &done),
            StepResult::Duplicate { .. }
        ));
    }
    #[test]
    fn stale_and_terminal_cancel_do_not_change_state() {
        let mut x = s();
        x.revision = 2;
        let mut stale = start();
        stale.revision = 1;
        stale.kind = ObservationKind::Cancel;
        assert!(matches!(
            HarnessStep::advance(&x, &stale),
            StepResult::Rejected {
                code: RejectCode::RevisionMismatch,
                ..
            }
        ));
        x.status = SessionStatus::Accepted;
        let mut terminal = start();
        terminal.revision = 2;
        terminal.kind = ObservationKind::Cancel;
        assert!(matches!(
            HarnessStep::advance(&x, &terminal),
            StepResult::Rejected {
                code: RejectCode::Terminal,
                ..
            }
        ));
    }
    #[test]
    fn outcome_unknown_preserves_active_effect_and_is_not_completed() {
        let mut x = s();
        x.revision = 1;
        x.active_effect = Some("s/a/1/model".into());
        x.active_intent = Some(ActiveIntent::Model);
        let o = Observation {
            observation_id: "unknown".into(),
            session_id: "s".into(),
            run_id: "r".into(),
            attempt_id: "a".into(),
            revision: 1,
            effect_id: x.active_effect.clone(),
            kind: ObservationKind::OutcomeUnknown,
        };
        let StepResult::Transition { session, .. } = HarnessStep::advance(&x, &o) else {
            panic!("unknown")
        };
        assert_eq!(session.status, SessionStatus::OutcomeUnknown);
        assert_eq!(session.active_effect, x.active_effect);
        assert!(session.completed_effects.is_empty());
    }

    #[test]
    fn complete_tool_feedback_verify_chain() {
        let s0 = s();
        let StepResult::Transition {
            session: s1,
            next_intent: Some(Intent::CallModel { effect_id: model1 }),
            ..
        } = HarnessStep::advance(&s0, &start())
        else {
            panic!("start")
        };
        let model = Observation {
            observation_id: "model".into(),
            session_id: "s".into(),
            run_id: "r".into(),
            attempt_id: "a".into(),
            revision: s1.revision,
            effect_id: Some(model1),
            kind: ObservationKind::ModelTurn {
                tool_call: Some(ToolCall {
                    call_id: "call".into(),
                    name: "read_file".into(),
                    args_ref: ArtifactRef {
                        hash: "args".into(),
                        len: 1,
                    },
                }),
                final_ref: None,
            },
        };
        let StepResult::Transition {
            session: s2,
            next_intent:
                Some(Intent::ExecuteTool {
                    effect_id: tool, ..
                }),
            ..
        } = HarnessStep::advance(&s1, &model)
        else {
            panic!("tool")
        };
        let tool_result = Observation {
            observation_id: "tool".into(),
            session_id: "s".into(),
            run_id: "r".into(),
            attempt_id: "a".into(),
            revision: s2.revision,
            effect_id: Some(tool),
            kind: ObservationKind::ToolResult {
                call_id: "call".into(),
                result_ref: ArtifactRef {
                    hash: "result".into(),
                    len: 2,
                },
            },
        };
        let StepResult::Transition {
            session: s3,
            next_intent: Some(Intent::CallModel { effect_id: model2 }),
            ..
        } = HarnessStep::advance(&s2, &tool_result)
        else {
            panic!("feedback")
        };
        let final_obs = Observation {
            observation_id: "final".into(),
            session_id: "s".into(),
            run_id: "r".into(),
            attempt_id: "a".into(),
            revision: s3.revision,
            effect_id: Some(model2),
            kind: ObservationKind::ModelTurn {
                tool_call: None,
                final_ref: Some(ArtifactRef {
                    hash: "final".into(),
                    len: 2,
                }),
            },
        };
        let StepResult::Transition {
            session: s4,
            next_intent: Some(Intent::Verify {
                effect_id: verify, ..
            }),
            ..
        } = HarnessStep::advance(&s3, &final_obs)
        else {
            panic!("verify")
        };
        let receipt = Observation {
            observation_id: "receipt".into(),
            session_id: "s".into(),
            run_id: "r".into(),
            attempt_id: "a".into(),
            revision: s4.revision,
            effect_id: Some(verify),
            kind: ObservationKind::VerifierResult {
                status: VerificationStatus::Passed,
                output_ref: ArtifactRef {
                    hash: "receipt".into(),
                    len: 6,
                },
            },
        };
        let StepResult::Transition {
            session,
            next_intent: None,
            ..
        } = HarnessStep::advance(&s4, &receipt)
        else {
            panic!("accepted")
        };
        assert_eq!(session.status, SessionStatus::Accepted);
    }
}
