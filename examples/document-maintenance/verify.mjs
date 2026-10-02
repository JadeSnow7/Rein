import { lstat, readFile } from 'node:fs/promises'
import { isAbsolute, relative, resolve, sep } from 'node:path'
const argv = process.argv.slice(2); const arg = (name) => { const i = argv.indexOf(name); return i >= 0 ? argv[i + 1] : undefined }
const rule = arg('--rule'); const targetArg = arg('--target'); const out = (value, code) => { console.log(JSON.stringify(value)); process.exit(code) }
if (!rule || !targetArg) out({ ok: false, error: { code: 'arguments_invalid', message: 'rule and target are required' } }, 2)
const root = process.cwd(); const target = resolve(root, targetArg)
const safeInput = async (file) => { const rootReal = await (await import('node:fs/promises')).realpath(root); const rel = relative(rootReal, resolve(file)); if (rel === '..' || rel.startsWith(`..${sep}`) || isAbsolute(rel)) throw new Error('target escapes workspace'); let cursor = rootReal; for (const part of rel.split(sep)) { if (!part) continue; cursor = resolve(cursor, part); if ((await lstat(cursor)).isSymbolicLink()) throw new Error('symlink path component') } const stat = await lstat(resolve(file)); if (!stat.isFile() || stat.isSymbolicLink()) throw new Error('source is not a regular file'); return resolve(file) }
try {
  await safeInput(target); const text = await readFile(target, 'utf8'); let ok = false; const details = {}
  if (rule === 'command-v1' || rule === 'document-maintenance-v1') { const claims = [...text.matchAll(/npm run ([A-Za-z0-9:_-]+)/g)].map(m => m[1]); details.claims = claims; const packagePath = await safeInput(resolve(root, 'package.json')); const scripts = JSON.parse(await readFile(packagePath, 'utf8')).scripts ?? {}; ok = claims.length > 0 && claims.every(name => Object.hasOwn(scripts, name)) }
  else if (rule === 'parameter-v1') { const helpPath = await safeInput(resolve(root, 'help.txt')); const help = await readFile(helpPath, 'utf8'); const claims = [...text.matchAll(/`(--[A-Za-z0-9_-]+) value`/g)].map(m => m[1]); const rules = [...help.matchAll(/参数替换：(--\S+) -> (--\S+)/g)].map(m => [m[1], m[2]]); details.claims = claims; ok = claims.length > 0 && claims.every(claim => rules.some(([, next]) => next === claim)) }
  else if (rule === 'relative-link-v1') { const links = [...text.matchAll(/\[[^\]]+\]\(([^)#]+)\)/g)].map(m => m[1]).filter(link => !/^(?:https?:|mailto:|#)/.test(link)); details.links = links; ok = links.length > 0; for (const link of links) { const candidate = resolve(resolve(target, '..'), decodeURIComponent(link)); if (isAbsolute(link)) ok = false; else { try { await safeInput(candidate) } catch { ok = false } } } }
  else out({ ok: false, rule, error: { code: 'rule_invalid', message: 'unsupported rule' } }, 2)
  out({ ok, rule, target: targetArg, details }, ok ? 0 : 1)
} catch (error) { out({ ok: false, rule, error: { code: 'verification_error', message: error instanceof Error ? error.message : 'verification failed' } }, 1) }
