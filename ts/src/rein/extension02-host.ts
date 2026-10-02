/** Disposable, single-invoke reference host. Application checks are not an OS sandbox. */
import { createHash } from 'node:crypto'
import { lstat, readFile, realpath, readdir } from 'node:fs/promises'
import { isAbsolute, relative, resolve, sep } from 'node:path'
import { dispatchReadonly } from './readonly'
import type { ToolCall } from './contracts'
import { applyApprovedCandidate, runVerification } from './maintenance-tools'
const protocol = 'rein-extension/0.2'
const limit = 256 * 1024
const resultSchema = { type: 'object', required: ['ok'], properties: { ok: { type: 'boolean' }, output: { type: 'string' }, error: { type: 'object' } } }
const tools: any[] = [
 { name: 'read_file', description: 'Read one UTF-8 file in the workspace.', argumentsSchema: { type: 'object', properties: { path: { type: 'string' } }, required: ['path'], additionalProperties: false }, resultSchema },
 { name: 'search_files', description: 'Search text in workspace files.', argumentsSchema: { type: 'object', properties: { needle: { type: 'string' } }, required: ['needle'], additionalProperties: false }, resultSchema },
]
const maintenance = process.env.REIN_EXTENSION_PROFILE === 'maintenance'
if (maintenance) tools.push(
 { name: 'run_verification', description: 'Run one trusted document verification rule.', argumentsSchema: { type: 'object', properties: { target: { type: 'string' }, rule: { type: 'string' } }, required: ['target','rule'], additionalProperties: false }, resultSchema: { type: 'object', required: ['ok','rule','target','output'], properties: { ok: { type: 'boolean' }, rule: { type: 'string' }, target: { type: 'string' }, output: { type: 'string' }, error: { type: 'object' } } } },
 { name: 'apply_patch', description: 'Apply one core-approved replacement to an existing UTF-8 file.', argumentsSchema: { type: 'object', properties: { path: { type: 'string' }, baselineSha256: { type: 'string' }, replacement: { type: 'string' }, digest: { type: 'string' } }, required: ['path','baselineSha256','replacement','digest'], additionalProperties: false }, resultSchema: { type: 'object', required: ['ok','path'], properties: { ok: { type: 'boolean' }, path: { type: 'string' }, postSha256: { type: 'string' }, error: { type: 'object' } } } },
)
type Wire = Record<string, unknown>
const object = (v: unknown): v is Wire => v !== null && typeof v === 'object' && !Array.isArray(v)
const keys = (v: Wire, allowed: string[]) => Object.keys(v).length === allowed.length && allowed.every(k => Object.hasOwn(v, k))
const identityKeys = ['sessionId','requestId','taskId','callId']
const identity = (v: Wire) => Object.fromEntries(identityKeys.map(k => [k, v[k]]))
let active: Wire | undefined
let activeController: AbortController | undefined
let cancelled = false
let ended = false
function finish(code = 0) { if (ended) return; ended = true; process.stdin.destroy(); process.stdout.end(() => { process.exitCode = code }) }
function send(v: Wire) { const line = JSON.stringify(v) + '\n'; if (Buffer.byteLength(line) > limit) throw new Error('output_frame_limit'); process.stdout.write(line) }
function fatal() { finish(1) }
async function safe(root: string, input: string) {
 if (!input || isAbsolute(input) || input.split(/[\\/]/).some(p => p === '..' || p === '.' || !p)) throw new Error('invalid_path')
 let p = root
 for (const part of input.split('/')) { p = resolve(p, part); if ((await lstat(p)).isSymbolicLink()) throw new Error('symlink_path') }
 const actual = await realpath(p); const rel = relative(root, actual)
 if (rel === '..' || rel.startsWith(`..${sep}`) || isAbsolute(rel) || !(await lstat(actual)).isFile()) throw new Error('invalid_path')
 return actual
}
async function snapshot(root: string): Promise<Array<{path:string;sha256:string}>> {
 const out: Array<{path:string;sha256:string}> = []
 const visit = async (dir: string): Promise<void> => {
  const entries = (await readdir(dir, { withFileTypes: true })).sort((a,b) => Buffer.compare(Buffer.from(a.name), Buffer.from(b.name)))
  for (const entry of entries) { const path = resolve(dir, entry.name); if (entry.isSymbolicLink()) throw new Error('symlink in workspace'); if (entry.isDirectory()) await visit(path); else if (entry.isFile()) out.push({ path: relative(root, path).split(sep).join('/'), sha256: createHash('sha256').update(await readFile(path)).digest('hex') }) }
 }
 await visit(root); return out
}
async function handle(v: unknown) {
 if (ended || !object(v) || v.protocol !== protocol) return fatal()
 if (v.type === 'cancel') {
  if (!active || !keys(v, ['protocol','type',...identityKeys]) || identityKeys.some(k => v[k] !== active![k])) return fatal()
  cancelled = true; activeController?.abort(); return
 }
 const allowed = ['protocol','type',...identityKeys,'tool','arguments', ...(Object.hasOwn(v,'evidenceTarget') ? ['evidenceTarget'] : [])]
 if (active || v.type !== 'invoke' || !keys(v, allowed) || identityKeys.some(k => typeof v[k] !== 'string' || !v[k]) || typeof v.tool !== 'string' || !object(v.arguments) || (Object.hasOwn(v,'evidenceTarget') && typeof v.evidenceTarget !== 'string')) return fatal()
 active = v
 activeController = new AbortController()
 const base = { protocol, ...identity(v) }
 send({ ...base, type: 'started' })
 const root = await realpath(process.env.REIN_HYBRID_WORKSPACE ?? process.cwd())
 const field = v.tool === 'read_file' ? 'path' : v.tool === 'search_files' ? 'needle' : undefined
 if (maintenance && v.tool === 'run_verification') {
  if (!keys(v.arguments, ['target','rule']) || typeof v.arguments.target !== 'string' || typeof v.arguments.rule !== 'string') { send({ ...base, type: 'terminal', status: 'tool_failed', result: { ok: false, error: { code: 'arguments_invalid', message: 'invalid verification arguments' } } }); return finish() }
  const result = await runVerification({ root }, v.arguments.target, v.arguments.rule, 5000, activeController.signal)
  await safe(root, v.arguments.target)
  if (cancelled) { send({ ...base, type: 'terminal', status: 'cancelled' }); return finish() }
  send({ ...base, type: 'terminal', status: 'succeeded', result: { ok: true, output: JSON.stringify(result) }, evidence: { taskId: v.taskId, callId: v.callId, tool: v.tool, arguments: v.arguments, sources: await snapshot(root) } }); return finish()
 }
 if (maintenance && v.tool === 'apply_patch') {
  const a = v.arguments
  if (!keys(a, ['path','baselineSha256','replacement','digest']) || Object.values(a).some(x => typeof x !== 'string') || process.env.REIN_APPROVED_PATCH_DIGEST !== a.digest) { send({ ...base, type: 'terminal', status: 'tool_failed', result: { ok: false, error: { code: 'approval_required', message: 'core approval token missing' } } }); return finish() }
  const result = await applyApprovedCandidate({ root }, { path: a.path as string, baselineSha256: a.baselineSha256 as string, replacement: a.replacement as string, digest: a.digest as string, diff: '' }, process.env.REIN_APPROVED_PATCH_DIGEST!)
  if (cancelled) { send({ ...base, type: 'terminal', status: 'cancelled' }); return finish() }
  if (!result.ok) { send({ ...base, type: 'terminal', status: 'tool_failed', result: { ok: false, error: result.error ?? { code: 'apply_failed', message: 'patch was not applied' } } }); return finish() }
  const sources = result.ok ? [{ path: a.path as string, sha256: result.postSha256 }] : []
  send({ ...base, type: 'terminal', status: 'succeeded', result: { ok: true, output: JSON.stringify(result) }, evidence: { taskId: v.taskId, callId: v.callId, tool: v.tool, arguments: v.arguments, sources } }); return finish()
 }
 if (!field || !keys(v.arguments,[field]) || typeof v.arguments[field] !== 'string' || !v.arguments[field]) {
  send({ ...base, type: 'terminal', status: 'tool_failed', result: { ok: false, error: { code: field ? 'arguments_invalid' : 'unknown_tool', message: 'tool or arguments do not match declaration' } } }); return finish()
 }
 const call: ToolCall = { id: v.callId as string, name: v.tool, arguments: v.arguments }
 try {
  if (call.name === 'read_file') await safe(root, call.arguments.path as string)
  const result = await dispatchReadonly(call, { root })
  if (cancelled) { send({ ...base, type: 'terminal', status: 'cancelled' }); return finish() }
  if (!result.ok) { send({ ...base, type: 'terminal', status: 'tool_failed', result: { ok: false, error: result.error } }); return finish() }
  const output = result.output ?? ''
  const paths = call.name === 'read_file' ? [call.arguments.path as string] : output.split('\n').filter(Boolean)
  const sources = []
  for (const path of paths) { const bytes = await readFile(await safe(root,path)); sources.push({ path, sha256: createHash('sha256').update(bytes).digest('hex') }) }
  if (cancelled) send({ ...base, type: 'terminal', status: 'cancelled' })
  else send({ ...base, type: 'terminal', status: 'succeeded', result: { ok: true, output }, evidence: { taskId: v.taskId, callId: v.callId, tool: v.tool, arguments: v.arguments, sources } })
  finish()
 } catch (e) {
  if (cancelled) send({ ...base, type: 'terminal', status: 'cancelled' })
  else send({ ...base, type: 'terminal', status: 'tool_failed', result: { ok: false, error: { code: 'tool_failed', message: e instanceof Error ? e.message : 'tool failed' } } })
  finish()
 }
}
send({ protocol, type: 'ready', tools, capabilities: tools.map(t => t.name), maxMessageBytes: limit })
let pending = Buffer.alloc(0)
process.stdin.on('data', (chunk: Buffer) => {
 if (ended) return
 // Append bounded pieces: never allocate a peer-controlled unbounded line.
 for (let offset = 0; offset < chunk.length && !ended;) {
  const room = limit - pending.length
  if (!room) return fatal()
  const take = Math.min(room, chunk.length - offset)
  pending = Buffer.concat([pending,chunk.subarray(offset,offset+take)]); offset += take
  let newline: number
  while ((newline = pending.indexOf(10)) >= 0 && !ended) {
   const line = pending.subarray(0,newline); pending = pending.subarray(newline+1)
   try { const value: unknown = JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(line)); void handle(value).catch(fatal) } catch { fatal() }
  }
 }
})
process.stdin.on('end', () => { if (pending.length || !active) fatal() })
process.stdin.on('error', fatal)
