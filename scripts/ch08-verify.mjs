import { cpSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'

const root = resolve(fileURLToPath(new URL('..', import.meta.url)))
const target = process.env.CARGO_TARGET_DIR || resolve(root, 'rust/target')
const run = (cmd, args, extra = {}) => spawnSync(cmd, args, { cwd: root, encoding: 'utf8', env: { ...process.env, CARGO_TARGET_DIR: target, CARGO_INCREMENTAL: '0', ...extra } })
const fail = (message) => { throw new Error(message) }
const compare = (args = []) => { const r = run('node', ['scripts/ch08-compare.mjs', ...args]); if (r.status !== 0) fail(r.stderr || `compare exited ${r.status}`); return JSON.parse(r.stdout) }
const base = compare()
if (base.unit !== 'estimated-bytes-v1' || base.results.length !== 16) fail('default compare must produce exactly 16 rows')
const keys = new Set(base.results.map(r => `${r.taskId}/${r.strategy}`))
if (keys.size !== 16) fail('task/strategy combinations are not unique')
for (const r of base.results) {
  if (!r.taskId || !r.strategy || !Array.isArray(r.messages) || r.serviceTokens !== null) fail('row schema contract failed')
  if (r.estimatedUnits !== r.messages.reduce((n, m) => n + 8 + Buffer.byteLength(m.role) + Buffer.byteLength(m.content), 0)) fail(`estimated units mismatch: ${r.taskId}/${r.strategy}`)
  if (r.status === 'completed' && r.modelCalls !== 1) fail(`model call count: ${r.taskId}/${r.strategy}`)
  if (r.taskId === 'task-04' && (r.quality !== 1 || r.answer?.claims?.length !== 0 || !r.answer?.insufficientEvidence)) fail('unknown task must be explicitly insufficient')
}
const filtered = compare(['--strategy', 'retrieval'])
if (filtered.results.length !== 4 || filtered.results.some(r => r.strategy !== 'retrieval')) fail('strategy filter contract failed')

// Exercise a copied data root: changing a fact must change the answer.
const mutation = mkdtempSync(resolve(tmpdir(), 'rein-ch08-verify-'))
cpSync(resolve(root, 'fixtures/ch08-context'), mutation, { recursive: true })
const idxPath = resolve(mutation, 'index.json'); const taskPath = resolve(mutation, 'tasks.json')
const idx = JSON.parse(readFileSync(idxPath, 'utf8')); const tasks = JSON.parse(readFileSync(taskPath, 'utf8')); const oracleBytes = readFileSync(taskPath)
const early = idx.documents.find(d => d.id === 'doc-01'); if (!early) fail('fixture early document missing'); early.keywords = ['传输', '协议'];
const originalEarly = readFileSync(resolve(mutation, early.path), 'utf8')
writeFileSync(resolve(mutation, early.path), originalEarly.replace('传输方式：JSON Lines', '传输方式：HTTP'))
const changed = compare(['--data-root', mutation, '--strategy', 'summary'])
const changedRow = changed.results.find(r => r.taskId === tasks.tasks[0].id)
if (!changedRow?.answer?.rawAnswer.includes('HTTP') || changedRow.quality !== 0.5 || changedRow.answer.claims.find(c => c.field === '传输方式')?.source !== 'doc-01') fail('mutation did not flow through answer/source/quality')
if (!Buffer.from(readFileSync(taskPath)).equals(oracleBytes)) fail('mutation changed tasks.json bytes')
const zero = compare(['--data-root', mutation, '--budget', '0'])
if (zero.results.some(r => r.status !== 'context_budget_exhausted' || r.modelCalls !== 0 || r.operations.length !== 0 || r.answer !== null)) fail('zero budget contract failed')

// Argument and metadata failures must stay explicit process failures.
for (const args of [['--budget', '-1'], ['--budget', '1.2'], ['--budget', 'NaN'], ['--budget', '9007199254740992'], ['--budget'], ['--strategy', 'bad'], ['--strategy', 'window', '--strategy', 'summary']]) {
  const r = run('node', ['scripts/ch08-compare.mjs', ...args]); if (r.status === 0 || !/(invalid|duplicate)/i.test(r.stderr || '')) fail(`invalid CLI accepted: ${args.join(' ')}`)
}
const duplicate = mkdtempSync(resolve(tmpdir(), 'rein-ch08-meta-')); cpSync(resolve(root, 'fixtures/ch08-context'), duplicate, { recursive: true })
const bad = JSON.parse(readFileSync(resolve(duplicate, 'index.json'), 'utf8')); bad.documents[1].id = bad.documents[0].id; writeFileSync(resolve(duplicate, 'index.json'), JSON.stringify(bad))
const badRun = run('node', ['scripts/ch08-compare.mjs', '--data-root', duplicate]); if (badRun.status === 0 || !/duplicate|invalid/i.test(badRun.stderr || '')) fail('duplicate metadata accepted')

// The public example is always executed through its real independent entry.
const example = run('node', ['examples/ch08-context-methods/entry.mjs'])
if (example.status !== 0 || !/"rows":4/.test(example.stdout)) fail(example.stderr || 'independent example failed')
const rust = run('cargo', ['test', '--quiet', '--locked', '--manifest-path', resolve(root, 'rust/Cargo.toml'), '--test', 'ch08_context'])
if (rust.status !== 0) fail(rust.stderr || 'Rust contract tests failed')
process.stdout.write(JSON.stringify({ unit: base.unit, rows: base.results.length, filteredRows: filtered.results.length, checks: ['default', 'filter', 'mutation', 'zero-budget', 'cli', 'metadata', 'example', 'rust-contract-tests'] }) + '\n')
