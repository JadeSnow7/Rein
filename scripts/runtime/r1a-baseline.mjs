import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const root = resolve(import.meta.dirname, '../..')
const baselinePath = resolve(root, 'fixtures/runtime-r1a/baseline.json')
const baseline = JSON.parse(readFileSync(baselinePath, 'utf8'))
const requiredProductFiles = [
  'Cargo.toml',
  'core/Cargo.toml',
  'runtime/Cargo.toml',
  'runtime/tests/r1a_acceptance.rs',
]

const missing = requiredProductFiles.filter((path) => !existsSync(resolve(root, path)))
const result = {
  baseline: baseline.spec,
  case_count: baseline.cases.length,
  cases: baseline.cases.map(({ id, name, expected }) => ({ id, name, expected })),
  product: {
    command: 'CARGO_TARGET_DIR=/private/tmp/rein-r1a-target cargo test --workspace --test r1a_acceptance',
    required_files: requiredProductFiles,
    missing,
  },
  status: missing.length === 0 ? 'ready' : 'blocked: product implementation missing',
}

console.log(JSON.stringify(result, null, 2))
if (missing.length > 0) process.exitCode = 1
