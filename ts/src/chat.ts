/**
 * 一次模型调用：构造请求、发送、解析响应、取出回答文本。
 *
 * 这里直接写 OpenAI 兼容接口的形状，不做 provider 抽象——第 04 章会拿另一种响应
 * 形状回来对照，抽象要从那时的具体麻烦里长出来（DECISIONS.md D3、D4）。
 */

import type { Config } from './config'
import { HttpError, ResponseFormatError } from './errors'
import type { Transport } from './transport'

export const DEFAULT_TIMEOUT_MS = 30_000

export interface ChatRequest {
  readonly prompt: string
  readonly system?: string
  readonly timeoutMs?: number
}

export interface ChatResult {
  /** 模型的回答文本 */
  readonly text: string
  /** 服务端回报的模型名，未必等于请求里写的那个 */
  readonly model: string
  readonly finishReason: string | undefined
  readonly usage: TokenUsage | undefined
}

export interface TokenUsage {
  readonly promptTokens: number | undefined
  readonly completionTokens: number | undefined
  readonly totalTokens: number | undefined
}

export async function chat(
  config: Config,
  transport: Transport,
  request: ChatRequest,
): Promise<ChatResult> {
  const url = `${config.baseUrl}/chat/completions`
  const messages: Array<{ role: string; content: string }> = []
  if (request.system !== undefined) {
    messages.push({ role: 'system', content: request.system })
  }
  messages.push({ role: 'user', content: request.prompt })

  const response = await transport.send(
    {
      method: 'POST',
      url,
      headers: {
        'content-type': 'application/json',
        authorization: `Bearer ${config.apiKey}`,
      },
      body: JSON.stringify({ model: config.model, messages }),
    },
    { timeoutMs: request.timeoutMs ?? DEFAULT_TIMEOUT_MS },
  )

  if (response.status < 200 || response.status >= 300) {
    throw new HttpError(url, response.status, response.statusText, response.body)
  }

  return parseChatResponse(url, response.body)
}

/**
 * 逐层落地响应形状。每一步对不上都指出具体路径——noUncheckedIndexedAccess 强制
 * 我们承认 choices[0] 可能不存在，这个约束在这里正好换成一条有用的错误信息。
 */
export function parseChatResponse(url: string, body: string): ChatResult {
  let payload: unknown
  try {
    payload = JSON.parse(body)
  } catch (cause) {
    const detail = cause instanceof Error ? `不是合法 JSON（${cause.message}）` : '不是合法 JSON'
    throw new ResponseFormatError(url, '(root)', detail, body)
  }

  const root = expectObject(url, '(root)', payload, body)

  const choices = root['choices']
  if (!Array.isArray(choices)) {
    throw new ResponseFormatError(url, 'choices', '不是数组', body)
  }
  const first = choices[0]
  if (first === undefined) {
    throw new ResponseFormatError(url, 'choices[0]', '不存在（choices 为空数组）', body)
  }

  const choice = expectObject(url, 'choices[0]', first, body)
  const message = expectObject(url, 'choices[0].message', choice['message'], body)

  const content = message['content']
  if (typeof content !== 'string') {
    throw new ResponseFormatError(
      url,
      'choices[0].message.content',
      `不是字符串，而是 ${describeType(content)}`,
      body,
    )
  }

  return {
    text: content,
    model: typeof root['model'] === 'string' ? root['model'] : '(未回报)',
    finishReason:
      typeof choice['finish_reason'] === 'string' ? choice['finish_reason'] : undefined,
    usage: readUsage(root['usage']),
  }
}

function expectObject(
  url: string,
  path: string,
  value: unknown,
  body: string,
): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new ResponseFormatError(url, path, `不是对象，而是 ${describeType(value)}`, body)
  }
  return value as Record<string, unknown>
}

function readUsage(value: unknown): TokenUsage | undefined {
  if (typeof value !== 'object' || value === null) return undefined
  const usage = value as Record<string, unknown>
  return {
    promptTokens: numberOrUndefined(usage['prompt_tokens']),
    completionTokens: numberOrUndefined(usage['completion_tokens']),
    totalTokens: numberOrUndefined(usage['total_tokens']),
  }
}

function numberOrUndefined(value: unknown): number | undefined {
  return typeof value === 'number' ? value : undefined
}

function describeType(value: unknown): string {
  if (value === null) return 'null'
  if (Array.isArray(value)) return '数组'
  return typeof value
}
