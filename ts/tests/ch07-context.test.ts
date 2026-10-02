import { describe, expect, it } from 'vitest'
import { runAgentLoop, estimateMessages } from '../src/rein/loop'
import type { ChatAdapter } from '../src/rein/contracts'
import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { tmpdir } from 'node:os'

const answer = (content: string) => ({ message: { role: 'assistant' as const, content }, toolCalls: [] })

describe('ch07 context management', () => {
  it('keeps rules, goal, and the complete latest multi-tool group while removing old groups', async () => {
    const history = [
      { role: 'user' as const, content: 'old' },
      { role: 'assistant' as const, content: '', toolCalls: [{ id: 'a', name: 'read_file', arguments: { path: 'a' } }, { id: 'b', name: 'read_file', arguments: { path: 'b' } }] },
      { role: 'tool' as const, toolCallId: 'a', content: 'A' }, { role: 'tool' as const, toolCallId: 'b', content: 'B' },
    ]
    let received: any[] = []
    const adapter: ChatAdapter = { async complete(messages) { received = structuredClone(messages) as any; return answer('ok') } }
    const result = await runAgentLoop(adapter, { root: process.cwd() }, 'goal', { context: { rules: ['rule'], history, budget: 139 } })
    expect(result.reason).toBe('final_answer')
    expect(received.filter(m => m.role === 'system').map(m => m.content)).toEqual(['rule'])
    expect(received.filter(m => m.role === 'tool').map(m => m.toolCallId)).toEqual(['a', 'b'])
    expect(result.messages).toHaveLength(history.length + 2)
    expect(result.events.find(e => e.type === 'context_prepared')).toMatchObject({ removedGroups: ['g0'], requiredGroups: ['g1', 'g4'] })
  })

  it('does not dispatch when required content exceeds budget', async () => {
    let calls = 0
    const adapter: ChatAdapter = { async complete() { calls++; return answer('bad') } }
    const result = await runAgentLoop(adapter, { root: process.cwd() }, 'goal', { context: { rules: ['rule'], history: [], budget: 1 } })
    expect(result).toMatchObject({ reason: 'context_budget_exhausted', state: 'failed' })
    expect(calls).toBe(0)
  })

  it('rejects malformed history and protects audit history from adapter mutation', async () => {
    const malformed = await runAgentLoop({ async complete() { return answer('never') } }, { root: process.cwd() }, 'goal', { context: { rules: [], history: [{ role: 'tool', toolCallId: 'orphan', content: 'x' }], budget: 100 } })
    expect(malformed.reason).toBe('invalid_context_history')
    const history = [{ role: 'user' as const, content: 'source' }]
    const result = await runAgentLoop({ async complete(messages) { (messages as any).push({ role: 'system', content: 'tampered' }); return answer('ok') } }, { root: process.cwd() }, 'goal', { context: { rules: [], history, budget: 100 } })
    expect(result.messages.map(message => message.content)).toEqual(['source', 'goal', 'ok'])
  })

  it('counts UTF-8 bytes deterministically', () => {
    expect(estimateMessages([{ role: 'user', content: '你好 🌏' }])).toBe(8 + 4 + 11)
  })

  it('runs two real tool rounds and replaces the latest required group', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-ch07-rounds-'))
    try {
      await writeFile(join(root, 'a.txt'), 'first real result')
      await writeFile(join(root, 'b.txt'), 'second real result')
      let calls = 0
      const adapter: ChatAdapter = { async complete(messages) {
        calls++
        const tools = messages.filter(message => message.role === 'tool')
        if (calls === 1) return { message: { role: 'assistant', content: '', toolCalls: [{ id: 'round-a', name: 'read_file', arguments: { path: 'a.txt' } }] }, toolCalls: [{ id: 'round-a', name: 'read_file', arguments: { path: 'a.txt' } }] }
        if (calls === 2) return { message: { role: 'assistant', content: '', toolCalls: [{ id: 'round-b', name: 'read_file', arguments: { path: 'b.txt' } }] }, toolCalls: [{ id: 'round-b', name: 'read_file', arguments: { path: 'b.txt' } }] }
        return answer(tools.at(-1)?.content ?? 'missing')
      } }
      const result = await runAgentLoop(adapter, { root }, 'inspect', { maxTurns: 3, context: { rules: [], history: [], budget: 200 } })
      expect(result.answer).toBe('second real result')
      expect(calls).toBe(3)
      const prepared = result.events.filter(event => event.type === 'context_prepared')
      expect(prepared.at(-1)).toMatchObject({ requiredGroups: ['g0', 'g3'], removedGroups: ['g1'] })
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('stops before the next model when a newly formed required group cannot fit', async () => {
    const root = await mkdtemp(join(tmpdir(), 'rein-ch07-overflow-'))
    try {
      await writeFile(join(root, 'a.txt'), 'large tool result '.repeat(8))
      let calls = 0
      const adapter: ChatAdapter = { async complete() { calls++; return { message: { role: 'assistant', content: '', toolCalls: [{ id: 'overflow-a', name: 'read_file', arguments: { path: 'a.txt' } }] }, toolCalls: [{ id: 'overflow-a', name: 'read_file', arguments: { path: 'a.txt' } }] } } }
      const result = await runAgentLoop(adapter, { root }, 'inspect', { maxTurns: 3, context: { rules: [], history: [], budget: 100 } })
      expect(result.reason).toBe('context_budget_exhausted')
      expect(calls).toBe(1)
    } finally { await rm(root, { recursive: true, force: true }) }
  })

  it('rejects malformed declarations and repeated IDs in a later round', async () => {
    const bad = await runAgentLoop({ async complete() { return answer('never') } }, { root: process.cwd() }, 'x', { context: { rules: [], history: [{ role: 'assistant', content: '', toolCalls: {} as any }], budget: 100 } })
    expect(bad.reason).toBe('invalid_context_history')
    let calls = 0
    const repeated = await runAgentLoop({ async complete() { calls++; return { message: { role: 'assistant', content: '', toolCalls: [{ id: 'same', name: 'read_file', arguments: { path: 'missing' } }] }, toolCalls: [{ id: 'same', name: 'read_file', arguments: { path: 'missing' } }] } } }, { root: process.cwd() }, 'x', { maxTurns: 3, context: { rules: [], history: [], budget: 300 } })
    expect(repeated.reason).toBe('invalid_context_history')
    expect(calls).toBe(2)
  })

  it('rejects empty non-tool IDs and malformed paired results before estimating', async () => {
    const emptyId = await runAgentLoop({ async complete() { return answer('never') } }, { root: process.cwd() }, 'x', { context: { rules: [], history: [{ role: 'user', content: 'x', toolCallId: '' } as any], budget: 100 } })
    expect(emptyId.reason).toBe('invalid_context_history')
    const emptyAssistantId = await runAgentLoop({ async complete() { return answer('never') } }, { root: process.cwd() }, 'x', { context: { rules: [], history: [{ role: 'assistant', content: 'x', toolCallId: '' } as any], budget: 100 } })
    expect(emptyAssistantId.reason).toBe('invalid_context_history')
    const pairedBad = await runAgentLoop({ async complete() { return answer('never') } }, { root: process.cwd() }, 'x', { context: { rules: [], history: [{ role: 'assistant', content: '', toolCalls: [{ id: 'c', name: 'read_file', arguments: {} }] }, { role: 'tool', toolCallId: 'c', content: 'x', toolCalls: {} as any }], budget: 100 } })
    expect(pairedBad.reason).toBe('invalid_context_history')
  })
})
