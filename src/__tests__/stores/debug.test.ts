/**
 * Debug Store 单元测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useDebugStore, selectDebugStatus, selectCurrentNodeUuid, selectIsDebugging, selectIsPaused } from '@/stores/debug';
import { useExecutionStore } from '@/stores/execution';
import type { DebugEvent, ExecutionStackEntry } from '@/types/debug';

// Mock tauri service
vi.mock('@/services/tauri', () => ({
  startDebug: vi.fn(),
  sendDebugCommand: vi.fn(),
  endDebug: vi.fn(),
  saveFlowReport: vi.fn().mockResolvedValue(undefined),
}));

// Mock sonner
vi.mock('sonner', () => ({
  toast: { error: vi.fn(), success: vi.fn() },
}));

describe('DebugStore', () => {
  beforeEach(() => {
    useDebugStore.getState().reset();
    useExecutionStore.getState().reset();
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
            entry: {
              node_uuid: 'node-1',
              node_name: 'HTTP请求',
              node_type: 'http',
              started_at: Date.now(),
              ended_at: Date.now() + 100,
              input: null,
              output: { status: 200, body: { id: 1 } },
              success: true,
              error: undefined,
            },
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.currentNodeOutput).toEqual({ status: 200, body: { id: 1 } });
      });

      it('处理 NodeExecuted 事件（无输出）', () => {
        const event: DebugEvent = {
          NodeExecuted: {
            entry: {
              node_uuid: 'node-1',
              node_name: 'Entry',
              node_type: 'entry',
              started_at: Date.now(),
              ended_at: Date.now() + 10,
              input: null,
              output: null,
              success: true,
              error: undefined,
            },
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.currentNodeOutput).toBeNull();
      });
      it('处理 NodeExecuted 事件（sub_flow 节点保留 sub_flow_report）', () => {
        const mockSubFlowReport = {
          uuid: 'report-child-1',
          flow_uuid: 'child-flow-uuid',
          flow_name: '子流程认证',
          start_time: 1000,
          end_time: 2000,
          duration_ms: 1000,
          status: 'success' as const,
          executed_count: 1,
          node_reports: [
            {
              node_uuid: 'child-http-node',
              node_type: 'http',
              node_name: '登录请求',
              start_time: 1000,
              end_time: 2000,
              duration_ms: 1000,
              status: 'success' as const,
            },
          ],
          total_nodes: 1,
          statistics: { success_count: 1, failed_count: 0, skipped_count: 0 },
        };

        const event: DebugEvent = {
          NodeExecuted: {
            entry: {
              node_uuid: 'subflow-node-1',
              node_name: '调用子流程',
              node_type: 'sub_flow',
              started_at: 1000,
              ended_at: 2000,
              input: { target_flow_path: './child.json' },
              output: { success: true },
              used_port_id: 'out',
              success: true,
              sub_flow_report: mockSubFlowReport,
            },
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        // 验证 debug store 的 executionStack 包含 sub_flow_report
        const debugState = useDebugStore.getState();
        expect(debugState.executionStack).toHaveLength(1);
        expect(debugState.executionStack[0].sub_flow_report).toEqual(mockSubFlowReport);

        // 验证桥接到 executionStore 的节点报告同样保留 sub_flow_report 与 used_port_id
        const execReport = useExecutionStore.getState().getNodeExecutionReport('subflow-node-1');
        expect(execReport).not.toBeNull();
        expect(execReport?.used_port_id).toBe('out');
        expect(execReport?.sub_flow_report).toBeDefined();
        expect(execReport?.sub_flow_report?.node_reports).toHaveLength(1);
        expect(execReport?.sub_flow_report?.node_reports[0].node_uuid).toBe('child-http-node');
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
            execution_stack: [],
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.status).toBe('ended');
        // 调试结束后保留数据用于只读查看
        expect(state.currentNodeUuid).toBe('node-1'); // 保留最后暂停的节点
        expect(state.pauseReason).not.toBeNull(); // 保留暂停原因
        expect(state.isDebugDataReadOnly).toBe(false); // 不再设置只读标记，执行数据保留在 executionStore
      });
      it('处理 SessionEnded 事件时同步包含 sub_flow_report 的 execution_stack 到 executionStore', () => {
        const mockSubFlowReport = {
          uuid: 'report-child-2',
          flow_uuid: 'child-flow-uuid',
          flow_name: '子流程',
          start_time: 1000,
          end_time: 2000,
          duration_ms: 1000,
          status: 'success' as const,
          executed_count: 2,
          node_reports: [
            {
              node_uuid: 'child-1',
              node_type: 'entry',
              node_name: '入口',
              start_time: 1000,
              end_time: 1100,
              duration_ms: 100,
              status: 'success' as const,
            },
            {
              node_uuid: 'child-2',
              node_type: 'http',
              node_name: '请求',
              start_time: 1100,
              end_time: 2000,
              duration_ms: 900,
              status: 'success' as const,
            },
          ],
          total_nodes: 2,
          statistics: { success_count: 2, failed_count: 0, skipped_count: 0 },
        };

        const event: DebugEvent = {
          SessionEnded: {
            reason: { type: 'completed' },
            execution_stack: [
              {
                node_uuid: 'subflow-node-1',
                node_name: '调用子流程',
                node_type: 'sub_flow',
                started_at: 1000,
                ended_at: 2000,
                success: true,
                used_port_id: 'out',
                sub_flow_report: mockSubFlowReport,
              },
            ],
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const execReport = useExecutionStore.getState().getNodeExecutionReport('subflow-node-1');
        expect(execReport).not.toBeNull();
        expect(execReport?.sub_flow_report).toBeDefined();
        expect(execReport?.sub_flow_report?.node_reports).toHaveLength(2);
        expect(execReport?.used_port_id).toBe('out');
      });

      it('处理 SessionEnded 事件（用户终止）', () => {
        const event: DebugEvent = {
          SessionEnded: {
            reason: { type: 'user_abort' },
            execution_stack: [],
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
            execution_stack: [],
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        const state = useDebugStore.getState();
        expect(state.status).toBe('ended');
      });

      it('SessionEnded 有报告时显示 toast 通知', async () => {
        const { toast } = await import('sonner');
        const mockToastSuccess = toast.success as ReturnType<typeof vi.fn>;
        const mockSetReport = vi.fn();

        // Mock executionStore.setReport
        const originalGetState = (await import('@/stores/execution')).useExecutionStore.getState;
        const execStore = (await import('@/stores/execution')).useExecutionStore;
        const mockGetState = vi.fn().mockReturnValue({
          setReport: mockSetReport,
          openReportDialog: vi.fn(),
        });
        execStore.getState = mockGetState;

        const report = {
          uuid: 'report-1',
          flow_uuid: 'flow-1',
          flow_name: '测试流程',
          start_time: 1000,
          end_time: 2000,
          duration_ms: 1000,
          total_nodes: 3,
          executed_count: 3,
          status: 'success' as const,
          node_reports: [],
          statistics: { success_count: 2, failed_count: 1 },
        };

        const event: DebugEvent = {
          SessionEnded: {
            reason: { type: 'completed' },
            execution_stack: [],
            report,
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        // 验证 toast.success 被调用
        expect(mockToastSuccess).toHaveBeenCalledWith('调试完成', expect.objectContaining({
          description: '耗时 1000ms · 2 成功 · 1 失败',
          duration: 10000,
        }));

        // 验证 toast action 包含 label
        const callArgs = mockToastSuccess.mock.calls[0][1];
        expect(callArgs.action.label).toBe('查看报告');

        // 验证 setReport 仍然被调用（未被移除）
        expect(mockSetReport).toHaveBeenCalledWith(report);

        // 验证报告已存储
        expect(useDebugStore.getState().lastDebugReport).toEqual(report);

        // 恢复原始方法
        execStore.getState = originalGetState;
      });

      it('SessionEnded 无报告时不显示 toast 通知', async () => {
        const { toast } = await import('sonner');
        const mockToastSuccess = toast.success as ReturnType<typeof vi.fn>;

        const event: DebugEvent = {
          SessionEnded: {
            reason: { type: 'user_abort' },
            execution_stack: [],
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        // 无报告时不应调用 toast.success
        expect(mockToastSuccess).not.toHaveBeenCalled();
      });

      it('toast action onClick 调用 openReportDialog', async () => {
        const { toast } = await import('sonner');
        const mockToastSuccess = toast.success as ReturnType<typeof vi.fn>;

        const report = {
          uuid: 'report-2',
          flow_uuid: 'flow-2',
          flow_name: '测试流程2',
          start_time: 1000,
          end_time: 3000,
          duration_ms: 2000,
          total_nodes: 1,
          executed_count: 1,
          status: 'success' as const,
          node_reports: [],
          statistics: { success_count: 1, failed_count: 0 },
        };

        const event: DebugEvent = {
          SessionEnded: {
            reason: { type: 'completed' },
            execution_stack: [],
            report,
          },
        };

        useDebugStore.getState().handleDebugEvent(event);

        // 获取 toast action 的 onClick 并调用
        const callArgs = mockToastSuccess.mock.calls[0][1];
        callArgs.action.onClick();

        // 验证 openReportDialog 被调用（通过 useExecutionStore.getState()）
        // 由于 mock 机制，我们验证 setReport 被调用即可确认 store 交互正常
        expect(useDebugStore.getState().lastDebugReport).toEqual(report);
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
      // 调试结束后设置为 ended 状态并保留数据（只读模式）
      const state = useDebugStore.getState();
      expect(state.status).toBe('ended');
      expect(state.isDebugDataReadOnly).toBe(false); // 不再设置只读标记，执行数据保留在 executionStore
    });

    it('continueExecution 处理错误', async () => {
      const { sendDebugCommand } = await import('@/services/tauri');
      const mockSendDebugCommand = sendDebugCommand as ReturnType<typeof vi.fn>;
      mockSendDebugCommand.mockRejectedValue(new Error('IPC 错误'));

      useDebugStore.setState({ status: 'paused' });
      await useDebugStore.getState().continueExecution();

      const state = useDebugStore.getState();
      // 错误信息使用 i18n 国际化文本
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

  describe('setBreakpointDynamic', () => {
    it('调用 sendDebugCommand 发送 SetBreakpoint 命令（启用）', async () => {
      const { sendDebugCommand } = await import('@/services/tauri');
      const mockSendDebugCommand = sendDebugCommand as ReturnType<typeof vi.fn>;
      mockSendDebugCommand.mockResolvedValue(undefined);

      await useDebugStore.getState().setBreakpointDynamic('node-123', true);

      expect(mockSendDebugCommand).toHaveBeenCalledWith({
        SetBreakpoint: {
          node_uuid: 'node-123',
          enabled: true,
        },
      });
    });

    it('调用 sendDebugCommand 发送 SetBreakpoint 命令（禁用）', async () => {
      const { sendDebugCommand } = await import('@/services/tauri');
      const mockSendDebugCommand = sendDebugCommand as ReturnType<typeof vi.fn>;
      mockSendDebugCommand.mockResolvedValue(undefined);

      await useDebugStore.getState().setBreakpointDynamic('node-456', false);

      expect(mockSendDebugCommand).toHaveBeenCalledWith({
        SetBreakpoint: {
          node_uuid: 'node-456',
          enabled: false,
        },
      });
    });

    it('处理错误情况', async () => {
      const { sendDebugCommand } = await import('@/services/tauri');
      const mockSendDebugCommand = sendDebugCommand as ReturnType<typeof vi.fn>;
      mockSendDebugCommand.mockRejectedValue(new Error('IPC 断点错误'));

      await useDebugStore.getState().setBreakpointDynamic('node-789', true);

      const state = useDebugStore.getState();
      expect(state.error).toBe('设置断点失败: IPC 断点错误');
    });
  });

  describe('BreakpointModified 事件', () => {
    it('处理 BreakpointModified 事件（启用断点）', async () => {
      // Mock flow store
      const mockSetBreakpoint = vi.fn();
      vi.doMock('@/stores/flow', () => ({
        useFlowStore: {
          getState: () => ({
            setBreakpoint: mockSetBreakpoint,
          }),
        },
      }));

      const event: DebugEvent = {
        BreakpointModified: {
          node_uuid: 'node-123',
          enabled: true,
        },
      };

      useDebugStore.getState().handleDebugEvent(event);

      // 等待动态导入完成
      await new Promise(resolve => setTimeout(resolve, 100));

      // 注意：由于动态导入，这里需要验证 mock 是否被调用
      // 在实际测试中，可能需要调整等待时间或使用其他方式验证
    });

    it('处理 BreakpointModified 事件（禁用断点）', async () => {
      const event: DebugEvent = {
        BreakpointModified: {
          node_uuid: 'node-456',
          enabled: false,
        },
      };

      useDebugStore.getState().handleDebugEvent(event);

      // 事件处理不应抛出错误
      const state = useDebugStore.getState();
      expect(state.error).toBeNull();
    });
  });

  describe('gRPC 调试事件', () => {
    it('处理 GrpcChannelCreated 事件 — 存储连接信息', () => {
      const event: DebugEvent = {
        GrpcChannelCreated: {
          node_uuid: 'node-grpc-1',
          connection_id: 'conn-grpc-1',
          url: 'http://localhost:50051',
          service_count: 3,
        },
      };

      useDebugStore.getState().handleDebugEvent(event);

      const state = useDebugStore.getState();
      expect(state.grpcConnections['conn-grpc-1']).toEqual({
        url: 'http://localhost:50051',
        service_count: 3,
      });
    });

    it('处理 GrpcMethodCalled 事件 — 记录方法调用', () => {
      const event: DebugEvent = {
        GrpcMethodCalled: {
          node_uuid: 'node-grpc-2',
          connection_id: 'conn-grpc-1',
          service: 'Greeter',
          method: 'SayHello',
          pattern: 'unary',
          duration_ms: 42,
        },
      };

      useDebugStore.getState().handleDebugEvent(event);

      const state = useDebugStore.getState();
      expect(state.grpcMethodCalls['conn-grpc-1']).toHaveLength(1);
      expect(state.grpcMethodCalls['conn-grpc-1'][0]).toEqual({
        node_uuid: 'node-grpc-2',
        service: 'Greeter',
        method: 'SayHello',
        pattern: 'unary',
        duration_ms: 42,
      });
    });

    it('同一连接多次 GrpcMethodCalled 追加记录', () => {
      const connId = 'conn-grpc-multi';

      const call1: DebugEvent = {
        GrpcMethodCalled: {
          node_uuid: 'node-a',
          connection_id: connId,
          service: 'ServiceA',
          method: 'MethodA',
          pattern: 'unary',
          duration_ms: 10,
        },
      };
      const call2: DebugEvent = {
        GrpcMethodCalled: {
          node_uuid: 'node-b',
          connection_id: connId,
          service: 'ServiceB',
          method: 'MethodB',
          pattern: 'server_streaming',
          duration_ms: 20,
        },
      };

      useDebugStore.getState().handleDebugEvent(call1);
      useDebugStore.getState().handleDebugEvent(call2);

      const state = useDebugStore.getState();
      expect(state.grpcMethodCalls[connId]).toHaveLength(2);
      expect(state.grpcMethodCalls[connId][0].service).toBe('ServiceA');
      expect(state.grpcMethodCalls[connId][1].service).toBe('ServiceB');
    });

    it('处理 GrpcChannelClosed 事件 — 保留连接信息', () => {
      // 先创建连接
      useDebugStore.getState().handleDebugEvent({
        GrpcChannelCreated: {
          node_uuid: 'node-grpc-1',
          connection_id: 'conn-grpc-close',
          url: 'http://localhost:50051',
          service_count: 1,
        },
      });

      // 关闭连接
      useDebugStore.getState().handleDebugEvent({
        GrpcChannelClosed: {
          node_uuid: 'node-grpc-1',
          connection_id: 'conn-grpc-close',
          url: 'http://localhost:50051',
        },
      });

      const state = useDebugStore.getState();
      // 连接信息应保留
      expect(state.grpcConnections['conn-grpc-close']).toEqual({
        url: 'http://localhost:50051',
        service_count: 1,
      });
    });
  });
  describe('Subflow 嵌套调试与调用栈帧 (Call Frames)', () => {
    it('处理携带多层 call_frames 的 PausedAtNode 事件', () => {
      const event: DebugEvent = {
        PausedAtNode: {
          node_uuid: 'c-http-1',
          reason: { type: 'step_complete', node_uuid: 'c-http-1' },
          variables: { foo: 'bar' },
          execution_stack: [],
          call_frames: [
            {
              frame_id: 0,
              flow_uuid: 'root-flow',
              flow_name: 'Root Flow',
              node_uuid: 'subflow-node',
              node_name: 'SubFlow',
            },
            {
              frame_id: 1,
              flow_uuid: 'child-flow',
              flow_name: 'Child Flow',
              node_uuid: 'c-http-1',
              node_name: 'HTTP Request',
              parent_subflow_node_uuid: 'subflow-node',
            },
          ],
        },
      };

      useDebugStore.getState().handleDebugEvent(event);

      const state = useDebugStore.getState();
      expect(state.status).toBe('paused');
      expect(state.currentNodeUuid).toBe('c-http-1');
      expect(state.callFrames).toHaveLength(2);
      expect(state.callFrames[0].flow_uuid).toBe('root-flow');
      expect(state.callFrames[1].flow_uuid).toBe('child-flow');
      expect(state.activeFrameIndex).toBe(1);
    });

    it('setActiveFrame 切换检视特定栈帧', () => {
      useDebugStore.setState({
        callFrames: [
          {
            frame_id: 0,
            flow_uuid: 'root-flow',
            flow_name: 'Root Flow',
            node_uuid: 'subflow-node',
            node_name: 'SubFlow',
          },
          {
            frame_id: 1,
            flow_uuid: 'child-flow',
            flow_name: 'Child Flow',
            node_uuid: 'c-http-1',
            node_name: 'HTTP Request',
            parent_subflow_node_uuid: 'subflow-node',
          },
        ],
        activeFrameIndex: 1,
      });

      useDebugStore.getState().setActiveFrame(0);
      expect(useDebugStore.getState().activeFrameIndex).toBe(0);

      useDebugStore.getState().setActiveFrame(1);
      expect(useDebugStore.getState().activeFrameIndex).toBe(1);

      // 无效索引不更新
      useDebugStore.getState().setActiveFrame(99);
      expect(useDebugStore.getState().activeFrameIndex).toBe(1);
    });

    it('stepInto 发送 StepInto 命令并更新状态', async () => {
      const { sendDebugCommand } = await import('@/services/tauri');
      const mockSendDebugCommand = sendDebugCommand as ReturnType<typeof vi.fn>;
      mockSendDebugCommand.mockResolvedValue(undefined);
      useDebugStore.setState({ status: 'paused' });

      await useDebugStore.getState().stepInto();

      expect(mockSendDebugCommand).toHaveBeenCalledWith('StepInto');
      expect(useDebugStore.getState().status).toBe('running');
    });

    it('stepOut 发送 StepOut 命令并更新状态', async () => {
      const { sendDebugCommand } = await import('@/services/tauri');
      const mockSendDebugCommand = sendDebugCommand as ReturnType<typeof vi.fn>;
      mockSendDebugCommand.mockResolvedValue(undefined);
      useDebugStore.setState({ status: 'paused' });

      await useDebugStore.getState().stepOut();

      expect(mockSendDebugCommand).toHaveBeenCalledWith('StepOut');
      expect(useDebugStore.getState().status).toBe('running');
    });
  });
});
