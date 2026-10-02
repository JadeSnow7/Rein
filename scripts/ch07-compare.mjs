import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'
import { isDeepStrictEqual } from 'node:util'
const root = resolve(fileURLToPath(new URL('..', import.meta.url)))
const fixture = resolve(root, 'fixtures/cases/ch07-context.json')
const run = (command, args, cwd) => { const result = spawnSync(command, args, { cwd, encoding: 'utf8' }); if (result.error) throw result.error; if (result.status !== 0) throw new Error(`${command} failed: ${result.stderr}`); return JSON.parse(result.stdout) }
const ts = run('npm', ['run', '--silent', 'ch07:context', '--', fixture], root)
const rust = run('cargo', ['run', '--quiet', '--manifest-path', resolve(root, 'rust/Cargo.toml'), '--example', 'ch07_context', '--', fixture], root)
const normalize = (value, path = []) => {
  if (Array.isArray(value)) return value.map((item, index) => normalize(item, [...path, index]))
  if (!value || typeof value !== 'object') return value
  const out = {}
  for (const [key, child] of Object.entries(value)) {
    const resultError = key === 'error' && path.at(-1) === 'result'
    if (resultError && child && typeof child === 'object') out[key] = { code: child.code ?? null }
    else if (resultError && typeof child === 'string') out[key] = null
    else out[key] = normalize(child, [...path, key])
  }
  return out
}
const fixtureCases = JSON.parse(readFileSync(fixture, 'utf8')).cases
const acceptedMetrics = new Map([['managed-window', { beforeUnits: 644, afterUnits: 239, requiredUnits: 239, keptGroups: ['g1', 'g4'], removedGroups: ['g0'] }], ['multi-tool-boundary', { beforeUnits: 382, afterUnits: 219, requiredUnits: 204, keptGroups: ['g4', 'g5', 'g8'], removedGroups: ['g0', 'g1'] }]])
const caseIds = fixtureCases.map(item => item.id)
if (new Set(caseIds).size !== caseIds.length || new Set(caseIds).size !== 7) throw new Error('fixture case IDs must be unique and complete')
for (const implementation of [ts, rust]) {
  if (implementation.unit !== 'estimated-bytes-v1' || implementation.cases.length !== fixtureCases.length) throw new Error('invalid ch07 output envelope')
  const actualIds = implementation.cases.map(item => item.id)
  if (new Set(actualIds).size !== actualIds.length || !isDeepStrictEqual([...actualIds].sort(), [...caseIds].sort())) throw new Error('implementation case IDs do not exactly match fixture')
  for (const actual of implementation.cases) {
    const expected = fixtureCases.find(item => item.id === actual.id)
    if (!expected) throw new Error(`unknown case ${actual.id}`)
    if (actual.result.reason !== expected.expected.reason || actual.requests !== expected.expected.requests) throw new Error(`${actual.id} does not satisfy fixture expectation`)
    const requested = actual.result.events.filter(event => event.type === 'model_requested')
    if (requested.length !== actual.requests || requested.length !== actual.sentMessages.length) throw new Error(`${actual.id} request/event mismatch`)
    for (let i = 0; i < requested.length; i++) if (!isDeepStrictEqual(requested[i].messages, actual.sentMessages[i])) throw new Error(`${actual.id} sent message audit mismatch`)
    const contextEvent = actual.result.events.find(event => event.type === 'context_prepared')
    const metrics = acceptedMetrics.get(actual.id)
    if (metrics && implementation === ts && (!contextEvent || !isDeepStrictEqual({ beforeUnits: contextEvent.beforeUnits, afterUnits: contextEvent.afterUnits, requiredUnits: contextEvent.requiredUnits, keptGroups: contextEvent.keptGroups, removedGroups: contextEvent.removedGroups }, metrics))) throw new Error(`${actual.id} context metrics differ from accepted values`)
    if (actual.result.reason === 'final_answer' && actual.result.answer && contextEvent) {
      const sent = actual.sentMessages.at(-1) ?? []
      const groups = sent.map((message, index) => message.role === 'assistant' && message.toolCalls?.length ? { index, calls: message.toolCalls } : undefined).filter(Boolean)
      const latest = groups.at(-1)
      if (latest) {
        const contentById = new Map(sent.filter(message => message.role === 'tool').map(message => [message.toolCallId, message.content]))
        const derived = latest.calls.map(call => contentById.get(call.id)).join('\n')
        if (derived !== actual.result.answer) throw new Error(`${actual.id} answer is not derived from latest tool group`)
      }
    }
  }
}
if (!isDeepStrictEqual(normalize(ts), normalize(rust))) throw new Error('ch07 TypeScript/Rust behavior differs')
process.stdout.write(JSON.stringify({ unit: ts.unit, cases: ts.cases.map(c => ({ id: c.id, reason: c.result.reason, requests: c.requests })) }) + '\n')
