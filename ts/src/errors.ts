/**
 * 第 01 章的失败分类。
 *
 * 每一类失败对应一件读者需要分别处理的事：配置没填好、网络没通、服务端拒绝、
 * 等太久、响应形状对不上。它们共享一个基类，但携带各自的现场信息——第 10 章
 * 会把这些信息组织成模型可用的形式，前提是第 01 章就没把它们压成一句字符串。
 */

export type ReinErrorKind =
  | 'config'
  | 'network'
  | 'http'
  | 'timeout'
  | 'response-format'

export abstract class ReinError extends Error {
  abstract readonly kind: ReinErrorKind

  constructor(message: string, options?: { cause?: unknown }) {
    super(message, options)
    this.name = new.target.name
  }
}

/** 环境变量缺失或格式不对。消息里必须出现具体变量名。 */
export class ConfigError extends ReinError {
  readonly kind = 'config' as const

  constructor(
    readonly variable: string,
    detail: string,
  ) {
    super(`${variable} ${detail}。参考 ts/.env.example，复制为 ts/.env 后填写。`)
  }
}

/** 请求没能送达：DNS、连接被拒、TLS、连接中断。 */
export class NetworkError extends ReinError {
  readonly kind = 'network' as const

  constructor(
    readonly url: string,
    cause: unknown,
  ) {
    super(`请求 ${url} 未能送达：${describeCause(cause)}`, { cause })
  }
}

/** 服务端返回了非 2xx。保留状态码与响应体片段，否则读者只能看到一个数字。 */
export class HttpError extends ReinError {
  readonly kind = 'http' as const

  constructor(
    readonly url: string,
    readonly status: number,
    readonly statusText: string,
    readonly body: string,
  ) {
    super(
      `${url} 返回 ${status} ${statusText}：${snippet(body)}` +
        `${hintForStatus(status)}`,
    )
  }
}

/** 超过 timeoutMs 仍未收到完整响应。 */
export class TimeoutError extends ReinError {
  readonly kind = 'timeout' as const

  constructor(
    readonly url: string,
    readonly timeoutMs: number,
  ) {
    super(`请求 ${url} 超过 ${timeoutMs}ms 未完成。`)
  }
}

/**
 * 响应能读到，但不是预期的形状。
 * `path` 指出第一个对不上的位置，`body` 保留原始片段——只说"解析失败"的错误
 * 会迫使读者回头加 console.log，这正是本书后面要避免的习惯。
 */
export class ResponseFormatError extends ReinError {
  readonly kind = 'response-format' as const

  constructor(
    readonly url: string,
    readonly path: string,
    detail: string,
    readonly body: string,
  ) {
    super(`${url} 的响应在 ${path} 处${detail}：${snippet(body)}`)
  }
}

function describeCause(cause: unknown): string {
  if (cause instanceof Error) {
    const inner = cause.cause
    if (inner instanceof Error && inner.message !== cause.message) {
      return `${cause.message}（${inner.message}）`
    }
    return cause.message
  }
  return String(cause)
}

function hintForStatus(status: number): string {
  if (status === 401 || status === 403) return ' 检查 REIN_API_KEY 是否有效。'
  if (status === 404) return ' 检查 REIN_BASE_URL 是否指向兼容端点的根路径。'
  if (status === 429) return ' 触发限流，稍后重试或降低并发。'
  if (status >= 500) return ' 服务端故障，请求本身可能是对的。'
  return ''
}

function snippet(body: string, limit = 400): string {
  const text = body.trim()
  if (text.length === 0) return '(响应体为空)'
  if (text.length <= limit) return text
  return `${text.slice(0, limit)}…(截断，共 ${text.length} 字符)`
}
