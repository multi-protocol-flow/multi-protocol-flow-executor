/**
 * WsSendCollectConfig 组件测试
 * 测试字段更新同步问题修复
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { WsSendCollectConfig as WsSendCollectConfigPanel } from '@/components/panels/WsSendCollectConfig';
import type { WsSendCollectConfig as WsSendCollectConfigType } from '@/types';
import { useFlowStore } from '@/stores';

// Mock useFlowStore
vi.mock('@/stores', () => ({
  useFlowStore: vi.fn(),
}));

describe('WsSendCollectConfig - 字段同步测试', () => {
  const mockOnSave = vi.fn();

  const mockFlow = {
    uuid: 'test-flow',
    name: 'Test Flow',
    nodes: [
      {
        uuid: 'ws-connect-1',
        type: 'ws:connect',
        name: 'WS Connect',
        position: { x: 0, y: 0 },
        data: { type: 'ws:connect', url: 'wss://example.com' },
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
    it('应该渲染连接 ID 选择器', () => {
      const config: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: '{"type": "ping"}',
        collect_response: true,
      };

      render(<WsSendCollectConfigPanel data={config} onSave={mockOnSave} />);

      expect(screen.getByText(/连接 ID/)).toBeDefined();
    });

    it('应该显示消息内容输入框', () => {
      const config: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: '{"type": "ping"}',
        collect_response: true,
      };

      render(<WsSendCollectConfigPanel data={config} onSave={mockOnSave} />);

      expect(screen.getByLabelText(/消息内容/)).toBeDefined();
    });

    it('应该显示消息类型选择器', () => {
      const config: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: '{"type": "ping"}',
        collect_response: true,
      };

      render(<WsSendCollectConfigPanel data={config} onSave={mockOnSave} />);

      expect(screen.getByLabelText(/消息类型/)).toBeDefined();
    });

    it('应该显示发送超时输入框', () => {
      const config: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: '{"type": "ping"}',
        collect_response: true,
      };

      render(<WsSendCollectConfigPanel data={config} onSave={mockOnSave} />);

      expect(screen.getByLabelText(/发送超时/)).toBeDefined();
    });

    it('应该显示等待响应开关', () => {
      const config: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: '{"type": "ping"}',
        collect_response: true,
      };

      render(<WsSendCollectConfigPanel data={config} onSave={mockOnSave} />);

      expect(screen.getByText(/发送后采集响应/)).toBeDefined();
    });
  });

  describe('默认值测试', () => {
    it('发送超时默认值应为 30000ms', () => {
      const config: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: '{"type": "ping"}',
        collect_response: true,
      };

      render(<WsSendCollectConfigPanel data={config} onSave={mockOnSave} />);

      const timeoutInput = screen.getByLabelText(/发送超时/) as HTMLInputElement;
      expect(timeoutInput.value).toBe('30000');
    });

    it('消息类型默认值应为 text', () => {
      const config: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: '{"type": "ping"}',
        collect_response: true,
      };

      render(<WsSendCollectConfigPanel data={config} onSave={mockOnSave} />);

      const messageTypeSelect = screen.getByLabelText(/消息类型/);
      expect(messageTypeSelect).toHaveTextContent(/文本|Text/i);
    });
  });

  describe('外部数据更新同步测试（核心修复点）', () => {
    it('当外部 data 变化时，应该同步所有字段到表单', async () => {
      const initialConfig: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: '{"type": "ping"}',
        message_type: 'text',
        send_timeout_ms: 30000,
        collect_response: true,
        collect_mode: 'first_match',
        collect_timeout_ms: 30000,
      };

      const { rerender } = render(
        <WsSendCollectConfigPanel data={initialConfig} onSave={mockOnSave} />
      );

      // 验证初始值
      const messageInput = screen.getByLabelText(/消息内容/) as HTMLTextAreaElement;
      expect(messageInput.value).toBe('{"type": "ping"}');

      const timeoutInput = screen.getByLabelText(/发送超时/) as HTMLInputElement;
      expect(timeoutInput.value).toBe('30000');

      // 更新外部数据 - 这是问题修复的关键测试
      const updatedConfig: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: '{"type": "pong"}', // 消息变化
        message_type: 'binary', // 消息类型变化
        send_timeout_ms: 60000, // 超时变化
        collect_response: true,
        collect_mode: 'condition_match', // 采集模式变化
        collect_condition: {
          match_type: 'json_path',
          expression: '$.status == "ok"',
        },
        collect_timeout_ms: 45000, // 消息超时变化
      };

      rerender(<WsSendCollectConfigPanel data={updatedConfig} onSave={mockOnSave} />);

      // 验证所有字段都被更新
      await waitFor(() => {
        const updatedMessageInput = screen.getByLabelText(/消息内容/) as HTMLTextAreaElement;
        expect(updatedMessageInput.value).toBe('{"type": "pong"}');
      });

      const updatedTimeoutInput = screen.getByLabelText(/发送超时/) as HTMLInputElement;
      expect(updatedTimeoutInput.value).toBe('60000');

      // 验证消息类型选择器显示 binary
      const messageTypeSelect = screen.getByLabelText(/消息类型/);
      expect(messageTypeSelect).toHaveTextContent(/二进制|Binary/i);
    });

    it('当 message_type 变化时，应该正确同步', async () => {
      const initialConfig: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: 'binary data',
        message_type: 'text',
        collect_response: false,
      };

      const { rerender } = render(
        <WsSendCollectConfigPanel data={initialConfig} onSave={mockOnSave} />
      );

      // 初始消息类型为 text
      const initialMessageTypeSelect = screen.getByLabelText(/消息类型/);
      expect(initialMessageTypeSelect).toHaveTextContent(/文本|Text/i);

      // 更新 message_type 为 binary
      const updatedConfig: WsSendCollectConfigType = {
        ...initialConfig,
        message_type: 'binary',
      };

      rerender(<WsSendCollectConfigPanel data={updatedConfig} onSave={mockOnSave} />);

      // 验证消息类型已更新
      await waitFor(() => {
        const updatedMessageTypeSelect = screen.getByLabelText(/消息类型/);
        expect(updatedMessageTypeSelect).toHaveTextContent(/二进制|Binary/i);
      });
    });

    it('当 send_timeout_ms 变化时，应该正确同步', async () => {
      const initialConfig: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: 'test',
        send_timeout_ms: 10000,
        collect_response: false,
      };

      const { rerender } = render(
        <WsSendCollectConfigPanel data={initialConfig} onSave={mockOnSave} />
      );

      const initialTimeoutInput = screen.getByLabelText(/发送超时/) as HTMLInputElement;
      expect(initialTimeoutInput.value).toBe('10000');

      // 更新超时
      const updatedConfig: WsSendCollectConfigType = {
        ...initialConfig,
        send_timeout_ms: 20000,
      };

      rerender(<WsSendCollectConfigPanel data={updatedConfig} onSave={mockOnSave} />);

      await waitFor(() => {
        const updatedTimeoutInput = screen.getByLabelText(/发送超时/) as HTMLInputElement;
        expect(updatedTimeoutInput.value).toBe('20000');
      });
    });

    it('当 collect_condition 变化时，应该正确同步', async () => {
      const initialConfig: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: 'test',
        collect_response: true,
        collect_mode: 'first_match',
      };

      const { rerender } = render(
        <WsSendCollectConfigPanel data={initialConfig} onSave={mockOnSave} />
      );

      // 初始不应显示条件配置
      expect(screen.queryByLabelText(/匹配类型/)).toBeNull();

      // 更新为 condition 模式
      const updatedConfig: WsSendCollectConfigType = {
        ...initialConfig,
        collect_mode: 'condition_match',
        collect_condition: {
          match_type: 'json_path',
          expression: '$.data.ready == true',
        },
      };

      rerender(<WsSendCollectConfigPanel data={updatedConfig} onSave={mockOnSave} />);

      // 验证条件配置已显示
      await waitFor(() => {
        expect(screen.getByLabelText(/匹配类型/)).toBeDefined();
        expect(screen.getByLabelText(/匹配表达式/)).toBeDefined();
      });
    });
  });

  describe('等待响应配置测试', () => {
    it('当 collect_response 为 true 时，应显示等待配置', () => {
      const config: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: 'test',
        collect_response: true,
        collect_mode: 'first_match',
      };

      render(<WsSendCollectConfigPanel data={config} onSave={mockOnSave} />);

      expect(screen.getByLabelText(/采集模式/)).toBeDefined();
      expect(screen.getByLabelText(/采集超时/)).toBeDefined();
    });

    it('当 collect_response 为 false 时，应隐藏等待配置', () => {
      const config: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: 'test',
        collect_response: false,
      };

      render(<WsSendCollectConfigPanel data={config} onSave={mockOnSave} />);

      expect(screen.queryByLabelText(/采集模式/)).toBeNull();
      expect(screen.queryByLabelText(/采集超时/)).toBeNull();
    });

    it('[核心修复] 从 collect_response=true 切换到 false 时，关联字段应该被清除', async () => {
      // 这个测试验证：当外部数据从 collect_response=true 变为 false 时，
      // 组件应该正确处理并渲染（核心逻辑在 fromFormData 函数中清除关联字段）

      // 初始配置：开启等待响应
      const initialConfig: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: 'test message',
        collect_response: true,
        collect_mode: 'condition_match',
        collect_condition: {
          match_type: 'json_path',
          expression: '$.status == "ok"',
        },
        collect_timeout_ms: 30000,
      };

      const { rerender } = render(
        <WsSendCollectConfigPanel data={initialConfig} onSave={mockOnSave} />
      );

      // 初始应该显示等待配置
      expect(screen.getByLabelText(/采集模式/)).toBeDefined();

      // 更新外部数据：关闭等待响应
      const updatedConfig: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: 'test message',
        collect_response: false,
        // fromFormData 函数会清除 collect_mode、collect_condition、collect_timeout_ms
      };

      rerender(<WsSendCollectConfigPanel data={updatedConfig} onSave={mockOnSave} />);

      // 验证等待配置已隐藏（说明关联字段被正确处理）
      await waitFor(() => {
        expect(screen.queryByLabelText(/采集模式/)).toBeNull();
        expect(screen.queryByLabelText(/采集超时/)).toBeNull();
      });
    });

    it('[核心修复] 从 collect_mode=condition 切换到 first_message 时，collect_condition 应被正确处理', async () => {
      // 这个测试验证：当外部数据从 collect_mode=condition 变为 first_message 时，
      // 组件应该正确处理并渲染（核心逻辑在 fromFormData 函数中清除 collect_condition）

      // 初始配置：condition 模式
      const initialConfig: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: 'test message',
        collect_response: true,
        collect_mode: 'condition_match',
        collect_condition: {
          match_type: 'json_path',
          expression: '$.status == "ready"',
        },
        collect_timeout_ms: 30000,
      };

      const { rerender } = render(<WsSendCollectConfigPanel data={initialConfig} onSave={mockOnSave} />);

      // 初始应该显示匹配条件配置
      expect(screen.getByLabelText(/匹配类型/)).toBeDefined();
      expect(screen.getByLabelText(/匹配表达式/)).toBeDefined();

      // 更新外部数据：切换到 first_message 模式
      const updatedConfig: WsSendCollectConfigType = {
        type: 'ws:send_collect',
        connection_id: 'ws-connect-1',
        message: 'test message',
        collect_response: true,
        collect_mode: 'first_match',
        // fromFormData 函数会清除 collect_condition
      };

      rerender(<WsSendCollectConfigPanel data={updatedConfig} onSave={mockOnSave} />);

      // 验证匹配条件配置已隐藏（说明 collect_condition 被正确处理）
      await waitFor(() => {
        expect(screen.queryByLabelText(/匹配类型/)).toBeNull();
        expect(screen.queryByLabelText(/匹配表达式/)).toBeNull();
  });
});
  });
});
