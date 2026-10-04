import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { tmpdir } from 'node:os'
import { runAgentLoop } from '../src/rein/loop'
import type { ChatAdapter, ModelTurn, ToolCall } from '../src/rein/contracts'

const source = JSON.parse(await readFile(resolve(process.argv[2] ?? '../fixtures/cases/ch05-loop.json'), 'utf8'))
const fixture = source.cases ? source.cases[Number(process.argv[3] ?? 0)] : source
const root = await mkdtemp(join(tmpdir(), 'rein-ch05-compare-'))
try {
  for (const [path, content] of Object.entries(fixture.workspaceFiles)) await writeFile(join(root, path), content as string)
  let index = 0
  const adapter: ChatAdapter = { async complete() { const turn = fixture.modelTurns[index++] as { content: string; toolCalls: ToolCall[] }; return { message: { role: 'assistant', content: turn.content, toolCalls: turn.toolCalls }, toolCalls: turn.toolCalls } } }
  const result = await runAgentLoop(adapter, { root }, fixture.prompt)
  const normalized = { state: result.state, reason: result.reason, answer: result.answer, messages: result.messages, events: result.events.map((event: any) => event.type === 'tool_result' ? { type: event.type, turn: event.turn, call: event.call, toolCallId: event.toolCallId, ok: event.result.ok, errorCode: event.result.error?.code } : event) }
  if (normalized.state !== fixture.expected.state || normalized.reason !== fixture.expected.reason || normalized.answer !== fixture.expected.answer) throw new Error('fixture expected mismatch')
  const errorCodes = result.events.filter(e => e.type === 'tool_result' && !(e as any).result.ok).map(e => (e as any).result.error?.code)
  if (JSON.stringify(errorCodes) !== JSON.stringify(fixture.expected.errorCodes ?? [])) throw new Error('fixture error code mismatch')
  if (JSON.stringify(normalized.answer) === JSON.stringify(fixture.modelTurns.at(-1)?.content) && result.events.filter(e => e.type === 'tool_result').length === 0) throw new Error('answer was not produced after tool execution')
  console.log(JSON.stringify(normalized))
} finally { await rm(root, { recursive: true, force: true }) }
