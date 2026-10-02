import { lstat, mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path'

const root = resolve(import.meta.dirname, '..')
const casesPath = resolve(root, 'fixtures/document-maintenance/cases.json')
const args = process.argv.slice(2)
const caseId = args.length >= 2 ? args[0] : 'command-1'
const destinationArg = args.length >= 2 ? args[1] : args[0]
if (!destinationArg || args.length > 2) usage('usage: node scripts/maintenance-fixture.mjs [case-id] <destination>')

const destination = resolve(destinationArg)
const cases = JSON.parse(await readFile(casesPath, 'utf8'))
const fixture = cases.cases.find((item) => item.id === caseId)
if (!fixture) usage(`unknown case id: ${caseId}`)
if (await exists(destination)) throw new Error(`destination already exists: ${destination}`)
await rejectSymlinkAncestors(destination)

const rule = { outdated_command: 'command-v1', changed_parameter: 'parameter-v1', broken_relative_link: 'relative-link-v1', no_change: 'command-v1' }[fixture.category]
if (!rule) throw new Error(`unsupported fixture category: ${fixture.category}`)
const plans = Object.entries(fixture.files || {}).map(([name, contents]) => {
  if (typeof contents !== 'string') throw new Error(`fixture content must be a string: ${name}`)
  return { name, contents, path: safeChild(destination, name) }
})
for (const plan of plans) await rejectSymlinkAncestors(plan.path)
// Claim the destination with a non-recursive mkdir after every input has passed validation.
// A concurrent creator therefore wins or causes this command to fail before any file write.
await mkdir(destination)
for (const { path, contents } of plans) {
  await rejectSymlinkAncestors(path)
  await mkdir(dirname(path), { recursive: true })
  await writeFile(path, contents, { flag: 'wx' })
}
console.log(JSON.stringify({ workspace: destination, caseId: fixture.id, document: fixture.document, sources: fixture.sources, rule }))

function usage(message) { console.error(message); process.exit(2) }
async function exists(path) { try { await lstat(path); return true } catch (error) { if (error.code === 'ENOENT') return false; throw error } }
function safeChild(parent, child) {
  if (isAbsolute(child) || child.split(/[\\/]/).includes('..')) throw new Error(`fixture file path escapes workspace: ${child}`)
  const path = resolve(parent, child)
  const rel = relative(parent, path)
  if (!rel || rel.startsWith(`..${sep}`) || isAbsolute(rel)) throw new Error(`fixture file path escapes workspace: ${child}`)
  return path
}
async function rejectSymlinkAncestors(path) {
  const absolute = resolve(path)
  const parts = absolute.split(sep)
  let cursor = parts.shift() || sep
  if (!cursor) cursor = sep
  for (const part of parts) {
    cursor = join(cursor, part)
    if (!await exists(cursor)) continue
    const info = await lstat(cursor)
    if (info.isSymbolicLink()) throw new Error(`symlink path component is not allowed: ${cursor}`)
  }
}
