import { createHash } from 'node:crypto'
import { access, copyFile, mkdir, readFile, writeFile } from 'node:fs/promises'
import { existsSync, lstatSync, realpathSync, statSync } from 'node:fs'
import { spawn } from 'node:child_process'
import { performance } from 'node:perf_hooks'
import { dirname, isAbsolute, relative, resolve, sep, basename } from 'node:path'

export class EvolutionPlanError extends Error {}

const sha256 = (value) => createHash('sha256').update(value).digest('hex')
const pathInside = (child, parent) => {
  const rel = relative(resolve(parent), resolve(child))
  return rel === '' || (!rel.startsWith(`..${sep}`) && rel !== '..' && !isAbsolute(rel))
}

const exists = (path) => existsSync(path)

function canonicalPath(path) {
  const absolute = resolve(path)
  let current = absolute
  const missing = []
  while (!existsSync(current)) {
    missing.unshift(basename(current))
    const parent = dirname(current)
    if (parent === current) return absolute
    current = parent
  }
  return resolve(realpathSync(current), ...missing)
}

function regularFile(path) {
  try { return lstatSync(path).isFile() } catch { return false }
}

function commandInputPaths(spec) {
  return spec.args.filter((arg) => isAbsolute(arg) && existsSync(arg)).map((arg) => canonicalPath(arg))
}

function commandSpec(value, name) {
  if (!value || typeof value.command !== 'string' || !value.command || !Array.isArray(value.args) || value.args.some((arg) => typeof arg !== 'string')) {
    throw new EvolutionPlanError(`${name} must contain a command and string args array`)
  }
  if (value.command.includes(' ') || value.command.includes('\t') || value.command.includes('\n')) throw new EvolutionPlanError(`${name}.command must be one executable, not a shell string`)
}

export function validatePlan(plan) {
  if (!plan || typeof plan !== 'object') throw new EvolutionPlanError('plan must be an object')
  if (typeof plan.sequenceId !== 'string' || !plan.sequenceId) throw new EvolutionPlanError('sequenceId is required')
  if (typeof plan.seedFile !== 'string' || !isAbsolute(plan.seedFile)) throw new EvolutionPlanError('seedFile must be an absolute path')
  if (typeof plan.outputDir !== 'string' || !isAbsolute(plan.outputDir)) throw new EvolutionPlanError('outputDir must be an absolute path')
  if (!Number.isInteger(plan.timeoutMs) || plan.timeoutMs < 1) throw new EvolutionPlanError('timeoutMs must be a positive integer')
  commandSpec(plan.adapter, 'adapter')
  commandSpec(plan.validator, 'validator')
  if (!Array.isArray(plan.rounds) || plan.rounds.length === 0) throw new EvolutionPlanError('rounds must be a non-empty array')
  const seedFile = resolve(plan.seedFile)
  const outputDir = resolve(plan.outputDir)
  const inputRoot = plan.inputRoot ? resolve(plan.inputRoot) : null
  if (exists(outputDir)) throw new EvolutionPlanError('outputDir must not already exist')
  if (!exists(seedFile) || !statSync(seedFile).isFile()) throw new EvolutionPlanError('seedFile must be an existing file')
  const ids = new Set()
  const active = new Set()
  const definitions = new Map()
  const reservedRoundIds = new Set(['plan.json', 'summary.json', 'input-seed.md'])
  for (const round of plan.rounds) {
    if (!round || typeof round.id !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(round.id) || reservedRoundIds.has(round.id) || ids.has(round.id)) throw new EvolutionPlanError('round ids must be safe, unique, and non-reserved')
    ids.add(round.id)
    if (typeof round.task !== 'string' || !round.task) throw new EvolutionPlanError(`round ${round.id} task is required`)
    if (!Array.isArray(round.checks)) throw new EvolutionPlanError(`round ${round.id} checks must be an array`)
    const listed = new Set()
    for (const check of round.checks) {
      if (!check || typeof check.id !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(check.id) || listed.has(check.id)) throw new EvolutionPlanError(`round ${round.id} has invalid or duplicate check id`)
      listed.add(check.id)
      if (check.retired) {
        if (!active.has(check.id)) throw new EvolutionPlanError(`round ${round.id} retires unknown check ${check.id}`)
        if (typeof check.retirementReason !== 'string' || !check.retirementReason.trim()) throw new EvolutionPlanError(`round ${round.id} retirement reason is required for ${check.id}`)
      } else {
        commandSpec(check, `check ${check.id}`)
        const prior = definitions.get(check.id)
        if (prior && (prior.command !== check.command || JSON.stringify(prior.args) !== JSON.stringify(check.args))) throw new EvolutionPlanError(`check ${check.id} cannot silently replace its command; use a new id and retire the old check`)
        definitions.set(check.id, { command: check.command, args: [...check.args] })
        active.add(check.id)
      }
    }
    for (const checkId of [...active]) if (listed.has(checkId) && round.checks.find((check) => check.id === checkId)?.retired) active.delete(checkId)
  }
  const canonicalOutput = canonicalPath(outputDir)
  const protectedInputs = [canonicalPath(seedFile)]
  if (inputRoot) protectedInputs.push(canonicalPath(inputRoot))
  for (const spec of [plan.adapter, plan.validator, ...plan.rounds.flatMap((round) => round.checks.filter((check) => !check.retired))]) protectedInputs.push(...commandInputPaths(spec))
  if (protectedInputs.some((input) => pathInside(canonicalOutput, input) || pathInside(input, canonicalOutput))) throw new EvolutionPlanError('outputDir overlaps a declared input or tool path')
  return true
}

