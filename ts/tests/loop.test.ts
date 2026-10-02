import { describe, expect, it } from 'vitest'
import { runAgentLoop } from '../src/rein/loop'
import { createOpenAIAdapter } from '../src/rein/adapters'
import { createFetchTransport } from '../src/transport'
import type { Transport } from '../src/transport'
import type { ChatAdapter, ModelTurn } from '../src/rein/contracts'
import { mkdtemp, writeFile, rm } from 'node:fs/promises'
import { join } from 'node:path'
import { tmpdir } from 'node:os'

const turn = (content: string, toolCalls: ModelTurn['toolCalls'] = []): ModelTurn => ({ message: { role: 'assistant', content, toolCalls }, toolCalls })
const tool = (id: string, name: string, args: Record<string, unknown>) => ({ id, name, arguments: args })

describe('ch05 agent loop', () => {
  it('searches and reads real content, then sends both results to the next model turn', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-loop-'))
    try {
      await writeFile(join(root, 'a.txt'), 'alpha secret')
      await writeFile(join(root, 'b.txt'), 'beta')
      const received: any[][] = []
      const adapter: ChatAdapter = { async complete(messages) { received.push(structuredClone(messages) as any); return received.length === 1 ? turn('', [tool('s', 'search_files', { needle: 'secret' }), tool('r', 'read_file', { path: 'a.txt' })]) : turn('总结：alpha secret') } }
      const result = await runAgentLoop(adapter, { root }, 'inspect')
      expect(result.answer).toBe('总结：alpha secret')
      expect(received[1]?.filter(m => m.role === 'tool').map(m => m.content)).toEqual(['a.txt', 'alpha secret'])
      expect(result.events.filter(e => e.type === 'tool_result').map(e => e.toolCallId)).toEqual(['s', 'r'])
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('returns structured failures and protects the turn budget', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-loop-'))
    try {
      const adapter: ChatAdapter = { async complete() { return turn('', [tool('bad', 'unknown', {})]) } }
      const result = await runAgentLoop(adapter, { root }, 'inspect', { maxTurns: 2 })
      expect(result.reason).toBe('max_turns')
      expect(result.events.some(e => e.type === 'tool_result' && !e.result.ok && e.result.error?.code === 'unknown_tool')).toBe(true)
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('feeds unknown, bad arguments, and missing files back so the model can recover', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-loop-recover-'))
    try {
      await writeFile(join(root, 'ok.txt'), 'recovered')
      let turnNumber = 0
      const received: any[][] = []
      const adapter: ChatAdapter = { async complete(messages) {
        turnNumber++
        received.push(structuredClone(messages) as any)
        if (turnNumber === 1) return turn('', [tool('u', 'unknown', {}), tool('a', 'read_file', { path: 3 }), tool('m', 'read_file', { path: 'missing.txt' })])
        if (turnNumber === 2) return turn('', [tool('r', 'read_file', { path: 'ok.txt' })])
        return turn('recovered')
      } }
      const result = await runAgentLoop(adapter, { root }, 'recover')
      expect(result).toMatchObject({ state: 'completed', reason: 'final_answer', answer: 'recovered' })
      expect(result.events.filter(e => e.type === 'tool_result').map(e => e.result.error?.code)).toEqual(['unknown_tool', 'arguments_invalid', 'path_invalid', undefined])
      expect(received[1]?.filter(m => m.role === 'tool').map(m => JSON.parse(m.content).error.code)).toEqual(['unknown_tool', 'arguments_invalid', 'path_invalid'])
      expect(received[2]?.filter(m => m.role === 'tool').slice(-1).map(m => m.toolCallId)).toEqual(['r'])
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('uses the default 32-turn protection and records every request', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-loop-max-'))
    try {
      const adapter: ChatAdapter = { async complete() { return turn('', [tool('u', 'unknown', {})]) } }
      const result = await runAgentLoop(adapter, { root }, 'max')
      expect(result.reason).toBe('max_turns')
      expect(result.events.filter(e => e.type === 'model_requested')).toHaveLength(32)
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('does not invent an answer for model errors or empty final messages', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-loop-'))
    try {
      const failed = await runAgentLoop({ async complete() { throw new Error('offline failure') } }, { root }, 'x')
      expect(failed).toMatchObject({ state: 'failed', reason: 'model_error', error: 'offline failure' })
      const empty = await runAgentLoop({ async complete() { return turn('   ') } }, { root }, 'x')
      expect(empty).toMatchObject({ state: 'failed', reason: 'empty_final' })
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('uses the search result to choose reads in either workspace and records reconstructable events', async () => {
    const cases: Array<Array<[string, string]>> = [[['a.md', 'marker: blue'], ['b.md', 'marker: sea']], [['changed.md', 'marker: gold'], ['other.md', 'marker: wind']]]
    for (const files of cases) {
      const root = await mkdtemp(join(tmpdir(), 'rein-loop-workspace-'))
      try {
        for (const [path, content] of files) await writeFile(join(root, path), content)
        const seen: any[] = []
        const adapter: ChatAdapter = { async complete(messages) {
          seen.push(structuredClone(messages))
          const toolMessages = messages.filter(m => m.role === 'tool')
          if (!toolMessages.length) return turn('', [tool('search', 'search_files', { needle: 'marker:' })])
          if (toolMessages.length === 1) return turn('', (toolMessages[0]?.content ?? '').split('\n').filter(Boolean).map((path, i) => tool(`read-${i}`, 'read_file', { path })))
          return turn(toolMessages.slice(1).map(m => m.content).join('|'))
        } }
        const result = await runAgentLoop(adapter, { root }, 'inspect')
        expect(result.answer).toContain(files[0]?.[1])
        expect(seen[2]?.filter((m: any) => m.role === 'tool').slice(1).map((m: any) => m.content)).toEqual(files.map(f => f[1]))
        const request = result.events.find((event) => event.type === 'model_requested')
        expect(request).toMatchObject({ type: 'model_requested', tools: expect.any(Array), messages: expect.any(Array) })
      } finally { await rm(root, { recursive: true, force: true }) }
    }
  })

  it('returns a model_error when replay responses are exhausted after a tool call', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-loop-exhausted-'))
    try {
      const result = await runAgentLoop({ async complete(messages) { if (messages.length === 1) return turn('', [tool('x', 'search_files', { needle: 'x' })]); throw new Error('replay exhausted') } }, { root }, 'inspect')
      expect(result).toMatchObject({ state: 'failed', reason: 'model_error', error: 'replay exhausted' })
      expect(result.events.some(e => e.type === 'tool_result' && e.toolCallId === 'x')).toBe(true)
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('enforces model and tool budgets and records unstarted calls', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-loop-control-'))
    try {
      let models = 0
      const adapter: ChatAdapter = { async complete() { models++; return turn('', [tool('a', 'search_files', { needle: 'x' }), tool('b', 'search_files', { needle: 'y' })]) } }
      const result = await runAgentLoop(adapter, { root }, 'budget', { maxTurns: 3, maxToolCalls: 1 })
      expect(result.reason).toBe('tool_budget_exhausted')
      expect(models).toBe(1)
      expect(result.events.filter(e => e.type === 'tool_result')).toHaveLength(1)
      expect(result.events.filter(e => e.type === 'action_skipped')).toMatchObject([{ callId: 'b', reason: 'tool_budget_exhausted' }])
      const zero = await runAgentLoop(adapter, { root }, 'zero', { maxTurns: 0 })
      expect(zero.reason).toBe('max_turns')
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('detects duplicate actions by canonical arguments even when ids and key order differ', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-loop-duplicate-'))
    try {
      let calls = 0
      const adapter: ChatAdapter = { async complete() { return turn('', [tool(`id-${++calls}`, 'search_files', calls === 1 ? { needle: 'x', extra: { b: 2, a: 1 } } : { extra: { a: 1, b: 2 }, needle: 'x' })]) } }
      const result = await runAgentLoop(adapter, { root }, 'duplicate', { duplicateLimit: 1, maxTurns: 3 })
      expect(result.reason).toBe('duplicate_action')
      expect(result.events.filter(e => e.type === 'tool_result')).toHaveLength(1)
      expect(result.events.filter(e => e.type === 'action_skipped')).toHaveLength(1)
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('stops on cancellation and deadline while a model is pending, without later dispatch', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-loop-cancel-'))
    try {
      const controller = new AbortController()
      let started = 0
      const adapter: ChatAdapter = { async complete() { started++; setTimeout(() => controller.abort(), 5); await new Promise(resolve => setTimeout(resolve, 30)); return turn('late') } }
      const result = await runAgentLoop(adapter, { root }, 'cancel', { signal: controller.signal })
      expect(result.reason).toBe('cancelled')
      await new Promise(resolve => setTimeout(resolve, 40))
      expect(started).toBe(1)
      const timed = await runAgentLoop(adapter, { root }, 'timeout', { timeoutMs: 5 })
      expect(timed.reason).toBe('timeout')
      await new Promise(resolve => setTimeout(resolve, 40))
      expect(started).toBe(2)
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('passes the external signal through the OpenAI adapter to transport', async () => {
    let received: AbortSignal | undefined
    const transport: Transport = { async send(_request, options) { received = options.signal; return { status: 200, statusText: 'OK', headers: {}, body: JSON.stringify({ choices: [{ message: { content: 'ok' } }] }) } } }
    const controller = new AbortController()
    await createOpenAIAdapter({ baseUrl: 'https://example.test', apiKey: 'test', model: 'model' }, transport).complete([{ role: 'user', content: 'x' }], [], { signal: controller.signal })
    expect(received).toBe(controller.signal)
  })

  it('cancels between two real tools after the trusted result observer', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-loop-between-tools-'))
    try {
      await writeFile(join(root, 'a.txt'), 'a')
      await writeFile(join(root, 'b.txt'), 'b')
      const controller = new AbortController()
      let calls = 0
      const result = await runAgentLoop({ async complete() { calls++; return turn('', [tool('one', 'read_file', { path: 'a.txt' }), tool('two', 'read_file', { path: 'b.txt' })]) } }, { root }, 'x', {
        maxToolCalls: 2,
        signal: controller.signal,
        onToolResult: (_call, value) => { if (value.toolCallId === 'one') controller.abort() },
      })
      expect(result).toMatchObject({ state: 'failed', reason: 'cancelled' })
      expect(result.events.filter(event => event.type === 'tool_result').map(event => event.toolCallId)).toEqual(['one'])
      expect(result.events.filter(event => event.type === 'action_skipped').map(event => [event.callId, event.reason])).toEqual([['two', 'cancelled']])
      expect(calls).toBe(1)
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('rejects a final answer when the adapter cancels synchronously before returning it', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-loop-return-cancel-'))
    try {
      const controller = new AbortController()
      const result = await runAgentLoop({ async complete() { controller.abort(); return turn('should be rejected') } }, { root }, 'x', { signal: controller.signal })
      expect(result).toMatchObject({ state: 'failed', reason: 'cancelled' })
      expect(result.answer).toBeUndefined()
      expect(result.events.some(event => event.type === 'model_received')).toBe(false)
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('cleans race control resources when the model throws synchronously', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-loop-sync-throw-'))
    try {
      const controller = new AbortController()
      const result = await runAgentLoop({ complete() { throw new Error('sync failure') } }, { root }, 'x', { signal: controller.signal, timeoutMs: 1000 })
      expect(result).toMatchObject({ state: 'failed', reason: 'model_error', error: 'sync failure' })
      controller.abort()
      await new Promise(resolvePromise => setTimeout(resolvePromise, 10))
      expect(result.events.at(-1)).toMatchObject({ type: 'stopped', reason: 'model_error' })
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('returns cancelled without an unhandled rejection when abort and sync throw coincide', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-loop-sync-abort-'))
    try {
      const controller = new AbortController()
      const result = await runAgentLoop({ complete() { controller.abort(); throw new Error('sync after abort') } }, { root }, 'x', { signal: controller.signal, timeoutMs: 1000 })
      expect(result).toMatchObject({ state: 'failed', reason: 'cancelled' })
      expect(result.answer).toBeUndefined()
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('passes cancellation into the real fetch transport path', async () => {
    const originalFetch = globalThis.fetch
    const controller = new AbortController()
    let observedSignal: AbortSignal | undefined
    globalThis.fetch = ((_url: string | URL, init?: RequestInit) => {
      observedSignal = init?.signal ?? undefined
      return new Promise<Response>((_, reject) => {
        init?.signal?.addEventListener('abort', () => reject(new Error('fetch aborted')), { once: true })
      })
    }) as typeof fetch
    try {
      const pending = createFetchTransport().send({ method: 'POST', url: 'https://example.test', headers: {}, body: '{}' }, { timeoutMs: 1000, signal: controller.signal })
      controller.abort()
      await expect(pending).rejects.toMatchObject({ name: 'NetworkError' })
      expect(observedSignal).toBeDefined()
      expect(observedSignal?.aborted).toBe(true)
    } finally { globalThis.fetch = originalFetch }
  })
})
