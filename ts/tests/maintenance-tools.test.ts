import { describe, expect, it, vi } from 'vitest'
import { access, chmod, stat, mkdtemp, mkdir, readFile, symlink, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { resolve } from 'node:path'
import cases from '../../fixtures/document-maintenance/cases.json'
import { applyApprovedCandidate, createCandidate, RepairBudget, runVerification } from '../src/rein/maintenance-tools'

async function ws() { return { root: await mkdtemp(join(tmpdir(), 'rein-maint-')) } }
describe('maintenance boundary', () => {
  it('applies exact approved replacement', async () => { const w=await ws(); await writeFile(join(w.root,'a.md'),'old'); const c=await createCandidate(w,'a.md','new'); expect((await applyApprovedCandidate(w,c,c.digest)).ok).toBe(true); expect(await readFile(join(w.root,'a.md'),'utf8')).toBe('new') })
  it('rejects mutated replacement and stale baseline', async () => { const w=await ws(); await writeFile(join(w.root,'a.md'),'old'); const c=await createCandidate(w,'a.md','new'); const forged={...c,replacement:'evil'}; expect((await applyApprovedCandidate(w,forged,c.digest)).ok).toBe(false); await writeFile(join(w.root,'a.md'),'changed'); expect((await applyApprovedCandidate(w,c,c.digest)).ok).toBe(false) })
  it('rejects symlink parent and forged approval', async () => { const w=await ws(); const outside=await mkdtemp(join(tmpdir(),'outside-')); await symlink(outside,join(w.root,'link')); await expect(createCandidate(w,'link/a.md','x')).rejects.toThrow(); await writeFile(join(w.root,'a.md'),'old'); const c=await createCandidate(w,'a.md','new'); expect((await applyApprovedCandidate(w,c,'bad')).ok).toBe(false) })
  it('keeps existing temp victim unchanged', async () => { const w=await ws(); await writeFile(join(w.root,'a.md'),'old'); await writeFile(join(w.root,'.a.md.rein-victim.tmp'),'victim'); const c=await createCandidate(w,'a.md','new'); expect((await applyApprovedCandidate(w,c,c.digest)).ok).toBe(true); expect(await readFile(join(w.root,'.a.md.rein-victim.tmp'),'utf8')).toBe('victim') })
  it('runs fixed verifier against actual command claims', async () => { const w=await ws(); await writeFile(join(w.root,'package.json'),JSON.stringify({scripts:{dev:'true'}})); await mkdir(join(w.root,'examples/document-maintenance'),{recursive:true}); await writeFile(join(w.root,'README.md'),'npm run dev'); expect((await runVerification(w,'README.md','document-maintenance-v1')).ok).toBe(true); await writeFile(join(w.root,'README.md'),'npm run missing'); expect((await runVerification(w,'README.md','document-maintenance-v1')).ok).toBe(false) })
  it('limits repairs to two attempts', () => { const b=new RepairBudget(); expect(b.nextAttemptId()).toBeTruthy(); expect(b.nextAttemptId()).toBeTruthy(); expect(b.nextAttemptId()).toBeUndefined(); expect(b.remaining).toBe(0) })
  it.each(cases.cases)('verifies fixture case $id', async (fixture) => {
    const w = await ws()
    for (const [path, content] of Object.entries(fixture.files)) { const full = join(w.root, path); await mkdir(resolve(full, '..'), { recursive: true }); await writeFile(full, content) }
    await mkdir(join(w.root, 'examples/document-maintenance'), { recursive: true }); await writeFile(join(w.root, 'examples/document-maintenance/verify.mjs'), 'await import("node:fs/promises").then(fs => fs.writeFile("marker", "bad"))')
    const rule = fixture.category === 'outdated_command' || fixture.category === 'no_change' ? 'command-v1' : fixture.category === 'changed_parameter' ? 'parameter-v1' : 'relative-link-v1'
    const first = await runVerification(w, fixture.document, rule); if (fixture.expected.decision === 'change') expect(first.ok).toBe(false); else expect(first.ok).toBe(true)
    const before = await readFile(join(w.root, fixture.document))
    if (fixture.expected.decision === 'change') { const candidate = await createCandidate(w, fixture.document, fixture.expected.content); expect((await applyApprovedCandidate(w, candidate, candidate.digest)).ok).toBe(true); expect((await runVerification(w, fixture.document, rule)).ok).toBe(true) } else expect(await readFile(join(w.root, fixture.document))).toEqual(before)
    await expect(access(join(w.root, 'marker'))).rejects.toThrow()
  })
})

it('does not remove an existing file when exclusive temp creation fails', async () => {
  const w=await ws(); await writeFile(join(w.root,'a.md'),'old')
  const c=await createCandidate(w,'a.md','new')
  const now=vi.spyOn(Date,'now').mockReturnValue(123)
  const random=vi.spyOn(Math,'random').mockReturnValue(0.5)
  const victim=join(w.root,`.a.md.rein-${process.pid}-123-8.tmp`)
  try {
    await writeFile(victim,'preserve')
    expect((await applyApprovedCandidate(w,c,c.digest)).ok).toBe(false)
    expect(await readFile(victim,'utf8')).toBe('preserve')
    expect(await readFile(join(w.root,'a.md'),'utf8')).toBe('old')
  } finally { now.mockRestore(); random.mockRestore() }
})
it('preserves file permissions and stops unknown repair attempts', async () => {
  const w=await ws(); const path=join(w.root,'a.md'); await writeFile(path,'old'); await chmod(path,0o640)
  const c=await createCandidate(w,'a.md','new'); expect((await applyApprovedCandidate(w,c,c.digest)).ok).toBe(true)
  expect((await stat(path)).mode & 0o777).toBe(0o640)
  const budget=new RepairBudget(); expect(budget.nextAttemptId()).toBe('repair-1'); budget.stopOnUnknown(); expect(budget.nextAttemptId()).toBeUndefined()
})
it('does not dispatch verification when already cancelled', async () => {
  const controller=new AbortController();controller.abort()
  expect((await runVerification({root:'/missing-workspace'},'a.md','command-v1',1000,controller.signal)).error?.code).toBe('cancelled')
})
