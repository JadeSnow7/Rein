/**
 * 录制一次真实的请求与响应，落盘到 fixtures/responses/<provider>/<场景>-<序号>.json。
 *
 *   cd ts
 *   npx tsx --env-file=.env scripts/record.ts --scenario hello
 *   npx tsx --env-file=.env scripts/record.ts --scenario hello --provider openai --prompt "..."
 *
 * provider 缺省取 REIN_BASE_URL 的主机名首段（api.openai.com → openai）。
 * 序号自动递增，不覆盖已有文件。
 *
 * 密钥与账号标识在写盘前替换；被替换的字段名记在 redacted 里，样本读者能看出
 * 哪里动过。响应体除密钥字面量外原样保留——录制的价值在于形状的真实性。
 */

import { mkdir, readdir, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { loadConfig } from '../src/config'
import { DEFAULT_TIMEOUT_MS } from '../src/chat'
import { ReinError } from '../src/errors'
import type { Recording, TransportRequest } from '../src/transport'
import { createFetchTransport } from '../src/transport'

const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
const RESPONSES_DIR = join(REPO_ROOT, 'fixtures', 'responses')

/** 请求头里一律不入库的字段 */
const SECRET_REQUEST_HEADERS = ['authorization', 'api-key', 'x-api-key', 'cookie']
/** 响应头只保留形状相关的几项，其余（request-id、组织标识、set-cookie）丢弃 */
const KEPT_RESPONSE_HEADERS = ['content-type', 'content-encoding', 'transfer-encoding']

interface Args {
  scenario: string
  provider: string | undefined
  prompt: string
}

const SLUG_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/

export function parseArgs(argv: readonly string[]): Args {
  const flags = new Map<string, string>()
  for (let i = 0; i < argv.length; i += 2) {
    const key = argv[i]
    const value = argv[i + 1]
    if (key === undefined || !key.startsWith('--') || value === undefined) {
      throw new Error(`参数格式为 --key value，收到：${argv.slice(i).join(' ')}`)
    }
    flags.set(key.slice(2), value)
  }

  const scenario = flags.get('scenario')
  if (scenario === undefined) {
    throw new Error('缺少 --scenario。用小写连字符命名被录制的场景，例如 --scenario hello')
  }
  if (!SLUG_PATTERN.test(scenario)) {
    throw new Error(`--scenario 需为小写连字符形式，收到：${scenario}`)
  }

  return {
    scenario,
    provider: validateProvider(flags.get('provider')),
    prompt: flags.get('prompt') ?? '用一句话说明什么是 Agent Harness。',
  }
}

function validateProvider(provider: string | undefined): string | undefined {
  if (provider !== undefined && !SLUG_PATTERN.test(provider)) {
    throw new Error(`--provider 需为小写连字符形式，收到：${provider}`)
  }
  return provider
}

export function providerFromBaseUrl(baseUrl: string): string {
  const host = new URL(baseUrl).hostname.toLowerCase()
  const parts = host.split('.').filter((part) => part !== 'api' && part !== 'www')
  const provider = parts[0] ?? host
  if (!SLUG_PATTERN.test(provider)) {
    throw new Error(`REIN_BASE_URL 推导出的 provider 不是小写连字符形式`)
  }
  return provider
}

function redactRequest(request: TransportRequest, apiKey: string): {
  request: TransportRequest
  redacted: string[]
} {
  const redacted: string[] = []
  const headers: Record<string, string> = {}
  for (const [name, value] of Object.entries(request.headers)) {
    if (SECRET_REQUEST_HEADERS.includes(name.toLowerCase())) {
      headers[name] = 'REDACTED'
      redacted.push(`request.headers.${name}`)
      continue
    }
    headers[name] = scrub(value, apiKey)
  }

  const body = scrub(request.body, apiKey)
  if (body !== request.body) redacted.push('request.body')

  return { request: { ...request, headers, body }, redacted }
}

function scrub(text: string, apiKey: string): string {
  return apiKey.length > 0 ? text.split(apiKey).join('REDACTED') : text
}

async function nextIndex(dir: string, scenario: string): Promise<number> {
  let entries: string[]
  try {
    entries = await readdir(dir)
  } catch {
    return 1
  }
  const pattern = new RegExp(`^${scenario}-(\\d+)\\.json$`)
  let max = 0
  for (const entry of entries) {
    const match = pattern.exec(entry)
    if (match?.[1] !== undefined) max = Math.max(max, Number(match[1]))
  }
  return max + 1
}

export async function main(argv: readonly string[]): Promise<number> {
  let args: Args
  try {
    args = parseArgs(argv)
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error))
    return 2
  }

  let recording: Recording
  let file: string
  try {
    const config = loadConfig()
    const provider = args.provider ?? providerFromBaseUrl(config.baseUrl)

    const request: TransportRequest = {
      method: 'POST',
      url: `${config.baseUrl}/chat/completions`,
      headers: {
        'content-type': 'application/json',
        authorization: `Bearer ${config.apiKey}`,
      },
      body: JSON.stringify({
        model: config.model,
        messages: [{ role: 'user', content: args.prompt }],
      }),
    }

    const response = await createFetchTransport().send(request, {
      timeoutMs: DEFAULT_TIMEOUT_MS,
    })

    const safe = redactRequest(request, config.apiKey)
    const headers: Record<string, string> = {}
    for (const [name, value] of Object.entries(response.headers)) {
      if (KEPT_RESPONSE_HEADERS.includes(name.toLowerCase())) headers[name] = value
    }
    const droppedHeaders = Object.keys(response.headers).filter(
      (name) => !(name in headers),
    )
    const body = scrub(response.body, config.apiKey)

    recording = {
      provider,
      scenario: args.scenario,
      recordedAt: new Date().toISOString(),
      request: safe.request,
      response: { ...response, headers, body },
      redacted: [
        ...safe.redacted,
        ...droppedHeaders.map((name) => `response.headers.${name}`),
        ...(body === response.body ? [] : ['response.body']),
      ],
    }

    const dir = join(RESPONSES_DIR, provider)
    await mkdir(dir, { recursive: true })
    file = join(dir, `${args.scenario}-${await nextIndex(dir, args.scenario)}.json`)
  } catch (error) {
    if (error instanceof ReinError) {
      console.error(`录制失败（${error.kind}）：${error.message}`)
      return 1
    }
    throw error
  }

  await writeFile(file, `${JSON.stringify(recording, null, 2)}\n`, { flag: 'wx' })
  console.log(`已录制 ${recording.response.status} → ${file}`)
  if (recording.redacted.length > 0) {
    console.log(`已去除：${recording.redacted.join('、')}`)
  }
  return 0
}

if (pathToFileURL(process.argv[1] ?? '').href === import.meta.url) {
  process.exitCode = await main(process.argv.slice(2))
}
