// Generated from JSON Schema. Do not edit.
// SPDX-License-Identifier: Apache-2.0

export type ArtifactRef = {
  hash: string;
  len: number;
};

export type ObservationKind = "Cancel" | "OutcomeUnknown" | "Start" | {
  ModelTurn: {
  final_ref?: ArtifactRef | null;
  tool_call?: ToolCall | null;
};
} | {
  ToolResult: {
  call_id: string;
  result_ref: ArtifactRef;
};
} | {
  ToolFailed: {
  call_id: string;
  error_ref: ArtifactRef;
};
} | {
  VerifierResult: {
  output_ref: ArtifactRef;
  status: VerificationStatus;
};
};

export type ToolCall = {
  args_ref: ArtifactRef;
  call_id: string;
  name: string;
};

export type VerificationStatus = "Failed" | "Passed" | "Undetermined";

export type Observation = {
  attempt_id: string;
  effect_id?: string | null;
  kind: ObservationKind;
  observation_id: string;
  revision: number;
  run_id: string;
  session_id: string;
};
