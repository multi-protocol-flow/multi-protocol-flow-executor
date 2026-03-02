/**
 * SettingsDialog 组件测试
 * 验证组件可以正确渲染和交互
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SettingsDialog } from '@/components/settings/SettingsDialog';
import { useUIStore } from '@/stores/ui';
import { useSettingsStore } from '@/stores/settings';

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      store = {};
    },
  };
})();

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
});

// Mock window.__TAURI__ to use mock service
Object.defineProperty(window, '__TAURI__', {
  value: {},
  writable: true,
});

describe('SettingsDialog', () => {
  beforeEach(() => {
    localStorageMock.clear();
    // 重置 UI store
    useUIStore.setState({
      showSettingsDialog: false,
    });
    // 重置 Settings store
    useSettingsStore.setState({
      reportDir: '',
      logDir: '',
      isLoading: true,
      error: null,
    });
  });

  describe('渲染测试', () => {
    it('showSettingsDialog=false 时不渲染', () => {
      useUIStore.setState({ showSettingsDialog: false });
      
      render(<SettingsDialog />);
      
      expect(screen.queryByText('全局设置')).not.toBeInTheDocument();
    });

    it('showSettingsDialog=true 时渲染对话框', async () => {
      useUIStore.setState({ showSettingsDialog: true });
      
      render(<SettingsDialog />);
      
      // 等待组件加载
      await waitFor(() => {
        expect(screen.getByText('全局设置')).toBeInTheDocument();
      });
    });

    it('显示正确的标签', async () => {
      useUIStore.setState({ showSettingsDialog: true });
      
      render(<SettingsDialog />);
      
      await waitFor(() => {
        expect(screen.getByText('报告目录')).toBeInTheDocument();
        expect(screen.getByText('日志目录')).toBeInTheDocument();
      });
    });

    it('显示正确的按钮', async () => {
      useUIStore.setState({ showSettingsDialog: true });
      
      render(<SettingsDialog />);
      
      await waitFor(() => {
        const buttons = screen.getAllByRole('button');
        // 应该有 4 个按钮：2 x 选择，2 x 打开
        expect(buttons.length).toBeGreaterThanOrEqual(4);
        
        // 检查按钮文本
        expect(screen.getAllByText('选择').length).toBe(2);
        expect(screen.getAllByText('打开').length).toBe(2);
      });
    });
  });

  describe('与 UI Store 集成', () => {
    it('关闭对话框时调用 closeSettingsDialog', async () => {
      useUIStore.setState({ showSettingsDialog: true });
      
      render(<SettingsDialog />);
      
      await waitFor(() => {
        expect(screen.getByText('全局设置')).toBeInTheDocument();
      });
      
      // 点击关闭按钮（通过 Dialog 组件的 onOpenChange）
      const closeButton = screen.getByRole('button', { name: /close/i });
      fireEvent.click(closeButton);
      
      await waitFor(() => {
        expect(useUIStore.getState().showSettingsDialog).toBe(false);
      });
    });
  });

  describe('加载状态显示', () => {
    it('isLoading=true 时显示加载中', async () => {
      useUIStore.setState({ showSettingsDialog: true });
      useSettingsStore.setState({ isLoading: true, reportDir: '', logDir: '' });
      
      render(<SettingsDialog />);
      
      await waitFor(() => {
        const inputs = screen.getAllByRole('textbox');
        const reportInput = inputs.find(input => input.id === 'report-dir');
        expect(reportInput).toHaveValue('加载中...');
      });
    });

    it('isLoading=false 时显示目录路径', async () => {
      useUIStore.setState({ showSettingsDialog: true });
      useSettingsStore.setState({
        isLoading: false,
        reportDir: '/test/reports/',
        logDir: '/test/logs/',
      });
      
      render(<SettingsDialog />);
      
      await waitFor(() => {
        const inputs = screen.getAllByRole('textbox');
        const reportInput = inputs.find(input => input.id === 'report-dir');
        const logInput = inputs.find(input => input.id === 'log-dir');
        
        expect(reportInput).toHaveValue('/test/reports/');
        expect(logInput).toHaveValue('/test/logs/');
      });
    });
  });

  describe('组件导入验证', () => {
    it('SettingsDialog 可以被成功导入', () => {
      expect(SettingsDialog).toBeDefined();
      expect(typeof SettingsDialog).toBe('function');
    });
  });
});
