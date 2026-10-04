/*
 * Copyright 2026 Rein contributors
 * SPDX-License-Identifier: Apache-2.0
 */

import { readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

export class SchemaError extends Error {
  constructor(message, path = '$') {
    super(`${message} at ${path}`)
    this.name = 'SchemaError'
  }
}

const IDENTIFIER = /^[A-Za-z_$][A-Za-z0-9_$]*$/
const META_KEYS = new Set(['$schema', '$id', 'title', 'description', 'default', 'examples', 'deprecated', 'readOnly', 'writeOnly'])

function fail(message, path) {
  throw new SchemaError(message, path)
}

function literal(value, path) {
  if (value === null || typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
    return JSON.stringify(value)
  }
  fail('const and enum values must be JSON scalar literals', path)
}

function refName(ref, path) {
  const prefix = ref.startsWith('#/definitions/') ? '#/definitions/' : ref.startsWith('#/$defs/') ? '#/$defs/' : null
  if (!prefix || ref.slice(prefix.length).includes('/')) fail(`unsupported reference ${ref}`, path)
  const name = ref.slice(prefix.length)
  if (!IDENTIFIER.test(name)) fail(`reference name is not a TypeScript identifier: ${name}`, path)
  return name
}

function propertyName(name) {
  return IDENTIFIER.test(name) ? name : JSON.stringify(name)
}

function assertKnownKeys(schema, allowed, path) {
  for (const key of Object.keys(schema)) {
    if (!allowed.has(key) && !META_KEYS.has(key)) fail(`unsupported schema keyword ${key}`, path)
  }
}

function renderSchema(schema, path, definitions) {
  if (!schema || typeof schema !== 'object' || Array.isArray(schema)) fail('schema must be an object', path)
  if ('$ref' in schema) {
    assertKnownKeys(schema, new Set(['$ref']), path)
    const name = refName(schema.$ref, path)
    if (!definitions.has(name)) fail(`unresolved reference ${schema.$ref}`, path)
    return name
  }
  if ('const' in schema) {
    assertKnownKeys(schema, new Set(['const', 'type']), path)
    return literal(schema.const, path)
  }
  if ('enum' in schema) {
    assertKnownKeys(schema, new Set(['enum', 'type', 'format', 'minimum', 'maximum', 'exclusiveMinimum', 'exclusiveMaximum', 'minLength', 'maxLength', 'pattern']), path)
    if (!Array.isArray(schema.enum) || schema.enum.length === 0) fail('enum must be a non-empty array', path)
    return [...new Set(schema.enum.map((value) => literal(value, path)))].sort().join(' | ')
  }
  for (const keyword of ['anyOf', 'oneOf']) {
    if (keyword in schema) {
      assertKnownKeys(schema, new Set([keyword]), path)
      if (!Array.isArray(schema[keyword]) || schema[keyword].length === 0) fail(`${keyword} must be a non-empty array`, path)
      const variants = schema[keyword].map((item, index) => renderSchema(item, `${path}.${keyword}[${index}]`, definitions))
      return variants.join(' | ')
    }
  }
  if ('allOf' in schema) {
    assertKnownKeys(schema, new Set(['allOf']), path)
    if (!Array.isArray(schema.allOf) || schema.allOf.length === 0) fail('allOf must be a non-empty array', path)
    const parts = schema.allOf.map((item, index) => renderSchema(item, `${path}.allOf[${index}]`, definitions))
    return parts.map((part) => / \| /.test(part) ? `(${part})` : part).join(' & ')
  }
  if (Array.isArray(schema.type)) {
    assertKnownKeys(schema, new Set(['type']), path)
    if (schema.type.length === 0) fail('type array must not be empty', path)
    return [...new Set(schema.type)].sort((a, b) => (a === 'null') - (b === 'null') || a.localeCompare(b)).map((type) => renderSchema({ type }, path)).join(' | ')
  }
  if (typeof schema.type !== 'string') fail('schema needs type, enum, const, reference, or combinator', path)
  switch (schema.type) {
    case 'null':
      assertKnownKeys(schema, new Set(['type']), path)
      return 'null'
    case 'string':
      assertKnownKeys(schema, new Set(['type', 'format', 'minLength', 'maxLength', 'pattern']), path)
      return 'string'
    case 'boolean':
      assertKnownKeys(schema, new Set(['type']), path)
      return 'boolean'
    case 'integer':
    case 'number':
      assertKnownKeys(schema, new Set(['type', 'format', 'minimum', 'maximum', 'exclusiveMinimum', 'exclusiveMaximum', 'multipleOf']), path)
      return 'number'
    case 'array':
      assertKnownKeys(schema, new Set(['type', 'items']), path)
      if (!schema.items || Array.isArray(schema.items)) fail('array items must be one schema object', path)
      const itemType = renderSchema(schema.items, `${path}.items`, definitions)
      return /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(itemType) || /^(string|number|boolean|null)$/.test(itemType)
        ? `${itemType}[]`
        : `Array<${itemType}>`
    case 'object':
      assertKnownKeys(schema, new Set(['type', 'properties', 'required', 'additionalProperties', 'minProperties', 'maxProperties']), path)
      if (schema.properties !== undefined && (!schema.properties || typeof schema.properties !== 'object' || Array.isArray(schema.properties))) fail('properties must be an object', path)
      if (schema.required !== undefined && (!Array.isArray(schema.required) || schema.required.some((x) => typeof x !== 'string'))) fail('required must be an array of property names', path)
      const required = new Set(schema.required ?? [])
      const properties = schema.properties ?? {}
      const fields = Object.keys(properties).sort().map((name) => {
        const type = renderSchema(properties[name], `${path}.properties.${name}`, definitions)
        return `  ${propertyName(name)}${required.has(name) ? '' : '?'}: ${type};`
      })
      if ('additionalProperties' in schema && typeof schema.additionalProperties !== 'boolean' && (!schema.additionalProperties || typeof schema.additionalProperties !== 'object')) fail('additionalProperties must be boolean or schema', path)
      if (schema.additionalProperties && typeof schema.additionalProperties === 'object') {
        const additionalType = renderSchema(schema.additionalProperties, `${path}.additionalProperties`, definitions)
        const incompatible = Object.keys(properties).find((name) => renderSchema(properties[name], `${path}.properties.${name}`, definitions) !== additionalType)
        if (incompatible) fail(`typed additionalProperties conflicts with property ${incompatible}`, path)
        fields.push(`  [key: string]: ${additionalType};`)
      }
      else if (schema.additionalProperties === true) fields.push('  [key: string]: unknown;')
      return fields.length ? `{\n${fields.join('\n')}\n}` : '{}'
    default:
      fail(`unsupported schema type ${schema.type}`, path)
  }
}

export function generateTypeScript(schema) {
  if (!schema || typeof schema !== 'object' || Array.isArray(schema)) fail('root schema must be an object', '$')
  const rawDefinitions = { ...(schema.$defs ?? {}), ...(schema.definitions ?? {}) }
  for (const name of Object.keys(rawDefinitions)) if (!IDENTIFIER.test(name)) fail(`definition name is not a TypeScript identifier: ${name}`, '$.definitions')
  const definitions = new Map(Object.keys(rawDefinitions).sort().map((name) => [name, rawDefinitions[name]]))
  const rootName = schema.title ?? 'Root'
  if (!IDENTIFIER.test(rootName)) fail(`root title is not a TypeScript identifier: ${rootName}`, '$.title')
  if (definitions.has(rootName)) fail(`root type collides with definition ${rootName}`, '$.title')
  const blocks = [...definitions].map(([name, definition]) => `export type ${name} = ${renderSchema(definition, `$.definitions.${name}`, definitions)};`)
  const { definitions: _definitions, $defs: _defs, ...rootSchema } = schema
  blocks.push(`export type ${rootName} = ${renderSchema(rootSchema, '$', definitions)};`)
  return `// Generated from JSON Schema. Do not edit.\n// SPDX-License-Identifier: Apache-2.0\n\n${blocks.join('\n\n')}\n`
}

function parseArgs(argv) {
  const args = {}
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === '--check') args.check = true
    else if (arg === '--input' || arg === '--output') {
      const value = argv[++i]
      if (!value || value.startsWith('--')) throw new Error(`${arg} requires a path`)
      args[arg.slice(2)] = value
    } else throw new Error(`unknown argument ${arg}`)
  }
  if (!args.input || !args.output) throw new Error('usage: schema-to-ts.mjs --input PATH --output PATH [--check]')
  return args
}

const invokedScript = process.argv[1]
if (invokedScript && import.meta.url === pathToFileURL(resolve(invokedScript)).href) {
  try {
    const args = parseArgs(process.argv.slice(2))
    const generated = generateTypeScript(JSON.parse(await readFile(args.input, 'utf8')))
    if (args.check) {
      const existing = await readFile(args.output, 'utf8')
      if (existing !== generated) throw new Error(`generated TypeScript drift/mismatch: ${args.output}`)
    } else {
      await writeFile(args.output, generated)
    }
  } catch (error) {
    console.error(error instanceof Error ? error.message : error)
    process.exitCode = 1
  }
}
