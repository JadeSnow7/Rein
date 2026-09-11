import { describe, expect, it } from 'vitest'
import { parseArgs, providerFromBaseUrl } from '../scripts/record'

describe('record CLI 参数', () => {
  it.each(['../escape', 'OpenAI', 'with space', 'a/b'])('拒绝非法 provider %s', (provider) => {
    expect(() => parseArgs(['--scenario', 'hello', '--provider', provider])).toThrowError(
      /--provider 需为小写连字符形式/,
    )
  })

  it('接受小写数字连字符 provider', () => {
    expect(parseArgs(['--scenario', 'hello', '--provider', 'openai-2']).provider).toBe('openai-2')
  })

  it('默认 provider 从 base URL 推导为合法 slug', () => {
    expect(providerFromBaseUrl('https://api.openai.com/v1')).toBe('openai')
    expect(providerFromBaseUrl('https://api.example-2.com/v1')).toBe('example-2')
  })
})
