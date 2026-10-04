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

export type SessionStatus = "Accepted" | "Cancelled" | "Completed" | "OutcomeUnknown" | "Running";

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