function runProcess(spec, args, timeoutMs, cwd) {
  return new Promise((resolveResult) => {
    const child = spawn(spec.command, [...spec.args, ...args], { cwd, shell: false })
    let stdout = ''
    let stderr = ''
    let settled = false
    let timedOut = false
    let graceTimer
    const finish = (result) => { if (!settled) { settled = true; clearTimeout(timer); clearTimeout(graceTimer); resolveResult({ ...result, stdout, stderr, argv: [spec.command, ...spec.args, ...args] }) } }
    child.stdout.on('data', (chunk) => { stdout += chunk })
    child.stderr.on('data', (chunk) => { stderr += chunk })
    child.on('error', (error) => finish({ status: 'error', error: error.message, exitCode: null }))
    child.on('close', (code, signal) => finish({ status: timedOut ? 'timeout' : code === 0 ? 'passed' : 'error', exitCode: code, signal }))
    const timer = setTimeout(() => {
      timedOut = true
      child.kill('SIGTERM')
      graceTimer = setTimeout(() => { child.kill('SIGKILL'); finish({ status: 'timeout', exitCode: null, signal: 'SIGKILL' }) }, Math.min(100, timeoutMs))
    }, timeoutMs)
  })
}

async function writeProcessEvidence(dir, prefix, processResult) {
  await writeFile(`${dir}/${prefix}-stdout`, processResult.stdout)
  await writeFile(`${dir}/${prefix}-stderr`, processResult.stderr)
}

async function writeJsonExclusive(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx' })
}

async function safeHash(path) {
  try {
    const bytes = await readFile(path)
    return { hash: sha256(bytes), bytes }
  } catch (error) {
    return { hash: null, bytes: null, error: error.code || error.message }
  }
}

