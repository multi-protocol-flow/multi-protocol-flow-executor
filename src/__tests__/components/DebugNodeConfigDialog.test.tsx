/**
 * @vitest-environment jsdom
 * DebugNodeConfigDialog 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DebugNodeConfigDialog } from '@/components/dialogs/DebugNodeConfigDialog';
import type { NodeData } from '@/types';

// Mock 配置组件（简化测试，不需要测试具体配置逻辑）
vi.mock('@/components/panels/EntryConfig', () => ({
  EntryConfig: ({ data, onSave }: { data: unknown; onSave: (data: unknown) => void }) => (
    <div data-testid="entry-config">
      <span>Entry Config</span>
      <button onClick={() => onSave(data)}>Mock Save</button>
    </div>
  ),
}));

vi.mock('@/components/panels/HttpConfig', () => ({
  HttpConfig: ({ data, onSave }: { data: unknown; onSave: (data: unknown) => void }) => (
    <div data-testid="http-config">
      <span>HTTP Config</span>
      <button onClick={() => onSave(data)}>Mock Save</button>
    </div>
  ),
}));

vi.mock('@/components/panels/ConditionalConfig', () => ({
  ConditionalConfig: ({ data, onSave }: { data: unknown; onSave: (data: unknown) => void }) => (
    <div data-testid="conditional-config">
      <span>Conditional Config</span>
      <button onClick={() => onSave(data)}>Mock Save</button>
    </div>
  ),
}));

vi.mock('@/components/panels/WsConnectConfig', () => ({
  WsConnectConfig: ({ data, onSave }: { data: unknown; onSave: (data: unknown) => void }) => (
    <div data-testid="ws-connect-config">
      <span>WS Connect Config</span>
      <button onClick={() => onSave(data)}>Mock Save</button>
    </div>
  ),
}));

vi.mock('@/components/panels/WsSendCollectConfig', () => ({
  WsSendCollectConfig: ({ data, onSave }: { data: unknown; onSave: (data: unknown) => void }) => (
    <div data-testid="ws-send-collect-config">
      <span>WS SendCollect Config</span>
      <button onClick={() => onSave(data)}>Mock Save</button>
    </div>
  ),
}));

vi.mock('@/components/panels/WsCloseConfig', () => ({
  WsCloseConfig: ({ data, onSave }: { data: unknown; onSave: (data: unknown) => void }) => (
    <div data-testid="ws-close-config">
      <span>WS Close Config</span>
      <button onClick={() => onSave(data)}>Mock Save</button>
    </div>
  ),
}));

// Mock toast
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe('DebugNodeConfigDialog', () => {
  const mockOnSave = vi.fn().mockResolvedValue(undefined);
  const mockOnCancel = vi.fn();

  const httpNodeData: NodeData = {
    type: 'http',
    url: 'https://example.com',
    method: 'get',
  };

  const entryNodeData: NodeData = {
    type: 'entry',
  };

  const conditionalNodeData: NodeData = {
    type: 'conditional',
    condition_type: 'simple',
    field_path: 'status',
    operator: 'equal',
    expected_value: 200,
  };

  const defaultProps = {
    open: true,
    nodeData: httpNodeData,
    nodeName: 'Test Node',
    onSave: mockOnSave,
    onCancel: mockOnCancel,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('基础渲染', () => {
    it('当 open=false 时不渲染对话框内容', () => {
      render(<DebugNodeConfigDialog {...defaultProps} open={false} />);

      expect(screen.queryByText('编辑节点配置')).toBeNull();
    });

    it('对话框打开时显示标题和节点名称', () => {
      render(<DebugNodeConfigDialog {...defaultProps} />);

      expect(screen.getByText('编辑节点配置')).toBeDefined();
      expect(screen.getByText(/Test Node/)).toBeDefined();
    });

    it('显示节点类型标签', () => {
      render(<DebugNodeConfigDialog {...defaultProps} />);

      expect(screen.getByText(/HTTP 请求/)).toBeDefined();
    });

    it('显示警告横幅提示仅调试会话生效', () => {
      render(<DebugNodeConfigDialog {...defaultProps} />);

      expect(screen.getByText(/仅调试会话生效/)).toBeDefined();
      expect(screen.getByText(/此修改仅影响当前调试会话/)).toBeDefined();
    });
  });

  describe('节点类型分发', () => {
    it('HTTP 节点渲染 HttpConfig 组件', () => {
      render(<DebugNodeConfigDialog {...defaultProps} nodeData={httpNodeData} />);

      expect(screen.getByTestId('http-config')).toBeDefined();
    });

    it('Entry 节点渲染 EntryConfig 组件', () => {
      render(
        <DebugNodeConfigDialog
          {...defaultProps}
          nodeData={entryNodeData}
        />
      );

      expect(screen.getByTestId('entry-config')).toBeDefined();
    });

    it('Conditional 节点渲染 ConditionalConfig 组件', () => {
      render(
        <DebugNodeConfigDialog
          {...defaultProps}
          nodeData={conditionalNodeData}
        />
      );

      expect(screen.getByTestId('conditional-config')).toBeDefined();
    });

    it('WS Connect 节点渲染 WsConnectConfig 组件', () => {
      const wsConnectData: NodeData = {
        type: 'ws_connect',
        url: 'wss://example.com',
      };

      render(
        <DebugNodeConfigDialog
          {...defaultProps}
          nodeData={wsConnectData}
        />
      );

      expect(screen.getByTestId('ws-connect-config')).toBeDefined();
    });

    it('WS SendCollect 节点渲染 WsSendCollectConfig 组件', () => {
      const wsSendCollectData: NodeData = {
        type: 'ws_send_collect',
        connection_id: 'conn-1',
        message: 'test',
        collect_response: true,
      };

      render(
        <DebugNodeConfigDialog
          {...defaultProps}
          nodeData={wsSendCollectData}
        />
      );

      expect(screen.getByTestId('ws-send-collect-config')).toBeDefined();
    });

    it('WS Close 节点渲染 WsCloseConfig 组件', () => {
      const wsCloseData: NodeData = {
        type: 'ws_close',
        connection_id: 'conn-1',
      };

      render(
        <DebugNodeConfigDialog
          {...defaultProps}
          nodeData={wsCloseData}
        />
      );

      expect(screen.getByTestId('ws-close-config')).toBeDefined();
    });
  });

  describe('按钮交互', () => {
    it('显示取消和保存按钮', () => {
      render(<DebugNodeConfigDialog {...defaultProps} />);

      expect(screen.getByRole('button', { name: '取消' })).toBeDefined();
      expect(screen.getByRole('button', { name: '保存修改' })).toBeDefined();
    });

    it('点击取消按钮调用 onCancel', async () => {
      const user = userEvent.setup();
      render(<DebugNodeConfigDialog {...defaultProps} />);

      await user.click(screen.getByRole('button', { name: '取消' }));
      expect(mockOnCancel).toHaveBeenCalledTimes(1);
    });

    it('点击保存按钮调用 onSave', async () => {
      const user = userEvent.setup();
      render(<DebugNodeConfigDialog {...defaultProps} />);

      await user.click(screen.getByRole('button', { name: '保存修改' }));

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledTimes(1);
        expect(mockOnSave).toHaveBeenCalledWith(httpNodeData);
      });
    });

    it('保存成功后关闭弹窗', async () => {
      const user = userEvent.setup();
      render(<DebugNodeConfigDialog {...defaultProps} />);

      await user.click(screen.getByRole('button', { name: '保存修改' }));

      await waitFor(() => {
        expect(mockOnCancel).toHaveBeenCalledTimes(1);
      });
    });

    it('保存失败时显示错误提示', async () => {
      const user = userEvent.setup();
      const errorSave = vi.fn().mockRejectedValue(new Error('保存失败'));

      render(
        <DebugNodeConfigDialog
          {...defaultProps}
          onSave={errorSave}
        />
      );

      await user.click(screen.getByRole('button', { name: '保存修改' }));

      await waitFor(() => {
        expect(errorSave).toHaveBeenCalledTimes(1);
      });

      // 不应该关闭弹窗
      expect(mockOnCancel).not.toHaveBeenCalled();
    });

    it('保存时禁用按钮并显示加载状态', async () => {
      const user = userEvent.setup();
      // 创建一个不会立即 resolve 的 Promise
      let resolvePromise: () => void;
      const slowSave = vi.fn().mockImplementation(
        () => new Promise<void>((resolve) => {
          resolvePromise = resolve;
        })
      );

      render(
        <DebugNodeConfigDialog
          {...defaultProps}
          onSave={slowSave}
        />
      );

      const saveButton = screen.getByRole('button', { name: '保存修改' });
      await user.click(saveButton);

      // 保存中状态
      expect(screen.getByRole('button', { name: '保存中...' })).toBeDefined();

      // 等待 onSave 被调用（组件内部有 150ms 延迟后才调用 onSave）
      await waitFor(() => {
        expect(slowSave).toHaveBeenCalled();
      });

      // 完成 Promise
      resolvePromise!();

      await waitFor(() => {
        expect(mockOnCancel).toHaveBeenCalled();
      });
    });

    it('nodeData 为 null 时禁用保存按钮', () => {
      render(
        <DebugNodeConfigDialog
          {...defaultProps}
          nodeData={null}
        />
      );

      const saveButton = screen.getByRole('button', { name: '保存修改' });
      expect(saveButton).toHaveProperty('disabled', true);
    });
  });

  describe('配置组件交互', () => {
    it('配置组件的 onSave 更新内部状态', async () => {
      const user = userEvent.setup();
      render(<DebugNodeConfigDialog {...defaultProps} />);

      // 点击配置组件内的 Mock Save 按钮
      await user.click(screen.getByText('Mock Save'));

      // 然后点击弹窗的保存按钮
      await user.click(screen.getByRole('button', { name: '保存修改' }));

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalled();
      });
    });
  });
});
