import test from 'node:test'
import assert from 'node:assert/strict'
import { chmod, mkdir, mkdtemp, readFile, readdir, symlink, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { runEvolution, validatePlan, EvolutionPlanError } from './runner.mjs'

const here = fileURLToPath(new URL('.', import.meta.url))

async function fixtureDir(prefix = 'evolution-test-') {
  return mkdtemp(join(tmpdir(), prefix))
}

async function nodeProgram(dir, name, source) {
  const file = join(dir, name.replace(/\.mjs$/, '.cjs'))
  await writeFile(file, `#!/usr/bin/env node\n${source}\n`)
  await chmod(file, 0o755)
  return { command: process.execPath, args: [file] }
}

function basePlan(seedFile, outputDir, adapter, validator, rounds) {
  return {
    sequenceId: 'fixture-sequence',
    seedFile,
    outputDir,
    timeoutMs: 500,
    adapter,
    validator,
    rounds,
  }
}

const check = (id, command, args = [], extra = {}) => ({ id, command, args, ...extra })

test('each round receives the previous round output and persists its own result', async () => {
  const root = await fixtureDir()
  const seedFile = join(root, 'seed.md')
  const outputDir = join(root, 'results')
  await writeFile(seedFile, 'seed-v0\n')
  const adapter = await nodeProgram(root, 'adapter.mjs', `
    const fs = require('node:fs')
    const args = Object.fromEntries(process.argv.slice(2).reduce((a, v, i, x) => i % 2 ? a : (a.push([v.slice(2), x[i + 1]]), a), []))
    if (!fs.readFileSync(args['task-file'], 'utf8').includes(args['round-id'])) process.exit(8)
    const previous = fs.readFileSync(args['input-file'], 'utf8')
    fs.writeFileSync(args['output-file'], previous.trim() + ' -> ' + args['round-id'] + '\\n')
  `)
  const validator = await nodeProgram(root, 'validator.mjs', `
    const fs = require('node:fs')
    const args = Object.fromEntries(process.argv.slice(2).reduce((a, v, i, x) => i % 2 ? a : (a.push([v.slice(2), x[i + 1]]), a), []))
    if (!fs.readFileSync(args['output-file'], 'utf8').includes(args['round-id'])) process.exit(3)
  `)
  const rounds = ['r01', 'r02', 'r03'].map((id) => ({ id, task: `write ${id}`, checks: [] }))
  const result = await runEvolution({ plan: basePlan(seedFile, outputDir, adapter, validator, rounds) })

  assert.equal(result.status, 'passed')
  assert.equal(result.rounds.length, 3)
  assert.equal(await readFile(join(outputDir, 'r03', 'artifact'), 'utf8'), 'seed-v0 -> r01 -> r02 -> r03\n')
  assert.equal(result.rounds[1].inputHash, result.rounds[0].outputHash)
  assert.equal(result.rounds[2].inputHash, result.rounds[1].outputHash)
  assert.equal(result.rounds[0].cost, null)
  assert.equal(JSON.parse(await readFile(join(outputDir, 'r01', 'result.json'), 'utf8')).id, 'r01')
  assert.equal(JSON.parse(await readFile(join(outputDir, 'summary.json'), 'utf8')).status, 'passed')
  assert.equal(JSON.parse(await readFile(join(outputDir, 'plan.json'), 'utf8')).sequenceId, 'fixture-sequence')
})

test('keeps old checks active and retires one only with an explicit replacement reason', async () => {
  const root = await fixtureDir()
  const seedFile = join(root, 'seed.md')
  await writeFile(seedFile, 'stable\n')
  const outputDir = join(root, 'results')
  const adapter = await nodeProgram(root, 'adapter.mjs', `
    const fs = require('node:fs')
    const a = Object.fromEntries(process.argv.slice(2).reduce((x, v, i, z) => i % 2 ? x : (x.push([v.slice(2), z[i + 1]]), x), []))
    fs.copyFileSync(a['input-file'], a['output-file'])
  `)
  const validator = await nodeProgram(root, 'validator.mjs', `process.exit(0)`)
  const oldCheck = check('old-contract', process.execPath, ['-e', 'process.exit(0)', '--'])
  const retiredCheck = check('old-format', process.execPath, ['-e', 'process.exit(0)', '--'])
  const replacementCheck = check('old-format-v2', process.execPath, ['-e', 'process.exit(0)', '--'])
  const result = await runEvolution({ plan: basePlan(seedFile, outputDir, adapter, validator, [
    { id: 'r01', task: 'introduce checks', checks: [oldCheck, retiredCheck] },
    { id: 'r02', task: 'replace one check', checks: [
      { id: 'old-format', retired: true, retirementReason: 'replaced by old-format-v2 after schema migration' },
      replacementCheck,
    ] },
  ]) })

  assert.equal(result.status, 'passed')
  assert.equal(result.rounds[0].checks.find((x) => x.id === 'old-contract').status, 'passed')
  assert.equal(result.rounds[1].checks.find((x) => x.id === 'old-contract').status, 'passed')
  assert.equal(result.rounds[1].checks.find((x) => x.id === 'old-format').status, 'retired')
  assert.match(result.rounds[1].checks.find((x) => x.id === 'old-format').reason, /schema migration/)
  assert.throws(() => validatePlan(basePlan(seedFile, join(root, 'bad-results'), adapter, validator, [
    { id: 'r01', task: 'introduce', checks: [oldCheck] },
    { id: 'r02', task: 'unknown retirement', checks: [{ ...oldCheck, id: 'missing', retired: true, retirementReason: 'no such active check' }] },
  ])), EvolutionPlanError)
  assert.throws(() => validatePlan(basePlan(seedFile, join(root, 'bad-results-2'), adapter, validator, [
    { id: 'r01', task: 'introduce', checks: [oldCheck] },
    { id: 'r02', task: 'retire without reason', checks: [{ id: 'old-contract', retired: true }] },
  ])), EvolutionPlanError)
})

test('does not treat adapter exit 0 as verification: an independent validator can fail', async () => {
  const root = await fixtureDir()
  const seedFile = join(root, 'seed.md')
  await writeFile(seedFile, 'expected\n')
  const adapter = await nodeProgram(root, 'adapter.mjs', `
    const fs = require('node:fs')
    const a = Object.fromEntries(process.argv.slice(2).reduce((x, v, i, z) => i % 2 ? x : (x.push([v.slice(2), z[i + 1]]), x), []))
    fs.writeFileSync(a['output-file'], 'wrong\\n')
  `)
  const validator = await nodeProgram(root, 'validator.mjs', `
    const fs = require('node:fs')
    const a = Object.fromEntries(process.argv.slice(2).reduce((x, v, i, z) => i % 2 ? x : (x.push([v.slice(2), z[i + 1]]), x), []))
    process.exit(fs.readFileSync(a['output-file'], 'utf8') === 'expected\\n' ? 0 : 7)
  `)
  const result = await runEvolution({ plan: basePlan(seedFile, join(root, 'results'), adapter, validator, [{ id: 'r01', task: 'bad change', checks: [] }]) })
  assert.equal(result.status, 'failed')
  assert.equal(result.rounds[0].adapterStatus, 'passed')
  assert.equal(result.rounds[0].verificationStatus, 'failed')
})

test('records a historical regression when a new check passes but an old check fails', async () => {
  const root = await fixtureDir()
  const seedFile = join(root, 'seed.md')
  await writeFile(seedFile, 'seed\n')
  const adapter = await nodeProgram(root, 'adapter.mjs', `
    const fs = require('node:fs')
    const a = Object.fromEntries(process.argv.slice(2).reduce((x, v, i, z) => i % 2 ? x : (x.push([v.slice(2), z[i + 1]]), x), []))
    fs.writeFileSync(a['output-file'], a['round-id'] === 'r01' ? 'old-ok\\n' : 'new-ok\\n')
  `)
  const validator = await nodeProgram(root, 'validator.mjs', `process.exit(0)`)
  const oldProgram = await nodeProgram(root, 'old-check.mjs', `
    const fs = require('node:fs')
    const args = Object.fromEntries(process.argv.slice(2).reduce((x, v, i, z) => i % 2 ? x : (x.push([v.slice(2), z[i + 1]]), x), []))
    process.exit(fs.readFileSync(args['output-file'], 'utf8') === 'old-ok\\n' ? 0 : 9)
  `)
  const old = check('history', oldProgram.command, oldProgram.args)
  const current = check('new-requirement', process.execPath, ['-e', 'process.exit(0)', '--'])
  const result = await runEvolution({ plan: basePlan(seedFile, join(root, 'results'), adapter, validator, [
    { id: 'r01', task: 'establish history', checks: [old] },
    { id: 'r02', task: 'regress history', checks: [current] },
  ]) })
  assert.equal(result.status, 'failed')
  assert.equal(result.rounds[1].newRequirementStatus, 'passed')
  assert.deepEqual(result.rounds[1].regressions, ['history'])
})

test('rejects illegal plans before any adapter process starts', async () => {
  const root = await fixtureDir()
  const marker = join(root, 'started')
  const seedFile = join(root, 'seed-dir', 'seed.md')
  await mkdir(join(root, 'seed-dir'))
  await writeFile(seedFile, 'seed\n')
  const adapter = await nodeProgram(root, 'adapter.mjs', `require('node:fs').writeFileSync(${JSON.stringify(marker)}, 'started')`)
  const plan = basePlan(seedFile, join(root, 'results'), adapter, adapter, [{ id: 'r01', task: 'bad', checks: [] }])
  plan.outputDir = seedFile
  assert.throws(() => validatePlan(plan), EvolutionPlanError)
  await assert.rejects(() => runEvolution({ plan }), EvolutionPlanError)
  await assert.rejects(() => readFile(marker, 'utf8'))
})

test('rejects overlapping output directories and shell command strings', async () => {
  const root = await fixtureDir()
  const seedRoot = join(root, 'seed-dir')
  const seedFile = join(seedRoot, 'seed.md')
  await mkdir(seedRoot)
  await writeFile(seedFile, 'seed\n')
  const valid = { command: process.execPath, args: ['-e', 'process.exit(0)', '--'] }
  const overlapPlan = basePlan(seedFile, join(root, 'seed-dir', 'nested'), valid, valid, [{ id: 'r01', task: 'overlap', checks: [] }])
  overlapPlan.inputRoot = seedRoot
  assert.throws(() => validatePlan(overlapPlan), EvolutionPlanError)
  assert.throws(() => validatePlan(basePlan(seedFile, join(root, 'results'), { command: 'node -e evil', args: [] }, valid, [])), EvolutionPlanError)
})

test('turns timeout and process errors into failed round evidence with stdout and stderr preserved', async () => {
  const root = await fixtureDir()
  const seedFile = join(root, 'seed.md')
  await writeFile(seedFile, 'seed\n')
  const timeout = await nodeProgram(root, 'timeout.mjs', `setTimeout(() => {}, 1000)`)
  const error = await nodeProgram(root, 'error.mjs', `process.stdout.write('before\\n'); process.stderr.write('boom\\n'); process.exit(4)`)
  const validator = await nodeProgram(root, 'validator.mjs', `process.exit(0)`)
  const first = await runEvolution({ plan: { ...basePlan(seedFile, join(root, 'timeout-results'), timeout, validator, [{ id: 'r01', task: 'hang', checks: [] }]), timeoutMs: 30 } })
  assert.equal(first.rounds[0].process.status, 'timeout')
  const second = await runEvolution({ plan: basePlan(seedFile, join(root, 'error-results'), error, validator, [{ id: 'r01', task: 'error', checks: [] }]) })
  assert.equal(second.rounds[0].process.status, 'error')
  assert.match(second.rounds[0].process.stdout, /before/)
  assert.match(second.rounds[0].process.stderr, /boom/)
})

test('refuses to overwrite an existing evidence directory and reports unknown costs as null', async () => {
  const root = await fixtureDir()
  const seedFile = join(root, 'seed.md')
  const outputDir = join(root, 'results')
  await writeFile(seedFile, 'seed\n')
  await writeFile(outputDir, 'sentinel\n')
  const adapter = { command: process.execPath, args: ['-e', 'process.exit(0)', '--'] }
  const validator = adapter
  await assert.rejects(() => runEvolution({ plan: basePlan(seedFile, outputDir, adapter, validator, [{ id: 'r01', task: 'overwrite', checks: [] }]) }), EvolutionPlanError)
  assert.equal(await readFile(outputDir, 'utf8'), 'sentinel\n')
})

test('keeps command arguments explicit and provides deterministic output evidence', async () => {
  const root = await fixtureDir()
  const seedFile = join(root, 'seed.md')
  await writeFile(seedFile, 'seed\n')
  const outputDir = join(root, 'results')
  const adapter = await nodeProgram(root, 'adapter.mjs', `
    const fs = require('node:fs')
    const a = Object.fromEntries(process.argv.slice(2).reduce((x, v, i, z) => i % 2 ? x : (x.push([v.slice(2), z[i + 1]]), x), []))
    fs.writeFileSync(a['output-file'], 'ok\\n')
  `)
  const validator = await nodeProgram(root, 'validator.mjs', `process.exit(0)`)
  const result = await runEvolution({ plan: basePlan(seedFile, outputDir, adapter, validator, [{ id: 'r01', task: 'deterministic', checks: [] }]) })
  assert.match(result.rounds[0].inputHash, /^[a-f0-9]{64}$/)
  assert.match(result.rounds[0].outputHash, /^[a-f0-9]{64}$/)
  assert.equal(result.rounds[0].cost, null)
  assert.ok(result.rounds[0].process.argv.includes('--input-file'))
  assert.ok(result.rounds[0].process.argv.includes('--task-file'))
  assert.deepEqual(result.rounds[0].checks, [])
  assert.equal(result.rounds[0].validator.exitCode, 0)
  assert.ok((await readdir(join(outputDir, 'r01'))).includes('adapter-stdout'))
  assert.ok(here.endsWith('/scripts/evolution/'))
})

test('captures input before execution and fails when adapter or validator mutates evidence', async () => {
  const root = await fixtureDir()
  const seedFile = join(root, 'seed.md')
  await writeFile(seedFile, 'seed\n')
  const adapter = await nodeProgram(root, 'adapter.mjs', `
    const fs = require('node:fs')
    const a = Object.fromEntries(process.argv.slice(2).reduce((x, v, i, z) => i % 2 ? x : (x.push([v.slice(2), z[i + 1]]), x), []))
    fs.writeFileSync(a['input-file'], 'tampered input\\n')
    fs.writeFileSync(a['output-file'], 'ok\\n')
  `)
  const validator = await nodeProgram(root, 'validator.mjs', `
    const fs = require('node:fs')
    const a = Object.fromEntries(process.argv.slice(2).reduce((x, v, i, z) => i % 2 ? x : (x.push([v.slice(2), z[i + 1]]), x), []))
    fs.writeFileSync(a['output-file'], 'validator changed output\\n')
  `)
  const result = await runEvolution({ plan: basePlan(seedFile, join(root, 'results'), adapter, validator, [{ id: 'r01', task: 'integrity', checks: [] }]) })
  assert.equal(result.status, 'failed')
  assert.equal(result.rounds[0].inputHash, '4a6689419b00b11700c9b6246bcfa8936c8f5e1e824db3a7e57030e2d1c1a684')
  assert.equal(result.rounds[0].integrity.reason, 'adapter_modified_input')
  assert.ok(await readFile(join(root, 'results', 'r01', 'result.json'), 'utf8'))

  const cleanAdapter = await nodeProgram(root, 'clean-adapter.mjs', `
    const fs = require('node:fs')
    const a = Object.fromEntries(process.argv.slice(2).reduce((x, v, i, z) => i % 2 ? x : (x.push([v.slice(2), z[i + 1]]), x), []))
    fs.copyFileSync(a['input-file'], a['output-file'])
  `)
  const validatorOnly = await runEvolution({ plan: basePlan(seedFile, join(root, 'validator-results'), cleanAdapter, validator, [{ id: 'r01', task: 'integrity', checks: [] }]) })
  assert.equal(validatorOnly.rounds[0].integrity.reason, 'process_modified_output')

  const taskTamper = await nodeProgram(root, 'task-tamper.mjs', `
    const fs = require('node:fs')
    const a = Object.fromEntries(process.argv.slice(2).reduce((x, v, i, z) => i % 2 ? x : (x.push([v.slice(2), z[i + 1]]), x), []))
    fs.writeFileSync(a['task-file'], 'forged task\\n')
    fs.copyFileSync(a['input-file'], a['output-file'])
  `)
  const taskResult = await runEvolution({ plan: basePlan(seedFile, join(root, 'task-results'), taskTamper, validator, [{ id: 'r01', task: 'integrity', checks: [] }]) })
  assert.equal(taskResult.rounds[0].integrity.reason, 'adapter_modified_task')

  const deletingValidator = await nodeProgram(root, 'deleting-validator.mjs', `
    const fs = require('node:fs')
    const a = Object.fromEntries(process.argv.slice(2).reduce((x, v, i, z) => i % 2 ? x : (x.push([v.slice(2), z[i + 1]]), x), []))
    fs.unlinkSync(a['output-file'])
  `)
  const deleted = await runEvolution({ plan: basePlan(seedFile, join(root, 'deleted-results'), cleanAdapter, deletingValidator, [
    { id: 'r01', task: 'delete output', checks: [] },
    { id: 'r02', task: 'must not run', checks: [] },
  ]) })
  assert.equal(deleted.rounds[0].integrity.reason, 'output_missing_or_unreadable')
  assert.deepEqual(deleted.rounds[0].notRun, ['r02'])
  assert.equal(JSON.parse(await readFile(join(root, 'deleted-results', 'summary.json'), 'utf8')).status, 'failed')
})

test('new requirement status requires both new checks and the independent validator', async () => {
  const root = await fixtureDir()
  const seedFile = join(root, 'seed.md')
  await writeFile(seedFile, 'seed\n')
  const adapter = await nodeProgram(root, 'adapter.mjs', `
    const fs = require('node:fs')
    const a = Object.fromEntries(process.argv.slice(2).reduce((x, v, i, z) => i % 2 ? x : (x.push([v.slice(2), z[i + 1]]), x), []))
    fs.writeFileSync(a['output-file'], 'new requirement\\n')
  `)
  const validator = await nodeProgram(root, 'validator.mjs', `process.exit(7)`)
  const newCheck = check('new', process.execPath, ['-e', 'process.exit(0)', '--'])
  const result = await runEvolution({ plan: basePlan(seedFile, join(root, 'results'), adapter, validator, [{ id: 'r01', task: 'new', checks: [newCheck] }]) })
  assert.equal(result.rounds[0].newRequirementStatus, 'failed')
  assert.equal(result.rounds[0].verificationStatus, 'failed')
  assert.equal(result.status, 'failed')
})

test('canonicalizes output parents before overlap checks', async () => {
  const root = await fixtureDir()
  const seedRoot = join(root, 'seed-root')
  const seedFile = join(seedRoot, 'seed.md')
  await mkdir(seedRoot)
  await writeFile(seedFile, 'seed\n')
  const alias = join(root, 'alias')
  await symlink(seedRoot, alias)
  const adapter = { command: process.execPath, args: ['-e', 'process.exit(0)', '--'] }
  const plan = basePlan(seedFile, join(alias, 'nested-results'), adapter, adapter, [{ id: 'r01', task: 'overlap', checks: [] }])
  plan.inputRoot = seedRoot
  assert.throws(() => validatePlan(plan), EvolutionPlanError)
})

test('rejects late rounds that collide with runner root evidence files', async () => {
  const root = await fixtureDir()
  const seedFile = join(root, 'seed.md')
  await writeFile(seedFile, 'seed\n')
  const marker = join(root, 'started')
  const adapter = await nodeProgram(root, 'adapter.mjs', `require('node:fs').writeFileSync(${JSON.stringify(marker)}, 'started')`)
  assert.throws(() => validatePlan(basePlan(seedFile, join(root, 'results'), adapter, adapter, [
    { id: 'r01', task: 'valid', checks: [] },
    { id: 'plan.json', task: 'collision', checks: [] },
  ])), EvolutionPlanError)
  await assert.rejects(() => readFile(marker, 'utf8'))
})
