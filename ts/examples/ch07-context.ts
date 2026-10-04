import { readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { runAgentLoop, estimateMessages } from '../src/rein/loop'
import type { ChatAdapter, Message, ModelTurn } from '../src/rein/contracts'
import { createHash } from 'node:crypto'

const fixturePath = process.argv[2] ? resolve(process.argv[2]) : fileURLToPath(new URL('../../fixtures/cases/ch07-context.json', import.meta.url))
const fixture = JSON.parse(await readFile(fixturePath, 'utf8')) as { unit: string; cases: any[] }
const outputs = []
for (const scenario of fixture.cases) {
  const requests: Message[][] = []
  const adapter: ChatAdapter = { async complete(messages) {
    requests.push(structuredClone(messages) as Message[])
    if (estimateMessages(messages) > scenario.budget) throw new Error('adapter_budget_exceeded')
    const candidates: string[] = []
    for (let i = 0; i < messages.length; i++) {
      const message = messages[i]!
      if (message.role !== 'assistant' || !message.toolCalls?.length) continue
      const byId = new Map(messages.slice(i + 1).filter(item => item.role === 'tool').map(item => [item.toolCallId, item.content]))
      if (message.toolCalls.every(call => byId.has(call.id))) candidates.push(message.toolCalls.map(call => byId.get(call.id)).join('\n'))
    }
    const content = candidates.length > 0 ? candidates.at(-1)! : messages.filter(message => message.role === 'user').at(-1)?.content ?? ''
    return { message: { role: 'assistant', content }, toolCalls: [] } satisfies ModelTurn
  } }
  const result = await runAgentLoop(adapter, { root: process.cwd() }, scenario.goal, { context: { rules: scenario.rules, history: scenario.history, budget: scenario.budget, manage: scenario.managed } })
  outputs.push({ id: scenario.id, result, requests: requests.length, sentMessages: requests })
}
const rendered = JSON.stringify({ unit: fixture.unit, cases: outputs })
process.stdout.write(rendered + '\n')
process.stderr.write(`fixture_sha256=${createHash('sha256').update(await readFile(fixturePath)).digest('hex')}\n`)
