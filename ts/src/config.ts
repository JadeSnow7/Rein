/**
 * 从环境变量读取一次模型调用需要的三件事。
 *
 * 这里不提供默认值。一个悄悄回退到某个内置端点的配置读取，会让"为什么调到了
 * 别的模型"变成一个无法从错误信息里回答的问题。
 */

import { ConfigError } from './errors'

export interface Config {
  /** OpenAI 兼容端点的根路径，末尾斜杠已去除 */
  readonly baseUrl: string
  readonly apiKey: string
  readonly model: string
}

export type Env = Record<string, string | undefined>

export function loadConfig(env: Env = process.env): Config {
  return {
    baseUrl: readBaseUrl(env),
    apiKey: readNonEmpty(env, 'REIN_API_KEY'),
    model: readNonEmpty(env, 'REIN_MODEL'),
  }
}

function readNonEmpty(env: Env, variable: string): string {
  const raw = env[variable]
  if (raw === undefined) throw new ConfigError(variable, '未设置')
  const value = raw.trim()
  if (value.length === 0) throw new ConfigError(variable, '为空')
  return value
}

function readBaseUrl(env: Env): string {
  const variable = 'REIN_BASE_URL'
  const value = readNonEmpty(env, variable)

  let url: URL
  try {
    url = new URL(value)
  } catch {
    throw new ConfigError(variable, '不是合法 URL')
  }

  if (url.username || url.password) {
    throw new ConfigError(variable, '不得包含用户名或密码')
  }
  // URL normalizes a trailing '?' or '#' away, but those delimiters would
  // still make string concatenation ambiguous, so inspect the original input.
  if (url.search || url.hash || value.includes('?') || value.includes('#')) {
    throw new ConfigError(variable, '不得包含 query 或 hash')
  }

  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    throw new ConfigError(
      variable,
      `协议必须是 http 或 https，当前为 ${url.protocol.replace(':', '')}`,
    )
  }

  return value.replace(/\/+$/, '')
}
