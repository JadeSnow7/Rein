import { afterEach, describe, expect, it } from 'vitest'
import { mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { createInterface } from 'node:readline'

const repo = resolve(import.meta.dirname, '../..')
const host = resolve(repo, 'ts/src/hybrid-host.ts')
const fixture = resolve(repo, 'ts/tests/fixtures/hybrid-fault-host.ts')
const children: ChildProcessWithoutNullStreams[] = []

afterEach(() => { for (const child of children.splice(0)) if (!child.killed) child.kill('SIGTERM') })

function start(script = host, env: Record<string, string> = {}) {
  const child = spawn(process.execPath, ['--import', 'tsx', script], { cwd: repo, env: { ...process.env, ...env }, stdio: ['pipe', 'pipe', 'pipe'] })
  children.push(child)
  child.on('error', error => console.error('hybrid child error', error))
  child.stderr.on('data', data => console.error('hybrid child stderr', data.toString()))
  const lines = createInterface({ input: child.stdout })
  const queue: Record<string, unknown>[] = []
  const waiters: ((value: Record<string, unknown>) => void)[] = []
  lines.on('line', line => {
    try {
      const value = JSON.parse(line) as Record<string, unknown>
      const waiter = waiters.shift()
      if (waiter) waiter(value); else queue.push(value)
    } catch { /* malformed output is asserted by the caller through stderr/exit */ }
  })
  const read = (): Promise<Record<string, unknown>> => queue.length ? Promise.resolve(queue.shift()!) : new Promise(resolve => waiters.push(resolve))
  const send = (value: Record<string, unknown>) => child.stdin.write(`${JSON.stringify(value)}\n`)
  return { child, read, send }
}

const request = { protocol: 'rein-extension/0.1', type: 'invoke', sessionId: 's-1', requestId: 'r-1', taskId: 't-1', callId: 'c-1', tool: 'read_file', path: 'fixtures/hybrid-marker.txt', ruleId: 'read-file-content-v1', targetVersion: 'hybrid-m1' }

describe('hybrid TS host subprocess', () => {
  it('runs one readonly call and returns complete evidence', async () => {
    const workspace = await mkdtemp(join(tmpdir(), 'rein-hybrid-host-'))
    await writeFile(join(workspace, 'fixtures-hybrid-marker.txt'), 'ok\n')
    const run = start(host, { REIN_HYBRID_WORKSPACE: workspace })
    expect(await run.read()).toMatchObject({ protocol: 'rein-extension/0.1', type: 'ready', capabilities: ['read_file'] })
    run.send({ ...request, path: 'fixtures-hybrid-marker.txt' })
    expect(await run.read()).toMatchObject({ type: 'started', sessionId: 's-1', requestId: 'r-1', taskId: 't-1', callId: 'c-1' })
    const terminal = await run.read()
    expect(terminal).toMatchObject({ type: 'terminal', status: 'succeeded', output: 'ok\n', evidence: { taskId: 't-1', callId: 'c-1', path: 'fixtures-hybrid-marker.txt', ruleId: 'read-file-content-v1', targetVersion: 'hybrid-m1' } })
    await new Promise<void>(resolve => run.child.once('close', () => resolve()))
  })

  it('rejects unknown request fields without entering a loop', async () => {
    const run = start(host, { REIN_HYBRID_WORKSPACE: repo })
    expect((await run.read()).type).toBe('ready')
    run.send({ ...request, unexpected: true })
    expect(await run.read()).toMatchObject({ type: 'error', code: 'invalid_request' })
    run.child.kill('SIGTERM')
  })

  it('does not cancel a call when any identifier is wrong', async () => {
    const workspace = await mkdtemp(join(tmpdir(), 'rein-hybrid-host-'))
    await writeFile(join(workspace, 'marker.txt'), 'ok\n')
    const run = start(host, { REIN_HYBRID_WORKSPACE: workspace })
    await run.read(); run.send({ ...request, path: 'marker.txt' }); run.send({ protocol: request.protocol, type: 'cancel', sessionId: request.sessionId, requestId: request.requestId, taskId: request.taskId, callId: 'wrong-call' })
    expect((await run.read()).type).toBe('started')
    const followup = await run.read()
    expect(followup).toMatchObject({ type: 'error', code: 'invalid_cancel' })
    expect((await run.read()).status).toBe('succeeded')
  })

  it('fixture supports a cancellation barrier and only cancels exact ids', async () => {
    const run = start(fixture, { REIN_HYBRID_FAULT_MODE: 'cancellation_wait' })
    await run.read(); run.send(request)
    expect((await run.read()).type).toBe('started')
    run.send({ ...request, type: 'cancel', requestId: 'wrong-request' });
    run.send({ ...request, type: 'cancel' })
    expect((await run.read()).status).toBe('cancelled')
  })
})
