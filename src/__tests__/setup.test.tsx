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

  it('renders the header with app title', () => {
    render(
      <TestWrapper>
        <App />
      </TestWrapper>
    )
    expect(screen.getByTestId('app-title')).toBeDefined()
  })

  it('does not render node library in sidebar', () => {
    render(
      <TestWrapper>
        <App />
      </TestWrapper>
    )
    // 节点库已移至画布底部 NodePalette，侧边栏不再显示
    expect(screen.queryByTestId('sidebar-node-library')).toBeNull()
  })
})
