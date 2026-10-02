import { cpSync, mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
const root = resolve(fileURLToPath(new URL('../..', import.meta.url)))
const work = mkdtempSync(resolve(tmpdir(), 'rein-ch08-example-'))
cpSync(fileURLToPath(new URL('input', import.meta.url)), resolve(work, 'input'), { recursive: true })
cpSync(fileURLToPath(new URL('docs', import.meta.url)), resolve(work, 'input/docs'), { recursive: true })
const r = spawnSync('node', ['scripts/ch08-compare.mjs', '--data-root', resolve(work, 'input')], { cwd: root, encoding: 'utf8' })
if (r.status !== 0) { process.stderr.write(r.stderr); process.exit(r.status ?? 1) }
const out = JSON.parse(r.stdout)
if (out.results.length !== 4) throw new Error('example must produce four strategies')
for (const row of out.results) {
  const claim = row.answer?.claims?.find((item) => item.field === '传输方式')
  if (row.status !== 'completed' || !claim || claim.value !== 'JSON Lines' || claim.source !== 'doc-demo' || row.quality !== 1) throw new Error(`unexpected result for ${row.strategy}`)
  if (!row.callRecords.length || !row.callRecords.every((record) => record.dispatched && record.reaped && record.terminal)) throw new Error(`missing real call record for ${row.strategy}`)
}
console.log(JSON.stringify({ example: 'ch08-context-methods', rows: out.results.length, results: out.results.map((row) => ({ strategy: row.strategy, quality: row.quality, source: row.answer.claims[0].source })) }))
