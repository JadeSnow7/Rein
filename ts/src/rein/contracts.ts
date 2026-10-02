export type Role = 'system' | 'user' | 'assistant' | 'tool'

export interface Message {
  readonly role: Role
  readonly content: string
  readonly toolCallId?: string
  readonly toolCalls?: readonly ToolCall[]
}

export interface ContextConfig {
  readonly rules: readonly string[]
  readonly history: readonly Message[]
  readonly budget: number
  readonly manage?: boolean
}

export interface ToolCall {
  readonly id: string
  readonly name: string
  readonly arguments: Readonly<Record<string, unknown>>
}

export interface ToolResult {
  readonly toolCallId: string
  readonly ok: boolean
  readonly output?: string
  readonly error?: { readonly code: string; readonly message: string }
}

export interface Turn {
  readonly messages: readonly Message[]
  readonly toolCalls: readonly ToolCall[]
  readonly toolResults: readonly ToolResult[]
}

export interface ChatAdapter {
  complete(messages: readonly Message[], tools?: readonly ToolDefinition[], options?: { readonly timeoutMs?: number; readonly signal?: AbortSignal }): Promise<ModelTurn>
}

export interface ToolDefinition { readonly name: string; readonly description: string; readonly inputSchema: Readonly<Record<string, unknown>> }
export interface ModelTurn { readonly message: Message; readonly toolCalls: readonly ToolCall[] }
