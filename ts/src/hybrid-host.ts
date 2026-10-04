import { dispatchReadonly, type Workspace } from './rein/readonly'
import type { ToolCall } from './rein/contracts'

export const PROTOCOL_VERSION = 'rein-extension/0.1'
const MAX_MESSAGE = 256 * 1024
const workspace: Workspace = { root: process.env.REIN_HYBRID_WORKSPACE ?? process.cwd() }
type Wire = Record<string, unknown>
type Request = { sessionId: string; requestId: string; taskId: string; callId: string; tool: string; path: string; ruleId: string; targetVersion: string }
type Cancel = Pick<Request, 'sessionId' | 'requestId' | 'taskId' | 'callId'>
let request: Request | undefined
let inFlight = false
let cancelRequested = false
let terminalSent = false
let closing = false
let ending = false

function send(value: Wire): void { if (!closing) process.stdout.write(`${JSON.stringify(value)}\n`) }
function error(code: string, message: string, requestId?: string): void { send({ protocol: PROTOCOL_VERSION, type: 'error', ...(requestId ? { requestId } : {}), code, message }) }
function isText(value: unknown): value is string { return typeof value === 'string' && value.length > 0 }
function exactKeys(value: Wire, keys: readonly string[]): boolean { const allowed = new Set(keys); return Object.keys(value).every(key => allowed.has(key)) }
function parseRequest(value: Wire): Request | undefined {
  const keys = ['protocol', 'type', 'sessionId', 'requestId', 'taskId', 'callId', 'tool', 'path', 'ruleId', 'targetVersion']
  if (!exactKeys(value, keys) || value.protocol !== PROTOCOL_VERSION || value.type !== 'invoke') return undefined
  return keys.slice(2).every(key => isText(value[key])) ? value as unknown as Request : undefined
}
function parseCancel(value: Wire): Cancel | undefined {
  const keys = ['protocol', 'type', 'sessionId', 'requestId', 'taskId', 'callId']
  if (!exactKeys(value, keys) || value.protocol !== PROTOCOL_VERSION || value.type !== 'cancel') return undefined
  return keys.slice(2).every(key => isText(value[key])) ? value as unknown as Cancel : undefined
}
function sameIds(a: Request, b: Cancel): boolean { return a.sessionId === b.sessionId && a.requestId === b.requestId && a.taskId === b.taskId && a.callId === b.callId }
function finish(): void {
  if (ending) return
  ending = true
  process.stdin.pause()
  process.stdin.destroy()
  // stdout.end drains the terminal line before the session exits. Do not use a
  // timer here: the caller's protocol requires the terminal to be observable.
  process.stdout.end(() => { closing = true; process.exitCode = 0 })
}
type Result = Awaited<ReturnType<typeof dispatchReadonly>>
function terminal(status: 'succeeded' | 'tool_failed' | 'cancelled', result?: Result): void {
  if (!request || terminalSent) return
  terminalSent = true
  const base = { protocol: PROTOCOL_VERSION, type: 'terminal', sessionId: request.sessionId, requestId: request.requestId, taskId: request.taskId, callId: request.callId, status }
  if (status === 'cancelled') send(base)
  else if (status === 'succeeded') send({ ...base, output: result?.output, evidence: { taskId: request.taskId, callId: request.callId, path: request.path, ruleId: request.ruleId, targetVersion: request.targetVersion } })
  else send({ ...base, error: result?.error })
  finish()
}
async function handle(value: unknown): Promise<void> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) { error('malformed_json', 'object required'); return }
  const message = value as Wire
  if (message.type === 'cancel') {
    const cancel = parseCancel(message)
    if (!cancel || !request || !sameIds(request, cancel) || !inFlight) { error('invalid_cancel', 'cancel identifiers do not match the active request', isText(message.requestId) ? message.requestId : undefined); return }
    cancelRequested = true
    return
  }
  const next = parseRequest(message)
  if (!next) { error(message.protocol !== PROTOCOL_VERSION ? 'protocol_version' : 'invalid_request', 'invoke fields or protocol are invalid', isText(message.requestId) ? message.requestId : undefined); return }
  if (request || inFlight) { error('session_busy', 'session accepts one invoke'); return }
  request = next
  if (request.tool !== 'read_file' || request.ruleId !== 'read-file-content-v1') { terminal('tool_failed', { ok: false, error: { code: 'permission_denied', message: 'tool denied by core policy' } } as Result); return }
  inFlight = true
  send({ protocol: PROTOCOL_VERSION, type: 'started', sessionId: request.sessionId, requestId: request.requestId, taskId: request.taskId, callId: request.callId })
  const result = await dispatchReadonly({ id: request.callId, name: 'read_file', arguments: { path: request.path } } as ToolCall, workspace)
  inFlight = false
  if (cancelRequested) terminal('cancelled')
  else terminal(result.ok ? 'succeeded' : 'tool_failed', result)
}

send({ protocol: PROTOCOL_VERSION, type: 'ready', capabilities: ['read_file'], maxMessageBytes: MAX_MESSAGE })
let pending = Buffer.alloc(0)
process.stdin.on('data', (chunk: Buffer | string) => {
  if (closing) return
  const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk)
  const firstNewline = bytes.indexOf(10)
  const frameLength = pending.length + (firstNewline >= 0 ? firstNewline : bytes.length)
  if (frameLength > MAX_MESSAGE) { error('message_too_large', 'message exceeds limit'); finish(); return }
  pending = Buffer.concat([pending, bytes])
  let newline = pending.indexOf(10)
  while (newline >= 0) {
    const line = pending.subarray(0, newline); pending = pending.subarray(newline + 1)
    if (line.length > MAX_MESSAGE) { error('message_too_large', 'message exceeds limit'); finish(); return }
    try {
      const text = new TextDecoder('utf-8', { fatal: true }).decode(line)
      void handle(JSON.parse(text))
    } catch { error('malformed_json', 'invalid JSON') }
    newline = pending.indexOf(10)
  }
})
