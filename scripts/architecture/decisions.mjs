#!/usr/bin/env node

import { createHash } from 'node:crypto'
import { lstatSync, readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { resolve, relative, isAbsolute, join, sep } from 'node:path'

export class RegistryError extends Error {
  constructor(message) {
    super(message)
    this.name = 'RegistryError'
  }
}

const ID = /^[A-Z][A-Z0-9-]{2,63}$/
const SHA256 = /^[a-f0-9]{64}$/
const HUMAN_STATUS = new Set(['accepted', 'proposed', 'superseded', 'rejected'])

const sha256 = (content) => createHash('sha256').update(content).digest('hex')

function assertString(value, label, { allowEmpty = false } = {}) {
  if (typeof value !== 'string' || (!allowEmpty && value.trim() === '')) {
    throw new RegistryError(`${label} must be a non-empty string`)
  }
}

function validatePath(root, rawPath) {
  assertString(rawPath, 'prerequisite path')
  if (isAbsolute(rawPath)) throw new RegistryError(`prerequisite path must be relative: ${rawPath}`)
  const absolute = resolve(root, rawPath)
  const fromRoot = relative(root, absolute)
  if (fromRoot === '..' || fromRoot.startsWith(`..${join('','/')}`) || isAbsolute(fromRoot)) {
    throw new RegistryError(`prerequisite path escapes repository root: ${rawPath}`)
  }
  const components = relative(root, absolute).split(sep).filter(Boolean)
  let current = root
  for (const component of components) {
    current = join(current, component)
    let stat
    try { stat = lstatSync(current) } catch (error) {
      if (error.code === 'ENOENT') return { absolute, status: 'undetermined', reason: 'missing' }
      return { absolute, status: 'undetermined', reason: error.code || 'unreadable' }
    }
    if (stat.isSymbolicLink()) throw new RegistryError(`prerequisite path may not be a symbolic link: ${rawPath}`)
  }
  const stat = lstatSync(absolute)
  if (!stat.isFile()) return { absolute, status: 'undetermined', reason: 'not_regular_file' }
  return { absolute, status: 'candidate' }
}

function validateRegistry(registry) {
  if (!registry || typeof registry !== 'object' || Array.isArray(registry)) throw new RegistryError('registry must be an object')
  if (registry.version !== 1) throw new RegistryError('registry version must be 1')
  if (!Array.isArray(registry.decisions)) throw new RegistryError('registry decisions must be an array')
  const ids = new Set()
  for (const decision of registry.decisions) {
    if (!decision || typeof decision !== 'object' || Array.isArray(decision)) throw new RegistryError('each decision must be an object')
    assertString(decision.id, 'decision id')
    if (!ID.test(decision.id) || ids.has(decision.id)) throw new RegistryError(`invalid or duplicate decision id: ${decision.id}`)
    ids.add(decision.id)
    if (!HUMAN_STATUS.has(decision.status)) throw new RegistryError(`invalid human status for ${decision.id}`)
    for (const field of ['title', 'rationale', 'scope']) assertString(decision[field], `${decision.id}.${field}`)
    for (const field of ['alternatives', 'reviewWhen', 'sources']) {
      if (!Array.isArray(decision[field]) || decision[field].some((item) => typeof item !== 'string')) {
        throw new RegistryError(`${decision.id}.${field} must be an array of strings`)
      }
    }
    if (!Array.isArray(decision.prerequisites)) throw new RegistryError(`${decision.id}.prerequisites must be an array`)
    for (const prerequisite of decision.prerequisites) {
      if (!prerequisite || typeof prerequisite !== 'object' || Array.isArray(prerequisite)) throw new RegistryError(`${decision.id} has an invalid prerequisite`)
      assertString(prerequisite.path, `${decision.id} prerequisite path`)
      if (typeof prerequisite.sha256 !== 'string' || !SHA256.test(prerequisite.sha256)) {
        throw new RegistryError(`${decision.id} prerequisite SHA-256 is invalid`)
      }
    }
  }
}

function checkPrerequisite(root, prerequisite) {
  const path = validatePath(root, prerequisite.path)
  if (path.status !== 'candidate') return { path: prerequisite.path, status: path.status, reason: path.reason }
  try {
    const actual = sha256(readFileSync(path.absolute))
    if (actual === prerequisite.sha256) return { path: prerequisite.path, status: 'applicable', expectedSha256: prerequisite.sha256, actualSha256: actual }
    return { path: prerequisite.path, status: 'review_required', reason: 'hash_mismatch', expectedSha256: prerequisite.sha256, actualSha256: actual }
  } catch (error) {
    return { path: prerequisite.path, status: 'undetermined', reason: error.code || 'unreadable' }
  }
}

export function evaluateRegistry({ root, registry }) {
  assertString(root, 'root')
  const repositoryRoot = resolve(root)
  validateRegistry(registry)
  const decisions = registry.decisions.map((decision) => {
    const prerequisites = decision.prerequisites.map((item) => checkPrerequisite(repositoryRoot, item))
    let status = 'applicable'
    let reason
    if (prerequisites.length === 0) {
      status = 'undetermined'
      reason = 'no declared prerequisites'
    } else if (prerequisites.some((item) => item.status === 'review_required')) {
      status = 'review_required'
    } else if (prerequisites.some((item) => item.status === 'undetermined')) {
      status = 'undetermined'
    }
    return { id: decision.id, title: decision.title, status, humanStatus: decision.status, prerequisites, ...(reason ? { reason } : {}) }
  })
  const accepted = decisions.filter((item) => item.humanStatus === 'accepted')
  const exitCode = accepted.length > 0 && accepted.every((item) => item.status === 'applicable') ? 0 : 1
  return { version: 1, root: repositoryRoot, exitCode, decisions }
}

function parseArgs(argv) {
  const args = {}
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index]
    if (arg === '--root' || arg === '--registry') {
      const value = argv[++index]
      if (!value || value.startsWith('--')) throw new RegistryError(`${arg} requires a value`)
      args[arg.slice(2)] = value
    } else {
      throw new RegistryError(`unknown argument: ${arg}`)
    }
  }
  if (!args.root || !args.registry) throw new RegistryError('usage: decisions.mjs --root <repo> --registry <json>')
  return args
}

export function main(argv) {
  try {
    const args = parseArgs(argv)
    const registry = JSON.parse(readFileSync(resolve(args.registry), 'utf8'))
    const result = evaluateRegistry({ root: args.root, registry })
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`)
    return result.exitCode
  } catch (error) {
    process.stderr.write(`${error.name || 'Error'}: ${error.message}\n`)
    return 2
  }
}

if (process.argv[1] && process.argv[1].endsWith('/decisions.mjs')) {
  process.exitCode = main(process.argv.slice(2))
}
