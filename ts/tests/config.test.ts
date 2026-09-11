import { describe, expect, it } from 'vitest'
import { loadConfig } from '../src/config'
import { ConfigError } from '../src/errors'

const complete = {
  REIN_BASE_URL: 'https://api.example.com/v1',
  REIN_API_KEY: 'sk-test',
  REIN_MODEL: 'test-model',
}

describe('loadConfig', () => {
  it('读出三个变量，并去掉 baseUrl 末尾的斜杠', () => {
    const config = loadConfig({ ...complete, REIN_BASE_URL: 'https://api.example.com/v1/' })
    expect(config).toEqual({
      baseUrl: 'https://api.example.com/v1',
      apiKey: 'sk-test',
      model: 'test-model',
    })
  })

  it.each(['REIN_BASE_URL', 'REIN_API_KEY', 'REIN_MODEL'])(
    '%s 缺失时报错，且错误信息里出现该变量名',
    (variable) => {
      const env = { ...complete, [variable]: undefined }
      expect(() => loadConfig(env)).toThrowError(ConfigError)
      try {
        loadConfig(env)
        expect.unreachable('应当抛出 ConfigError')
      } catch (error) {
        expect(error).toBeInstanceOf(ConfigError)
        expect((error as ConfigError).variable).toBe(variable)
        expect((error as ConfigError).message).toContain(variable)
      }
    },
  )

  it('空字符串与纯空白同样视为未配置，不静默接受', () => {
    expect(() => loadConfig({ ...complete, REIN_API_KEY: '' })).toThrowError(ConfigError)
    expect(() => loadConfig({ ...complete, REIN_MODEL: '   ' })).toThrowError(ConfigError)
  })

  it('baseUrl 不是合法 URL 时报错', () => {
    expect(() => loadConfig({ ...complete, REIN_BASE_URL: 'api.example.com/v1' })).toThrowError(
      /REIN_BASE_URL 不是合法 URL/,
    )
  })

  it('baseUrl 协议不是 http/https 时报错', () => {
    expect(() => loadConfig({ ...complete, REIN_BASE_URL: 'ftp://example.com' })).toThrowError(
      /REIN_BASE_URL 协议必须是 http 或 https/,
    )
  })

  it.each([
    ['https://user:password@example.com/v1', '不得包含用户名或密码'],
    ['https://api.example.com/v1?key=secret', '不得包含 query 或 hash'],
    ['https://api.example.com/v1#secret', '不得包含 query 或 hash'],
    ['https://api.example.com/v1?', '不得包含 query 或 hash'],
    ['https://api.example.com/v1#', '不得包含 query 或 hash'],
  ])('baseUrl 含敏感或拼接歧义组件时拒绝且不回显 URL', (baseUrl, detail) => {
    expect(() => loadConfig({ ...complete, REIN_BASE_URL: baseUrl })).toThrowError(
      new RegExp(`REIN_BASE_URL ${detail}`),
    )
    try {
      loadConfig({ ...complete, REIN_BASE_URL: baseUrl })
    } catch (error) {
      expect((error as Error).message).not.toContain(baseUrl)
      expect((error as Error).message).not.toContain('secret')
    }
  })

  it('不为任何变量提供默认值：全空环境直接失败', () => {
    expect(() => loadConfig({})).toThrowError(ConfigError)
  })
})
