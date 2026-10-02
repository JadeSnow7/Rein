import { readdir, readFile, realpath, lstat } from 'node:fs/promises'
import { resolve, relative, isAbsolute, sep } from 'node:path'
import type { ToolCall, ToolResult } from './contracts'

export interface Workspace { readonly root: string }

function inside(root: string, candidate: string): boolean {
  const rel = relative(root, candidate)
  return rel === '' || (!isAbsolute(rel) && !rel.startsWith(`..${sep}`) && rel !== '..')
}

async function safePath(workspace: Workspace, input: string): Promise<string> {
  const root = await realpath(workspace.root)
  const candidate = resolve(root, input)
  if (!inside(root, candidate)) throw new Error('path escapes workspace')
  const actual = await realpath(candidate)
  if (!inside(root, actual)) throw new Error('path escapes workspace')
  if ((await lstat(actual)).isDirectory()) throw new Error('path is a directory')
  return actual
}

function readResult(callId: string, path: Promise<string>): Promise<ToolResult> {
  return path.then(file => readFile(file, 'utf8')).then(output => ({ toolCallId: callId, ok: true, output })).catch(error => ({
    toolCallId: callId, ok: false,
    error: { code: error instanceof Error && error.message === 'path escapes workspace' ? 'path_escape' : isNodeMissing(error) ? 'path_invalid' : 'read_failed', message: error instanceof Error ? error.message : 'read failed' },
  }))
}

export function readFileTool(workspace: Workspace, path: string, toolCallId = 'read_file'): Promise<ToolResult> {
  return readResult(toolCallId, safePath(workspace, path))
}

export async function searchFilesTool(workspace: Workspace, needle: string, toolCallId = 'search_files'): Promise<ToolResult> {
  try {
    const root = await realpath(workspace.root)
    const found: string[] = []
    async function walk(dir: string): Promise<void> {
      for (const entry of (await readdir(dir, { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
        if (entry.name === 'node_modules' || entry.name === 'target' || entry.name === '.git') continue
        const path = resolve(dir, entry.name)
        if (entry.isSymbolicLink()) continue
        if (entry.isDirectory()) await walk(path)
        else if (entry.isFile() && (await readFile(path, 'utf8')).includes(needle)) found.push(relative(root, path))
      }
    }
    await walk(root)
    return { toolCallId, ok: true, output: found.sort().join('\n') }
  } catch (error) {
    return { toolCallId, ok: false, error: { code: 'search_failed', message: error instanceof Error ? error.message : 'search failed' } }
  }
}

export async function dispatchReadonly(call: ToolCall, workspace: Workspace): Promise<ToolResult> {
  if (call.name === 'read_file') {
    if (!isObject(call.arguments) || typeof call.arguments.path !== 'string') return invalid(call.id, 'read_file requires string path')
    return readFileTool(workspace, call.arguments.path, call.id)
  }
  if (call.name === 'search_files') {
    if (!isObject(call.arguments) || typeof call.arguments.needle !== 'string') return invalid(call.id, 'search_files requires string needle')
    return searchFilesTool(workspace, call.arguments.needle, call.id)
  }
  return { toolCallId: call.id, ok: false, error: { code: 'unknown_tool', message: `unknown tool: ${call.name}` } }
}

export const executeTool = dispatchReadonly
export const execute_tool = dispatchReadonly
function isObject(value: unknown): value is Record<string, unknown> { return typeof value === 'object' && value !== null && !Array.isArray(value) }
function invalid(id: string, message: string): ToolResult { return { toolCallId: id, ok: false, error: { code: 'arguments_invalid', message } } }
function isNodeMissing(error: unknown): boolean { return typeof error === 'object' && error !== null && 'code' in error && error.code === 'ENOENT' }
