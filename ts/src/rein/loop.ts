import type { ChatAdapter, ContextConfig, Message, ToolCall, ToolDefinition, ToolResult } from './contracts'
import type { Workspace } from './readonly'
import { dispatchReadonly } from './readonly'

export type LoopState = 'running' | 'completed' | 'failed'
export type StopReason = 'final_answer' | 'empty_final' | 'model_error' | 'max_turns' | 'tool_budget_exhausted' | 'duplicate_action' | 'cancelled' | 'timeout' | 'context_budget_exhausted' | 'invalid_context_history'
export type LoopEvent =
  | { readonly type: 'model_requested'; readonly turn: number; readonly messages: readonly Message[]; readonly tools: readonly ToolDefinition[] }
  | { readonly type: 'model_received'; readonly turn: number; readonly message: Message; readonly toolCallIds: readonly string[]; readonly text: string }
  | { readonly type: 'tool_result'; readonly turn: number; readonly call: ToolCall; readonly toolCallId: string; readonly result: ToolResult }
  | { readonly type: 'context_prepared'; readonly turn: number; readonly unit: 'estimated-bytes-v1'; readonly budget: number; readonly beforeUnits: number; readonly afterUnits: number; readonly requiredUnits: number; readonly keptGroups: readonly string[]; readonly removedGroups: readonly string[]; readonly requiredGroups: readonly string[]; readonly mode: 'managed' | 'unmanaged' }
  | { readonly type: 'context_rejected'; readonly turn: number; readonly reason: 'context_budget_exhausted' | 'invalid_context_history'; readonly unit: 'estimated-bytes-v1'; readonly budget?: number; readonly beforeUnits?: number; readonly requiredUnits?: number; readonly mode: 'managed' | 'unmanaged'; readonly errorCode?: string }
  | { readonly type: 'action_skipped'; readonly turn: number; readonly action: 'model' | 'tool'; readonly callId?: string; readonly reason: StopReason }
  | { readonly type: 'stopped'; readonly state: Exclude<LoopState, 'running'>; readonly reason: StopReason }

export interface LoopOptions {
  readonly maxTurns?: number
  /** Maximum number of tool calls actually started. */
  readonly maxToolCalls?: number
  /** Number of equal (tool name + canonical arguments) dispatches allowed. */
  readonly duplicateLimit?: number
  readonly signal?: AbortSignal
  /** Whole-loop deadline, including model and tool work. */
  readonly timeoutMs?: number
  readonly tools?: readonly ToolDefinition[]
  /** Trusted local observation hook, called after each real tool result. */
  readonly onToolResult?: (call: ToolCall, result: ToolResult) => void
  readonly context?: ContextConfig
}

export interface LoopResult {
  readonly state: LoopState
  readonly reason: StopReason
  readonly answer?: string
  readonly messages: readonly Message[]
  readonly events: readonly LoopEvent[]
  readonly error?: string
}

export const readonlyToolDefinitions: readonly ToolDefinition[] = [
  { name: 'search_files', description: 'Search text in workspace files.', inputSchema: { type: 'object', properties: { needle: { type: 'string' } }, required: ['needle'], additionalProperties: false } },
  { name: 'read_file', description: 'Read one UTF-8 file in the workspace.', inputSchema: { type: 'object', properties: { path: { type: 'string' } }, required: ['path'], additionalProperties: false } },
]

export function estimateMessages(messages: readonly Message[]): number {
  return messages.reduce((sum, message) => sum + estimateMessage(message), 0)
}

function utf8(value: string): number { return new TextEncoder().encode(value).byteLength }
function validJson(value: unknown): boolean {
  if (value === null || typeof value === 'string' || typeof value === 'boolean') return true
  if (typeof value === 'number') return Number.isFinite(value)
  if (Array.isArray(value)) return value.every(validJson)
  if (typeof value === 'object') return Object.entries(value as Record<string, unknown>).every(([key, item]) => typeof key === 'string' && validJson(item))
  return false
}
function estimateJson(value: unknown): number {
  if (value === null) return 4
  if (typeof value === 'boolean') return value ? 4 : 5
  if (typeof value === 'number') return 8
  if (typeof value === 'string') return 2 + utf8(value)
  if (Array.isArray(value)) return 2 + value.reduce((n, item) => n + estimateJson(item), 0) + Math.max(0, value.length - 1)
  if (typeof value === 'object') { const entries = Object.entries(value as Record<string, unknown>); return 2 + entries.reduce((n, [key, item]) => n + 2 + utf8(key) + 1 + estimateJson(item), 0) + Math.max(0, entries.length - 1) }
  return 0
}
function estimateMessage(message: Message): number {
  return 8 + utf8(message.role) + utf8(message.content) + (message.toolCallId ? utf8(message.toolCallId) : 0) + (message.toolCalls ?? []).reduce((n, call) => n + 8 + utf8(call.id) + utf8(call.name) + estimateJson(call.arguments), 0)
}

