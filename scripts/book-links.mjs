import { readFile, readdir, stat } from 'node:fs/promises'
import { resolve, relative, posix } from 'node:path'

const root = resolve(import.meta.dirname, '..')
const docs = resolve(root, 'docs')
const dist = resolve(docs, '.vitepress/dist')
const failures = []
const checked = []
const cache = new Map()
const expectedSlugs = ['task-map', 'model-hello', 'model-hello-rust', 'task-spec', 'tool-roundtrip', 'provider-adapter', 'rust-migration', 'agent-loop']
const milestoneSlugs = ['evidence-qa']

const exists = async (path) => { try { await stat(path); return true } catch { return false } }
if (!await exists(dist)) {
  console.error(`missing build output: ${dist}`)
  process.exitCode = 1
  process.exit()
}

async function htmlFiles(dir) {
  const output = []
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = resolve(dir, entry.name)
    if (entry.isDirectory()) output.push(...await htmlFiles(path))
    else if (entry.isFile() && path.endsWith('.html')) output.push(path)
  }
  return output
}

const decode = (value) => { try { return decodeURIComponent(value) } catch { return value } }
const external = (href) => /^(?:https?:|mailto:|tel:|data:|javascript:|\/\/)/i.test(href)
const attrs = (html, name) => new Set([...html.matchAll(new RegExp(`\\b${name}\\s*=\\s*(["'])(.*?)\\1`, 'gis'))].map((match) => decode(match[2])))
const links = (html) => [...html.matchAll(/<a\b[^>]*\bhref\s*=\s*(["'])(.*?)\1[^>]*>/gis)].map((match) => match[2])
const config = await readFile(resolve(docs, '.vitepress/config.mts'), 'utf8')
const base = (config.match(/\bbase\s*:\s*['"]([^'"]+)['"]/)?.[1] || '/').replace(/\/+$/, '/')

function targetFor(href, source) {
  if (!href || external(href)) return null
  if (href.startsWith('#')) return { candidate: source, fragment: decode(href.slice(1)) }
  const sourceRoute = '/' + relative(dist, source).split('/').join('/')
  const baseRoot = base === '/' ? '' : base.replace(/\/$/, '')
  const url = new URL(href, `https://local${baseRoot}${sourceRoute}`)
  let route = decode(url.pathname)
  if (baseRoot && (route === baseRoot || route.startsWith(`${baseRoot}/`))) route = route.slice(baseRoot.length)
  route = '/' + route.replace(/^\/+/, '')
  if (route.endsWith('/')) route += 'index.html'
  else if (!posix.extname(route)) route += '.html'
  if (!route.endsWith('.html')) return { candidate: null, fragment: '' }
  const candidate = resolve(dist, `.${route}`)
  return { candidate: candidate.startsWith(`${dist}/`) ? candidate : null, fragment: decode(url.hash.slice(1)) }
}

async function check(href, source) {
  const target = targetFor(href, source)
  if (!target?.candidate) return true
  checked.push({ source: relative(root, source), href })
  if (!await exists(target.candidate)) {
    failures.push(`${relative(root, source)} -> ${href} (missing target)`)
    return false
  }
  if (!target.fragment) return true
  const html = cache.get(target.candidate) ?? await readFile(target.candidate, 'utf8')
  cache.set(target.candidate, html)
  const anchors = new Set([...attrs(html, 'id'), ...attrs(html, 'name')])
  if (!anchors.has(target.fragment)) {
    failures.push(`${relative(root, source)} -> ${href} (missing fragment #${target.fragment})`)
    return false
  }
  return true
}

for (const file of await htmlFiles(dist)) {
  const html = await readFile(file, 'utf8')
  cache.set(file, html)
  for (const href of links(html)) await check(href, file)
}
for (const slug of expectedSlugs) await check(`${base}chapters/${slug}.html`, resolve(dist, 'index.html'))
for (let order = 0; order <= 16; order += 1) await check(`${base}chapters/${String(order).padStart(2, '0')}.html`, resolve(dist, 'index.html'))
for (const slug of milestoneSlugs) await check(`${base}milestones/${slug}.html`, resolve(dist, 'toc.html'))
for (const href of ['/readings/00.html', '/readings/00-ts.html', '/readings/00-rust.html', '/chapters/01.html', '/chapters/01-rust.html', '/chapters/model-hello.html', '/chapters/model-hello-rust.html']) {
  await check(`${base.replace(/\/$/, '')}${href}`, resolve(dist, 'index.html'))
}

console.log(JSON.stringify({ base, html_files: (await htmlFiles(dist)).length, checked: checked.length, failures }, null, 2))
if (failures.length) process.exitCode = 1
