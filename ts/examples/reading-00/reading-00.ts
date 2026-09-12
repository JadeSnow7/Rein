/** 阅读 0 的最小本地响应：只模拟 status 与 JSON body，不访问网络。 */
export type LocalResponse = Readonly<{
  status: number
  body: string
}>

export type ReadingErrorKind = 'http' | 'json' | 'shape'

export class ReadingError extends Error {
  constructor(
    public readonly kind: ReadingErrorKind,
    message: string,
  ) {
    super(message)
    this.name = 'ReadingError'
  }
}

function isTextPayload(value: unknown): value is { text: string } {
  if (typeof value !== 'object' || value === null || !('text' in value)) return false
  const text = value.text
  return typeof text === 'string' && text.trim().length > 0
}

export function extractText(response: LocalResponse): string {
  if (response.status < 200 || response.status >= 300) {
    throw new ReadingError('http', `HTTP 请求失败：status=${response.status}`)
  }

  let parsed: unknown
  try {
    parsed = JSON.parse(response.body)
  } catch (cause) {
    throw new ReadingError('json', `响应不是合法 JSON：${String(cause)}`)
  }

  if (Array.isArray(parsed) || !isTextPayload(parsed)) {
    throw new ReadingError('shape', '响应必须包含非空白字符串字段 text')
  }
  return parsed.text
}

export async function readText(response: LocalResponse): Promise<string> {
  return extractText(response)
}
