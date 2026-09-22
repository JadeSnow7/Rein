import OpenAI, { type ClientOptions } from 'openai'
import { pathToFileURL } from 'node:url'

import type { Config } from './config'

export const HELLO_TIMEOUT_MS = 30_000

export interface HelloOptions {
  readonly prompt?: string
  readonly fetch?: ClientOptions['fetch']
  readonly timeoutMs?: number
}

/** 最小的非流式 Chat Completions 调用，适合第一次把 hello 送到模型。 */
export async function requestHello(
  config: Config,
  options: HelloOptions = {},
): Promise<string> {
  const client = new OpenAI({
    apiKey: config.apiKey,
    baseURL: config.baseUrl,
    maxRetries: 0,
    timeout: HELLO_TIMEOUT_MS,
    ...(options.fetch === undefined ? {} : { fetch: options.fetch }),
  })
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), options.timeoutMs ?? HELLO_TIMEOUT_MS)
  let response: unknown
  try {
    response = await client.chat.completions.create({
      model: config.model,
      messages: [{ role: 'user', content: options.prompt ?? 'hello' }],
      stream: false,
    }, { signal: controller.signal })
  } finally {
    clearTimeout(timer)
  }
  const text = readHelloText(response)
  if (typeof text !== 'string' || text.trim().length === 0) {
    throw new Error('模型返回了空的回答文本。')
  }
  return text
}

export async function main(argv: readonly string[] = process.argv.slice(2)): Promise<number> {
  const { loadConfig } = await import('./config')
  try {
    const text = await requestHello(loadConfig(), { prompt: argv.join(' ').trim() || 'hello' })
    console.log(text)
    return 0
  } catch (error) {
    console.error('调用失败，请运行 hello-safe.ts 查看经过分类的诊断信息。')
    return 1
  }
}

if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = await main()
}

function readHelloText(value: unknown): string {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) throw new Error('模型响应不是对象。')
  const choices = (value as { choices?: unknown }).choices
  if (!Array.isArray(choices) || choices.length === 0) throw new Error('模型响应缺少 choices[0]。')
  const first = choices[0]
  if (typeof first !== 'object' || first === null || Array.isArray(first)) throw new Error('模型响应缺少 choices[0].message。')
  const message = (first as { message?: unknown }).message
  if (typeof message !== 'object' || message === null || Array.isArray(message)) throw new Error('模型响应缺少 choices[0].message。')
  const text = (message as { content?: unknown }).content
  if (typeof text !== 'string' || text.trim().length === 0) throw new Error('模型返回了空的回答文本。')
  return text
}
