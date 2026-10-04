import { describe, expect, it } from 'vitest'
import { readFile, mkdtemp, symlink, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { anthropicRequest, anthropicResponseToMessage, createOpenAIAdapter, createReplayAdapter } from '../src/rein/adapters'
import { dispatchReadonly, readFileTool, searchFilesTool } from '../src/rein/readonly'
import type { Message, ToolCall } from '../src/rein/contracts'

const fixture = JSON.parse(await readFile(join(process.cwd(), '../fixtures/cases/prerequisites.json'), 'utf8')) as any
const workspace = { root: join(process.cwd(), '../fixtures/workspaces/prerequisites') }

describe('ch05 prerequisites', () => {
  it('reads shared fixture and handles Anthropic text, tools, system, and grouped results', () => {
    const request = JSON.parse(anthropicRequest([
      { role: 'system', content: 'rules' },
      { role: 'assistant', content: '', toolCalls: [{ id: 'u1', name: 'read_file', arguments: { path: 'README.md' } }, { id: 'u2', name: 'search_files', arguments: { needle: 'workspace' } }] },
      { role: 'tool', toolCallId: 'u1', content: 'one' }, { role: 'tool', toolCallId: 'u2', content: 'two' },
    ], 'claude-test'))
    expect(request.system).toBe('rules')
    expect(request.messages[0]!.content).toHaveLength(2)
    const turn = anthropicResponseToMessage(JSON.stringify(fixture.anthropic_response))
    expect(turn.message.content).toBe(fixture.expected.assistant_text)
    expect(turn.toolCalls.map(call => call.id)).toEqual(fixture.expected.anthropic_tool_ids)
    expect(request.messages[1]!.content).toHaveLength(2)
  })

  it('dispatches real fixture tools with call ids and rejects unknown/invalid paths', async () => {
    const call: ToolCall = { id: 'call-real', name: 'read_file', arguments: { path: 'README.md' } }
    await expect(dispatchReadonly(call, workspace)).resolves.toMatchObject({ toolCallId: 'call-real', ok: true, output: expect.stringContaining('前置 workspace') })
    await expect(dispatchReadonly({ id: 'unknown-1', name: 'nope', arguments: {} }, workspace)).resolves.toMatchObject({ toolCallId: 'unknown-1', error: { code: 'unknown_tool' } })
    await expect(dispatchReadonly({ id: 'bad-1', name: 'read_file', arguments: { path: '../README.md' } }, workspace)).resolves.toMatchObject({ toolCallId: 'bad-1', error: { code: 'path_escape' } })
    await expect(dispatchReadonly({ id: 'bad-2', name: 'read_file', arguments: { path: 'missing' } }, workspace)).resolves.toMatchObject({ toolCallId: 'bad-2', error: { code: 'path_invalid' } })
    await expect(searchFilesTool(workspace, '前置')).resolves.toMatchObject({ ok: true, output: 'README.md' })
    await expect(readFileTool(workspace, '.')).resolves.toMatchObject({ ok: false })
  })

  it('rejects a symlink that escapes the workspace', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-prereq-'))
    try {
      await symlink(join(process.cwd(), 'package.json'), join(root, 'outside.txt'))
      await expect(readFileTool({ root }, 'outside.txt')).resolves.toMatchObject({ ok: false, error: { code: 'path_escape' } })
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('captures two OpenAI protocol requests and preserves multiple tool calls/results', async () => {
    const bodies: string[] = []
    const responses = [JSON.stringify(fixture.openai_response), JSON.stringify(fixture.openai_followup_response)]
    const transport = { async send(request: { body: string }) { bodies.push(request.body); return { status: 200, statusText: 'OK', headers: {}, body: responses[bodies.length - 1]! } } }
    const adapter = createOpenAIAdapter({ baseUrl: 'http://loopback.test', apiKey: 'key', model: 'model' }, transport)
    const first = await adapter.complete([{ role: 'user', content: 'inspect fixture' }])
    const results = await Promise.all(first.toolCalls.map(call => dispatchReadonly(call, workspace)))
    const secondMessages: Message[] = [{ role: 'user', content: 'inspect fixture' }, { role: 'assistant', content: '', toolCalls: first.toolCalls }, ...results.map(result => ({ role: 'tool' as const, toolCallId: result.toolCallId, content: result.output ?? result.error?.message ?? '' }))]
    await expect(adapter.complete(secondMessages)).resolves.toMatchObject({ message: { content: 'done' } })
    expect(bodies).toHaveLength(2)
    const sent = JSON.parse(bodies[1]!)
    const assistant = sent.messages.find((message: { role: string }) => message.role === 'assistant')
    const toolMessages = sent.messages.filter((message: { role: string }) => message.role === 'tool')
    expect(assistant!.tool_calls).toEqual(first.toolCalls.map(call => ({ id: call.id, type: 'function', function: { name: call.name, arguments: JSON.stringify(call.arguments) } })))
    expect(toolMessages.map((message: { tool_call_id: string }) => message.tool_call_id)).toEqual(first.toolCalls.map(call => call.id))
    expect(toolMessages.map((message: { content: string }) => message.content)).toEqual(results.map(result => result.output))
  })

  it('replay records messages, supports pure tools, and exhausts explicitly', async () => {
    const replay = createReplayAdapter([JSON.stringify(fixture.anthropic_response), JSON.stringify(fixture.anthropic_response)])
    const turn = await replay.complete([{ role: 'user', content: 'read' }])
    expect(turn.toolCalls.map(call => call.id)).toEqual(fixture.expected.anthropic_tool_ids)
    expect(replay.receivedMessages).toHaveLength(1)
    const firstHistory = replay.receivedMessages[0]!
    const secondInput: Message[] = [{ role: 'user', content: 'second' }]
    await replay.complete(secondInput)
    secondInput.push({ role: 'user', content: 'mutated-after-call' })
    expect(replay.receivedMessages[1]).toEqual([{ role: 'user', content: 'second' }])
    expect(firstHistory).toEqual([{ role: 'user', content: 'read' }])
    await expect(replay.complete([])).rejects.toThrow('replay exhausted')
  })

  it('rejects every malformed shared fixture response and preserves null tool content', async () => {
    for (const sample of fixture.malformed_responses) {
      const parse = sample.provider === 'openai' ? () => createOpenAIAdapter({ baseUrl: 'http://loopback.test', apiKey: 'key', model: 'model' }, { async send() { return { status: 200, statusText: 'OK', headers: {}, body: JSON.stringify(sample.body) } } }).complete([]) : () => Promise.resolve().then(() => anthropicResponseToMessage(JSON.stringify(sample.body)))
      await expect(parse()).rejects.toThrow()
    }
    const pureTools = createOpenAIAdapter({ baseUrl: 'http://loopback.test', apiKey: 'key', model: 'model' }, { async send() { return { status: 200, statusText: 'OK', headers: {}, body: JSON.stringify(fixture.openai_response) } } })
    await expect(pureTools.complete([])).resolves.toMatchObject({ toolCalls: [{ id: 'call-1' }, { id: 'call-2' }] })
  })

  it('serializes the canonical optional-field sample like Rust', () => {
    expect(JSON.stringify(fixture.canonical_sample)).toBe(JSON.stringify({ role: 'user', content: 'canonical' }))
  })
})
