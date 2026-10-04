import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const root = resolve(import.meta.dirname, '..')
const args = process.argv.slice(2)
const registryArg = args.find((arg, index) => !arg.startsWith('--') && args[index - 1] !== '--frozen')
const registryPath = resolve(root, registryArg || 'book/chapters.json')
const registry = JSON.parse(readFileSync(registryPath, 'utf8'))
const failures = []
const warnings = []
const fail = (message) => failures.push(message)
const chapters = registry.chapters || []
const slugs = new Set()
const ranges = [[0, 3], [4, 8], [9, 15], [16, 20], [21, 24], [25, 29]]
if ((registry.parts || []).length !== 6) fail(`expected 6 parts, found ${(registry.parts || []).length}`)
if (chapters.length !== 30) fail(`expected 30 chapters, found ${chapters.length}`)
if (JSON.stringify(chapters.slice(0, 4).map((c) => c.slug)) !== JSON.stringify(['minimal-agent', 'python-model-call', 'python-file-read', 'python-suggestions'])) fail('chapters 00-03 do not use the current stable slugs')
if (chapters[4]?.slug !== 'agent-capabilities' || chapters[4]?.title !== 'Agent 的核心能力' || chapters[4]?.route !== '/chapters/agent-capabilities.html') fail('chapter 04 contract is incorrect')
for (const [index, chapter] of chapters.entries()) {
  if (chapter.order !== index) fail(`order mismatch at chapter ${index}`)
  if (!chapter.slug || slugs.has(chapter.slug)) fail(`missing or duplicate slug at chapter ${index}`)
  slugs.add(chapter.slug)
  if (chapter.previous_slug !== (index ? chapters[index - 1].slug : null)) fail(`broken previous_slug at chapter ${index}`)
  if (!['draft', 'planned'].includes(chapter.status)) fail(`invalid status at chapter ${index}`)
  if (chapter.status === 'planned' && (chapter.actual || chapter.acceptance_status !== 'not_run' || chapter.content_status !== 'planned' || chapter.implementation_status !== 'not_started')) fail(`planned chapter has completion evidence: ${chapter.slug}`)
  if (!chapter.access?.includes('永久免费') || !chapter.access?.includes('Apache-2.0')) fail(`missing permanent free license: ${chapter.slug}`)
  const expectedPart = index < 4 ? 1 : index < 9 ? 2 : index < 16 ? 3 : index < 21 ? 4 : index < 25 ? 5 : 6
  if (chapter.part !== `part-${expectedPart}`) fail(`wrong part for chapter ${chapter.slug}`)
  if (index < 4 && !chapter.route.startsWith('/chapters/')) fail(`chapter 0${index} must use a chapter route`)
  if (index >= 5 && !new RegExp(`^/roadmap/part-0${expectedPart}\\.html#ch${String(index).padStart(2, '0')}$`).test(chapter.route)) fail(`planned chapter route mismatch: ${chapter.slug}`)
  const [routePath, anchor] = chapter.route.split('#')
  const sourcePath = resolve(root, routePath.startsWith('/chapters/') ? `docs/chapters/${routePath.slice('/chapters/'.length).replace(/\.html$/, '.md')}` : `docs${routePath.replace(/\.html$/, '.md')}`)
  if (!existsSync(sourcePath)) fail(`chapter source missing for ${chapter.slug}: ${sourcePath}`)
  if (anchor) {
    const source = readFileSync(sourcePath, 'utf8')
    const anchorPattern = new RegExp(`(^|\\n)#{1,6}[^\\n]*\\b${anchor}\\b|(^|\\n)<[^>]+\\bid=["']${anchor}["']`, 'm')
    if (!anchorPattern.test(source)) fail(`chapter anchor missing for ${chapter.slug}: #${anchor}`)
  }
}
for (const [i, part] of (registry.parts || []).entries()) {
  const range = ranges[i]
  if (part.id !== `part-${i + 1}` || JSON.stringify(part.range) !== JSON.stringify(range)) fail(`part range mismatch: ${part.id}`)
}
const milestoneOrders = [3, 8, 15, 20, 24, 29]
if ((registry.milestones || []).length !== 6) fail('expected six milestones')
for (const [i, milestone] of (registry.milestones || []).entries()) {
  if (milestone.after_order !== milestoneOrders[i] || milestone.path !== `/milestones/part-0${i + 1}.html`) fail(`milestone ${i + 1} mapping mismatch`)
  const sourcePath = resolve(root, `docs${milestone.path.replace(/\.html$/, '.md')}`)
  if (!existsSync(sourcePath)) fail(`milestone source missing: ${sourcePath}`)
  else if (!new RegExp(`^# .*${milestone.title.replace(/[.*+?^${}()|[\\]\\]/g, '\\$&')}$`, 'm').test(readFileSync(sourcePath, 'utf8'))) fail(`milestone title mismatch: ${milestone.path}`)
}
const historicalPath = resolve(root, 'book/history/chapters-v2.json')
if (!existsSync(historicalPath)) fail('historical v2 registry is missing')
else {
  const historical = JSON.parse(readFileSync(historicalPath, 'utf8'))
  if (historical.chapters?.length !== 25 || historical.parts?.length !== 5) fail('historical v2 registry is not the preserved 5/25 registry')
  if (historical.chapters?.[4]?.slug !== 'provider-adapter') fail('historical v2 chapter 04 identity was not preserved')
}
const result = { chapters: chapters.length, parts: (registry.parts || []).length, milestones: (registry.milestones || []).length, historical_v2: existsSync(historicalPath), warnings, failures }
console.log(JSON.stringify(result, null, 2))
if (failures.length) process.exitCode = 1