type Group = { id: string; start: number; end: number; messages: Message[] }
function validateHistory(history: readonly Message[]): string | undefined {
  const ids = new Set<string>()
  for (let i = 0; i < history.length; i++) {
    const message = history[i]!
    if (!['user', 'assistant', 'tool'].includes(message.role)) return 'invalid_context_history'
    if (typeof message.content !== 'string' || (message.toolCalls !== undefined && (!Array.isArray(message.toolCalls) || !validJson(message.toolCalls)))) return 'invalid_context_history'
    if ((message.toolCalls ?? []).some(call => !call || typeof call !== 'object' || typeof call.id !== 'string' || !call.id || typeof call.name !== 'string' || !call.name || !validJson(call.arguments))) return 'invalid_context_history'
    if (message.role === 'tool') {
      if ((message.toolCalls ?? []).length) return 'invalid_context_history'
      if (!message.toolCallId || ids.has(message.toolCallId)) return 'invalid_context_history'
      const previous = history[i - 1]
      if (!previous || previous.role !== 'assistant' || !(previous.toolCalls ?? []).some(call => call.id === message.toolCallId)) return 'invalid_context_history'
      ids.add(message.toolCallId)
      continue
    }
    if (message.role === 'user' && ((message.toolCalls ?? []).length || message.toolCallId !== undefined)) return 'invalid_context_history'
    if (message.role === 'assistant') {
      if (message.toolCallId !== undefined) return 'invalid_context_history'
      const calls = message.toolCalls ?? []
      const callIds = calls.map(call => call.id)
      if (new Set(callIds).size !== callIds.length || callIds.some(id => !id || ids.has(id))) return 'invalid_context_history'
      if (calls.length) {
        const results = history.slice(i + 1, i + 1 + calls.length)
        if (results.length !== calls.length || results.some(result => !isValidToolResultMessage(result) || !callIds.includes(result.toolCallId)) || new Set(results.map(result => result.toolCallId)).size !== calls.length) return 'invalid_context_history'
        callIds.forEach(id => ids.add(id))
        i += calls.length
      }
    }
  }
  return undefined
}

function isValidToolResultMessage(message: Message): message is Message & { readonly toolCallId: string } {
  return message.role === 'tool' && typeof message.toolCallId === 'string' && message.toolCallId.length > 0 && typeof message.content === 'string' && (message.toolCalls === undefined || Array.isArray(message.toolCalls)) && (message.toolCalls ?? []).length === 0
}

function makeGroups(messages: readonly Message[]): Group[] {
  const groups: Group[] = []
  for (let i = 0; i < messages.length; i++) {
    const message = messages[i]!
    if (message.role === 'tool') continue
    const end = message.role === 'assistant' && (message.toolCalls ?? []).length ? i + (message.toolCalls ?? []).length : i
    groups.push({ id: `g${i}`, start: i, end, messages: messages.slice(i, end + 1) as Message[] })
    i = end
  }
  return groups
}

function prepareContext(messages: readonly Message[], context: ContextConfig, currentGoalIndex: number): { messages: Message[]; beforeUnits: number; afterUnits: number; requiredUnits: number; keptGroups: string[]; removedGroups: string[]; requiredGroups: string[]; mode: 'managed' | 'unmanaged'; rejected?: boolean } {
  const groups = makeGroups(messages)
  const goal = groups.find(group => group.start === currentGoalIndex) ?? groups.at(-1)
  const latestTool = [...groups].reverse().find(group => group.messages.some(message => message.role === 'assistant' && (message.toolCalls ?? []).length))
  const required = new Set([goal?.id, latestTool?.id].filter((id): id is string => Boolean(id)))
  const rulesMessages = context.rules.map(content => ({ role: 'system' as const, content }))
  const beforeUnits = estimateMessages([...rulesMessages, ...messages])
  const requiredUnits = estimateMessages(rulesMessages) + groups.filter(group => required.has(group.id)).reduce((n, group) => n + estimateMessages(group.messages), 0)
  const mode = context.manage === false ? 'unmanaged' : 'managed'
  if (mode === 'managed' && requiredUnits > context.budget) return { messages: [], beforeUnits, afterUnits: beforeUnits, requiredUnits, keptGroups: [], removedGroups: [], requiredGroups: [...required].sort((a, b) => Number(a.slice(1)) - Number(b.slice(1))), mode, rejected: true }
  const kept = new Set(groups.map(group => group.id))
  if (mode === 'managed') for (const group of groups) {
    if (estimateMessages(rulesMessages) + groups.filter(candidate => kept.has(candidate.id)).reduce((n, candidate) => n + estimateMessages(candidate.messages), 0) <= context.budget) break
    if (!required.has(group.id)) kept.delete(group.id)
  }
  const keptGroups = groups.filter(group => kept.has(group.id))
  const removedGroups = groups.filter(group => !kept.has(group.id))
  const audit = keptGroups.flatMap(group => group.messages)
  const sent = [...rulesMessages, ...audit]
  return { messages: sent, beforeUnits, afterUnits: mode === 'unmanaged' ? beforeUnits : estimateMessages(sent), requiredUnits, keptGroups: keptGroups.map(group => group.id), removedGroups: removedGroups.map(group => group.id), requiredGroups: [...required].sort((a, b) => Number(a.slice(1)) - Number(b.slice(1))), mode }
}

