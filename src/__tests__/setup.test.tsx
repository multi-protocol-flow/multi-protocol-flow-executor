// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ReactFlowProvider } from '@xyflow/react'
import App from '../App'
import { useFlowStore, useFlowListStore, useUIStore } from '@/stores'

// 测试包装器
function TestWrapper({ children }: { children: React.ReactNode }) {
  return <ReactFlowProvider>{children}</ReactFlowProvider>
}

describe('App', () => {
  beforeEach(() => {
    // 重置所有 store
    useFlowStore.getState().reset()
    useFlowListStore.getState().reset()
    useUIStore.getState().resetPanels()
  })

  it('renders the header with Chinese title', () => {
    render(
      <TestWrapper>
        <App />
      </TestWrapper>
    )
    expect(screen.getByText('多协议流程执行器')).toBeDefined()
  })

  it('renders the header with English subtitle', () => {
    render(
      <TestWrapper>
        <App />
      </TestWrapper>
    )
    expect(screen.getByText('Flow Executor')).toBeDefined()
  })


  it('renders the sidebar with flow list', () => {
    render(
      <TestWrapper>
        <App />
      </TestWrapper>
    )
    expect(screen.getByText('流程列表')).toBeDefined()
  })

  it('renders the node library', () => {
    render(
      <TestWrapper>
        <App />
      </TestWrapper>
    )
    expect(screen.getByText('节点库')).toBeDefined()
  })
})
