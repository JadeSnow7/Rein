import OpenAI, { APIError, APIConnectionError, APIConnectionTimeoutError, type ClientOptions } from 'openai'
import { pathToFileURL } from 'node:url'

import { loadConfig, type Config } from './config'

export const SAFE_TIMEOUT_MS = 30_000

export type SafeFailureKind =
  | 'config'
  | 'sdk'
  | 'network'
  | 'timeout'
  | 'unauthorized'
  | 'rate-limit'
  | 'quota'
  | 'http'
  | 'not-found'
  | 'server'
  | 'response'

export class SafeCallError extends Error {
  constructor(readonly kind: SafeFailureKind, message: string, options?: { cause?: unknown }) {
    super(message, options)
    this.name = 'SafeCallError'
  }
}

export interface SafeHelloOptions {
  readonly prompt?: string
  readonly fetch?: ClientOptions['fetch']
  readonly timeoutMs?: number
}

export async function requestSafeHello(
  config: Config,
  options: SafeHelloOptions = {},
): Promise<string> {
  const client = new OpenAI({
    apiKey: config.apiKey,
    baseURL: config.baseUrl,
    maxRetries: 0,
    timeout: SAFE_TIMEOUT_MS,
    ...(options.fetch === undefined ? {} : { fetch: options.fetch }),
  })
  let deadlineAborted = false
  try {
    const controller = new AbortController()
    const timer = setTimeout(() => { deadlineAborted = true; controller.abort() }, options.timeoutMs ?? SAFE_TIMEOUT_MS)
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
    return readHelloText(response)
  } catch (error) {
    if (error instanceof SafeCallError) throw error
    throw classifyOpenAIError(error, deadlineAborted)
  }
}

export async function main(argv: readonly string[] = process.argv.slice(2)): Promise<number> {
  try {
    const text = await requestSafeHello(loadConfig(), { prompt: argv.join(' ').trim() || 'hello' })
    console.log(text)
    return 0
  } catch (error) {
    if (error instanceof Error && error.name === 'ConfigError') {
      console.error(`失败（config）：${error.message}`)
      return 1
    }
    const failure = error instanceof SafeCallError
      ? error
      : new SafeCallError('sdk', '本地 SDK 或程序发生未预期错误。', { cause: error })
    console.error(`失败（${failure.kind}）：${failure.message}`)
    return 1
  }
}

function classifyOpenAIError(error: unknown, deadlineAborted = false): SafeCallError {
  if (deadlineAborted || error instanceof APIConnectionTimeoutError) {
    return new SafeCallError('timeout', '请求超过 30 秒仍未完成。超时不能证明服务端没有执行请求。', { cause: error })
  }
  if (error instanceof APIConnectionError || (error instanceof Error && error.name === 'APIConnectionError')) {
    return new SafeCallError('network', '网络连接失败，请检查 DNS、代理、TLS 和端点地址。', { cause: error })
  }
  if (error instanceof APIError || hasStatus(error)) {
    const status = readStatus(error)
    const code = readErrorCode(error)
    if (status === undefined) return new SafeCallError('sdk', 'SDK 返回了无法识别的错误状态。', { cause: error })
    if (status === 401 || status === 403) return new SafeCallError('unauthorized', 'API key 无效、过期或没有权限，请检查 REIN_API_KEY。', { cause: error })
    if (status === 429 && /quota|billing|insufficient|balance|credit|usage/i.test(code ?? '')) {
      return new SafeCallError('quota', '请求被拒绝，响应指向额度、余额或用量上限，请到服务商控制台检查。', { cause: error })
    }
    if (status === 429 && /rate_limit|too_many_requests/i.test(code ?? '')) {
      return new SafeCallError('rate-limit', '请求触发限流，请降低频率并稍后重试。', { cause: error })
    }
    if (status === 429) return new SafeCallError('http', '收到 HTTP 429，请检查响应 code，确认是限流还是额度、余额或用量上限。', { cause: error })
    if (status === 404) return new SafeCallError('not-found', '接口或模型不存在，请检查 REIN_BASE_URL 和 REIN_MODEL。', { cause: error })
    if (status >= 500) return new SafeCallError('server', '服务端暂时失败，请稍后重试；这不等于请求一定没有执行。', { cause: error })
    return new SafeCallError('http', `服务端返回 HTTP ${status}，请检查服务商文档和配置。`, { cause: error })
  }
  if (error instanceof SyntaxError) {
    return new SafeCallError('response', '服务端响应无法解析或缺少需要的字段。', { cause: error })
  }
  return new SafeCallError('sdk', '本地 SDK 或程序错误，请检查 Node.js、依赖安装和调用参数。', { cause: error })
}

function readHelloText(value: unknown): string {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) throw new SafeCallError('response', '模型响应不是对象。')
  const choices = (value as { choices?: unknown }).choices
  if (!Array.isArray(choices) || choices.length === 0) throw new SafeCallError('response', '模型响应缺少 choices[0]。')
  const first = choices[0]
  if (typeof first !== 'object' || first === null || Array.isArray(first)) throw new SafeCallError('response', '模型响应缺少 choices[0].message。')
  const message = (first as { message?: unknown }).message
  if (typeof message !== 'object' || message === null || Array.isArray(message)) throw new SafeCallError('response', '模型响应缺少 choices[0].message。')
  const text = (message as { content?: unknown }).content
  if (typeof text !== 'string' || text.trim().length === 0) throw new SafeCallError('response', '模型响应缺少非空的 choices[0].message.content。')
  return text
}

function hasStatus(error: unknown): boolean {
  return readStatus(error) !== undefined
}

function readStatus(error: unknown): number | undefined {
  if (typeof error !== 'object' || error === null) return undefined
  const status = (error as { status?: unknown }).status
  return typeof status === 'number' ? status : undefined
}

function readErrorCode(error: unknown): string | undefined {
  if (typeof error !== 'object' || error === null) return undefined
  const direct = (error as { code?: unknown }).code
  if (typeof direct === 'string') return direct
  const nested = (error as { error?: { code?: unknown } }).error?.code
  return typeof nested === 'string' ? nested : undefined
}

if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = await main()
}
