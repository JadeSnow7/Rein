/**
 * 可替换的 transport 层。
 *
 * 这是第 01 章唯一一处"多做一步"的结构：把"把一个 HTTP 请求发出去、拿回状态码和
 * 响应体"单独切出来，让调用逻辑不知道字节从网络来还是从录制样本来。
 *
 * 之所以现在就要有它，是因为测试全程不许触网，CI 在没有密钥的环境里运行；
 * 见 DECISIONS.md 的 D3。等到第 03 章有了工具调用再补，就得回头改已经写好的代码。
 *
 * 注意这一层只搬运字节，不认识 messages、不认识 choices。识别形状是 chat.ts 的事。
 */

import { readFile } from 'node:fs/promises'
import { NetworkError, TimeoutError } from './errors'

export interface TransportRequest {
  readonly method: 'POST'
  readonly url: string
  readonly headers: Readonly<Record<string, string>>
  /** 已序列化的请求体 */
  readonly body: string
}

export interface TransportResponse {
  readonly status: number
  readonly statusText: string
  readonly headers: Readonly<Record<string, string>>
  readonly body: string
}

export interface SendOptions {
  readonly timeoutMs: number
}

export interface Transport {
  send(request: TransportRequest, options: SendOptions): Promise<TransportResponse>
}

/** 真实实现：走 Node 22 内置的 fetch。 */
export function createFetchTransport(): Transport {
  return {
    async send(request, options) {
      const signal = AbortSignal.timeout(options.timeoutMs)
      let response: Response
      try {
        response = await fetch(request.url, {
          method: request.method,
          headers: { ...request.headers },
          body: request.body,
          signal,
        })
      } catch (cause) {
        // AbortSignal.timeout 触发时抛出 name 为 TimeoutError 的 DOMException。
        if (isTimeoutAbort(signal, cause)) {
          throw new TimeoutError(request.url, options.timeoutMs)
        }
        throw new NetworkError(request.url, cause)
      }

      let body: string
      try {
        body = await response.text()
      } catch (cause) {
        // 响应头已到，连接在读 body 的过程中断开。
        if (isTimeoutAbort(signal, cause)) {
          throw new TimeoutError(request.url, options.timeoutMs)
        }
        throw new NetworkError(request.url, cause)
      }

      return {
        status: response.status,
        statusText: response.statusText,
        headers: Object.fromEntries(response.headers),
        body,
      }
    },
  }
}

function isTimeoutAbort(signal: AbortSignal, cause: unknown): boolean {
  if (!signal.aborted) return false
  const reason = signal.reason
  return reason === cause || (reason instanceof Error && reason.name === 'TimeoutError')
}

/**
 * 一次录制：请求与响应成对落盘。
 * 文件写在 fixtures/responses/<provider>/<场景>-<序号>.json，命名见 fixtures/README.md。
 */
export interface Recording {
  readonly provider: string
  readonly scenario: string
  readonly recordedAt: string
  readonly request: TransportRequest
  readonly response: TransportResponse
  /** 录制时被替换掉的字段名，便于读者知道样本哪里动过 */
  readonly redacted: readonly string[]
}

export async function loadRecording(file: string): Promise<Recording> {
  const text = await readFile(file, 'utf8')
  return JSON.parse(text) as Recording
}

/**
 * 回放实现：按给定顺序吐出录制好的响应，不碰网络。
 * 请求本身被忽略——第 01 章只发一次请求，比对请求是第 09 章验证器的题目。
 */
export function createReplayTransport(recordings: readonly Recording[]): Transport {
  let next = 0
  return {
    async send(request) {
      const recording = recordings[next]
      if (recording === undefined) {
        throw new Error(
          `回放样本已用尽：第 ${next + 1} 次请求（${request.url}）没有对应录制，` +
            `当前共 ${recordings.length} 条。`,
        )
      }
      next += 1
      return recording.response
    },
  }
}
