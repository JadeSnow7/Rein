import { describe, expect, it } from 'vitest'
import { chat } from '../src/chat'
import type { Config } from '../src/config'
import {
  HttpError,
  NetworkError,
  ResponseFormatError,
  TimeoutError,
} from '../src/errors'
import type { Transport, TransportRequest, TransportResponse } from '../src/transport'

const config: Config = {
  baseUrl: 'https://api.example.com/v1',
  apiKey: 'sk-test',
  model: 'test-model',
}

/** 本地桩：记录收到的请求，返回给定响应或抛出给定错误。 */
function stub(outcome: TransportResponse | Error): Transport & { sent: TransportRequest[] } {
  const sent: TransportRequest[] = []
  return {
    sent,
    async send(request) {
      sent.push(request)
      if (outcome instanceof Error) throw outcome
      return outcome
    },
  }
}

function ok(body: string): TransportResponse {
  return { status: 200, statusText: 'OK', headers: { 'content-type': 'application/json' }, body }
}

const successBody = JSON.stringify({
  id: 'chatcmpl-local',
  model: 'test-model-0001',
  choices: [{ index: 0, finish_reason: 'stop', message: { role: 'assistant', content: '你好。' } }],
  usage: { prompt_tokens: 11, completion_tokens: 3, total_tokens: 14 },
})

describe('chat 成功路径', () => {
  it('取出回答文本、回报的模型名、finish_reason 与 usage', async () => {
    const result = await chat(config, stub(ok(successBody)), { prompt: '你好' })
    expect(result.text).toBe('你好。')
    expect(result.model).toBe('test-model-0001')
    expect(result.finishReason).toBe('stop')
    expect(result.usage).toEqual({ promptTokens: 11, completionTokens: 3, totalTokens: 14 })
  })

  it('请求打在 <baseUrl>/chat/completions，带上 model 与 user 消息', async () => {
    const transport = stub(ok(successBody))
    await chat(config, transport, { prompt: '你好' })

    const request = transport.sent[0]
    expect(request).toBeDefined()
    expect(request?.url).toBe('https://api.example.com/v1/chat/completions')
    expect(request?.headers['authorization']).toBe('Bearer sk-test')
    expect(JSON.parse(request?.body ?? '')).toEqual({
      model: 'test-model',
      messages: [{ role: 'user', content: '你好' }],
    })
  })

  it('给了 system 就放在 user 之前，没给就不出现', async () => {
    const withSystem = stub(ok(successBody))
    await chat(config, withSystem, { prompt: '你好', system: '简短回答。' })
    expect(JSON.parse(withSystem.sent[0]?.body ?? '').messages).toEqual([
      { role: 'system', content: '简短回答。' },
      { role: 'user', content: '你好' },
    ])
  })
})