/** Runs the small, replayable tool loop. One model turn may produce many tools. */
export async function runAgentLoop(adapter: ChatAdapter, workspace: Workspace, prompt: string, options: LoopOptions = {}): Promise<LoopResult> {
  const maxTurns = options.maxTurns ?? 32
  const maxToolCalls = options.maxToolCalls ?? Number.POSITIVE_INFINITY
  const duplicateLimit = options.duplicateLimit ?? Number.POSITIVE_INFINITY
  const tools = options.tools ?? readonlyToolDefinitions
  const context = options.context ? structuredClone(options.context) : undefined
  const contextGoalIndex = context?.history.length
  const messages: Message[] = context ? structuredClone(context.history).concat({ role: 'user', content: prompt }) : [{ role: 'user', content: prompt }]
  const events: LoopEvent[] = []
  const startedAt = Date.now()
  const deadline = options.timeoutMs === undefined ? undefined : startedAt + Math.max(0, options.timeoutMs)
  const controlReason = (): StopReason | undefined => {
    if (options.signal?.aborted) return 'cancelled'
    if (deadline !== undefined && Date.now() >= deadline) return 'timeout'
    return undefined
  }
  let toolStarted = 0
  const seen = new Map<string, number>()
  if (context) {
    const invalid = validateHistory(context.history)
    const mode = context.manage === false ? 'unmanaged' : 'managed'
    if (invalid) { events.push({ type: 'context_rejected', turn: 0, reason: 'invalid_context_history', unit: 'estimated-bytes-v1', mode, errorCode: invalid }); return finish(messages, events, 'failed', 'invalid_context_history', undefined, invalid) }
    if (!Number.isSafeInteger(context.budget) || context.budget < 0 || context.rules.some(rule => typeof rule !== 'string')) { events.push({ type: 'context_rejected', turn: 0, reason: 'invalid_context_history', unit: 'estimated-bytes-v1', mode, errorCode: 'invalid_context_config' }); return finish(messages, events, 'failed', 'invalid_context_history', undefined, 'invalid_context_config') }
  }
  if (maxTurns <= 0) return finish(messages, events, 'failed', 'max_turns')
  const initialReason = controlReason()
  if (initialReason) return finish(messages, events, 'failed', initialReason)
  for (let turn = 1; turn <= maxTurns; turn++) {
    const beforeModel = controlReason()
    if (beforeModel) return finish(messages, events, 'failed', beforeModel)
    let sentMessages = messages
    if (context) {
      const invalid = validateHistory(messages)
      if (invalid) { events.push({ type: 'context_rejected', turn, reason: 'invalid_context_history', unit: 'estimated-bytes-v1', mode: context.manage === false ? 'unmanaged' : 'managed', errorCode: invalid }); return finish(messages, events, 'failed', 'invalid_context_history', undefined, invalid) }
      const prepared = prepareContext(messages, context, contextGoalIndex!)
      if (prepared.rejected) {
        events.push({ type: 'context_rejected', turn, reason: 'context_budget_exhausted', unit: 'estimated-bytes-v1', budget: context.budget, beforeUnits: prepared.beforeUnits, requiredUnits: prepared.requiredUnits, mode: prepared.mode })
        return finish(messages, events, 'failed', 'context_budget_exhausted')
      }
      sentMessages = prepared.messages
      events.push({ type: 'context_prepared', turn, unit: 'estimated-bytes-v1', budget: context.budget, beforeUnits: prepared.beforeUnits, afterUnits: prepared.afterUnits, requiredUnits: prepared.requiredUnits, keptGroups: prepared.keptGroups, removedGroups: prepared.removedGroups, requiredGroups: prepared.requiredGroups, mode: prepared.mode })
    }
    events.push({ type: 'model_requested', turn, messages: structuredClone(sentMessages), tools: structuredClone(tools) })
    let model
    try {
      const remaining = deadline === undefined ? undefined : Math.max(0, deadline - Date.now())
      model = await raceControl(() => adapter.complete(structuredClone(sentMessages), tools, { timeoutMs: remaining, signal: options.signal }), options.signal, remaining)
    }
    catch (error) {
      const reason = controlReason()
      if (reason) return finish(messages, events, 'failed', reason)
      return finish(messages, events, 'failed', 'model_error', undefined, error instanceof Error ? error.message : 'model error')
    }
    const afterModel = controlReason()
    if (afterModel) return finish(messages, events, 'failed', afterModel)
    messages.push(model.message)
    events.push({ type: 'model_received', turn, message: structuredClone(model.message), toolCallIds: model.toolCalls.map(call => call.id), text: model.message.content })
    if (model.toolCalls.length === 0) {
      if (model.message.content.trim().length === 0) return finish(messages, events, 'failed', 'empty_final')
      return finish(messages, events, 'completed', 'final_answer', model.message.content)
    }
    if (model.toolCalls.length > 0 && maxToolCalls <= toolStarted) {
      for (const call of model.toolCalls) events.push({ type: 'action_skipped', turn, action: 'tool', callId: call.id, reason: 'tool_budget_exhausted' })
      return finish(messages, events, 'failed', 'tool_budget_exhausted')
    }
    for (let index = 0; index < model.toolCalls.length; index++) {
      const call = model.toolCalls[index]!
      const reason = controlReason()
      if (reason) {
        for (const skipped of model.toolCalls.slice(index)) events.push({ type: 'action_skipped', turn, action: 'tool', callId: skipped.id, reason })
        return finish(messages, events, 'failed', reason)
      }
      if (toolStarted >= maxToolCalls) {
        for (const skipped of model.toolCalls.slice(index)) events.push({ type: 'action_skipped', turn, action: 'tool', callId: skipped.id, reason: 'tool_budget_exhausted' })
        return finish(messages, events, 'failed', 'tool_budget_exhausted')
      }
      const identity = `${call.name}\0${canonicalJson(call.arguments)}`
      const occurrences = (seen.get(identity) ?? 0) + 1
      if (occurrences > duplicateLimit) {
        for (const skipped of model.toolCalls.slice(index)) events.push({ type: 'action_skipped', turn, action: 'tool', callId: skipped.id, reason: 'duplicate_action' })
        return finish(messages, events, 'failed', 'duplicate_action')
      }
      seen.set(identity, occurrences)
      toolStarted++
      const result = await dispatchReadonly(call, workspace)
      events.push({ type: 'tool_result', turn, call: structuredClone(call), toolCallId: call.id, result })
      messages.push({ role: 'tool', toolCallId: result.toolCallId, content: result.ok ? result.output ?? '' : JSON.stringify({ ok: false, error: result.error }) })
      options.onToolResult?.(call, result)
    }
  }
  const finalControl = controlReason()
  return finish(messages, events, 'failed', finalControl ?? 'max_turns')
}

