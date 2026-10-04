import { access, mkdir, readFile, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { runEvolution } from './runner.mjs'

const scriptDir = dirname(fileURLToPath(import.meta.url))
const fixtureDir = resolve(scriptDir, '../../fixtures/evolution')
const rounds = Array.from({ length: 12 }, (_, index) => ({
  id: `r${String(index + 1).padStart(2, '0')}`,
  task: `Update the maintained document for requirement ${index + 1}.`,
  checks: index < 6
    ? [{ id: 'legacy-contract', command: process.execPath, args: [join(fixtureDir, 'legacy-check.cjs')] }]
    : [
        { id: 'current-contract', command: process.execPath, args: [join(fixtureDir, 'current-check.cjs')] },
      ],
}))

function plan(seedFile, outputDir, adapter) {
  return {
    sequenceId: `offline-${adapter}`,
    seedFile,
    outputDir,
    timeoutMs: 1000,
    adapter: { command: process.execPath, args: [join(fixtureDir, adapter)] },
    validator: { command: process.execPath, args: [join(fixtureDir, 'validator.cjs')] },
    rounds,
  }
}

const outputArg = process.argv[2] === '--output' ? process.argv[3] : null
if (process.argv.length > 2 && (!outputArg || process.argv.length > 4)) {
  process.stderr.write('usage: node scripts/evolution/demo.mjs [--output NEW_DIRECTORY]\n')
  process.exit(2)
}
const root = outputArg ? resolve(outputArg) : join(tmpdir(), `rein-evolution-demo-${Date.now()}-${process.pid}`)
try { await access(root); throw new Error('demo output directory already exists; choose a new directory') } catch (error) { if (error.message.includes('already exists')) throw error }
await mkdir(root)
const seedFile = join(root, 'seed.md')
await writeFile(seedFile, await readFile(join(fixtureDir, 'seed.md')))
const good = await runEvolution({ plan: plan(seedFile, join(root, 'good-results'), 'adapter-good.cjs') })
const regressing = await runEvolution({ plan: plan(seedFile, join(root, 'regressing-results'), 'adapter-regressing.cjs') })
const r07 = regressing.rounds.find((round) => round.id === 'r07')
const calibrationPassed = good.status === 'passed' && good.rounds.length === 12 && regressing.status === 'failed' && r07?.newRequirementStatus === 'passed' && r07.regressions.includes('legacy-contract')
const report = { root, calibrationPassed, good, regressing }
const reportFile = join(root, 'report.json')
await writeFile(reportFile, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' })
process.stdout.write(JSON.stringify({ reportFile, root, calibrationPassed, good: good.status, regressing: regressing.status, regressions: regressing.rounds.flatMap((round) => round.regressions) }, null, 2) + '\n')
process.exitCode = calibrationPassed ? 0 : 1
