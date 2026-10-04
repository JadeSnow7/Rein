import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
const root = resolve(fileURLToPath(new URL('..', import.meta.url)))
const args = process.argv.slice(2)
let data = resolve(root, 'fixtures/ch08-context'); let budget = null; let strategy = null
const seen = new Set()
for (let i = 0; i < args.length; i += 1) {
  const key = args[i]; const value = args[i + 1]
  if (!['--data-root', '--budget', '--strategy'].includes(key) || value === undefined || value.startsWith('--')) throw new Error(`invalid argument: ${key}`)
  if (seen.has(key)) throw new Error(`duplicate ${key}`); seen.add(key)
  if (key === '--data-root') data = resolve(value)
  if (key === '--budget') { if (!/^\d+$/.test(value) || Number(value) > Number.MAX_SAFE_INTEGER) throw new Error('invalid --budget'); budget = value }
  if (key === '--strategy') { if (!['on-demand','window','summary','retrieval'].includes(value)) throw new Error('invalid --strategy'); strategy = value }
  i += 1
}
const rustArgs = [ 'run', '--quiet', '--locked', '--manifest-path', resolve(root, 'rust/Cargo.toml'), '--example', 'ch08_context', '--', data ]
if (budget !== null) rustArgs.push(`--budget=${budget}`)
if (strategy !== null) rustArgs.push(`--strategy=${strategy}`)
const result = spawnSync('cargo', rustArgs, { cwd: root, encoding: 'utf8' })
if (result.status !== 0) { process.stderr.write(result.stderr); process.exit(result.status ?? 1) }
process.stdout.write(result.stdout)
