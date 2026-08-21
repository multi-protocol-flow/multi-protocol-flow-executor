import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { LeftSidebarPanel } from '@/components/sidebar/LeftSidebarPanel';
import { useUIStore, useDebugStore, useDatasourceStore } from '@/stores';
import { switchLocale } from '@/i18n';

// Mock flowStorage
vi.mock('@/services/storage', () => ({
  flowStorage: {
    listFlows: vi.fn().mockResolvedValue([]),
    loadFlow: vi.fn(),
    deleteFlow: vi.fn(),
    saveFlow: vi.fn(),
    copyFlow: vi.fn(),
    refreshFlows: vi.fn(),
    listFolders: vi.fn().mockResolvedValue([]),
    createFolder: vi.fn(),
    deleteFolder: vi.fn(),
    renameFolder: vi.fn(),
    moveFlowToFolder: vi.fn(),
  },
}));

vi.mock('@/services/tauri/datasource', () => ({
  listDatasources: vi.fn().mockResolvedValue([]),
  loadConnections: vi.fn().mockResolvedValue([]),
  saveConnections: vi.fn().mockResolvedValue(undefined),
  datasourceBrowse: vi.fn().mockResolvedValue([]),
  datasourceValidateConfig: vi.fn().mockResolvedValue({ valid: true }),
}));

describe('LeftSidebarPanel', () => {
  beforeEach(() => {
    switchLocale('zh-CN');
    useUIStore.getState().resetPanels();
    useDebugStore.getState().reset();
    useDatasourceStore.getState().reset();
  });

  it('renders data-source panel collapsed by default', async () => {
    render(<LeftSidebarPanel />);

    const datasourcePanelHeader = screen.getByText('数据源');
    expect(datasourcePanelHeader).toBeInTheDocument();

    const datasourceHeaderButton = datasourcePanelHeader.closest('[role="button"]');
    expect(datasourceHeaderButton).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('无已安装的数据源插件')).not.toBeInTheDocument();

    // 点击展开
    fireEvent.click(datasourcePanelHeader);
    expect(datasourceHeaderButton).toHaveAttribute('aria-expanded', 'true');
    await waitFor(() => {
      expect(screen.getByText('无已安装的数据源插件')).toBeInTheDocument();
    });
  });
});