function canonicalJson(value: unknown): string {
  if (value === null || typeof value !== 'object') return JSON.stringify(value)
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`
  return `{${Object.keys(value as Record<string, unknown>).sort().map(key => `${JSON.stringify(key)}:${canonicalJson((value as Record<string, unknown>)[key])}`).join(',')}}`
}

async function raceControl<T>(start: () => Promise<T>, signal: AbortSignal | undefined, timeoutMs: number | undefined): Promise<T> {
  if (signal === undefined && timeoutMs === undefined) return start()
  let timer: ReturnType<typeof setTimeout> | undefined
  let onAbort: (() => void) | undefined
  let rejectControl!: (error: Error) => void
  const control = new Promise<never>((_, reject) => {
    rejectControl = reject
    onAbort = () => reject(new Error('cancelled'))
    if (signal?.aborted) rejectControl(new Error('cancelled'))
    else signal?.addEventListener('abort', onAbort, { once: true })
    if (timeoutMs !== undefined) timer = setTimeout(() => rejectControl(new Error('timeout')), timeoutMs)
  })
  let work: Promise<T>
  try {
    if (signal?.aborted) throw new Error('cancelled')
    // Turn synchronous adapter throws into a promise before racing. This also
    // attaches the control rejection handler even when abort and throw happen
    // in the same call stack.
    work = Promise.resolve().then(() => {
      if (signal?.aborted) throw new Error('cancelled')
      return start()
    })
    return await Promise.race([work, control])
  }
  finally {
    if (timer !== undefined) clearTimeout(timer)
    if (onAbort) signal?.removeEventListener('abort', onAbort)
  }
}

function finish(messages: Message[], events: LoopEvent[], state: Exclude<LoopState, 'running'>, reason: StopReason, answer?: string, error?: string): LoopResult {
  events.push({ type: 'stopped', state, reason })
  return { state, reason, ...(answer === undefined ? {} : { answer }), messages, events, ...(error === undefined ? {} : { error }) }
}
