import { describe, it, expect } from 'vitest'

describe('Vitest 基础测试', () => {
  it('应该成功运行基础测试', () => {
    expect(1 + 1).toBe(2)
  })

  it('应该支持异步测试', async () => {
    const promise = Promise.resolve('异步结果')
    await expect(promise).resolves.toBe('异步结果')
  })
})
