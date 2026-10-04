import { spawnSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { isDeepStrictEqual } from 'node:util'

const root = resolve(new URL('..', import.meta.url).pathname)
const fixture = JSON.parse(readFileSync(new URL('../fixtures/cases/ch06-control.json', import.meta.url), 'utf8'))
const expectedModes = [...fixture.modes].sort()
const declaredModes = Object.keys(fixture.expected).sort()
if (JSON.stringify(expectedModes) !== JSON.stringify(declaredModes)) throw new Error('fixture modes and expected entries differ')

function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', env: process.env })
  if (result.error) throw result.error
  if (result.status !== 0) throw new Error(`${command} ${args.join(' ')} exited ${result.status}: ${result.stderr}`)
  const start = result.stdout.indexOf('{')
  if (start < 0) throw new Error(`${command} produced no JSON output`)
  return JSON.parse(result.stdout.slice(start))
}

function normalize(raw) {
  const result = raw.result
  const events = result.events ?? []
  const toolEvents = events.filter(event => event.type === 'tool_result')
  const toolMessageIds = (result.messages ?? []).filter(message => message.role === 'tool').map(message => message.toolCallId)
  const eventToolIds = toolEvents.map(event => event.toolCallId)
  if (JSON.stringify(toolMessageIds) !== JSON.stringify(eventToolIds)) throw new Error(`${raw.mode}: tool results and tool messages do not correspond`)
  if (toolEvents.some(event => event.toolCallId !== event.call.id)) throw new Error(`${raw.mode}: tool result call id mismatch`)
  if ((raw.result.reason === 'cancelled' || raw.result.reason === 'timeout') && raw.result.answer !== undefined) throw new Error(`${raw.mode}: cancelled run fabricated an answer`)
  const toolCalls = toolEvents.map(event => ({ id: event.call.id, name: event.call.name, arguments: event.call.arguments }))
  const skipped = events.filter(event => event.type === 'action_skipped' && event.action === 'tool').map(event => [event.callId, event.reason])
  const answer = result.answer ?? undefined
  return {
    state: result.state,
    reason: result.reason,
    requests: raw.requests,
    settledRequests: raw.settledRequests,
    modelRequests: events.filter(event => event.type === 'model_requested').length,
    toolCalls,
    skipped,
    ...(answer === undefined ? {} : { answer }),
  }
}

function assertEqual(label, actual, expected) {
  if (!isDeepStrictEqual(actual, expected)) throw new Error(`${label}\nexpected ${JSON.stringify(expected)}\nactual   ${JSON.stringify(actual)}`)
}
const report = []
for (const mode of fixture.modes) {
  const ts = run('npm', ['run', '--silent', 'ch06:offline', '--workspace', 'ts', '--', mode])
  const rust = run('cargo', ['run', '--quiet', '--manifest-path', 'rust/Cargo.toml', '--example', 'ch06_loop', '--', mode])
  const expected = fixture.expected[mode]
  const tsNormalized = normalize(ts)
  const rustNormalized = normalize(rust)
  assertEqual(`${mode}: TypeScript vs fixture`, tsNormalized, expected)
  assertEqual(`${mode}: Rust vs fixture`, rustNormalized, expected)
  assertEqual(`${mode}: language parity`, tsNormalized, rustNormalized)
  report.push({ mode, typescript: tsNormalized, rust: rustNormalized })
}
console.log(JSON.stringify({ passed: true, modes: report }, null, 2))
