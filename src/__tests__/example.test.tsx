import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'

// @vitest-environment jsdom
describe('Vitest 测试基础设施', () => {
  it('应该成功运行基础测试', () => {
    expect(1 + 1).toBe(2)
  })

  it('应该支持 jsdom 环境', () => {
    // 测试 jsdom 是否正确设置
    expect(typeof document).toBe('object')
    expect(typeof window).toBe('object')
  })

  it('应该支持 React Testing Library', () => {
    const TestComponent = () => <div>测试组件</div>
    render(<TestComponent />)
    expect(screen.getByText('测试组件')).toBeInTheDocument()
  })

  it('应该支持异步测试', async () => {
    const promise = Promise.resolve('异步结果')
    await expect(promise).resolves.toBe('异步结果')
  })
})
