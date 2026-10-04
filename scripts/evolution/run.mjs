import { access, readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { runEvolution } from './runner.mjs'

const args = process.argv.slice(2)
const value = (name) => {
  const index = args.indexOf(name)
  return index >= 0 ? args[index + 1] : null
}
const planPath = value('--plan')
const reportPath = value('--report')
if (!planPath || !reportPath || args.some((arg, index) => arg.startsWith('--') && !['--plan', '--report'].includes(arg) || ['--plan', '--report'].includes(arg) && !args[index + 1])) {
  process.stderr.write('usage: node scripts/evolution/run.mjs --plan PLAN.json --report REPORT.json\n')
  process.exit(2)
}
const plan = JSON.parse(await readFile(resolve(planPath), 'utf8'))
const report = resolve(reportPath)
try { await access(report); throw new Error('report path already exists; choose a new evidence path') } catch (error) { if (error.message.includes('already exists')) throw error }
if (report === resolve(plan.outputDir) || report.startsWith(`${resolve(plan.outputDir)}/`)) throw new Error('report path overlaps plan outputDir')
if (report === resolve(plan.seedFile)) throw new Error('report path overlaps seedFile')
const result = await runEvolution({ plan })
await writeFile(report, JSON.stringify(result, null, 2) + '\n', { flag: 'wx' })
process.stdout.write(JSON.stringify({ status: result.status, report }) + '\n')
process.exit(result.status === 'passed' ? 0 : 1)
