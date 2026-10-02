// Generated from JSON Schema. Do not edit.
// SPDX-License-Identifier: Apache-2.0

export type ActiveIntent = "Model" | "Verify" | {
  Tool: {
  args_ref: ArtifactRef;
  call_id: string;
  name: string;
};
};

export type ArtifactRef = {
  hash: string;
  len: number;
};

export type DomainEvent = "Cancelled" | "ToolBudgetExhausted" | {
  ObservationAccepted: {
  observation_id: string;
  revision: number;
};
} | {
  EffectScheduled: {
  effect_id: string;
};
};

export type HarnessSession = {
  active_effect?: string | null;
  active_intent?: ActiveIntent | null;
  attempt_id: string;
  completed_effects: string[];
  final_ref?: ArtifactRef | null;
  history: string[];
  revision: number;
  run_id: string;
  session_id: string;
  status: SessionStatus;
  tool_budget: number;
  tool_result_refs: ArtifactRef[];
  verification_plan_ref: ArtifactRef;
};

export type Intent = {
  CallModel: {
  effect_id: string;
};
} | {
  ExecuteTool: {
  call: ToolCall;
  effect_id: string;
};
} | {
  Verify: {
  effect_id: string;
  final_ref: ArtifactRef;
  plan_ref: ArtifactRef;
};
};

export type RejectCode = "AttemptMismatch" | "BudgetExhausted" | "InvalidEffect" | "InvalidSequence" | "RevisionMismatch" | "RunMismatch" | "SessionMismatch" | "Terminal";

export type SessionStatus = "Accepted" | "Cancelled" | "Completed" | "OutcomeUnknown" | "Running";

export type ToolCall = {
  args_ref: ArtifactRef;
  call_id: string;
  name: string;
};

export type StepResult = {
  Transition: {
  events: DomainEvent[];
  expected_revision: number;
  next_intent?: Intent | null;
  next_revision: number;
  session: HarnessSession;
};
} | {
  Duplicate: {
  observation_id: string;
  revision: number;
};
} | {
  Rejected: {
  code: RejectCode;
  expected_revision: number;
};
};
