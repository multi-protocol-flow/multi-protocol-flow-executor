/**
 * Tauri IPC flow.ts 服务测试
 * 测试 createFlow, executeFlowWithChannel 函数及错误处理
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke, Channel } from '@tauri-apps/api/core';

import {
  createFlow,
  executeFlowWithChannel,
  buildFlowSlice,
  startInteractiveSliceExecution,
  IpcError,
} from '@/services/tauri/flow';
import type { Flow } from '@/types/flow';
import type { ExecuteFlowCommandOutput } from '@/types/ipc';
import { useEnvironmentStore } from '@/stores/environment';
import { useExecutionStore, useUIStore } from '@/stores';

// Mock platform to test Tauri IPC branch
vi.mock('@/lib/platform', () => ({
  isTauriEnv: () => true,
}));

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
    useEnvironmentStore.getState().reset();
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

    it('激活全局环境时应把 environment_name 传给后端', async () => {
      const flow = createTestFlow();
      const mockOutput: ExecuteFlowCommandOutput = {
        success: true,
        execution_time: 50,
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockOutput);

      useEnvironmentStore.setState({ activeEnvironmentName: 'dev' });

      await executeFlowWithChannel(flow, vi.fn());

      expect(invoke).toHaveBeenCalledWith(
        'execute_flow',
        expect.objectContaining({
          input: expect.objectContaining({ flow_id: flow.uuid, environment_name: 'dev' }),
        }),
      );
    });

    it('未激活环境时不发送 environment_name 字段', async () => {
      const flow = createTestFlow();
      vi.mocked(invoke).mockResolvedValueOnce({ success: true, execution_time: 1 });

      await executeFlowWithChannel(flow, vi.fn());

      const payload = vi.mocked(invoke).mock.calls[0][1] as { input: Record<string, unknown> };
      expect(payload.input).not.toHaveProperty('environment_name');
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

  describe('buildFlowSlice', () => {
    it('应该正确提取指定节点和内部连线', () => {
      const flow: Flow = {
        uuid: 'full-flow',
        name: 'Full Flow',
        nodes: [
          { uuid: 'node-1', type: 'http:request', name: 'Node 1', position: { x: 0, y: 0 }, data: { type: 'http:request', url: 'https://example.com' } },
          { uuid: 'node-2', type: 'script', name: 'Node 2', position: { x: 100, y: 0 }, data: { type: 'script', script: 'return 1;' } },
          { uuid: 'node-3', type: 'assertion', name: 'Node 3', position: { x: 200, y: 0 }, data: { type: 'assertion', assertions: [], mode: 'all', on_failure: 'route_false', continue_on_failure: false } },
        ],
        connections: [
          { id: 'c1', source_node_uuid: 'node-1', target_node_uuid: 'node-2', source_port_id: 'out', target_port_id: 'in' },
          { id: 'c2', source_node_uuid: 'node-2', target_node_uuid: 'node-3', source_port_id: 'out', target_port_id: 'in' },
          { id: 'c3', source_node_uuid: 'node-1', target_node_uuid: 'node-3', source_port_id: 'out', target_port_id: 'in' },
        ],
        setup: { nodes: [], connections: [] },
        teardown: { nodes: [], connections: [] },
        initial_variables: {},
      };

      const slice = buildFlowSlice(flow, ['node-2', 'node-3']);
      expect(slice.uuid).toMatch(/^slice-\d+/);
      expect(slice.name).toBe('Slice (2 nodes)');
      expect(slice.nodes).toHaveLength(2);
      expect(slice.nodes.map((n) => n.uuid)).toEqual(['node-2', 'node-3']);
      expect(slice.connections).toHaveLength(1);
      expect(slice.connections[0].id).toBe('c2');
      expect(slice.setup).toBeUndefined();
      expect(slice.teardown).toBeUndefined();
    });

    it('单个节点切片应该以该节点名称命名', () => {
      const flow = createTestFlow();
      const slice = buildFlowSlice(flow, ['http-node-uuid']);
      expect(slice.nodes).toHaveLength(1);
      expect(slice.name).toBe('Test: HTTP 请求');
      expect(slice.connections).toHaveLength(0);
    });
  });

  describe('startInteractiveSliceExecution', () => {
    it('应该展开底部面板并进入 running 标签页', async () => {
      const flow = createTestFlow();
      const mockOutput: ExecuteFlowCommandOutput = {
        success: true,
        execution_time: 42,
        flow_report: {
          uuid: 'rep-slice-1',
          flow_uuid: 'slice-123456789',
          flow_name: 'Test: HTTP 请求',
          start_time: Date.now(),
          end_time: Date.now() + 42,
          duration_ms: 42,
          total_nodes: 1,
          executed_count: 1,
          status: 'success',
          node_reports: [],
          statistics: {
            success_count: 1,
            failed_count: 0,
            avg_node_duration_ms: 42,
            max_node_duration_ms: 42,
            min_node_duration_ms: 42,
          },
          execution_mode: 'normal',
        },
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockOutput);

      await startInteractiveSliceExecution(flow, ['http-node-uuid']);

      expect(invoke).toHaveBeenCalledWith(
        'execute_flow',
        expect.objectContaining({
          flow: expect.objectContaining({
            uuid: expect.stringMatching(/^slice-\d+/),
            name: 'Test: HTTP 请求',
          }),
        })
      );
      expect(useUIStore.getState().isBottomPanelExpanded).toBe(true);
      expect(useUIStore.getState().bottomPanelTab).toBe('running');
      expect(useExecutionStore.getState().report?.flow_uuid).toMatch(/^slice-\d+/);
      expect(useExecutionStore.getState().report?.status).toBe('success');
      expect(useExecutionStore.getState().history).toHaveLength(0);
      expect(useExecutionStore.getState().isExecuting).toBe(false);
    });
  });
});
