import test from 'node:test'
import assert from 'node:assert/strict'
import { copyFile, mkdir, mkdtemp, readFile, symlink, writeFile } from 'node:fs/promises'
import { createHash } from 'node:crypto'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'

import { evaluateRegistry, RegistryError } from './decisions.mjs'

const accepted = (overrides = {}) => ({
  id: 'DEC-001',
  title: 'Keep persistence behind a repository boundary',
  status: 'accepted',
  rationale: 'The application can change storage without changing callers.',
  scope: 'core/',
  alternatives: ['Direct file access from feature modules'],
  prerequisites: [{ path: 'facts.txt', sha256: sha256('stable\n') }],
  reviewWhen: ['A second process writes the same store'],
  sources: ['D9'],
  ...overrides,
})

const sha256 = (value) => createHash('sha256').update(value).digest('hex')

async function fixture() {
  const root = await mkdtemp(join(tmpdir(), 'rein-decisions-'))
  await writeFile(join(root, 'facts.txt'), 'stable\n')
  return root
}

function registry(...decisions) {
  return { version: 1, decisions }
}

test('reports an accepted decision as applicable when every declared prerequisite matches', async () => {
  const root = await fixture()
  const result = evaluateRegistry({ root, registry: registry(accepted()) })

  assert.equal(result.exitCode, 0)
  assert.equal(result.decisions[0].status, 'applicable')
  assert.equal(result.decisions[0].humanStatus, 'accepted')
  assert.equal(result.decisions[0].prerequisites[0].status, 'applicable')
})

test('reports changed prerequisites as review_required without refreshing the recorded hash', async () => {
  const root = await fixture()
  const record = accepted()
  await writeFile(join(root, 'facts.txt'), 'changed\n')

  const result = evaluateRegistry({ root, registry: registry(record) })

  assert.equal(result.exitCode, 1)
  assert.equal(result.decisions[0].status, 'review_required')
  assert.equal(result.decisions[0].prerequisites[0].status, 'review_required')
  assert.equal(result.decisions[0].prerequisites[0].expectedSha256, sha256('stable\n'))
  assert.equal(record.prerequisites[0].sha256, sha256('stable\n'))
})

test('treats missing files and an empty prerequisite list as undetermined', async () => {
  const root = await fixture()
  const missing = accepted({ id: 'DEC-002', prerequisites: [{ path: 'missing.txt', sha256: 'a'.repeat(64) }] })
  const empty = accepted({ id: 'DEC-003', prerequisites: [] })
  await mkdir(join(root, 'directory'))
  const nonRegular = accepted({ id: 'DEC-004', prerequisites: [{ path: 'directory', sha256: 'b'.repeat(64) }] })

  const result = evaluateRegistry({ root, registry: registry(missing, empty, nonRegular) })

  assert.equal(result.exitCode, 1)
  assert.equal(result.decisions[0].status, 'undetermined')
  assert.equal(result.decisions[0].prerequisites[0].reason, 'missing')
  assert.equal(result.decisions[1].status, 'undetermined')
  assert.match(result.decisions[1].reason, /prerequisite/i)
  assert.equal(result.decisions[2].status, 'undetermined')
  assert.equal(result.decisions[2].prerequisites[0].reason, 'not_regular_file')
})

test('does not treat an empty registry as a passing empty set', async () => {
  const root = await fixture()
  const result = evaluateRegistry({ root, registry: registry() })

  assert.equal(result.exitCode, 1)
  assert.deepEqual(result.decisions, [])
})

test('keeps mixed prerequisite outcomes and unknown status from becoming a pass', async () => {
  const root = await fixture()
  const mixed = accepted({
    prerequisites: [
      { path: 'facts.txt', sha256: sha256('stable\n') },
      { path: 'unknown.txt', sha256: 'b'.repeat(64) },
    ],
  })

  const result = evaluateRegistry({ root, registry: registry(mixed) })

  assert.equal(result.exitCode, 1)
  assert.equal(result.decisions[0].status, 'undetermined')
  assert.deepEqual(result.decisions[0].prerequisites.map((item) => item.status), ['applicable', 'undetermined'])
})

