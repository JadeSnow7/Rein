import type { ClientOptions } from 'openai'
import { describe, expect, it } from 'vitest'

import type { Config } from '../src/config'
import { requestHello } from '../src/hello'
import { main as safeMain, requestSafeHello, SafeCallError } from '../src/hello-safe'

const config: Config = {
  baseUrl: 'https://api.example.test/v1',
  apiKey: 'sk-test',
  model: 'test-model',
}

function fakeFetch(
  body: string,
  status = 200,
  calls: { count: number } = { count: 0 },
): NonNullable<ClientOptions['fetch']> {
  return async (input, init) => {
    calls.count += 1
    expect(String(input)).toBe('https://api.example.test/v1/chat/completions')
    expect(init?.method).toBe('POST')
    expect(init?.headers).toMatchObject({ authorization: 'Bearer sk-test', 'content-type': 'application/json' })
    expect(JSON.parse(String(init?.body))).toEqual({
      model: 'test-model',
      messages: [{ role: 'user', content: 'hello' }],
      stream: false,
    })
    return new Response(body, {
      status,
      headers: { 'content-type': 'application/json' },
    })
  }
}

describe('hello SDK entry points', () => {
  it('最小入口发送单条 user hello，读取非空回答且不重试', async () => {
    const calls = { count: 0 }
    const text = await requestHello(config, {
      fetch: fakeFetch(JSON.stringify({ choices: [{ message: { content: 'hello back' } }] }), 200, calls),
    })
    expect(text).toBe('hello back')
    expect(calls.count).toBe(1)
  })

  it('最小入口拒绝空回答', async () => {
    await expect(requestHello(config, {
      fetch: fakeFetch(JSON.stringify({ choices: [{ message: { content: '   ' } }] })),
    })).rejects.toThrow('空的回答文本')
  })

  it('最小入口对 429 只发一次请求', async () => {
    const calls = { count: 0 }
    await expect(requestHello(config, {
      fetch: fakeFetch(JSON.stringify({ error: { code: 'rate_limit_exceeded' } }), 429, calls),
    })).rejects.toBeDefined()
    expect(calls.count).toBe(1)
  })

  it.each([
    '{}',
    '{"choices":[]}',
    '{"choices":[{"message":{}}]}',
    'not-json',
  ])('安全入口拒绝异常响应 %s', async (body) => {
    const calls = { count: 0 }
    const error = await requestSafeHello(config, { fetch: fakeFetch(body, 200, calls) }).catch((value: unknown) => value)
    expect(error).toBeInstanceOf(SafeCallError)
    expect((error as SafeCallError).kind).toBe('response')
    expect(calls.count).toBe(1)
  })

  it.each([
    [401, undefined, 'unauthorized'],
    [403, undefined, 'unauthorized'],
    [429, 'rate_limit_exceeded', 'rate-limit'],
    [429, 'insufficient_quota', 'quota'],
    [404, undefined, 'not-found'],
    [503, undefined, 'server'],
  ] as const)('把 HTTP %s 按响应 code 分类为 %s', async (status, code, kind) => {
    const calls = { count: 0 }
    const error = await requestSafeHello(config, {
      fetch: fakeFetch(JSON.stringify({ error: { message: 'provider detail', code } }), status, calls),
    }).catch((value: unknown) => value)
    expect(error).toBeInstanceOf(SafeCallError)
    expect((error as SafeCallError).kind).toBe(kind)
    expect((error as SafeCallError).message).not.toContain('provider detail')
    expect((error as SafeCallError).message).not.toContain('sk-test')
    expect(calls.count).toBe(1)
  })

  it('429 没有可识别 code 时不武断归为限流', async () => {
    const calls = { count: 0 }
    const error = await requestSafeHello(config, {
      fetch: fakeFetch(JSON.stringify({ error: { message: 'check dashboard' } }), 429, calls),
    }).catch((value: unknown) => value)
    expect((error as SafeCallError).kind).toBe('http')
    expect(calls.count).toBe(1)
  })

  it('网络失败归类为 network', async () => {
    const fetchImpl: NonNullable<ClientOptions['fetch']> = async () => {
      throw new Error('ECONNREFUSED')
    }
    const error = await requestSafeHello(config, { fetch: fetchImpl }).catch((value: unknown) => value)
    expect(error).toBeInstanceOf(SafeCallError)
    expect((error as SafeCallError).kind).toBe('network')
  })

  it('安全 CLI 缺配置时返回退出码 1，且不打印密钥', async () => {
    const previous = {
      REIN_BASE_URL: process.env.REIN_BASE_URL,
      REIN_API_KEY: process.env.REIN_API_KEY,
      REIN_MODEL: process.env.REIN_MODEL,
    }
    delete process.env.REIN_BASE_URL
    delete process.env.REIN_API_KEY
    delete process.env.REIN_MODEL
    try {
      expect(await safeMain(['hello'])).toBe(1)
    } finally {
      for (const [key, value] of Object.entries(previous)) {
        if (value === undefined) delete process.env[key]
        else process.env[key] = value
      }
    }
  })

  it('响应体一直不结束时由整体 deadline 归类为 timeout', async () => {
    const fetchImpl: NonNullable<ClientOptions['fetch']> = async (_input, init) => {
      const stream = new ReadableStream<Uint8Array>({
        start(controller) {
          controller.enqueue(new TextEncoder().encode('{'))
          init?.signal?.addEventListener('abort', () => controller.error(new DOMException('aborted', 'AbortError')))
        },
      })
      return new Response(stream, { headers: { 'content-type': 'application/json' } })
    }
    const error = await requestSafeHello(config, { fetch: fetchImpl, timeoutMs: 5 }).catch((value: unknown) => value)
    expect(error).toBeInstanceOf(SafeCallError)
    expect((error as SafeCallError).kind).toBe('timeout')
  })

  it('最小入口也会在响应体挂起时中止读取', async () => {
    const fetchImpl: NonNullable<ClientOptions['fetch']> = async (_input, init) => new Response(
      new ReadableStream<Uint8Array>({
        start(controller) {
          controller.enqueue(new TextEncoder().encode('{'))
          init?.signal?.addEventListener('abort', () => controller.error(new DOMException('aborted', 'AbortError')))
        },
      }),
      { headers: { 'content-type': 'application/json' } },
    )
    await expect(requestHello(config, { fetch: fetchImpl, timeoutMs: 5 })).rejects.toBeDefined()
  })
})
