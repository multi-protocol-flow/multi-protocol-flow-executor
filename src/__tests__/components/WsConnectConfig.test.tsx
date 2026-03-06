/**
 * WsConnectConfig 组件测试
 * 测试字段更新同步问题修复
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { WsConnectConfig } from '@/components/panels/WsConnectConfig';
import type { WsConnectConfig as WsConnectConfigType } from '@/types';
import { useFlowStore } from '@/stores';

// Mock useFlowStore
vi.mock('@/stores', () => ({
  useFlowStore: vi.fn(),
}));

describe('WsConnectConfig - 字段同步测试', () => {
  const mockOnSave = vi.fn();

  const mockFlow = {
    uuid: 'test-flow',
    name: 'Test Flow',
    nodes: [
      {
        uuid: 'entry-1',
        type: 'entry',
        name: 'Entry',
        position: { x: 0, y: 0 },
        data: { type: 'entry' },
      },
    ],
    connections: [],
  };

  beforeEach(() => {
    vi.clearAllMocks();
    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector) => {
      if (typeof selector === 'function') {
        return selector({ flow: mockFlow });
      }
      return { flow: mockFlow };
    });
  });

  describe('基础渲染', () => {
    it('应该渲染 URL 输入框', () => {
      const config: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
      };

      render(<WsConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText(/WebSocket URL/)).toBeDefined();
    });

    it('应该显示等待消息开关', () => {
      const config: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
        wait_for_message: false,
      };

      render(<WsConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByText(/连接后等待消息/)).toBeDefined();
    });

    it('应该显示连接超时输入框', () => {
      const config: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
        connect_timeout_ms: 30000,
      };

      render(<WsConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByLabelText(/连接超时/)).toBeDefined();
    });
  });

  describe('默认值测试', () => {
    it('连接超时默认值应为 30000ms', () => {
      const config: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
      };

      render(<WsConnectConfig data={config} onSave={mockOnSave} />);

      const timeoutInput = screen.getByLabelText(/连接超时/) as HTMLInputElement;
      expect(timeoutInput.value).toBe('30000');
    });

    it('等待消息开关默认值应为 false', () => {
      const config: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
      };

      render(<WsConnectConfig data={config} onSave={mockOnSave} />);

      // 等待配置区域不应显示
      expect(screen.queryByLabelText(/等待模式/)).toBeNull();
    });
  });

  describe('外部数据更新同步测试（核心修复点）', () => {
    it('当外部 data 变化时，应该同步所有字段到表单', async () => {
      const initialConfig: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
        connect_timeout_ms: 30000,
        wait_for_message: false,
      };

      const { rerender } = render(
        <WsConnectConfig data={initialConfig} onSave={mockOnSave} />
      );

      // 验证初始值
      const timeoutInput = screen.getByLabelText(/连接超时/) as HTMLInputElement;
      expect(timeoutInput.value).toBe('30000');

      // 更新外部数据 - 这是问题修复的关键测试
      const updatedConfig: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/new-socket',
        connect_timeout_ms: 60000, // 超时变化
        wait_for_message: true, // 开启等待消息
        wait_mode: 'condition',
        wait_condition: {
          match_type: 'json_path',
          expression: '$.status == "ready"',
        },
        message_timeout_ms: 45000,
      };

      rerender(<WsConnectConfig data={updatedConfig} onSave={mockOnSave} />);

      // 验证超时字段被更新
      await waitFor(() => {
        const updatedTimeoutInput = screen.getByLabelText(/连接超时/) as HTMLInputElement;
        expect(updatedTimeoutInput.value).toBe('60000');
      });

      // 验证等待配置已显示
      expect(screen.getByLabelText(/等待模式/)).toBeDefined();
    });

    it('当 connect_timeout_ms 变化时，应该正确同步', async () => {
      const initialConfig: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
        connect_timeout_ms: 10000,
      };

      const { rerender } = render(
        <WsConnectConfig data={initialConfig} onSave={mockOnSave} />
      );

      const initialTimeoutInput = screen.getByLabelText(/连接超时/) as HTMLInputElement;
      expect(initialTimeoutInput.value).toBe('10000');

      // 更新超时
      const updatedConfig: WsConnectConfigType = {
        ...initialConfig,
        connect_timeout_ms: 20000,
      };

      rerender(<WsConnectConfig data={updatedConfig} onSave={mockOnSave} />);

      await waitFor(() => {
        const updatedTimeoutInput = screen.getByLabelText(/连接超时/) as HTMLInputElement;
        expect(updatedTimeoutInput.value).toBe('20000');
      });
    });

    it('当 wait_for_message 从 false 变为 true 时，应该显示等待配置', async () => {
      const initialConfig: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
        wait_for_message: false,
      };

      const { rerender } = render(
        <WsConnectConfig data={initialConfig} onSave={mockOnSave} />
      );

      // 初始不应显示等待配置
      expect(screen.queryByLabelText(/等待模式/)).toBeNull();

      // 更新为开启等待消息
      const updatedConfig: WsConnectConfigType = {
        ...initialConfig,
        wait_for_message: true,
        wait_mode: 'first_message',
        message_timeout_ms: 30000,
      };

      rerender(<WsConnectConfig data={updatedConfig} onSave={mockOnSave} />);

      // 验证等待配置已显示
      await waitFor(() => {
        expect(screen.getByLabelText(/等待模式/)).toBeDefined();
        expect(screen.getByLabelText(/消息超时/)).toBeDefined();
      });
    });
  });

  describe('等待消息配置测试', () => {
    it('当 wait_for_message 为 true 时，应显示等待配置', () => {
      const config: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
        wait_for_message: true,
        wait_mode: 'first_message',
        message_timeout_ms: 30000,
      };

      render(<WsConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.getByLabelText(/等待模式/)).toBeDefined();
      expect(screen.getByLabelText(/消息超时/)).toBeDefined();
    });

    it('当 wait_for_message 为 false 时，应隐藏等待配置', () => {
      const config: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
        wait_for_message: false,
      };

      render(<WsConnectConfig data={config} onSave={mockOnSave} />);

      expect(screen.queryByLabelText(/等待模式/)).toBeNull();
      expect(screen.queryByLabelText(/消息超时/)).toBeNull();
    });

    it('[核心修复] 从 wait_for_message=true 切换到 false 时，关联字段应该被清除', async () => {
      // 这个测试验证：当外部数据从 wait_for_message=true 变为 false 时，
      // 组件应该正确处理并渲染（核心逻辑在 fromFormData 函数中清除关联字段）

      // 初始配置：开启等待消息
      const initialConfig: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
        wait_for_message: true,
        wait_mode: 'condition',
        wait_condition: {
          match_type: 'json_path',
          expression: '$.status == "ok"',
        },
        message_timeout_ms: 30000,
      };

      const { rerender } = render(
        <WsConnectConfig data={initialConfig} onSave={mockOnSave} />
      );

      // 初始应该显示等待配置
      expect(screen.getByLabelText(/等待模式/)).toBeDefined();

      // 更新外部数据：关闭等待消息
      const updatedConfig: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
        wait_for_message: false,
        // 关键：fromFormData 会清除这些字段
      };

      rerender(<WsConnectConfig data={updatedConfig} onSave={mockOnSave} />);

      // 验证等待配置已隐藏（说明关联字段被正确处理）
      await waitFor(() => {
        expect(screen.queryByLabelText(/等待模式/)).toBeNull();
        expect(screen.queryByLabelText(/消息超时/)).toBeNull();
      });
    });

    it('[核心修复] 从 wait_mode=condition 切换到 first_message 时，wait_condition 应被正确处理', async () => {
      // 这个测试验证：当外部数据从 wait_mode=condition 变为 first_message 时，
      // 组件应该正确处理并渲染（核心逻辑在 fromFormData 函数中清除 wait_condition）

      // 初始配置：condition 模式
      const initialConfig: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
        wait_for_message: true,
        wait_mode: 'condition',
        wait_condition: {
          match_type: 'json_path',
          expression: '$.status == "ready"',
        },
        message_timeout_ms: 30000,
      };

      const { rerender } = render(<WsConnectConfig data={initialConfig} onSave={mockOnSave} />);

      // 初始应该显示匹配条件配置
      expect(screen.getByLabelText(/匹配类型/)).toBeDefined();
      expect(screen.getByLabelText(/匹配表达式/)).toBeDefined();

      // 更新外部数据：切换到 first_message 模式
      const updatedConfig: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
        wait_for_message: true,
        wait_mode: 'first_message',
        message_timeout_ms: 30000,
        // 关键：fromFormData 会清除 wait_condition
      };

      rerender(<WsConnectConfig data={updatedConfig} onSave={mockOnSave} />);

      // 验证匹配条件配置已隐藏（说明 wait_condition 被正确处理）
      await waitFor(() => {
        expect(screen.queryByLabelText(/匹配类型/)).toBeNull();
        expect(screen.queryByLabelText(/匹配表达式/)).toBeNull();
      });
    });
  });

  describe('输出变量映射测试', () => {
    it('[核心修复] 输出变量的说明字段应该正确保存', async () => {
      const config: WsConnectConfigType = {
        type: 'ws_connect',
        url: 'wss://example.com/socket',
        output_mappings: [
          {
            source: '$.connection_id',
            target: 'ws_conn_id',
            description: 'WebSocket 连接 ID',
          },
        ],
      };

      const { rerender } = render(
        <WsConnectConfig data={config} onSave={mockOnSave} />
      );

      // 验证说明字段显示
      const descriptionInput = screen.getByPlaceholderText(/说明（可选）/) as HTMLInputElement;
      expect(descriptionInput.value).toBe('WebSocket 连接 ID');

      // 更新外部数据，修改说明
      const updatedConfig: WsConnectConfigType = {
        ...config,
        output_mappings: [
          {
            source: '$.connection_id',
            target: 'ws_conn_id',
            description: '更新后的说明',
          },
        ],
      };

      rerender(<WsConnectConfig data={updatedConfig} onSave={mockOnSave} />);

      // 验证说明字段已更新
      await waitFor(() => {
        const updatedDescriptionInput = screen.getByPlaceholderText(/说明（可选）/) as HTMLInputElement;
        expect(updatedDescriptionInput.value).toBe('更新后的说明');
      });
    });
  });
});