test('preserves proposed and superseded human lifecycle states separately from machine status', async () => {
  const root = await fixture()
  const proposed = accepted({ id: 'DEC-002', status: 'proposed' })
  const superseded = accepted({ id: 'DEC-003', status: 'superseded' })

  const result = evaluateRegistry({ root, registry: registry(proposed, superseded) })

  assert.equal(result.exitCode, 1)
  assert.deepEqual(result.decisions.map((item) => item.status), ['applicable', 'applicable'])
  assert.deepEqual(result.decisions.map((item) => item.humanStatus), ['proposed', 'superseded'])
})

test('rejects duplicate and malformed decision identifiers and hashes', async () => {
  const root = await fixture()
  assert.throws(() => evaluateRegistry({ root, registry: registry(accepted(), accepted({ title: 'duplicate' })) }), RegistryError)
  assert.throws(() => evaluateRegistry({ root, registry: registry(accepted({ id: 'bad id' })) }), RegistryError)
  assert.throws(() => evaluateRegistry({ root, registry: registry(accepted({ prerequisites: [{ path: 'facts.txt', sha256: 'not-a-sha' }] })) }), RegistryError)
})

test('rejects absolute, parent-traversal, and symlinked prerequisite paths', async () => {
  const root = await fixture()
  await writeFile(join(root, 'outside.txt'), 'outside\n')
  await symlink(join(root, 'outside.txt'), join(root, 'linked.txt'))

  for (const path of [resolve(root, 'facts.txt'), '../outside.txt', 'linked.txt']) {
    assert.throws(
      () => evaluateRegistry({ root, registry: registry(accepted({ prerequisites: [{ path, sha256: sha256('stable\n') }] })) }),
      RegistryError,
    )
  }
})

test('rejects symlinks in parent components instead of only checking the leaf', async () => {
  const root = await fixture()
  const outside = await mkdtemp(join(tmpdir(), 'rein-decisions-outside-'))
  await writeFile(join(outside, 'facts.txt'), 'stable\n')
  await symlink(outside, join(root, 'linked-parent'))

  assert.throws(
    () => evaluateRegistry({ root, registry: registry(accepted({ prerequisites: [{ path: 'linked-parent/facts.txt', sha256: sha256('stable\n') }] })) }),
    RegistryError,
  )
})

test('does not mutate the registry input while checking it', async () => {
  const root = await fixture()
  const input = registry(accepted())
  const before = JSON.stringify(input)

  evaluateRegistry({ root, registry: input })

  assert.equal(JSON.stringify(input), before)
})

test('CLI emits JSON and uses exit code 1 for a changed prerequisite', async () => {
  const root = await fixture()
  const registryPath = join(root, 'decisions.json')
  const record = registry(accepted())
  await writeFile(registryPath, JSON.stringify(record, null, 2) + '\n')
  await writeFile(join(root, 'facts.txt'), 'changed\n')

  const cli = join(process.cwd(), 'scripts/architecture/decisions.mjs')
  const result = spawnSync(process.execPath, [cli, '--root', root, '--registry', registryPath], { encoding: 'utf8' })

  assert.equal(result.status, 1)
  const output = JSON.parse(result.stdout)
  assert.equal(output.decisions[0].status, 'review_required')
  assert.equal(await readFile(registryPath, 'utf8'), JSON.stringify(record, null, 2) + '\n')
})

test('CLI works when the repository path contains spaces', async () => {
  const parent = await mkdtemp(join(tmpdir(), 'rein decisions space-'))
  const root = join(parent, 'repo with spaces')
  await mkdir(root)
  await writeFile(join(root, 'facts.txt'), 'stable\n')
  const registryPath = join(root, 'decisions.json')
  await writeFile(registryPath, JSON.stringify(registry(accepted()), null, 2) + '\n')
  const cli = join(root, 'decisions.mjs')
  await copyFile(join(process.cwd(), 'scripts/architecture/decisions.mjs'), cli)
  const result = spawnSync(process.execPath, [cli, '--root', root, '--registry', registryPath], { encoding: 'utf8' })
  assert.equal(result.status, 0, result.stderr)
  assert.equal(JSON.parse(result.stdout).exitCode, 0)
})
