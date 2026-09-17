/**
 * Tauri IPC flow.ts 服务测试
 * 测试 createFlow, executeFlowWithChannel 函数及错误处理
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke, Channel } from '@tauri-apps/api/core';

import {
  createFlow,
  executeFlowWithChannel,
  IpcError,
} from '@/services/tauri/flow';
import type { Flow } from '@/types/flow';
import type { ExecuteFlowCommandOutput } from '@/types/ipc';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  Channel: vi.fn().mockImplementation(() => ({
    onmessage: null,
  })),
}));

// 测试用的流程数据
function createTestFlow(): Flow {
  return {
    uuid: 'test-flow-uuid',
    name: '测试流程',
    nodes: [
      {
        uuid: 'entry-node-uuid',
        type: 'entry',
        name: '开始',
        position: { x: 0, y: 0 },
        data: { type: 'entry' },
      },
      {
        uuid: 'http-node-uuid',
        type: 'http:request',
        name: 'HTTP 请求',
        position: { x: 200, y: 0 },
        data: { type: 'http:request', url: 'https://example.com', method: 'get' },
      },
    ],
    connections: [
      {
        id: 'conn-001',
        source_node_uuid: 'entry-node-uuid',
        target_node_uuid: 'http-node-uuid',
        source_port_id: 'output',
        target_port_id: 'input',
      },
    ],
    initial_variables: {},
  };
}

describe('Tauri IPC flow.ts 服务', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('IpcError', () => {
    it('应该创建带有消息和命令的 IpcError', () => {
      const error = new IpcError('测试错误', 'create_flow');

      expect(error).toBeInstanceOf(Error);
      expect(error).toBeInstanceOf(IpcError);
      expect(error.name).toBe('IpcError');
      expect(error.message).toBe('测试错误');
      expect(error.command).toBe('create_flow');
      expect(error.cause).toBeUndefined();
    });

    it('应该创建带有 cause 的 IpcError', () => {
      const cause = new Error('原始错误');
      const error = new IpcError('包装错误', 'execute_flow', cause);

      expect(error.message).toBe('包装错误');
      expect(error.command).toBe('execute_flow');
      expect(error.cause).toBe(cause);
    });
  });

  describe('createFlow', () => {
    it('应该成功创建流程', async () => {
      const mockFlow: Flow = {
        uuid: 'new-flow-uuid',
        name: '新流程',
        nodes: [
          {
            uuid: 'entry-new-flow-uuid',
            type: 'entry',
            name: '开始',
            position: { x: 0, y: 0 },
            data: { type: 'entry' },
          },
        ],
        connections: [],
        initial_variables: {},
      };

      vi.mocked(invoke).mockResolvedValueOnce({ flow: mockFlow });

      const result = await createFlow('新流程');

      expect(invoke).toHaveBeenCalledWith('create_flow', {
        input: { name: '新流程' },
      });
      expect(result).toEqual(mockFlow);
    });

    it('字符串错误应该抛出 IpcError', async () => {
      vi.mocked(invoke).mockRejectedValueOnce('创建失败');

      try {
        await createFlow('测试流程');
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as IpcError).message).toBe('创建失败');
        expect((error as IpcError).command).toBe('create_flow');
        expect((error as IpcError).cause).toBeUndefined();
      }
    });

    it('Error 实例应该抛出带 cause 的 IpcError', async () => {
      const originalError = new Error('原始错误信息');
      vi.mocked(invoke).mockRejectedValueOnce(originalError);

      try {
        await createFlow('测试流程');
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as IpcError).message).toBe('原始错误信息');
        expect((error as IpcError).command).toBe('create_flow');
        expect((error as IpcError).cause).toBe(originalError);
      }
    });

    it('未知错误类型应该格式化为 IpcError', async () => {
      vi.mocked(invoke).mockRejectedValueOnce(12345);

      try {
        await createFlow('测试流程');
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as IpcError).command).toBe('create_flow');
        expect((error as IpcError).message).toContain('Unknown error');
      }
    });
  });

  describe('executeFlowWithChannel', () => {
    it('应该创建 Channel 并传递给 invoke', async () => {
      const flow = createTestFlow();
      const mockOutput: ExecuteFlowCommandOutput = {
        success: true,
        execution_time: 50,
      };

      vi.mocked(invoke).mockResolvedValueOnce(mockOutput);

      const onLog = vi.fn();
      const result = await executeFlowWithChannel(flow, onLog);

      // 验证 Channel 被创建
      expect(Channel).toHaveBeenCalled();

      // 验证 invoke 调用参数
      expect(invoke).toHaveBeenCalledWith('execute_flow', {
        input: {
          flow_id: flow.uuid,
          variables: null,
        },
        flow,
        onEvent: expect.any(Object),
        onExecutionEvent: expect.any(Object),
        onWsEvent: expect.any(Object),
      });

      expect(result).toEqual(mockOutput);
    });

    it('应该传递执行变量', async () => {
      const flow = createTestFlow();
      const variables = { token: 'abc123' };
      const mockOutput: ExecuteFlowCommandOutput = {
        success: true,
        execution_time: 50,
      };

      vi.mocked(invoke).mockResolvedValueOnce(mockOutput);

      const onLog = vi.fn();
      const result = await executeFlowWithChannel(flow, onLog, vi.fn(), vi.fn(), variables);

      expect(invoke).toHaveBeenCalledWith('execute_flow', {
        input: {
          flow_id: flow.uuid,
          variables,
        },
        flow,
        onEvent: expect.any(Object),
        onExecutionEvent: expect.any(Object),
        onWsEvent: expect.any(Object),
      });

      expect(result).toEqual(mockOutput);
    });

    it('字符串错误应该抛出 IpcError', async () => {
      const flow = createTestFlow();
      vi.mocked(invoke).mockRejectedValueOnce('通道执行失败');

      const onLog = vi.fn();

      try {
        await executeFlowWithChannel(flow, onLog);
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as IpcError).command).toBe('execute_flow');
        expect((error as IpcError).message).toBe('通道执行失败');
      }
    });

    it('Error 实例应该抛出带 cause 的 IpcError', async () => {
      const flow = createTestFlow();
      const originalError = new Error('连接超时');
      vi.mocked(invoke).mockRejectedValueOnce(originalError);

      const onLog = vi.fn();

      try {
        await executeFlowWithChannel(flow, onLog);
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as IpcError).command).toBe('execute_flow');
        expect((error as IpcError).cause).toBe(originalError);
      }
    });

    it('未知错误类型应该格式化为 IpcError', async () => {
      const flow = createTestFlow();
      vi.mocked(invoke).mockRejectedValueOnce({ error: 'unknown' });

      const onLog = vi.fn();

      try {
        await executeFlowWithChannel(flow, onLog);
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as IpcError).command).toBe('execute_flow');
        expect((error as IpcError).message).toContain('Unknown error');
      }
    });

    it('应该设置 onmessage 回调', async () => {
      const flow = createTestFlow();
      const mockOutput: ExecuteFlowCommandOutput = {
        success: true,
        execution_time: 50,
      };

      // 捕获所有 Channel 实例的 onmessage
      type ChannelWithOnmessage = { onmessage: ((event: unknown) => void) | null };
      const capturedOnmessages: ((event: unknown) => void)[] = [];

      vi.mocked(Channel).mockImplementation(() => {
        let _cb: ((event: unknown) => void) | null = null;
        const channel = {
          set onmessage(cb: ((event: unknown) => void) | null) {
            _cb = cb;
            capturedOnmessages.push(cb!);
          },
          get onmessage() {
            return _cb;
          },
        } as ChannelWithOnmessage;
        return channel as unknown as InstanceType<typeof Channel>;
      });
      vi.mocked(invoke).mockResolvedValueOnce(mockOutput);

      const onLog = vi.fn();
      await executeFlowWithChannel(flow, onLog);

      // 验证 onmessage 被设置（logChannel 应包含 onLog 回调）
      expect(capturedOnmessages).toContain(onLog);
    });
  });
});
