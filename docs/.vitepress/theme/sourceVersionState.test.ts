import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { resolveAnchorHash, sourceEditions, sourcePathFor, sourceTopicForPath } from './sourceVersionState'

test('resolves the TypeScript and Rust language routes', () => {
  assert.equal(sourceTopicForPath('/readings/00-rust.html'), 'reading-00')
  assert.equal(sourceTopicForPath('/chapters/01-rust.html'), 'chapter-01')
  assert.equal(sourcePathFor('chapter-01', 'rust'), '/chapters/01-rust.html')
  assert.equal(sourcePathFor('chapter-01', 'ts'), '/chapters/01.html')

  assert.equal(sourceTopicForPath('/chapters/model-hello.html'), 'chapter-model-hello')
  assert.equal(sourceTopicForPath('/chapters/model-hello-rust.html'), 'chapter-model-hello')
  assert.equal(sourcePathFor('chapter-model-hello', 'rust'), '/chapters/model-hello-rust.html')
  assert.equal(sourcePathFor('chapter-model-hello', 'ts'), '/chapters/model-hello.html')
})

test('keeps only recognized common anchors and maps language-specific aliases', () => {
  const reading = sourceEditions['reading-00']
  assert.equal(resolveAnchorHash('setup', reading), '#setup')
  assert.equal(resolveAnchorHash('ownership', reading), '#types')
  assert.equal(resolveAnchorHash('modules', reading), '#source')
  assert.equal(resolveAnchorHash('exercise-03', reading), '#exercise-03')
  assert.equal(resolveAnchorHash('terminal', reading), '')
  assert.equal(resolveAnchorHash('install', reading, 'setup'), '#setup')
  assert.equal(resolveAnchorHash('detail', reading, 'terminal'), '')
  assert.equal(resolveAnchorHash(undefined, reading), '')
})

test('keeps chapter 01 common anchors available to both editions', () => {
  const chapter = sourceEditions['chapter-01']
  assert.equal(resolveAnchorHash('failures', chapter), '#failures')
  assert.equal(resolveAnchorHash('comparison', chapter), '')
  assert.equal(resolveAnchorHash('recording', chapter), '')
  assert.equal(resolveAnchorHash('legacy', chapter), '')
})

test('does not invent language pairs for core and legacy chapter routes', () => {
  assert.equal(sourceTopicForPath('/chapters/05-rust.html'), undefined)
  assert.equal(sourceTopicForPath('/chapters/06-rust.html'), undefined)
  assert.equal(sourceTopicForPath('/chapters/07-rust.html'), undefined)
  assert.equal(sourceTopicForPath('/chapters/05.html'), undefined)
  assert.equal(sourceTopicForPath('/chapters/06.html'), undefined)
  assert.equal(sourceTopicForPath('/chapters/07.html'), undefined)
})

test('supports the published model hello pair while retaining the legacy chapter route', () => {
  assert.equal(sourceTopicForPath('/chapters/model-hello.html'), 'chapter-model-hello')
  assert.equal(sourcePathFor('chapter-model-hello', 'rust'), '/chapters/model-hello-rust.html')
  assert.equal(sourceTopicForPath('/chapters/01.html'), 'chapter-01')
})

test('sidebar is registry-driven and keeps a historical entry', () => {
  const config = readFileSync(resolve(import.meta.dirname, '../config.mts'), 'utf8')
  assert.match(config, /book\.parts\.map/)
  assert.match(config, /chapter\.route/)
  assert.match(config, /旧版五部分目录与页面/)
})

test('keeps the published hello pair on shared anchors', () => {
  const chapter = sourceEditions['chapter-model-hello']
  assert.equal(resolveAnchorHash('request-response', chapter), '#request-response')
  assert.equal(resolveAnchorHash('api-key', chapter, 'live-call'), '#live-call')
  assert.equal(resolveAnchorHash('setup', chapter), '')
})
