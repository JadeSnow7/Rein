import assert from 'node:assert/strict'
import test from 'node:test'
import { resolveAnchorHash, sourceEditions, sourcePathFor, sourceTopicForPath } from './sourceVersionState'

test('resolves editions and safely falls back when Rust is unavailable', () => {
  assert.equal(sourceTopicForPath('/readings/00-rust.html'), 'reading-00')
  assert.equal(sourcePathFor('chapter-01', 'rust'), '/chapters/01.html')
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
