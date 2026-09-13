import assert from 'node:assert/strict'
import test from 'node:test'
import { resolveAnchorHash, sourceEditions, sourcePathFor, sourceTopicForPath } from './sourceVersionState'

test('resolves the TypeScript and Rust language routes', () => {
  assert.equal(sourceTopicForPath('/readings/00-rust.html'), 'reading-00')
  assert.equal(sourceTopicForPath('/chapters/01-rust.html'), 'chapter-01')
  assert.equal(sourcePathFor('chapter-01', 'rust'), '/chapters/01-rust.html')
  assert.equal(sourcePathFor('chapter-01', 'ts'), '/chapters/01-ts.html')
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
  assert.equal(resolveAnchorHash('comparison', chapter), '#comparison')
  assert.equal(resolveAnchorHash('recording', chapter), '#recording')
  assert.equal(resolveAnchorHash('legacy', chapter), '')
})
