import { execFileSync } from 'node:child_process'
import { deepStrictEqual } from 'node:assert'
import { resolve } from 'node:path'
import { cargoTargetDir } from './build-artifacts.mjs'
const root = process.cwd()
const target = cargoTargetDir(root, 'target')
const fixture = 'fixtures/cases/ch05-loop.json'
const lastJson = text => JSON.parse(text.trim().split('\n').filter(Boolean).at(-1))
const source = JSON.parse(await (await import('node:fs/promises')).readFile(resolve(root, fixture), 'utf8'))
const cases = source.cases ?? [source]
const runCase = index => {
  const ts = lastJson(execFileSync('npm', ['run', 'ch05:compare', '--workspace', 'ts', '--', `../${fixture}`, String(index)], { cwd: root, encoding: 'utf8' }))
  const rust = lastJson(execFileSync('cargo', ['run', '--quiet', '--manifest-path', 'rust/Cargo.toml', '--example', 'ch05_compare', '--', fixture, String(index)], { cwd: root, encoding: 'utf8', env: { ...process.env, CARGO_TARGET_DIR: target } }))
  return { ts, rust }
}
const message = m => ({ role: m.role, content: m.role === 'tool' ? canonicalToolContent(m.content) : m.content, toolCallId: m.toolCallId ?? null, toolCalls: (m.toolCalls ?? []).map(call) })
const canonicalToolContent = content => { try { const value = JSON.parse(content); if (value?.ok === false && value.error?.code) return JSON.stringify({ ok: false, error: { code: value.error.code } }) } catch {} return content }
const call = c => ({ id: c.id, name: c.name, arguments: c.arguments })
const normalize = result => ({
  state: result.state, reason: result.reason, answer: result.answer ?? null,
  messages: result.messages.map(message),
  events: result.events.map(event => event.type === 'model_requested'
    ? { type: event.type, turn: event.turn, messages: event.messages.map(message), tools: event.tools.map(tool => ({ name: tool.name, description: tool.description, inputSchema: tool.inputSchema ?? tool.input_schema })) }
    : event.type === 'model_received'
      ? { type: event.type, turn: event.turn, message: message(event.message), toolCallIds: event.toolCallIds ?? event.tool_call_ids, text: event.text }
      : event.type === 'tool_result'
        ? { type: event.type, turn: event.turn, call: call(event.call), toolCallId: event.toolCallId ?? event.tool_call_id, ok: event.ok ?? event.result?.ok, errorCode: event.errorCode ?? event.result?.error?.code ?? null }
        : { type: event.type, state: event.state, reason: event.reason })
})
const results = cases.map((expected, index) => {
  const { ts, rust } = runCase(index); const tsNormalized = normalize(ts); const rustNormalized = normalize(rust)
  deepStrictEqual(rustNormalized, tsNormalized)
  const readPaths = tsNormalized.events.filter(event => event.type === 'tool_result' && event.call.name === 'read_file').map(event => event.call.arguments.path)
  const errorCodes = tsNormalized.events.filter(event => event.type === 'tool_result' && event.errorCode).map(event => event.errorCode)
  deepStrictEqual({ state: tsNormalized.state, reason: tsNormalized.reason, answer: tsNormalized.answer, readPaths, errorCodes }, { state: expected.expected.state, reason: expected.expected.reason, answer: expected.expected.answer, readPaths: expected.expected.readPaths ?? [], errorCodes: expected.expected.errorCodes ?? [] })
  return { index, normalized: tsNormalized }
})
console.log(JSON.stringify({ equal: true, cases: results }))
