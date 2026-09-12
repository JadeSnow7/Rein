import { readText, type LocalResponse } from './reading-00'

const responses: Record<string, LocalResponse> = {
  success: { status: 200, body: '{"text":"你好，Rein"}' },
  http: { status: 500, body: '{"text":"服务端错误"}' },
  json: { status: 200, body: '{' },
  shape: { status: 200, body: '{}' },
}
const response: LocalResponse = responses[process.argv[2] ?? 'success'] ?? {
  status: 200,
  body: '{"text":"你好，Rein"}',
}

try {
  console.log(await readText(response))
} catch (error: unknown) {
  if (error instanceof Error && 'kind' in error) console.error(`kind=${error.kind}`)
  else if (error instanceof Error) console.error(error.message)
  else console.error('未知错误')
  process.exitCode = 1
}
