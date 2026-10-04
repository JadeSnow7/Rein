// Generated from JSON Schema. Do not edit.
// SPDX-License-Identifier: Apache-2.0

export type ArtifactRef = {
  hash: string;
  len: number;
};

export type ToolCall = {
  args_ref: ArtifactRef;
  call_id: string;
  name: string;
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
