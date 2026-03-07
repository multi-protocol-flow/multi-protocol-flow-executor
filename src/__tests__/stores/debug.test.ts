/**
 * Debug Store 单元测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useDebugStore, selectDebugStatus, selectCurrentNodeUuid, selectIsDebugging, selectIsPaused } from '@/stores/debug';
import type { DebugEvent, ExecutionStackEntry } from '@/types/debug';

// Mock tauri service
vi.mock('@/services/tauri', () => ({
  startDebug: vi.fn(),
  sendDebugCommand: vi.fn(),
  endDebug: vi.fn(),
}));

describe('DebugStore', () => {
  beforeEach(() => {
    useDebugStore.getState().reset();
    vi.clearAllMocks();
  });

  describe('初始状态', () => {
    it('初始状态正确', () => {
      const state = useDebugStore.getState();

      expect(state.sessionId).toBeNull();
      expect(state.status).toBe('idle');
      expect(state.currentNodeUuid).toBeNull();
      expect(state.pauseReason).toBeNull();
      expect(state.variables).toEqual({});
      expect(state.executionStack).toEqual([]);
      expect(state.currentNodeOutput).toBeNull();
      expect(state.error).toBeNull();
    });
  });

  describe('handleDebugEvent', () => {
    describe('SessionStarted 事件', () => {
      it('处理 SessionStarted 事件', () => {
        const event: DebugEvent = {
          SessionStarted: { flow_uuid: 'flow-123' },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.sessionId).toBe('flow-123');
        expect(state.status).toBe('running');
      });
    });

    describe('PausedAtNode 事件', () => {
      it('处理 PausedAtNode 事件（断点暂停）', () => {
        const event: DebugEvent = {
          PausedAtNode: {
            node_uuid: 'node-1',
            reason: { type: 'breakpoint', node_uuid: 'node-1' },
            variables: { status: 200, body: { data: 'test' } },
            execution_stack: [],
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.status).toBe('paused');
        expect(state.currentNodeUuid).toBe('node-1');
        expect(state.pauseReason).toEqual({ type: 'breakpoint', node_uuid: 'node-1' });
        expect(state.variables).toEqual({ status: 200, body: { data: 'test' } });
        expect(state.currentNodeOutput).toBeNull();
      });

      it('处理 PausedAtNode 事件（单步完成暂停）', () => {
        const stackEntry: ExecutionStackEntry = {
          node_uuid: 'node-1',
          node_name: 'HTTP Request',
          node_type: 'http',
          started_at: 1000,
          ended_at: 2000,
          input: { url: 'https://example.com' },
          output: { status: 200 },
          success: true,
          error: undefined,
        };

        const event: DebugEvent = {
          PausedAtNode: {
            node_uuid: 'node-2',
            reason: { type: 'step_complete', node_uuid: 'node-2' },
            variables: { count: 1 },
            execution_stack: [stackEntry],
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.status).toBe('paused');
        expect(state.currentNodeUuid).toBe('node-2');
        expect(state.pauseReason).toEqual({ type: 'step_complete', node_uuid: 'node-2' });
        expect(state.executionStack).toHaveLength(1);
        expect(state.executionStack[0].node_name).toBe('HTTP Request');
      });

      it('处理 PausedAtNode 事件（错误暂停）', () => {
        const event: DebugEvent = {
          PausedAtNode: {
            node_uuid: 'node-1',
            reason: { type: 'error', message: '网络超时' },
            variables: {},
            execution_stack: [],
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.status).toBe('paused');
        expect(state.pauseReason).toEqual({ type: 'error', message: '网络超时' });
      });
    });

    describe('NodeExecuted 事件', () => {
      it('处理 NodeExecuted 事件（有输出）', () => {
        const event: DebugEvent = {
          NodeExecuted: {
            node_uuid: 'node-1',
            output: { status: 200, body: { id: 1 } },
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.currentNodeOutput).toEqual({ status: 200, body: { id: 1 } });
      });

      it('处理 NodeExecuted 事件（无输出）', () => {
        const event: DebugEvent = {
          NodeExecuted: {
            node_uuid: 'node-1',
            output: undefined,
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.currentNodeOutput).toBeUndefined();
      });
    });

    describe('SessionEnded 事件', () => {
      it('处理 SessionEnded 事件（正常完成）', () => {
        // 先设置一个暂停状态
        useDebugStore.setState({
          status: 'paused',
          currentNodeUuid: 'node-1',
          pauseReason: { type: 'breakpoint', node_uuid: 'node-1' },
        });

        const event: DebugEvent = {
          SessionEnded: {
            reason: { type: 'completed' },
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.status).toBe('ended');
        expect(state.currentNodeUuid).toBeNull();
        expect(state.pauseReason).toBeNull();
      });

      it('处理 SessionEnded 事件（用户终止）', () => {
        const event: DebugEvent = {
          SessionEnded: {
            reason: { type: 'user_abort' },
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.status).toBe('ended');
      });

      it('处理 SessionEnded 事件（错误结束）', () => {
        const event: DebugEvent = {
          SessionEnded: {
            reason: { type: 'error', message: '执行失败' },
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.status).toBe('ended');
      });
    });

    describe('VariableModified 事件', () => {
      it('处理 VariableModified 事件（新增变量）', () => {
        const event: DebugEvent = {
          VariableModified: {
            name: 'newVar',
            value: 123,
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.variables.newVar).toBe(123);
      });

      it('处理 VariableModified 事件（修改已有变量）', () => {
        useDebugStore.setState({
          variables: { status: 200, body: { data: 'old' } },
        });

        const event: DebugEvent = {
          VariableModified: {
            name: 'status',
            value: 404,
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.variables.status).toBe(404);
        expect(state.variables.body).toEqual({ data: 'old' }); // 其他变量不变
      });

      it('处理 VariableModified 事件（复杂值）', () => {
        const event: DebugEvent = {
          VariableModified: {
            name: 'response',
            value: { headers: { 'Content-Type': 'application/json' }, data: [1, 2, 3] },
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.variables.response).toEqual({
          headers: { 'Content-Type': 'application/json' },
          data: [1, 2, 3],
        });
      });
    });

    describe('Error 事件', () => {
      it('处理 Error 事件', () => {
        const event: DebugEvent = {
          Error: {
            message: '连接超时',
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.status).toBe('ended');
        expect(state.error).toBe('连接超时');
      });
    });

    describe('Heartbeat 事件 - 状态同步', () => {
      it('心跳事件：状态一致时不改变前端状态', () => {
        // 设置前端状态为 running
        useDebugStore.setState({ status: 'running', currentNodeUuid: 'node-1' });

        const event: DebugEvent = {
          Heartbeat: {
            status: 'running',
            current_node_uuid: 'node-1',
            timestamp: Date.now(),
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.status).toBe('running');
        expect(state.currentNodeUuid).toBe('node-1');
      });

      it('心跳事件：前端 running，后端 paused 时同步状态（修复工具栏不可点击问题）', () => {
        // 模拟 PausedAtNode 事件丢失的情况
        // 前端状态是 running（发送命令后），但后端已经 paused
        useDebugStore.setState({
          status: 'running',
          currentNodeUuid: 'node-old',
          isCommandPending: true,
          previousStatus: 'paused',
        });

        const event: DebugEvent = {
          Heartbeat: {
            status: 'paused',
            current_node_uuid: 'node-http-1',
            timestamp: Date.now(),
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        // 关键修复：前端状态应同步为 paused
        expect(state.status).toBe('paused');
        expect(state.currentNodeUuid).toBe('node-http-1');
        // pauseReason 应该被设置
        expect(state.pauseReason).toEqual({ type: 'breakpoint', node_uuid: 'node-http-1' });
        // 命令等待状态应该被清除
        expect(state.isCommandPending).toBe(false);
        expect(state.previousStatus).toBeNull();
      });

      it('心跳事件：后端没有 current_node_uuid 时使用空值', () => {
        useDebugStore.setState({ status: 'running' });

        const event: DebugEvent = {
          Heartbeat: {
            status: 'paused',
            current_node_uuid: undefined,
            timestamp: Date.now(),
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.status).toBe('paused');
        expect(state.currentNodeUuid).toBeNull();
      });

      it('心跳事件：更新心跳时间戳', () => {
        const beforeTime = Date.now();
        useDebugStore.setState({ lastHeartbeatTime: null });

        const event: DebugEvent = {
          Heartbeat: {
            status: 'running',
            timestamp: Date.now(),
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.lastHeartbeatTime).toBeGreaterThanOrEqual(beforeTime);
      });

      it('心跳事件：前端 paused，后端 running 时不同步（前端状态优先）', () => {
        // 这种情况不应该发生，但如果发生，前端状态应该保持
        useDebugStore.setState({ status: 'paused', currentNodeUuid: 'node-1' });

        const event: DebugEvent = {
          Heartbeat: {
            status: 'running',
            current_node_uuid: 'node-2',
            timestamp: Date.now(),
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        // 状态不应该改变（只有 running->paused 才同步）
        expect(state.status).toBe('paused');
        expect(state.currentNodeUuid).toBe('node-1');
      });
    });
  });

  describe('Selectors', () => {
    it('selectDebugStatus 返回正确状态', () => {
      expect(selectDebugStatus(useDebugStore.getState())).toBe('idle');

      useDebugStore.setState({ status: 'running' });
      expect(selectDebugStatus(useDebugStore.getState())).toBe('running');

      useDebugStore.setState({ status: 'paused' });
      expect(selectDebugStatus(useDebugStore.getState())).toBe('paused');

      useDebugStore.setState({ status: 'ended' });
      expect(selectDebugStatus(useDebugStore.getState())).toBe('ended');
    });

    it('selectCurrentNodeUuid 返回当前节点 UUID', () => {
      expect(selectCurrentNodeUuid(useDebugStore.getState())).toBeNull();

      useDebugStore.setState({ currentNodeUuid: 'node-123' });
      expect(selectCurrentNodeUuid(useDebugStore.getState())).toBe('node-123');
    });

    it('selectIsDebugging 返回是否正在调试', () => {
      // idle 状态：不调试
      expect(selectIsDebugging(useDebugStore.getState())).toBe(false);

      // running 状态：调试中
      useDebugStore.setState({ status: 'running' });
      expect(selectIsDebugging(useDebugStore.getState())).toBe(true);

      // paused 状态：调试中
      useDebugStore.setState({ status: 'paused' });
      expect(selectIsDebugging(useDebugStore.getState())).toBe(true);

      // ended 状态：不调试
      useDebugStore.setState({ status: 'ended' });
      expect(selectIsDebugging(useDebugStore.getState())).toBe(false);
    });

    it('selectIsPaused 返回是否已暂停', () => {
      // idle 状态：不暂停
      expect(selectIsPaused(useDebugStore.getState())).toBe(false);

      // running 状态：不暂停
      useDebugStore.setState({ status: 'running' });
      expect(selectIsPaused(useDebugStore.getState())).toBe(false);

      // paused 状态：暂停
      useDebugStore.setState({ status: 'paused' });
      expect(selectIsPaused(useDebugStore.getState())).toBe(true);

      // ended 状态：不暂停
      useDebugStore.setState({ status: 'ended' });
      expect(selectIsPaused(useDebugStore.getState())).toBe(false);
    });
  });

  describe('reset', () => {
    it('reset 恢复初始状态', () => {
      // 设置一些状态
      useDebugStore.setState({
        sessionId: 'session-123',
        status: 'paused',
        currentNodeUuid: 'node-1',
        pauseReason: { type: 'breakpoint', node_uuid: 'node-1' },
        variables: { status: 200 },
        executionStack: [
          {
            node_uuid: 'node-1',
            node_name: 'Entry',
            node_type: 'entry',
            started_at: 1000,
            ended_at: 1005,
            input: null,
            output: null,
            success: true,
            error: undefined,
          },
        ],
        currentNodeOutput: { result: 'ok' },
        error: 'some error',
      });

      useDebugStore.getState().reset();

      const state = useDebugStore.getState();
      expect(state.sessionId).toBeNull();
      expect(state.status).toBe('idle');
      expect(state.currentNodeUuid).toBeNull();
      expect(state.pauseReason).toBeNull();
      expect(state.variables).toEqual({});
      expect(state.executionStack).toEqual([]);
      expect(state.currentNodeOutput).toBeNull();
      expect(state.error).toBeNull();
    });
  });

  describe('Actions（Mock IPC）', () => {
    it('continueExecution 调用 sendDebugCommand', async () => {
      const { sendDebugCommand } = await import('@/services/tauri');
      const mockSendDebugCommand = sendDebugCommand as ReturnType<typeof vi.fn>;
      mockSendDebugCommand.mockResolvedValue(undefined);

      await useDebugStore.getState().continueExecution();

      expect(mockSendDebugCommand).toHaveBeenCalledWith('Continue');
    });

    it('stepOver 调用 sendDebugCommand', async () => {
      const { sendDebugCommand } = await import('@/services/tauri');
      const mockSendDebugCommand = sendDebugCommand as ReturnType<typeof vi.fn>;
      mockSendDebugCommand.mockResolvedValue(undefined);

      await useDebugStore.getState().stepOver();

      expect(mockSendDebugCommand).toHaveBeenCalledWith('StepOver');
    });

    it('skipNode 调用 sendDebugCommand（带参数）', async () => {
      const { sendDebugCommand } = await import('@/services/tauri');
      const mockSendDebugCommand = sendDebugCommand as ReturnType<typeof vi.fn>;
      mockSendDebugCommand.mockResolvedValue(undefined);

      await useDebugStore.getState().skipNode('true', { status: 200 });

      expect(mockSendDebugCommand).toHaveBeenCalledWith({
        SkipNode: {
          selected_port: 'true',
          simulated_output: { status: 200 },
        },
      });
    });

    it('skipNode 调用 sendDebugCommand（无参数）', async () => {
      const { sendDebugCommand } = await import('@/services/tauri');
      const mockSendDebugCommand = sendDebugCommand as ReturnType<typeof vi.fn>;
      mockSendDebugCommand.mockResolvedValue(undefined);

      await useDebugStore.getState().skipNode();

      expect(mockSendDebugCommand).toHaveBeenCalledWith({
        SkipNode: {
          selected_port: undefined,
          simulated_output: undefined,
        },
      });
    });

    it('runToEnd 调用 sendDebugCommand', async () => {
      const { sendDebugCommand } = await import('@/services/tauri');
      const mockSendDebugCommand = sendDebugCommand as ReturnType<typeof vi.fn>;
      mockSendDebugCommand.mockResolvedValue(undefined);

      await useDebugStore.getState().runToEnd();

      expect(mockSendDebugCommand).toHaveBeenCalledWith('RunToEnd');
    });

    it('endDebugSession 调用 endDebug', async () => {
      const { endDebug } = await import('@/services/tauri');
      const mockEndDebug = endDebug as ReturnType<typeof vi.fn>;
      mockEndDebug.mockResolvedValue(undefined);

      await useDebugStore.getState().endDebugSession();

      expect(mockEndDebug).toHaveBeenCalled();
      // 验证状态已重置
      const state = useDebugStore.getState();
      expect(state.status).toBe('idle');
    });

    it('continueExecution 处理错误', async () => {
      const { sendDebugCommand } = await import('@/services/tauri');
      const mockSendDebugCommand = sendDebugCommand as ReturnType<typeof vi.fn>;
      mockSendDebugCommand.mockRejectedValue(new Error('IPC 错误'));

      useDebugStore.setState({ status: 'paused' });
      await useDebugStore.getState().continueExecution();

      const state = useDebugStore.getState();
      // 新的错误格式包含操作前缀
      expect(state.error).toBe('继续执行失败: IPC 错误');
      // 命令失败时应回滚到之前状态
      expect(state.status).toBe('paused');
    });

    it('stepOver 更新状态为 running', async () => {
      const { sendDebugCommand } = await import('@/services/tauri');
      const mockSendDebugCommand = sendDebugCommand as ReturnType<typeof vi.fn>;
      mockSendDebugCommand.mockResolvedValue(undefined);

      useDebugStore.setState({ status: 'paused' });
      await useDebugStore.getState().stepOver();

      const state = useDebugStore.getState();
      expect(state.status).toBe('running');
    });
  });
});
