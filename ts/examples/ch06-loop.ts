import { mkdtemp, writeFile, rm } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { tmpdir } from 'node:os'
import { runAgentLoop } from '../src/rein/loop'
import type { ChatAdapter, ModelTurn, ToolCall } from '../src/rein/contracts'

const mode = process.argv[2] ?? 'normal'
const requestedWorkspace = process.argv[3]
const supported = new Set(['normal', 'budget', 'duplicate', 'zero', 'tool-zero', 'cancel-before', 'cancel', 'cancel-at-return', 'cancel-between-tools', 'timeout'])
if (!supported.has(mode)) { console.error(`unknown ch06 mode: ${mode}`); process.exit(2) }
const root = requestedWorkspace ? resolve(requestedWorkspace) : await mkdtemp(join(tmpdir(), 'rein-ch06-'))
if (!requestedWorkspace) {
  await writeFile(join(root, 'README.md'), 'marker: ch06 alpha\n')
  await writeFile(join(root, 'guide.md'), 'marker: ch06 beta\n')
}
const call = (id: string, name: string, arguments_: Record<string, unknown>): ToolCall => ({ id, name, arguments: arguments_ })
const turn = (content: string, toolCalls: ToolCall[] = []): ModelTurn => ({ message: { role: 'assistant', content, toolCalls }, toolCalls })
let requests = 0
const controller = new AbortController()
const adapter: ChatAdapter = { async complete(messages) {
  requests++
  const toolMessages = messages.filter(message => message.role === 'tool')
  if (mode === 'normal') {
    if (toolMessages.length === 0) return turn('', [call('search-1', 'search_files', { needle: 'marker: ch06' })])
    if (toolMessages.length === 1) {
      const searchOutput = toolMessages[0]!.content
      let businessResult: unknown
      try {
        businessResult = JSON.parse(searchOutput) as unknown
      } catch {
        businessResult = undefined
      }
      if (typeof businessResult === 'object' && businessResult !== null && !Array.isArray(businessResult) && (businessResult as { ok?: unknown }).ok === false) {
        const error = (businessResult as { error?: { message?: unknown } }).error
        throw new Error(typeof error?.message === 'string' ? error.message : 'search failed')
      }
      const paths = searchOutput.split('\n').filter(Boolean)
      if (paths.length === 0) throw new Error('no files match marker: ch06')
      return turn('', paths.map((path, index) => call(`read-${index + 1}`, 'read_file', { path })))
    }
    return turn(`完成：${toolMessages.slice(1).map(message => message.content.trim()).join(' | ')}`)
  }
  if (mode === 'duplicate') return turn('', [call(`duplicate-${requests}`, 'read_file', { path: 'README.md' })])
  if (mode === 'budget' || mode === 'tool-zero' || mode === 'cancel-between-tools') return turn('', [call('one', 'read_file', { path: 'README.md' }), call('two', 'read_file', { path: 'guide.md' })])
  if (mode === 'cancel-at-return') { controller.abort(); return turn('迟到的最终答案') }
  if (mode === 'cancel' || mode === 'timeout') {
    setTimeout(() => { if (mode === 'cancel') controller.abort() }, 20)
    await new Promise(resolvePromise => setTimeout(resolvePromise, 120))
    return turn('迟到的最终答案')
  }
  return turn('')
} }
const result = await runAgentLoop(adapter, { root }, '读取 README.md 与 guide.md', {
  maxTurns: mode === 'zero' ? 0 : 4,
  maxToolCalls: mode === 'budget' ? 1 : mode === 'tool-zero' ? 0 : undefined,
  duplicateLimit: mode === 'duplicate' ? 1 : undefined,
  timeoutMs: mode === 'timeout' ? 30 : undefined,
  signal: mode === 'cancel-before' ? (() => { controller.abort(); return controller.signal })() : controller.signal,
  onToolResult: (_call, toolResult) => { if (mode === 'cancel-between-tools' && toolResult.toolCallId === 'one') controller.abort() },
})
const requestsAtStop = requests
if (mode === 'cancel' || mode === 'timeout') await new Promise(resolvePromise => setTimeout(resolvePromise, 140))
const settledRequests = requests
console.log(JSON.stringify({ mode, requests: requestsAtStop, settledRequests, result }, null, 2))
if (!requestedWorkspace) await rm(root, { recursive: true, force: true })
