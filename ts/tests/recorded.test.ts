import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { chat } from '../src/chat'
import type { Config } from '../src/config'
import { HttpError } from '../src/errors'
import { createReplayTransport, loadRecording } from '../src/transport'

/**
 * 这一组用例读 fixtures/responses/ 下的真实录制，不访问网络。
 * 与 chat.test.ts 里的本地桩互补：桩证明分支正确，录制证明我们对真实响应形状的
 * 判断没有写偏。样本由 ts/scripts/record.ts 录制，密钥与账号标识已去除。
 */
const RESPONSES_DIR = join(
  fileURLToPath(new URL('../../fixtures/responses', import.meta.url)),
)

const config: Config = {
  baseUrl: 'https://api.example.com/v1',
  apiKey: 'sk-not-used-in-replay',
  model: 'test-model',
}

describe('回放真实录制', () => {
  it('openai/insufficient-quota-1：429 映射为 HttpError，并保留可读的服务端说明', async () => {
    const recording = await loadRecording(
      join(RESPONSES_DIR, 'openai', 'insufficient-quota-1.json'),
    )
    expect(recording.response.status).toBe(429)

    const error = await chat(config, createReplayTransport([recording]), {
      prompt: '用一句话说明什么是 Agent Harness。',
    }).catch((e: unknown) => e)

    expect(error).toBeInstanceOf(HttpError)
    const httpError = error as HttpError
    expect(httpError.kind).toBe('http')
    expect(httpError.status).toBe(429)
    // 服务端给的原因要原样出现在错误信息里，读者不必回去翻响应体。
    expect(httpError.message).toContain('insufficient_quota')
    expect(httpError.message).toContain('可能触发限流，也可能是额度、余额或用量上限问题')
    expect(httpError.message).toContain('请先查看响应体原因')
    expect(httpError.message).toContain('额度问题请检查账号余额与限制')
    expect(httpError.message).not.toContain('触发限流，稍后重试或降低并发。')
  })

  it('opencode-go/hello-1：200 取出回答文本、模型名与 usage', async () => {
    const recording = await loadRecording(
      join(RESPONSES_DIR, 'opencode-go', 'hello-1.json'),
    )
    expect(recording.response.status).toBe(200)

    const result = await chat(config, createReplayTransport([recording]), {
      prompt: '用一句话说明什么是 Agent Harness。',
    })

    // 断言的是形状，不是模型说了什么——回答内容每次都不同，不该被固定下来。
    expect(result.text.length).toBeGreaterThan(0)
    expect(result.model).toBe('mimo-v2.5')
    expect(result.finishReason).toBe('stop')
    expect(result.usage?.totalTokens).toBeGreaterThan(0)
    expect(result.usage?.promptTokens).toBeGreaterThan(0)
    expect(result.usage?.completionTokens).toBeGreaterThan(0)
  })

  it('样本里不含密钥：authorization 已置换，redacted 记下了改动', async () => {
    for (const file of [
      join(RESPONSES_DIR, 'openai', 'insufficient-quota-1.json'),
      join(RESPONSES_DIR, 'opencode-go', 'hello-1.json'),
    ]) {
      const recording = await loadRecording(file)
      expect(recording.request.headers['authorization']).toBe('REDACTED')
      expect(recording.redacted).toContain('request.headers.authorization')
      expect(JSON.stringify(recording)).not.toMatch(/sk-[A-Za-z0-9_-]{20,}/)
    }
  })
})
