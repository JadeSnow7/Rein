import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawnSync } from 'node:child_process'

import { generateTypeScript, SchemaError } from './schema-to-ts.mjs'

const schema = {
  $schema: 'http://json-schema.org/draft-07/schema#',
  title: 'Envelope',
  definitions: {
    User: {
      type: 'object',
      required: ['id', 'name'],
      properties: {
        id: { type: 'integer' },
        name: { type: 'string' },
        nickname: { type: ['string', 'null'] },
      },
      additionalProperties: false,
    },
    Event: {
      oneOf: [
        {
          type: 'object',
          required: ['kind', 'message'],
          properties: {
            kind: { const: 'message' },
            message: { type: 'string' },
          },
          additionalProperties: false,
        },
        {
          type: 'object',
          required: ['kind', 'code'],
          properties: {
            kind: { const: 'error' },
            code: { type: 'integer' },
          },
          additionalProperties: false,
        },
      ],
    },
  },
  type: 'object',
  required: ['user', 'event'],
  properties: {
    user: { $ref: '#/definitions/User' },
    event: { $ref: '#/definitions/Event' },
    tags: { type: 'array', items: { type: 'string' } },
    priority: { enum: ['low', 'high'] },
  },
  additionalProperties: false,
}

test('generates deterministic typed refs, optionals, and tagged enum variants', () => {
  const first = generateTypeScript(schema)
  const second = generateTypeScript(JSON.parse(JSON.stringify(schema)))
  assert.equal(first, second)
  assert.match(first, /export type User = \{/) 
  assert.match(first, /id: number;/)
  assert.match(first, /nickname\?: string \| null;/)
  assert.match(first, /export type Event = \{[\s\S]*kind: "message";/)
  assert.match(first, /export type Envelope = \{[\s\S]*user: User;/)
  assert.match(first, /tags\?: string\[\];/)
  assert.match(first, /priority\?: "high" \| "low";/)
})

test('rejects unsupported schema shapes instead of emitting any', () => {
  assert.throws(() => generateTypeScript({ type: 'object', properties: { x: {} } }), SchemaError)
  assert.throws(() => generateTypeScript({ type: 'string', contentEncoding: 'base64' }), SchemaError)
  assert.throws(() => generateTypeScript({ type: 'array', items: { type: 'string' }, minItems: 1 }), SchemaError)
  assert.doesNotMatch(generateTypeScript(schema), /\bany\b/)
})

test('supports allOf intersections and typed additional properties', () => {
  const output = generateTypeScript({
    title: 'Config',
    definitions: { Base: { type: 'object', required: ['id'], properties: { id: { type: 'string' } }, additionalProperties: false } },
    allOf: [{ $ref: '#/definitions/Base' }, { type: 'object', properties: { labels: { type: 'object', additionalProperties: { type: 'number' } } }, additionalProperties: false }],
  })
  assert.match(output, /export type Config = Base & \{/) 
  assert.match(output, /\[key: string\]: number;/)
  assert.match(generateTypeScript({ title: 'Choice', allOf: [{ anyOf: [{ type: 'string' }, { type: 'number' }] }, { type: 'string' }] }), /\(string \| number\) & string/)
  assert.throws(() => generateTypeScript({ title: 'BadMap', type: 'object', properties: { count: { type: 'integer' } }, additionalProperties: { type: 'string' } }), /conflicts with property count/)
})

test('accepts schemars-style validation metadata while preserving TypeScript validity', async () => {
  const output = generateTypeScript({
    title: 'SchemarsShape',
    definitions: {
      Tagged: {
        type: 'object',
        minProperties: 1,
        maxProperties: 2,
        required: ['kind'],
        properties: { kind: { type: 'string', enum: ['a', 'b'] }, count: { type: 'integer', format: 'uint64', minimum: 0 } },
        additionalProperties: false,
      },
    },
    type: 'object',
    required: ['tagged'],
    properties: { tagged: { $ref: '#/definitions/Tagged' } },
    additionalProperties: false,
  })
  const dir = await mkdtemp(join(tmpdir(), 'schema-to-ts-schemars-'))
  const file = join(dir, 'generated.ts')
  await writeFile(file, output)
  const tsc = spawnSync(join(process.cwd(), 'node_modules/typescript/bin/tsc'), ['--noEmit', '--strict', '--skipLibCheck', file], { encoding: 'utf8' })
  assert.equal(tsc.status, 0, tsc.stderr)
  assert.match(output, /kind: "a" \| "b";/)
  assert.match(output, /count\?: number;/)
  const badFile = join(dir, 'bad-index-signature.ts')
  await writeFile(badFile, 'type Bad = { foo?: string; [key: string]: string; };\n')
  const badTsc = spawnSync(join(process.cwd(), 'node_modules/typescript/bin/tsc'), ['--noEmit', '--strict', '--skipLibCheck', badFile], { encoding: 'utf8' })
  assert.notEqual(badTsc.status, 0)
})

test('rejects invalid definition names', () => {
  assert.throws(() => generateTypeScript({ definitions: { 'not-valid': { type: 'string' } }, type: 'string' }), /definition name is not a TypeScript identifier/)
})

test('CLI writes generated output and check detects a tampered file', async () => {
  const dir = await mkdtemp(join(tmpdir(), 'schema-to-ts-'))
  const input = join(dir, 'schema.json')
  const output = join(dir, 'types.ts')
  await writeFile(input, JSON.stringify(schema, null, 2) + '\n')
  const script = join(process.cwd(), 'scripts/runtime/schema-to-ts.mjs')
  const generated = spawnSync(process.execPath, [script, '--input', input, '--output', output], { encoding: 'utf8' })
  assert.equal(generated.status, 0, generated.stderr)
  assert.equal(await readFile(output, 'utf8'), generateTypeScript(schema))
  const clean = spawnSync(process.execPath, [script, '--input', input, '--output', output, '--check'], { encoding: 'utf8' })
  assert.equal(clean.status, 0, clean.stderr)
  await writeFile(output, (await readFile(output, 'utf8')).replace('export type Envelope', 'export type Tampered'))
  const drift = spawnSync(process.execPath, [script, '--input', input, '--output', output, '--check'], { encoding: 'utf8' })
  assert.notEqual(drift.status, 0)
  assert.match(drift.stderr, /drift|mismatch/i)
})