export async function runEvolution({ plan }) {
  validatePlan(plan)
  await mkdir(plan.outputDir, { recursive: false })
  await writeJsonExclusive(resolve(plan.outputDir, 'plan.json'), plan)
  const seedInput = resolve(plan.outputDir, 'input-seed.md')
  await copyFile(plan.seedFile, seedInput)
  let inputFile = seedInput
  const active = new Map()
  const historical = new Set()
  const lastCheckStatus = new Map()
  const rounds = []
  let overall = 'passed'
  for (const round of plan.rounds) {
    const startedAt = performance.now()
    const roundDir = resolve(plan.outputDir, round.id)
    await mkdir(roundDir)
    const roundInput = resolve(roundDir, 'input.md')
    await copyFile(inputFile, roundInput)
    inputFile = roundInput
    const inputBefore = await safeHash(inputFile)
    const outputFile = resolve(roundDir, 'artifact')
    const taskFile = resolve(roundDir, 'task.txt')
    await writeFile(taskFile, `${round.task}\n`)
    const taskBefore = await safeHash(taskFile)
    const adapterProcess = await runProcess(plan.adapter, ['--input-file', inputFile, '--output-file', outputFile, '--task-file', taskFile, '--round-id', round.id], plan.timeoutMs, dirname(plan.seedFile))
    await writeProcessEvidence(roundDir, 'adapter', adapterProcess)
    const evidence = { id: round.id, task: round.task, inputHash: inputBefore.hash, process: adapterProcess, adapterStatus: adapterProcess.status, cost: null, elapsedMs: null, modifiedFiles: null, tokenCount: null, humanTime: null, regressions: [], checks: [] }
    const inputAfterAdapter = await safeHash(inputFile)
    const taskAfterAdapter = await safeHash(taskFile)
    const inputIntegrityFailed = inputBefore.hash !== inputAfterAdapter.hash
    const taskIntegrityFailed = taskBefore.hash !== taskAfterAdapter.hash
    evidence.inputAfterHash = inputAfterAdapter.hash
    evidence.taskHash = taskBefore.hash
    evidence.taskAfterHash = taskAfterAdapter.hash
    if (inputIntegrityFailed) evidence.integrity = { status: 'failed', reason: 'adapter_modified_input' }
    else if (taskIntegrityFailed) evidence.integrity = { status: 'failed', reason: 'adapter_modified_task' }
    if (adapterProcess.status !== 'passed' || !regularFile(outputFile) || inputIntegrityFailed || taskIntegrityFailed) {
      evidence.verificationStatus = 'undetermined'
      evidence.status = 'failed'
      overall = 'failed'
      evidence.notRun = plan.rounds.slice(plan.rounds.indexOf(round) + 1).map((item) => item.id)
      evidence.elapsedMs = Math.round(performance.now() - startedAt)
      rounds.push(evidence)
      await writeJsonExclusive(resolve(roundDir, 'result.json'), evidence)
      break
    }
    const outputAfterAdapter = await safeHash(outputFile)
    evidence.outputHash = outputAfterAdapter.hash
    evidence.modifiedFiles = evidence.inputHash === evidence.outputHash ? 0 : 1
    const requested = new Map(round.checks.map((check) => [check.id, check]))
    for (const [id, previous] of active) {
      if (!requested.has(id)) requested.set(id, previous)
    }
    const newlyAdded = new Set()
    for (const check of round.checks) {
      if (check.retired) {
        active.delete(check.id)
        evidence.checks.push({ id: check.id, status: 'retired', reason: check.retirementReason })
      } else {
        if (!active.has(check.id)) newlyAdded.add(check.id)
        active.set(check.id, check)
      }
    }
    for (const [id, check] of [...requested]) {
      if (check.retired) continue
      const result = await runProcess(check, ['--input-file', inputFile, '--output-file', outputFile, '--task-file', taskFile, '--round-id', round.id], plan.timeoutMs, dirname(plan.seedFile))
      await writeProcessEvidence(roundDir, `check-${id}`, result)
      const status = result.status === 'passed' ? 'passed' : result.status
      evidence.checks.push({ id, status, process: result, exitCode: result.exitCode, stdout: result.stdout, stderr: result.stderr })
      if (historical.has(id) && status !== 'passed') {
        if (lastCheckStatus.get(id) === 'passed') evidence.regressions.push(id)
        else (evidence.stillFailing ??= []).push(id)
      }
      if (newlyAdded.has(id) && status !== 'passed') evidence.newRequirementStatus = 'failed'
      if (newlyAdded.has(id) && status === 'passed') evidence.newRequirementStatus = evidence.newRequirementStatus === 'failed' ? 'failed' : 'passed'
      if (status !== 'passed') overall = 'failed'
      if (status === 'passed') historical.add(id)
      lastCheckStatus.set(id, status)
    }
    const validatorProcess = await runProcess(plan.validator, ['--input-file', inputFile, '--output-file', outputFile, '--task-file', taskFile, '--round-id', round.id], plan.timeoutMs, dirname(plan.seedFile))
    await writeProcessEvidence(roundDir, 'validator', validatorProcess)
    evidence.validator = validatorProcess
    const inputAfterValidation = await safeHash(inputFile)
    const outputAfterValidation = await safeHash(outputFile)
    const taskAfterValidation = await safeHash(taskFile)
    evidence.finalInputHash = inputAfterValidation.hash
    evidence.finalOutputHash = outputAfterValidation.hash
    evidence.finalTaskHash = taskAfterValidation.hash
    if (inputAfterValidation.hash !== inputBefore.hash) evidence.integrity = { status: 'failed', reason: 'process_modified_input' }
    else if (taskAfterValidation.hash !== taskBefore.hash) evidence.integrity = { status: 'failed', reason: 'process_modified_task' }
    if (!outputAfterValidation.hash) evidence.integrity = { status: 'failed', reason: 'output_missing_or_unreadable' }
    else if (outputAfterValidation.hash !== evidence.outputHash) evidence.integrity = { status: 'failed', reason: 'process_modified_output' }
    const checkFailed = evidence.checks.some((item) => item.status !== 'passed' && item.status !== 'retired')
    const newChecksPassed = [...newlyAdded].every((id) => evidence.checks.find((item) => item.id === id)?.status === 'passed')
    evidence.newRequirementStatus = validatorProcess.status === 'passed' && newChecksPassed && !evidence.integrity ? 'passed' : 'failed'
    evidence.verificationStatus = validatorProcess.status === 'passed' && !checkFailed && evidence.regressions.length === 0 && !evidence.integrity ? 'passed' : 'failed'
    if (evidence.verificationStatus !== 'passed') overall = 'failed'
    evidence.status = evidence.status === 'failed' || evidence.verificationStatus !== 'passed' ? 'failed' : 'passed'
    evidence.elapsedMs = Math.round(performance.now() - startedAt)
    if (regularFile(outputFile) && evidence.finalOutputHash === evidence.outputHash) await copyFile(outputFile, resolve(roundDir, 'output'))
    if (evidence.integrity) {
      evidence.notRun = plan.rounds.slice(plan.rounds.indexOf(round) + 1).map((item) => item.id)
      rounds.push(evidence)
      await writeJsonExclusive(resolve(roundDir, 'result.json'), evidence)
      break
    }
    rounds.push(evidence)
    await writeJsonExclusive(resolve(roundDir, 'result.json'), evidence)
    inputFile = outputFile
  }
  const summary = { sequenceId: plan.sequenceId, status: overall, rounds }
  await writeJsonExclusive(resolve(plan.outputDir, 'summary.json'), summary)
  return summary
}