describe('chat 失败分类', () => {
  it('网络错误原样上浮，kind 为 network', async () => {
    const cause = new NetworkError('https://api.example.com/v1/chat/completions', new Error('ECONNREFUSED'))
    await expect(chat(config, stub(cause), { prompt: '你好' })).rejects.toSatisfy(
      (error: unknown) => error instanceof NetworkError && error.kind === 'network',
    )
  })

  it('超时原样上浮，kind 为 timeout', async () => {
    const cause = new TimeoutError('https://api.example.com/v1/chat/completions', 30_000)
    await expect(chat(config, stub(cause), { prompt: '你好' })).rejects.toBeInstanceOf(TimeoutError)
  })

  it('非 2xx 抛 HttpError，保留状态码与响应体', async () => {
    const body = JSON.stringify({ error: { message: 'Invalid API key', type: 'invalid_request_error' } })
    const error = await chat(config, stub({ status: 401, statusText: 'Unauthorized', headers: {}, body }), {
      prompt: '你好',
    }).catch((e: unknown) => e)

    expect(error).toBeInstanceOf(HttpError)
    const httpError = error as HttpError
    expect(httpError.kind).toBe('http')
    expect(httpError.status).toBe(401)
    expect(httpError.body).toBe(body)
    // 失败信息要能直接读懂：状态码、响应体片段、下一步该看哪个变量。
    expect(httpError.message).toContain('401')
    expect(httpError.message).toContain('Invalid API key')
    expect(httpError.message).toContain('REIN_API_KEY')
  })

  it('429 提示先查看原因，再按限流或额度分别处理', async () => {
    const body = JSON.stringify({
      error: { message: 'Too many requests', type: 'rate_limit_error', code: 'rate_limit_exceeded' },
    })
    const error = await chat(config, stub({ status: 429, statusText: 'Too Many Requests', headers: {}, body }), {
      prompt: '你好',
    }).catch((e: unknown) => e)

    expect(error).toBeInstanceOf(HttpError)
    const httpError = error as HttpError
    expect(httpError.body).toBe(body)
    expect(httpError.message).toContain('可能触发限流')
    expect(httpError.message).toContain('请先查看响应体原因')
    expect(httpError.message).toContain('限流时降低请求频率')
    expect(httpError.message).toContain('稍后重试')
    expect(httpError.message).toContain('额度问题请检查账号余额与限制')
  })

  it.each([
    ['未知原因', JSON.stringify({ error: { message: 'Request rejected' } })],
    ['非 JSON 响应', '<html>429 Too Many Requests</html>'],
  ])('429 %s 时保留响应体并提示先检查原因', async (_name, body) => {
    const error = await chat(config, stub({ status: 429, statusText: 'Too Many Requests', headers: {}, body }), {
      prompt: '你好',
    }).catch((e: unknown) => e)

    expect(error).toBeInstanceOf(HttpError)
    const httpError = error as HttpError
    expect(httpError.body).toBe(body)
    expect(httpError.message).toContain('可能触发限流，也可能是额度、余额或用量上限问题')
    expect(httpError.message).toContain('请先查看响应体原因')
    expect(httpError.message).toContain('限流时降低请求频率或稍后重试')
    expect(httpError.message).toContain('额度问题请检查账号余额与限制')
  })

  it('2xx 边界：299 仍按成功状态处理，失败发生在解析阶段', async () => {
    const error = await chat(config, stub({ status: 299, statusText: 'OK', headers: {}, body: '{}' }), {
      prompt: '你好',
    }).catch((e: unknown) => e)
    expect(error).toBeInstanceOf(ResponseFormatError)
    expect(error).not.toBeInstanceOf(HttpError)
  })

  it.each([
    ['响应体不是 JSON', '<html>502 Bad Gateway</html>', '(root)'],
    ['顶层是数组', '[]', '(root)'],
    ['缺少 choices', '{"id":"x"}', 'choices'],
    ['choices 为空数组', '{"choices":[]}', 'choices[0]'],
    ['缺少 message', '{"choices":[{"index":0}]}', 'choices[0].message'],
    ['content 不是字符串', '{"choices":[{"message":{"content":null}}]}', 'choices[0].message.content'],
  ])('%s：抛 ResponseFormatError 并指出路径 %s', async (_name, body, path) => {
    const error = await chat(config, stub(ok(body)), { prompt: '你好' }).catch((e: unknown) => e)

    expect(error).toBeInstanceOf(ResponseFormatError)
    const formatError = error as ResponseFormatError
    expect(formatError.kind).toBe('response-format')
    expect(formatError.path).toBe(path)
    expect(formatError.body).toBe(body)
    expect(formatError.message).toContain(path)
  })

  it('缺少 model 与 usage 不算失败：正文只承诺 text 必有', async () => {
    const result = await chat(config, stub(ok('{"choices":[{"message":{"content":"ok"}}]}')), {
      prompt: '你好',
    })
    expect(result.text).toBe('ok')
    expect(result.model).toBe('(未回报)')
    expect(result.finishReason).toBeUndefined()
    expect(result.usage).toBeUndefined()
  })
})
