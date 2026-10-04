import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'

const root = resolve(import.meta.dirname, '..')
const registry = JSON.parse(readFileSync(resolve(root, 'book/chapters.json'), 'utf8'))
const statusLabel = (status) => ({ planned: '规划中', draft: '初稿' }[status] ?? status)
const start = '<!-- book:toc:start -->'
const end = '<!-- book:toc:end -->'
const generated = [start, '', ...registry.parts.flatMap((part) => [`## ${part.title}`, '', ...registry.chapters.filter((chapter) => chapter.part === part.id).flatMap((chapter) => [`- ${String(chapter.order).padStart(2, '0')} [${chapter.title}](${chapter.route.replace(/\.html(#.*)?$/, '.md$1')}) · ${statusLabel(chapter.status)} · ${chapter.access}`, ...(registry.milestones.filter((milestone) => milestone.after_order === chapter.order).map((milestone) => `- [阶段汇总：${milestone.title}](${milestone.path.replace(/\.html$/, '.md')})`))]), '']), end].join('\n')
const path = resolve(root, 'docs/toc.md')
const current = readFileSync(path, 'utf8')
const pattern = new RegExp(`${start.replace(/[.*+?^${}()|[\\]\\]/g, '\\$&')}[\\s\\S]*?${end.replace(/[.*+?^${}()|[\\]\\]/g, '\\$&')}`)
if (process.argv.includes('--check')) {
  const match = current.match(pattern)
  console.log(JSON.stringify({ marker: Boolean(match), synced: Boolean(match && match[0] === generated) }))
  if (!match || match[0] !== generated) process.exitCode = 1
} else if (process.argv.includes('--write')) {
  writeFileSync(path, pattern.test(current) ? current.replace(pattern, generated) : `${current.trimEnd()}\n\n${generated}\n`)
  console.log(JSON.stringify({ path: 'docs/toc.md', written: true }))
} else process.stdout.write(`${generated}\n`)
