import { createServer, type Server } from 'node:http'
import type { AddressInfo } from 'node:net'
import { mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterAll, describe, expect, it } from 'vitest'
import { NetworkError, TimeoutError } from '../src/errors'
import type { Recording, TransportRequest } from '../src/transport'
import { createFetchTransport, createReplayTransport, loadRecording } from '../src/transport'

/**
 * 这些用例只用回环地址上的本地服务，不访问外网——DECISIONS.md D3 的要求对测试
 * 自身同样成立。
 */
const servers: Server[] = []

afterAll(async () => {
  await Promise.all(
    servers.map((server) => new Promise<void>((resolve) => server.close(() => resolve()))),
  )
})

async function listen(handler: Parameters<typeof createServer>[1]): Promise<string> {
  const server = createServer(handler)
  servers.push(server)
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject)
    server.listen(0, '127.0.0.1', resolve)
  })
  const { port } = server.address() as AddressInfo
  return `http://127.0.0.1:${port}`
}

function request(url: string): TransportRequest {
  return {
    method: 'POST',
    url,
    headers: { 'content-type': 'application/json' },
    body: '{"ping":true}',
  }
}

describe('createFetchTransport', () => {
  it('原样搬运状态码、状态文本与响应体，不解释内容', async () => {
    const base = await listen((_req, res) => {
      res.writeHead(418, 'I am a teapot', { 'content-type': 'application/json' })
      res.end('{"not":"json-shaped-by-transport"}')
    })

    const response = await createFetchTransport().send(request(base), { timeoutMs: 2_000 })
    expect(response.status).toBe(418)
    expect(response.statusText).toBe('I am a teapot')
    expect(response.body).toBe('{"not":"json-shaped-by-transport"}')
    expect(response.headers['content-type']).toContain('application/json')
  })

  it('把请求体送到服务端', async () => {
    let received = ''
    const base = await listen((req, res) => {
      req.on('data', (chunk: Buffer) => {
        received += chunk.toString('utf8')
      })
      req.on('end', () => {
        res.writeHead(200)
        res.end('{}')
      })
    })

    await createFetchTransport().send(request(base), { timeoutMs: 2_000 })
    expect(received).toBe('{"ping":true}')
  })

  it('连接不上时抛 NetworkError，消息里带上 URL', async () => {
    // 先起后关，拿到一个确定没人监听的端口。
    const base = await listen((_req, res) => res.end())
    const server = servers.pop()
    await new Promise<void>((resolve) => server?.close(() => resolve()))

    const error = await createFetchTransport()
      .send(request(base), { timeoutMs: 2_000 })
      .catch((e: unknown) => e)

    expect(error).toBeInstanceOf(NetworkError)
    expect((error as NetworkError).kind).toBe('network')
    expect((error as NetworkError).message).toContain(base)
  })

  it('服务端不响应时抛 TimeoutError，消息里带上超时值', async () => {
    const base = await listen(() => {
      // 故意不调用 res.end()，让请求悬着。
    })

    const error = await createFetchTransport()
      .send(request(base), { timeoutMs: 120 })
      .catch((e: unknown) => e)

    expect(error).toBeInstanceOf(TimeoutError)
    expect((error as TimeoutError).kind).toBe('timeout')
    expect((error as TimeoutError).timeoutMs).toBe(120)
    expect((error as TimeoutError).message).toContain('120ms')
  })

  it('响应头已到但响应体悬挂时也抛 TimeoutError', async () => {
    const base = await listen((_req, res) => {
      res.writeHead(200, { 'content-type': 'text/plain' })
      res.flushHeaders()
      res.write('partial')
      // 故意保持响应体未结束，覆盖 headers 到达后的读取路径。
    })

    const error = await createFetchTransport()
      .send(request(base), { timeoutMs: 120 })
      .catch((e: unknown) => e)

    expect(error).toBeInstanceOf(TimeoutError)
    expect((error as TimeoutError).timeoutMs).toBe(120)
  })
})

describe('createReplayTransport', () => {
  /** 这里的样本是为了检查回放器本身，不是模型响应的录制样本——真实响应由 scripts/record.ts 录制。 */
  function recording(scenario: string, body: string): Recording {
    return {
      provider: 'local',
      scenario,
      recordedAt: '2026-01-01T00:00:00.000Z',
      request: request('http://recorded.invalid/v1/chat/completions'),
      response: { status: 200, statusText: 'OK', headers: {}, body },
      redacted: ['request.headers.authorization'],
    }
  }

  it('按顺序回放，不发起任何请求', async () => {
    const transport = createReplayTransport([recording('a', '{"n":1}'), recording('a', '{"n":2}')])
    const url = 'http://unreachable.invalid/v1/chat/completions'

    expect((await transport.send(request(url), { timeoutMs: 1 })).body).toBe('{"n":1}')
    expect((await transport.send(request(url), { timeoutMs: 1 })).body).toBe('{"n":2}')
  })

  it('样本用尽时报错，说明第几次请求没有对应录制', async () => {
    const transport = createReplayTransport([recording('a', '{}')])
    const url = 'http://unreachable.invalid/v1/chat/completions'
    await transport.send(request(url), { timeoutMs: 1 })

    await expect(transport.send(request(url), { timeoutMs: 1 })).rejects.toThrowError(
      /第 2 次请求.*没有对应录制/,
    )
  })
})

describe('loadRecording', () => {
  it('从磁盘读回一条录制', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'rein-recording-'))
    const file = join(dir, 'hello-1.json')
    const original: Recording = {
      provider: 'local',
      scenario: 'hello',
      recordedAt: '2026-01-01T00:00:00.000Z',
      request: request('http://recorded.invalid/v1/chat/completions'),
      response: { status: 200, statusText: 'OK', headers: {}, body: '{"ok":true}' },
      redacted: [],
    }
    await writeFile(file, JSON.stringify(original, null, 2))

    expect(await loadRecording(file)).toEqual(original)
  })
})
