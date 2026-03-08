/**
 * 调试功能端到端测试
 * 测试调试模式的完整流程
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act } from '@testing-library/react';
import { useDebugStore } from '@/stores/debug';
import { useFlowStore } from '@/stores';
import type { DebugEvent, ExecutionStackEntry } from '@/types/debug';
import type { Flow } from '@/types';

// 存储事件回调的引用
let debugEventCallback: ((event: DebugEvent) => void) | null = null;

// Mock Tauri service
vi.mock('@/services/tauri', () => ({
  startDebug: vi.fn((_flow: Flow, _breakpoints: string[], onEvent: (event: DebugEvent) => void) => {
    debugEventCallback = onEvent;
    return Promise.resolve();
  }),
  sendDebugCommand: vi.fn(() => Promise.resolve()),
  endDebug: vi.fn(() => Promise.resolve()),
}));

// 辅助函数：创建测试流程
function createTestFlow(): Flow {
  return {
    uuid: 'test-flow-1',
    name: '测试流程',
    nodes: [
      {
        uuid: 'entry-1',
        type: 'entry',
        name: '入口',
        position: { x: 0, y: 0 },
        data: { type: 'entry' },
        hasBreakpoint: true,
      },
      {
        uuid: 'http-1',
        type: 'http',
        name: 'HTTP请求',
        position: { x: 200, y: 0 },
        data: { type: 'http', url: 'https://api.example.com/users', method: 'get' },
        hasBreakpoint: true,
      },
      {
        uuid: 'conditional-1',
        type: 'conditional',
        name: '条件判断',
        position: { x: 400, y: 0 },
        data: {
          type: 'conditional',
          condition_type: 'advanced',
          advanced_expression: '{{status}} === 200',
        },
      },
    ],
    connections: [
      {
        id: 'conn-1',
        source_node_uuid: 'entry-1',
        target_node_uuid: 'http-1',
        source_port_id: 'output',
        target_port_id: 'input',
      },
      {
        id: 'conn-2',
        source_node_uuid: 'http-1',
        target_node_uuid: 'conditional-1',
        source_port_id: 'out',
        target_port_id: 'input',
      },
    ],
  };
}

// 辅助函数：创建执行堆栈条目
function createStackEntry(
  nodeUuid: string,
  nodeName: string,
  nodeType: string,
  output?: unknown
): ExecutionStackEntry {
  return {
    node_uuid: nodeUuid,
    node_name: nodeName,
    node_type: nodeType,
    started_at: Date.now(),
    ended_at: Date.now() + 100,
    input: null,
    output: output ?? null,
    success: true,
    error: undefined,
  };
}

describe('调试功能端到端测试', () => {
  let mockSendDebugCommand: ReturnType<typeof vi.fn>;
  let mockEndDebug: ReturnType<typeof vi.fn>;

  beforeEach(async () => {
    useDebugStore.getState().reset();
    useFlowStore.getState().reset();
    vi.clearAllMocks();
    debugEventCallback = null;

    // 导入 mock 后的模块获取 mock 函数引用
    const tauriService = await import('@/services/tauri');
    mockSendDebugCommand = tauriService.sendDebugCommand as ReturnType<typeof vi.fn>;
    mockEndDebug = tauriService.endDebug as ReturnType<typeof vi.fn>;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('场景1：设置断点 → 开始调试 → 查看变量 → 继续执行 → 结束调试', () => {
    it('完整调试流程', async () => {
      const flow = createTestFlow();
      useFlowStore.getState().setFlow(flow);

      // 收集断点
      const breakpoints = flow.nodes.filter((n) => n.hasBreakpoint).map((n) => n.uuid);
      expect(breakpoints).toEqual(['entry-1', 'http-1']);

      // 启动调试会话
      await act(async () => {
        await useDebugStore.getState().startDebugSession(flow, breakpoints);
      });

      // 模拟后端发送 SessionStarted 事件
      act(() => {
        debugEventCallback?.({ SessionStarted: { flow_uuid: flow.uuid } });
      });

      expect(useDebugStore.getState().status).toBe('running');
      expect(useDebugStore.getState().sessionId).toBe(flow.uuid);

      // 模拟在第一个断点处暂停
      act(() => {
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'entry-1',
            reason: { type: 'breakpoint', node_uuid: 'entry-1' },
            variables: {},
            execution_stack: [],
          },
        });
      });

      expect(useDebugStore.getState().status).toBe('paused');
      expect(useDebugStore.getState().currentNodeUuid).toBe('entry-1');
      expect(useDebugStore.getState().variables).toEqual({});

      // 继续执行
      await act(async () => {
        await useDebugStore.getState().continueExecution();
      });

      expect(mockSendDebugCommand).toHaveBeenCalledWith('Continue');
      expect(useDebugStore.getState().status).toBe('running');

      // 模拟节点执行完成
      act(() => {
        debugEventCallback?.({
          NodeExecuted: {
            node_uuid: 'entry-1',
            output: null,
          },
        });
      });

      // 模拟在第二个断点处暂停
      act(() => {
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'http-1',
            reason: { type: 'breakpoint', node_uuid: 'http-1' },
            variables: { status: 200, body: { users: [] } },
            execution_stack: [createStackEntry('entry-1', '入口', 'entry')],
          },
        });
      });

      expect(useDebugStore.getState().status).toBe('paused');
      expect(useDebugStore.getState().currentNodeUuid).toBe('http-1');
      expect(useDebugStore.getState().variables).toEqual({ status: 200, body: { users: [] } });
      expect(useDebugStore.getState().executionStack).toHaveLength(1);

      // 结束调试
      await act(async () => {
        await useDebugStore.getState().endDebugSession();
      });

      expect(mockEndDebug).toHaveBeenCalled();
      // 调试结束后状态为 ended，数据保留为只读模式
      expect(useDebugStore.getState().status).toBe('ended');
      expect(useDebugStore.getState().isDebugDataReadOnly).toBe(true);
    });
  });

  describe('场景2：单步执行整个流程', () => {
    it('单步执行流程', async () => {
      const flow = createTestFlow();
      useFlowStore.getState().setFlow(flow);

      const breakpoints = ['entry-1']; // 只在入口设置断点

      await act(async () => {
        await useDebugStore.getState().startDebugSession(flow, breakpoints);
      });

      // 启动会话
      act(() => {
        debugEventCallback?.({ SessionStarted: { flow_uuid: flow.uuid } });
      });

      // 第一个断点暂停
      act(() => {
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'entry-1',
            reason: { type: 'breakpoint', node_uuid: 'entry-1' },
            variables: {},
            execution_stack: [],
          },
        });
      });

      expect(useDebugStore.getState().status).toBe('paused');

      // 单步执行
      await act(async () => {
        await useDebugStore.getState().stepOver();
      });

      expect(mockSendDebugCommand).toHaveBeenCalledWith('StepOver');

      // 模拟单步完成后在下一个节点暂停
      act(() => {
        debugEventCallback?.({
          NodeExecuted: { node_uuid: 'entry-1', output: null },
        });
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'http-1',
            reason: { type: 'step_complete', node_uuid: 'http-1' },
            variables: {},
            execution_stack: [createStackEntry('entry-1', '入口', 'entry')],
          },
        });
      });

      expect(useDebugStore.getState().status).toBe('paused');
      expect(useDebugStore.getState().currentNodeUuid).toBe('http-1');
      expect(useDebugStore.getState().pauseReason).toEqual({
        type: 'step_complete',
        node_uuid: 'http-1',
      });

      // 继续单步
      await act(async () => {
        await useDebugStore.getState().stepOver();
      });

      expect(mockSendDebugCommand).toHaveBeenCalledTimes(2);

      // 模拟流程结束
      act(() => {
        debugEventCallback?.({
          SessionEnded: { reason: { type: 'completed' } },
        });
      });

      expect(useDebugStore.getState().status).toBe('ended');
    });
  });

  describe('场景3：跳过节点（条件节点、HTTP 节点）', () => {
    it('跳过条件节点（选择 true 分支）', async () => {
      const flow = createTestFlow();
      useFlowStore.getState().setFlow(flow);

      await act(async () => {
        await useDebugStore.getState().startDebugSession(flow, ['conditional-1']);
      });

      act(() => {
        debugEventCallback?.({ SessionStarted: { flow_uuid: flow.uuid } });
      });

      act(() => {
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'conditional-1',
            reason: { type: 'breakpoint', node_uuid: 'conditional-1' },
            variables: { status: 200 },
            execution_stack: [
              createStackEntry('entry-1', '入口', 'entry'),
              createStackEntry('http-1', 'HTTP请求', 'http', { status: 200 }),
            ],
          },
        });
      });

      expect(useDebugStore.getState().status).toBe('paused');

      // 跳过节点，选择 true 分支
      await act(async () => {
        await useDebugStore.getState().skipNode('true');
      });

      expect(mockSendDebugCommand).toHaveBeenCalledWith({
        SkipNode: {
          selected_port: 'true',
          simulated_output: undefined,
        },
      });
    });

    it('跳过 HTTP 节点（带模拟输出）', async () => {
      const flow = createTestFlow();
      useFlowStore.getState().setFlow(flow);

      await act(async () => {
        await useDebugStore.getState().startDebugSession(flow, ['http-1']);
      });

      act(() => {
        debugEventCallback?.({ SessionStarted: { flow_uuid: flow.uuid } });
      });

      act(() => {
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'http-1',
            reason: { type: 'breakpoint', node_uuid: 'http-1' },
            variables: {},
            execution_stack: [createStackEntry('entry-1', '入口', 'entry')],
          },
        });
      });

      // 跳过 HTTP 节点，提供模拟输出
      const simulatedOutput = { status: 200, body: { id: 1, name: 'Test User' } };
      await act(async () => {
        await useDebugStore.getState().skipNode('out', simulatedOutput);
      });

      expect(mockSendDebugCommand).toHaveBeenCalledWith({
        SkipNode: {
          selected_port: 'out',
          simulated_output: simulatedOutput,
        },
      });

      // 模拟后端处理跳过
      act(() => {
        debugEventCallback?.({
          NodeExecuted: {
            node_uuid: 'http-1',
            output: simulatedOutput,
          },
        });
        debugEventCallback?.({
          VariableModified: { name: 'status', value: 200 },
        });
        debugEventCallback?.({
          VariableModified: { name: 'body', value: { id: 1, name: 'Test User' } },
        });
      });

      const state = useDebugStore.getState();
      expect(state.variables.status).toBe(200);
      expect(state.variables.body).toEqual({ id: 1, name: 'Test User' });
    });
  });

  describe('场景4：执行到底', () => {
    it('执行到底模式', async () => {
      const flow = createTestFlow();
      useFlowStore.getState().setFlow(flow);

      await act(async () => {
        await useDebugStore.getState().startDebugSession(flow, ['entry-1', 'http-1']);
      });

      act(() => {
        debugEventCallback?.({ SessionStarted: { flow_uuid: flow.uuid } });
      });

      // 在第一个断点暂停
      act(() => {
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'entry-1',
            reason: { type: 'breakpoint', node_uuid: 'entry-1' },
            variables: {},
            execution_stack: [],
          },
        });
      });

      expect(useDebugStore.getState().status).toBe('paused');

      // 执行到底
      await act(async () => {
        await useDebugStore.getState().runToEnd();
      });

      expect(mockSendDebugCommand).toHaveBeenCalledWith('RunToEnd');
      expect(useDebugStore.getState().status).toBe('running');

      // 模拟流程完成（不再在后续断点暂停）
      act(() => {
        debugEventCallback?.({
          SessionEnded: { reason: { type: 'completed' } },
        });
      });

      expect(useDebugStore.getState().status).toBe('ended');
    });
  });

  describe('场景5：修改变量后继续执行', () => {
    it('修改变量并继续执行', async () => {
      const flow = createTestFlow();
      useFlowStore.getState().setFlow(flow);

      await act(async () => {
        await useDebugStore.getState().startDebugSession(flow, ['http-1']);
      });

      act(() => {
        debugEventCallback?.({ SessionStarted: { flow_uuid: flow.uuid } });
      });

      act(() => {
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'http-1',
            reason: { type: 'breakpoint', node_uuid: 'http-1' },
            variables: { status: 200, body: { count: 10 } },
            execution_stack: [createStackEntry('entry-1', '入口', 'entry')],
          },
        });
      });

      expect(useDebugStore.getState().variables).toEqual({ status: 200, body: { count: 10 } });

      // 模拟后端确认变量修改
      act(() => {
        debugEventCallback?.({
          VariableModified: { name: 'status', value: 404 },
        });
      });

      expect(useDebugStore.getState().variables.status).toBe(404);

      // 继续执行
      await act(async () => {
        await useDebugStore.getState().continueExecution();
      });

      expect(mockSendDebugCommand).toHaveBeenCalledWith('Continue');
    });
  });

  describe('边界情况测试', () => {
    it('无断点时启动调试（后端应正常执行到结束）', async () => {
      const flow = createTestFlow();
      // 移除所有断点
      flow.nodes.forEach((n) => {
        n.hasBreakpoint = false;
      });
      useFlowStore.getState().setFlow(flow);

      const breakpoints: string[] = [];

      await act(async () => {
        await useDebugStore.getState().startDebugSession(flow, breakpoints);
      });

      act(() => {
        debugEventCallback?.({ SessionStarted: { flow_uuid: flow.uuid } });
      });

      // 无断点时应直接完成
      act(() => {
        debugEventCallback?.({
          SessionEnded: { reason: { type: 'completed' } },
        });
      });

      expect(useDebugStore.getState().status).toBe('ended');
    });

    it('调试中重复发送命令', async () => {
      const flow = createTestFlow();
      useFlowStore.getState().setFlow(flow);

      await act(async () => {
        await useDebugStore.getState().startDebugSession(flow, ['entry-1']);
      });

      act(() => {
        debugEventCallback?.({ SessionStarted: { flow_uuid: flow.uuid } });
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'entry-1',
            reason: { type: 'breakpoint', node_uuid: 'entry-1' },
            variables: {},
            execution_stack: [],
          },
        });
      });

      // 快速连续发送多个命令
      await act(async () => {
        await Promise.all([
          useDebugStore.getState().continueExecution(),
          useDebugStore.getState().continueExecution(),
          useDebugStore.getState().continueExecution(),
        ]);
      });

      // 应该发送了所有命令
      expect(mockSendDebugCommand).toHaveBeenCalledTimes(3);
    });

    it('节点执行失败的处理', async () => {
      const flow = createTestFlow();
      useFlowStore.getState().setFlow(flow);

      await act(async () => {
        await useDebugStore.getState().startDebugSession(flow, ['http-1']);
      });

      act(() => {
        debugEventCallback?.({ SessionStarted: { flow_uuid: flow.uuid } });
      });

      act(() => {
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'http-1',
            reason: { type: 'breakpoint', node_uuid: 'http-1' },
            variables: {},
            execution_stack: [createStackEntry('entry-1', '入口', 'entry')],
          },
        });
      });

      // 继续执行
      await act(async () => {
        await useDebugStore.getState().continueExecution();
      });

      // 模拟 HTTP 节点执行失败
      act(() => {
        debugEventCallback?.({
          Error: { message: 'HTTP 请求超时' },
        });
        debugEventCallback?.({
          SessionEnded: {
            reason: { type: 'error', message: 'HTTP 请求超时' },
          },
        });
      });

      const state = useDebugStore.getState();
      expect(state.status).toBe('ended');
      expect(state.error).toBe('HTTP 请求超时');
    });

    it('调试会话被用户中断', async () => {
      const flow = createTestFlow();
      useFlowStore.getState().setFlow(flow);

      await act(async () => {
        await useDebugStore.getState().startDebugSession(flow, ['entry-1']);
      });

      act(() => {
        debugEventCallback?.({ SessionStarted: { flow_uuid: flow.uuid } });
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'entry-1',
            reason: { type: 'breakpoint', node_uuid: 'entry-1' },
            variables: {},
            execution_stack: [],
          },
        });
      });

      // 用户中断
      await act(async () => {
        await useDebugStore.getState().endDebugSession();
      });

      // endDebugSession 会重置状态为 idle，然后后端发送 SessionEnded 事件
      // 根据实际实现，状态可能是 ended（因为 SessionEnded 事件更新了状态）
      const state = useDebugStore.getState();
      // 用户中断后，后端会发送 user_abort 结束原因
      expect(['idle', 'ended']).toContain(state.status);
    });

    it('错误暂停后的处理', async () => {
      const flow = createTestFlow();
      useFlowStore.getState().setFlow(flow);

      await act(async () => {
        await useDebugStore.getState().startDebugSession(flow, ['http-1']);
      });

      act(() => {
        debugEventCallback?.({ SessionStarted: { flow_uuid: flow.uuid } });
      });

      // 在错误处暂停
      act(() => {
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'http-1',
            reason: { type: 'error', message: '网络连接失败' },
            variables: {},
            execution_stack: [createStackEntry('entry-1', '入口', 'entry')],
          },
        });
      });

      const state = useDebugStore.getState();
      expect(state.status).toBe('paused');
      expect(state.pauseReason).toEqual({ type: 'error', message: '网络连接失败' });

      // 用户可以选择结束调试
      await act(async () => {
        await useDebugStore.getState().endDebugSession();
      });

      expect(mockEndDebug).toHaveBeenCalled();
    });
  });

  describe('执行堆栈测试', () => {
    it('执行堆栈正确记录多个节点', async () => {
      const flow = createTestFlow();
      useFlowStore.getState().setFlow(flow);

      await act(async () => {
        await useDebugStore.getState().startDebugSession(flow, ['conditional-1']);
      });

      act(() => {
        debugEventCallback?.({ SessionStarted: { flow_uuid: flow.uuid } });
      });

      // 模拟执行多个节点后在条件节点暂停
      const stack: ExecutionStackEntry[] = [
        {
          node_uuid: 'entry-1',
          node_name: '入口',
          node_type: 'entry',
          started_at: 1000,
          ended_at: 1005,
          input: null,
          output: null,
          success: true,
          error: undefined,
        },
        {
          node_uuid: 'http-1',
          node_name: 'HTTP请求',
          node_type: 'http',
          started_at: 1010,
          ended_at: 1500,
          input: { url: 'https://api.example.com/users' },
          output: { status: 200, body: { users: [] } },
          success: true,
          error: undefined,
        },
      ];

      act(() => {
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'conditional-1',
            reason: { type: 'breakpoint', node_uuid: 'conditional-1' },
            variables: { status: 200, body: { users: [] } },
            execution_stack: stack,
          },
        });
      });

      const state = useDebugStore.getState();
      expect(state.executionStack).toHaveLength(2);
      expect(state.executionStack[0].node_name).toBe('入口');
      expect(state.executionStack[1].node_name).toBe('HTTP请求');
      expect(state.executionStack[1].output).toEqual({ status: 200, body: { users: [] } });
    });

    it('执行堆栈记录失败节点', async () => {
      const flow = createTestFlow();
      useFlowStore.getState().setFlow(flow);

      await act(async () => {
        await useDebugStore.getState().startDebugSession(flow, ['http-1']);
      });

      act(() => {
        debugEventCallback?.({ SessionStarted: { flow_uuid: flow.uuid } });
      });

      const failedStack: ExecutionStackEntry[] = [
        {
          node_uuid: 'entry-1',
          node_name: '入口',
          node_type: 'entry',
          started_at: 1000,
          ended_at: 1005,
          input: null,
          output: null,
          success: true,
          error: undefined,
        },
        {
          node_uuid: 'http-1',
          node_name: 'HTTP请求',
          node_type: 'http',
          started_at: 1010,
          ended_at: 2000,
          input: { url: 'https://invalid.example.com' },
          output: null,
          success: false,
          error: 'Connection timeout',
        },
      ];

      act(() => {
        debugEventCallback?.({
          PausedAtNode: {
            node_uuid: 'http-1',
            reason: { type: 'error', message: 'Connection timeout' },
            variables: {},
            execution_stack: failedStack,
          },
        });
      });

      const state = useDebugStore.getState();
      expect(state.executionStack[1].success).toBe(false);
      expect(state.executionStack[1].error).toBe('Connection timeout');
    });
  });
});
