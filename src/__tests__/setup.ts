// Vitest 测试设置文件
import '@testing-library/jest-dom'
import { configure } from '@testing-library/react'

// 配置 testing-library 行为
configure({
  // 在 jsdom 环境中支持 act()
  reactStrictMode: true,
})

// ResizeObserver polyfill for React Flow
class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}

// 确保 global 和 window 存在后再设置
if (typeof global !== 'undefined') {
  global.ResizeObserver = ResizeObserverMock as unknown as typeof ResizeObserver
}

// Mock window.matchMedia for components that check media queries
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {}, // deprecated
    removeListener: () => {}, // deprecated
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
})

// Mock window.getComputedStyle for React Flow (仅在 window 存在时)
if (typeof window !== 'undefined') {
  const originalGetComputedStyle = window.getComputedStyle
  window.getComputedStyle = (element: Element) => {
    return originalGetComputedStyle(element)
  }
}
