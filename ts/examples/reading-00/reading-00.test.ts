import { describe, expect, it } from 'vitest'
import { readText, type LocalResponse } from './reading-00'

const response = (status: number, body: string): LocalResponse => ({ status, body })

describe('阅读 0 本地响应练习', () => {
  it('成功提取原文 text', async () => {
    await expect(readText(response(200, '{"text":"你好，Rein"}'))).resolves.toBe('你好，Rein')
    await expect(readText(response(200, '{"text":"  保留两端空白  "}'))).resolves.toBe('  保留两端空白  ')
    await expect(readText(response(200, '{"text":"你好，Rein","extra":true}'))).resolves.toBe('你好，Rein')
  })

  it.each([
    ['http', response(500, '{')],
    ['json', response(200, '{')],
    ['shape', response(200, '{}')],
    ['shape', response(200, '{"text":null}')],
    ['shape', response(200, '{"text":42}')],
    ['shape', response(200, '{"text":"   "}')],
    ['shape', response(200, '[]')],
    ['shape', response(200, '["text"]')],
    ['shape', response(200, '"text"')],
    ['shape', response(200, 'null')],
    ['shape', response(200, '42')],
    ['shape', response(200, '{"text":""}')],
  ])('将 %s 输入分类为对应错误', async (kind, input) => {
    await expect(readText(input)).rejects.toMatchObject({ kind })
  })
})
