import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { ResizablePanel } from '@/components/ui/ResizablePanel'

describe('ResizablePanel', () => {
  beforeEach(() => {
    // Mock window.innerWidth
    Object.defineProperty(window, 'innerWidth', {
      writable: true,
      configurable: true,
      value: 1920,
    })
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('渲染测试', () => {
    it('可以正确渲染左右面板内容', () => {
      render(
        <ResizablePanel
          leftPanel={<div>左侧面板</div>}
          rightPanel={<div>右侧面板</div>}
        />
      )
      
      expect(screen.getByText('左侧面板')).toBeDefined()
      expect(screen.getByText('右侧面板')).toBeDefined()
    })

    it('使用默认宽度 280px', () => {
      const { container } = render(
        <ResizablePanel
          leftPanel={<div>左侧</div>}
          rightPanel={<div>右侧</div>}
        />
      )
      
      const leftPanel = container.firstChild?.firstChild as HTMLElement
      expect(leftPanel?.style.width).toBe('280px')
    })

    it('支持自定义默认宽度', () => {
      const { container } = render(
        <ResizablePanel
          leftPanel={<div>左侧</div>}
          rightPanel={<div>右侧</div>}
          defaultWidth={300}
        />
      )
      
      const leftPanel = container.firstChild?.firstChild as HTMLElement
      expect(leftPanel?.style.width).toBe('300px')
    })

    it('支持自定义 className', () => {
      const { container } = render(
        <ResizablePanel
          leftPanel={<div>左侧</div>}
          rightPanel={<div>右侧</div>}
          className="custom-class"
        />
      )
      
      expect(container.firstChild).toHaveClass('custom-class')
    })
  })

  describe('拖拽功能测试', () => {
    it('分隔栏存在且可拖拽', () => {
      const { container } = render(
        <ResizablePanel
          leftPanel={<div>左侧</div>}
          rightPanel={<div>右侧</div>}
        />
      )
      
      // 查找分隔栏（第二个子元素）
      const divider = container.firstChild?.childNodes[1] as HTMLElement
      expect(divider).toBeDefined()
      expect(divider.className).toContain('cursor-col-resize')
    })

    it('鼠标按下分隔栏后开始拖拽', () => {
      const { container } = render(
        <ResizablePanel
          leftPanel={<div>左侧</div>}
          rightPanel={<div>右侧</div>}
        />
      )
      
      const divider = container.firstChild?.childNodes[1] as HTMLElement
      fireEvent.mouseDown(divider)
      
      // 验证容器添加了 select-none 类
      expect(container.firstChild).toHaveClass('select-none')
    })

    it('拖拽时移动鼠标更新宽度', () => {
      const { container } = render(
        <ResizablePanel
          leftPanel={<div>左侧</div>}
          rightPanel={<div>右侧</div>}
        />
      )
      
      const divider = container.firstChild?.childNodes[1] as HTMLElement
      const leftPanel = container.firstChild?.firstChild as HTMLElement
      
      // 开始拖拽
      fireEvent.mouseDown(divider)
      
      // 模拟鼠标移动到 x=350
      fireEvent.mouseMove(document, { clientX: 350 })
      
      // 验证宽度更新
      expect(leftPanel.style.width).toBe('350px')
    })

    it('鼠标释放后停止拖拽', () => {
      const { container } = render(
        <ResizablePanel
          leftPanel={<div>左侧</div>}
          rightPanel={<div>右侧</div>}
        />
      )
      
      const divider = container.firstChild?.childNodes[1] as HTMLElement
      
      // 开始拖拽
      fireEvent.mouseDown(divider)
      expect(container.firstChild).toHaveClass('select-none')
      
      // 释放鼠标
      fireEvent.mouseUp(document)
      
      // 验证移除了 select-none 类
      expect(container.firstChild).not.toHaveClass('select-none')
    })
  })

  describe('宽度限制测试', () => {
    it('宽度不能小于 minWidth (默认 180)', () => {
      const { container } = render(
        <ResizablePanel
          leftPanel={<div>左侧</div>}
          rightPanel={<div>右侧</div>}
        />
      )
      
      const divider = container.firstChild?.childNodes[1] as HTMLElement
      const leftPanel = container.firstChild?.firstChild as HTMLElement
      
      fireEvent.mouseDown(divider)
      fireEvent.mouseMove(document, { clientX: 100 })
      
      expect(leftPanel.style.width).toBe('180px')
    })

    it('宽度不能大于 maxWidth (默认 400)', () => {
      const { container } = render(
        <ResizablePanel
          leftPanel={<div>左侧</div>}
          rightPanel={<div>右侧</div>}
        />
      )
      
      const divider = container.firstChild?.childNodes[1] as HTMLElement
      const leftPanel = container.firstChild?.firstChild as HTMLElement
      
      fireEvent.mouseDown(divider)
      fireEvent.mouseMove(document, { clientX: 500 })
      
      expect(leftPanel.style.width).toBe('400px')
    })

    it('支持自定义 minWidth 和 maxWidth', () => {
      const { container } = render(
        <ResizablePanel
          leftPanel={<div>左侧</div>}
          rightPanel={<div>右侧</div>}
          minWidth={200}
          maxWidth={600}
        />
      )
      
      const divider = container.firstChild?.childNodes[1] as HTMLElement
      const leftPanel = container.firstChild?.firstChild as HTMLElement
      
      // 测试最小值
      fireEvent.mouseDown(divider)
      fireEvent.mouseMove(document, { clientX: 150 })
      expect(leftPanel.style.width).toBe('200px')
      
      // 测试最大值
      fireEvent.mouseMove(document, { clientX: 700 })
      expect(leftPanel.style.width).toBe('600px')
    })

    it('宽度在范围内时正常更新', () => {
      const { container } = render(
        <ResizablePanel
          leftPanel={<div>左侧</div>}
          rightPanel={<div>右侧</div>}
          minWidth={200}
          maxWidth={400}
        />
      )
      
      const divider = container.firstChild?.childNodes[1] as HTMLElement
      const leftPanel = container.firstChild?.firstChild as HTMLElement
      
      fireEvent.mouseDown(divider)
      fireEvent.mouseMove(document, { clientX: 300 })
      
      expect(leftPanel.style.width).toBe('300px')
    })
  })
})
