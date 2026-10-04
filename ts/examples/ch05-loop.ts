import { resolve } from 'node:path'
import { runAgentLoop } from '../src/rein/loop'
import type { ChatAdapter, ModelTurn, Message } from '../src/rein/contracts'

const args = process.argv.slice(2)
const mode = args[0] === 'workspace' ? args[2] ?? 'multi' : args[0] ?? 'multi'
const root = resolve(args[0] === 'workspace' ? args[1] ?? '.' : '../fixtures/workspaces/prerequisites')
if (!['single', 'multi', 'recovery', 'empty', 'exhausted', 'limit'].includes(mode)) throw new Error('用法：ch05:offline workspace <目录> [single|multi|recovery|empty|exhausted|limit]')
const single = mode === 'single'
const adapter: ChatAdapter = { async complete(messages): Promise<ModelTurn> {
  if (mode === 'empty') return turn('   ')
  if (mode === 'exhausted' && messages.filter(m => m.role === 'tool').length > 0) throw new Error('replay exhausted')
  if (mode === 'limit') return turn('', [{ id: 'limit-1', name: 'unknown', arguments: {} }])
  const tools = messages.filter(m => m.role === 'tool')
  if (mode === 'recovery') {
    const last = tools.at(-1)
    if (!last) return turn('', [{ id: 'missing-1', name: 'read_file', arguments: { path: 'missing.md' } }])
    if (last.content.includes('path_invalid')) return turn('', [{ id: 'ok-1', name: 'read_file', arguments: { path: 'README.md' } }])
    return turn(`恢复内容：${last.content.trim()}`)
  }
  if (single) {
    if (tools.length === 0) return turn('', [{ id: 'read-1', name: 'read_file', arguments: { path: 'README.md' } }])
    return turn(`单文件内容：${tools[0]?.content.trim()}`)
  }
  if (tools.length === 0) return turn('', [{ id: 'search-1', name: 'search_files', arguments: { needle: 'marker:' } }])
  if (tools.length === 1) {
    const only = tools[0]
    if (!only) throw new Error('missing search result')
    const paths = only.content.split('\n').filter(Boolean).slice(0, 2)
    return turn('', paths.map((path, i) => ({ id: `read-${i + 1}`, name: 'read_file', arguments: { path } })))
  }
  return turn(`多文件摘要：${tools.slice(1).map(m => m.content.trim()).join(' | ')}`)
} }
function turn(content: string, toolCalls: ModelTurn['toolCalls'] = []): ModelTurn { const message: Message = { role: 'assistant', content, toolCalls }; return { message, toolCalls } }
const result = await runAgentLoop(adapter, { root }, single ? '请读取 README.md。' : mode === 'recovery' ? '请先读取 missing.md，失败后读取 README.md。' : '请搜索 marker:，再读取命中的两个文件并总结。', { maxTurns: mode === 'limit' ? 2 : 32 })
console.log(JSON.stringify({ state: result.state, reason: result.reason, answer: result.answer, events: result.events }, null, 2))
