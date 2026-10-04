import { createServer } from 'node:http'
import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { spawn } from 'node:child_process'
import { describe, expect, it } from 'vitest'

const root = resolve(import.meta.dirname, '../..')
type CliResult = { readonly status: number | null; readonly stdout: string; readonly stderr: string }

async function withServer(status: number, run: (baseUrl: string) => Promise<void>): Promise<void> {
  const server = createServer((_request, response) => {
    response.writeHead(status, { 'content-type': 'application/json' })
    response.end(status === 200 ? JSON.stringify({ choices: [{ message: { role: 'assistant', content: 'ok' } }] }) : '{"error":"synthetic"}')
  })
  await new Promise<void>((resolveListen, reject) => {
    server.once('error', reject)
    server.listen(0, '127.0.0.1', resolveListen)
  })
  const address = server.address()
  if (!address || typeof address === 'string') throw new Error('server did not expose a port')
  try { await run(`http://127.0.0.1:${address.port}`) } finally {
    await new Promise<void>((resolveClose) => server.close(() => resolveClose()))
  }
}

function runCli(baseUrl: string): Promise<CliResult> {
  const workspace = mkdtempSync(join(tmpdir(), 'rein-live-test-'))
  writeFileSync(join(workspace, 'README.md'), 'marker: test\n')
  return new Promise((resolveResult, reject) => {
    const child = spawn('node', ['--import', 'tsx', 'ts/examples/ch05-live.ts', '--live', workspace, '请搜索 marker: 并读取命中文件'], {
      cwd: root,
      env: { ...process.env, REIN_BASE_URL: baseUrl, REIN_API_KEY: 'synthetic', REIN_MODEL: 'synthetic' },
    })
    let stdout = ''; let stderr = ''
    child.stdout.on('data', (chunk: Buffer) => { stdout += chunk.toString() })
    child.stderr.on('data', (chunk: Buffer) => { stderr += chunk.toString() })
    child.once('error', reject)
    child.once('close', (status) => resolveResult({ status, stdout, stderr }))
  })
}

describe('live CLI exit contract', () => {
  it('returns zero only for completed final_answer', async () => {
    await withServer(200, async (baseUrl) => {
      const result = await runCli(baseUrl)
      expect(result.status).toBe(0)
      expect(JSON.parse(result.stdout)).toMatchObject({ state: 'completed', reason: 'final_answer' })
    })
  }, 30_000)

  it('returns one and preserves the failure result for a live model error', async () => {
    await withServer(500, async (baseUrl) => {
      const result = await runCli(baseUrl)
      expect(result.status).toBe(1)
      expect(JSON.parse(result.stdout)).toMatchObject({ state: 'failed', reason: 'model_error' })
    })
  }, 30_000)
})
