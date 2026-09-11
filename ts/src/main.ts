/**
 * 最小可运行入口。
 *
 *   cd ts
 *   npx tsx --env-file=.env src/main.ts "用一句话说明什么是 Agent Harness"
 *
 * 不带参数时使用一句默认提示词。
 */

import { chat } from './chat'
import { loadConfig } from './config'
import { ReinError } from './errors'
import { createFetchTransport } from './transport'

const DEFAULT_PROMPT = '用一句话说明什么是 Agent Harness。'

async function main(argv: readonly string[]): Promise<number> {
  const prompt = argv.join(' ').trim() || DEFAULT_PROMPT

  try {
    const config = loadConfig()
    const result = await chat(config, createFetchTransport(), { prompt })

    console.log(result.text)
    console.error(
      `\n[${result.model}] finish_reason=${result.finishReason ?? '(未回报)'}` +
        ` tokens=${result.usage?.totalTokens ?? '(未回报)'}`,
    )
    return 0
  } catch (error) {
    if (error instanceof ReinError) {
      console.error(`失败（${error.kind}）：${error.message}`)
      return 1
    }
    throw error
  }
}

process.exitCode = await main(process.argv.slice(2))
