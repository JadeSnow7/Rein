import type { Config } from '../config'
import type { Transport } from '../transport'
import type { ChatAdapter, Message, ModelTurn, ToolCall, ToolDefinition } from './contracts'

export function createOpenAIAdapter(config: Config, transport: Transport): ChatAdapter {
  return {
    async complete(messages, tools = [], options): Promise<ModelTurn> {
      const response = await transport.send({ method: 'POST', url: `${config.baseUrl}/chat/completions`, headers: { 'content-type': 'application/json', authorization: `Bearer ${config.apiKey}` }, body: JSON.stringify({ model: config.model, messages: messages.map(serializeOpenAIMessage), tools: tools.map((tool) => ({ type: 'function', function: { name: tool.name, description: tool.description, parameters: tool.inputSchema } })) }) }, { timeoutMs: options?.timeoutMs ?? 30_000, signal: options?.signal })
      if (response.status < 200 || response.status >= 300) throw new Error(`OpenAI HTTP ${response.status}`)
      const root = JSON.parse(response.body) as unknown
      if (!isRecord(root) || !Array.isArray(root.choices) || !isRecord(root.choices[0]) || !isRecord(root.choices[0].message)) throw new Error('OpenAI response lacks choices[0].message')
      const raw = root.choices[0].message
      const content = raw.content === undefined || raw.content === null ? '' : typeof raw.content === 'string' ? raw.content : (() => { throw new Error('OpenAI message content must be string or null') })()
      if (raw.tool_calls !== undefined && !Array.isArray(raw.tool_calls)) throw new Error('OpenAI tool_calls must be an array')
      const toolCalls: ToolCall[] = (raw.tool_calls ?? []).map((call: any) => {
        if (!isRecord(call) || typeof call.id !== 'string' || call.id.length === 0 || !isRecord(call.function) || typeof call.function.name !== 'string' || call.function.name.length === 0) throw new Error('OpenAI tool call lacks id or name')
        if (typeof call.function.arguments !== 'string') throw new Error('OpenAI tool arguments must be JSON text')
        const args = JSON.parse(call.function.arguments) as unknown
        if (!isRecord(args)) throw new Error('OpenAI tool arguments must be an object')
        return { id: call.id, name: call.function.name, arguments: args }
      })
      return { message: { role: 'assistant', content, toolCalls }, toolCalls }
    },
  }
}

function serializeOpenAIMessage(message: Message): Record<string, unknown> {
  if (message.role === 'tool') return { role: 'tool', tool_call_id: message.toolCallId, content: message.content }
  if (message.role === 'assistant' && message.toolCalls?.length) return { role: 'assistant', content: message.content || null, tool_calls: message.toolCalls.map((call) => ({ id: call.id, type: 'function', function: { name: call.name, arguments: JSON.stringify(call.arguments) } })) }
  return { role: message.role, content: message.content }
}

export function anthropicRequest(messages: readonly Message[], model: string): string {
  const system = messages.find((message) => message.role === 'system')?.content
  const converted: unknown[] = []
  for (const message of messages.filter((m) => m.role !== 'system')) {
    if (message.role === 'tool') {
      const previous = converted.at(-1) as { role?: string; content?: unknown[] } | undefined
      if (previous?.role === 'user' && Array.isArray(previous.content) && previous.content.every(block => (block as { type?: string }).type === 'tool_result')) previous.content.push({ type: 'tool_result', tool_use_id: message.toolCallId, content: message.content })
      else converted.push({ role: 'user', content: [{ type: 'tool_result', tool_use_id: message.toolCallId, content: message.content }] })
    } else if (message.role === 'assistant' && message.toolCalls?.length) {
      const content: unknown[] = []
      if (message.content) content.push({ type: 'text', text: message.content })
      content.push(...message.toolCalls.map(call => ({ type: 'tool_use', id: call.id, name: call.name, input: call.arguments })))
      converted.push({ role: 'assistant', content })
    } else converted.push({ role: message.role === 'assistant' ? 'assistant' : 'user', content: message.content })
  }
  return JSON.stringify({ model, max_tokens: 1024, ...(system === undefined ? {} : { system }), messages: converted })
}

export function anthropicResponseToMessage(body: string): ModelTurn {
  const payload = JSON.parse(body) as unknown
  if (!isRecord(payload) || !Array.isArray(payload.content)) throw new Error('Anthropic response lacks content')
  const blocks = payload.content
  const toolCalls = blocks.filter((block): block is Record<string, unknown> => isRecord(block) && block.type === 'tool_use').map((block) => { if (typeof block.id !== 'string' || block.id.length === 0 || typeof block.name !== 'string' || block.name.length === 0 || !isRecord(block.input)) throw new Error('Anthropic tool_use lacks id/name or has invalid input'); return { id: block.id, name: block.name, arguments: block.input } })
  for (const block of blocks) {
    if (!isRecord(block) || (block.type !== 'text' && block.type !== 'tool_use')) throw new Error('Anthropic content block is invalid')
    if (block.type === 'text' && typeof block.text !== 'string') throw new Error('Anthropic text block is invalid')
    if (block.type === 'tool_use' && (typeof block.id !== 'string' || block.id.length === 0 || typeof block.name !== 'string' || block.name.length === 0 || !isRecord(block.input))) throw new Error('Anthropic tool_use lacks id/name or has invalid input')
  }
  if (!toolCalls.length && !blocks.some(block => block.type === 'text')) throw new Error('Anthropic response lacks text content')
  return { message: { role: 'assistant', content: blocks.filter((block) => block.type === 'text').map((block) => block.text ?? '').join(''), toolCalls }, toolCalls }
}

export function createReplayAdapter(responses: readonly string[]): ChatAdapter & { readonly receivedMessages: readonly (readonly Message[])[] } {
  let index = 0
  const receivedMessages: (readonly Message[])[] = []
  return { receivedMessages, async complete(messages) {
    receivedMessages.push(structuredClone(messages))
    const body = responses[index++]
    if (body === undefined) throw new Error('replay exhausted')
    return anthropicResponseToMessage(body)
  } }
}

function isRecord(value: unknown): value is Record<string, any> { return typeof value === 'object' && value !== null && !Array.isArray(value) }
