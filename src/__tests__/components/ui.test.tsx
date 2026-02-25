import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Button } from '@/components/ui/button'

describe('UI Components', () => {
  describe('Button', () => {
    it('可以正确渲染按钮', () => {
      render(<Button>测试按钮</Button>)
      expect(screen.getByRole('button', { name: '测试按钮' })).toBeDefined()
    })

    it('支持 variant 属性', () => {
      const { container } = render(<Button variant="destructive">删除</Button>)
      expect(container.firstChild).toBeDefined()
    })

    it('支持 size 属性', () => {
      const { container } = render(<Button size="sm">小按钮</Button>)
      expect(container.firstChild).toBeDefined()
    })

    it('支持 disabled 状态', () => {
      render(<Button disabled>禁用按钮</Button>)
      const button = screen.getByRole('button', { name: '禁用按钮' })
      expect(button).toHaveProperty('disabled', true)
    })
  })
})
