import { createHash } from 'node:crypto'
import { lstat, readFile, realpath, rename, writeFile, stat, unlink } from 'node:fs/promises'
import { dirname, relative, resolve, sep } from 'node:path'
import { spawn } from 'node:child_process'

export interface MaintenanceWorkspace { readonly root: string }
export interface PatchCandidate { readonly path: string; readonly baselineSha256: string; readonly replacement: string; readonly digest: string; readonly diff: string }
export interface ApplyResult { readonly ok: boolean; readonly path: string; readonly postSha256?: string; readonly error?: { code: string; message: string } }
export interface VerificationResult { readonly ok: boolean; readonly rule: string; readonly target: string; readonly output: string; readonly error?: { code: string; message: string } }

const digest = (value: string) => createHash('sha256').update(value).digest('hex')
const inside = (root: string, file: string) => { const rel = relative(root, file); return rel === '' || (!rel.startsWith(`..${sep}`) && rel !== '..' && !rel.startsWith(sep)) }
async function safeRegularFile(workspace: MaintenanceWorkspace, input: string): Promise<{ root: string; path: string }> {
  const root = await realpath(workspace.root); const path = resolve(root, input)
  if (input.startsWith('/') || !inside(root, path)) throw new Error('path escapes workspace')
  let cursor = root; const parts = relative(root, path).split(sep)
  for (const part of parts) { cursor = resolve(cursor, part); const stat = await lstat(cursor); if (stat.isSymbolicLink()) throw new Error('symlink path component') }
  const stat = await lstat(path)
  if (!stat.isFile() || stat.isSymbolicLink()) throw new Error('target must be a regular file')
  return { root, path }
}
export async function createCandidate(workspace: MaintenanceWorkspace, path: string, replacement: string): Promise<PatchCandidate> {
  const target = await safeRegularFile(workspace, path); const original = await readFile(target.path, 'utf8')
  if (original === replacement) throw new Error('no_change')
  const baselineSha256 = digest(original); const diff = `--- a/${path}\n+++ b/${path}\n@@\n-${original}+${replacement}`
  return { path, baselineSha256, replacement, digest: digest(`${path}\0${baselineSha256}\0${replacement}`), diff }
}
export async function applyApprovedCandidate(workspace: MaintenanceWorkspace, candidate: PatchCandidate, approvedDigest: string): Promise<ApplyResult> {
  try {
    const canonicalDigest = digest(`${candidate.path}\0${candidate.baselineSha256}\0${candidate.replacement}`)
    if (approvedDigest !== canonicalDigest || candidate.digest !== canonicalDigest) return { ok: false, path: candidate.path, error: { code: 'approval_mismatch', message: 'approval is not bound to candidate content' } }
    const target = await safeRegularFile(workspace, candidate.path); const current = await readFile(target.path, 'utf8')
    if (digest(current) !== candidate.baselineSha256) return { ok: false, path: candidate.path, error: { code: 'baseline_mismatch', message: 'file changed after approval' } }
    const temp = resolve(dirname(target.path), `.${target.path.split(sep).pop()}.rein-${process.pid}-${Date.now()}-${Math.random().toString(16).slice(2)}.tmp`)
    let created = false
    try { const originalMode = (await stat(target.path)).mode & 0o777; const handle = await (await import('node:fs/promises')).open(temp, 'wx'); created = true; try { await handle.writeFile(candidate.replacement, 'utf8'); await handle.chmod(originalMode) } finally { await handle.close() }; if (digest(await readFile(target.path,'utf8')) !== candidate.baselineSha256) throw new Error('file changed before rename'); await rename(temp, target.path) } catch (error) { if (created) { try { await unlink(temp) } catch {} } ; throw error }
    return { ok: true, path: candidate.path, postSha256: digest(candidate.replacement) }
  } catch (error) { return { ok: false, path: candidate.path, error: { code: 'apply_failed', message: error instanceof Error ? error.message : 'apply failed' } } }
}
export function runVerification(workspace: MaintenanceWorkspace, target: string, rule: string, timeoutMs = 5000, signal?: AbortSignal): Promise<VerificationResult> {
  if (!['document-maintenance-v1', 'command-v1', 'parameter-v1', 'relative-link-v1'].includes(rule)) return Promise.resolve({ ok: false, rule, target, output: '', error: { code: 'rule_invalid', message: 'unsupported verification rule' } })
  if (signal?.aborted) return Promise.resolve({ ok: false, rule, target, output: '', error: { code: 'cancelled', message: 'verification cancelled before dispatch' } })
  return new Promise(resolveResult => {
    const verifier = resolve(import.meta.dirname, '../../../examples/document-maintenance/verify.mjs')
    const child = spawn(process.execPath, [verifier, '--rule', rule, '--target', target], { cwd: workspace.root, shell: false })
    let output = Buffer.alloc(0)
    let reason: 'timeout' | 'cancelled' | 'output_limit' | undefined
    let killTimer: NodeJS.Timeout | undefined
    let spawnError: Error | undefined
    const stop = (value: typeof reason) => {
      if (reason) return
      reason = value
      child.kill('SIGTERM')
      killTimer = setTimeout(() => {
        if (child.exitCode === null && child.signalCode === null) child.kill('SIGKILL')
      }, 100)
    }
    const append = (data: Buffer) => {
      const room = 64 * 1024 - output.length
      output = Buffer.concat([output, data.subarray(0, room)])
      if (data.length > room) stop('output_limit')
    }
    const timer = setTimeout(() => stop('timeout'), timeoutMs)
    const abort = () => stop('cancelled')
    signal?.addEventListener('abort', abort, { once: true })
    if (signal?.aborted) abort()
    child.stdout.on('data', append); child.stderr.on('data', append)
    child.on('error', error => { spawnError = error })
    child.on('close', code => {
      clearTimeout(timer); clearTimeout(killTimer); signal?.removeEventListener('abort', abort)
      const ok = code === 0 && !reason && !spawnError
      resolveResult({ ok, rule, target, output: output.toString('utf8'), ...(ok ? {} : {
        error: { code: spawnError ? 'spawn_failed' : reason ?? 'verification_failed', message: spawnError?.message ?? reason ?? `exit ${code}` }
      }) })
    })
  })
}

export class RepairBudget { readonly max = 2; private used = 0; nextAttemptId(): string | undefined { if (this.used >= this.max) return undefined; this.used += 1; return `repair-${this.used}` } stopOnUnknown(): void { this.used = this.max } get remaining() { return this.max - this.used } }
