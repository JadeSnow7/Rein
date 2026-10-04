import { loadConfig } from '../src/config'
import { createFetchTransport } from '../src/transport'
import { createOpenAIAdapter } from '../src/rein/adapters'
import { runAgentLoop } from '../src/rein/loop'
import { resolve } from 'node:path'

if (process.argv[2] !== '--live') throw new Error('必须显式传入 --live')
const result = await runAgentLoop(createOpenAIAdapter(loadConfig(), createFetchTransport()), { root: resolve(process.argv[3] ?? '.') }, process.argv.slice(4).join(' ') || '请搜索 marker: 并读取命中文件', { maxTurns: 4 })
console.log(JSON.stringify(result, null, 2))
process.exitCode = result.state === 'completed' && result.reason === 'final_answer' ? 0 : 1
