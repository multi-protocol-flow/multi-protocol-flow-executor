/**
 * Tauri IPC 调试服务测试
 * 测试 debug.ts 中的调试相关 IPC 调用
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  startDebug,
  sendDebugCommand,
  endDebug,
  get_debug_status,
  force_end_debug,
} from '@/services/tauri/debug';
import { IpcError } from '@/services/tauri/flow';
import type { Flow } from '@/types/flow';
import type { DebugCommand } from '@/types/debug';

// Mock @tauri-apps/api/core
const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
  Channel: vi.fn().mockImplementation(() => ({
    onmessage: null,
  })),
}));

// 测试用流程对象
function createTestFlow(): Flow {
  return {
    uuid: 'test-flow-uuid',
    name: '测试流程',
    nodes: [
      {
        uuid: 'entry-node-uuid',
        type: 'entry',
        name: '入口节点',
        position: { x: 0, y: 0 },
        data: { type: 'entry' },
      },
    ],
    connections: [],
    initial_variables: {},
  };
}

describe('Tauri IPC 调试服务', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('startDebug', () => {
    it('应该调用 invoke 并传递正确的参数', async () => {
      const flow = createTestFlow();
      const breakpoints = ['node-uuid-1', 'node-uuid-2'];
      const onEvent = vi.fn();
      const onLog = vi.fn();

      mockInvoke.mockResolvedValueOnce(undefined);

      await startDebug(flow, breakpoints, onEvent, onLog);

      expect(mockInvoke).toHaveBeenCalledTimes(1);
      const callArgs = mockInvoke.mock.calls[0];
      expect(callArgs[0]).toBe('start_debug');
      expect(callArgs[1]).toMatchObject({
        flow,
        breakpoints,
      });
      // 验证 onEvent 和 onLog 是包含 onmessage 属性的对象
      expect(callArgs[1]).toHaveProperty('onEvent.onmessage');
      expect(callArgs[1]).toHaveProperty('onLog.onmessage');
    });

    it('应该成功启动调试会话', async () => {
      const flow = createTestFlow();
      const breakpoints: string[] = [];
      const onEvent = vi.fn();
      const onLog = vi.fn();

      mockInvoke.mockResolvedValueOnce(undefined);

      await expect(startDebug(flow, breakpoints, onEvent, onLog)).resolves.not.toThrow();
    });

    it('应该处理字符串类型的错误', async () => {
      const flow = createTestFlow();
      const breakpoints: string[] = [];
      const onEvent = vi.fn();
      const onLog = vi.fn();

      mockInvoke.mockRejectedValueOnce('调试会话已存在');

      try {
        await startDebug(flow, breakpoints, onEvent, onLog);
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as Error).message).toBe('调试会话已存在');
      }
    });

    it('应该处理 Error 类型的错误', async () => {
      const flow = createTestFlow();
      const breakpoints: string[] = [];
      const onEvent = vi.fn();
      const onLog = vi.fn();

      mockInvoke.mockRejectedValueOnce(new Error('连接失败'));

      try {
        await startDebug(flow, breakpoints, onEvent, onLog);
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as Error).message).toBe('连接失败');
      }
    });

    it('应该处理未知类型的错误', async () => {
      const flow = createTestFlow();
      const breakpoints: string[] = [];
      const onEvent = vi.fn();
      const onLog = vi.fn();

      mockInvoke.mockRejectedValueOnce({ code: 500 });

      try {
        await startDebug(flow, breakpoints, onEvent, onLog);
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as Error).message).toContain('Unknown error');
      }
    });

    it('IpcError 应该包含正确的 command 信息', async () => {
      const flow = createTestFlow();
      const breakpoints: string[] = [];
      const onEvent = vi.fn();
      const onLog = vi.fn();

      mockInvoke.mockRejectedValueOnce('测试错误');

      try {
        await startDebug(flow, breakpoints, onEvent, onLog);
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as IpcError).command).toBe('start_debug');
      }
    });
  });

  describe('sendDebugCommand', () => {
    it('应该调用 invoke 并传递正确的命令', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      const command: DebugCommand = 'Continue';
      await sendDebugCommand(command);

      expect(mockInvoke).toHaveBeenCalledTimes(1);
      expect(mockInvoke).toHaveBeenCalledWith('send_debug_command', { command });
    });

    it('应该支持 StepOver 命令', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      const command: DebugCommand = 'StepOver';
      await expect(sendDebugCommand(command)).resolves.not.toThrow();
    });

    it('应该支持 EndDebug 命令', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      const command: DebugCommand = 'EndDebug';
      await expect(sendDebugCommand(command)).resolves.not.toThrow();
    });

    it('应该支持 SkipNode 命令', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      const command: DebugCommand = {
        SkipNode: { selected_port: 'output-true', simulated_output: { status: 200 } },
      };
      await expect(sendDebugCommand(command)).resolves.not.toThrow();
    });

    it('应该处理字符串类型的错误', async () => {
      mockInvoke.mockRejectedValueOnce('无效的调试命令');

      try {
        await sendDebugCommand('Continue');
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as Error).message).toBe('无效的调试命令');
      }
    });

    it('应该处理 Error 类型的错误', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('命令执行失败'));

      try {
        await sendDebugCommand('Continue');
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as Error).message).toBe('命令执行失败');
      }
    });

    it('IpcError 应该包含正确的 command 信息', async () => {
      mockInvoke.mockRejectedValueOnce('测试错误');

      try {
        await sendDebugCommand('Continue');
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as IpcError).command).toBe('send_debug_command');
      }
    });
  });

  describe('endDebug', () => {
    it('应该调用 invoke 的 end_debug 命令', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await endDebug();

      expect(mockInvoke).toHaveBeenCalledTimes(1);
      expect(mockInvoke).toHaveBeenCalledWith('end_debug');
    });

    it('应该成功结束调试会话', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await expect(endDebug()).resolves.not.toThrow();
    });

    it('应该处理字符串类型的错误', async () => {
      mockInvoke.mockRejectedValueOnce('无活跃的调试会话');

      try {
        await endDebug();
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as Error).message).toBe('无活跃的调试会话');
      }
    });

    it('应该处理 Error 类型的错误', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('结束调试失败'));

      try {
        await endDebug();
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as Error).message).toBe('结束调试失败');
      }
    });

    it('IpcError 应该包含正确的 command 信息', async () => {
      mockInvoke.mockRejectedValueOnce('测试错误');

      try {
        await endDebug();
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as IpcError).command).toBe('end_debug');
      }
    });
  });

  describe('get_debug_status', () => {
    it('应该调用 invoke 的 get_debug_status 命令', async () => {
      mockInvoke.mockResolvedValueOnce(true);

      await get_debug_status();

      expect(mockInvoke).toHaveBeenCalledTimes(1);
      expect(mockInvoke).toHaveBeenCalledWith('get_debug_status');
    });

    it('应该返回 true 当有活跃的调试会话', async () => {
      mockInvoke.mockResolvedValueOnce(true);

      const status = await get_debug_status();
      expect(status).toBe(true);
    });

    it('应该返回 false 当没有活跃的调试会话', async () => {
      mockInvoke.mockResolvedValueOnce(false);

      const status = await get_debug_status();
      expect(status).toBe(false);
    });

    it('应该处理字符串类型的错误', async () => {
      mockInvoke.mockRejectedValueOnce('获取状态失败');

      try {
        await get_debug_status();
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as Error).message).toBe('获取状态失败');
      }
    });

    it('应该处理 Error 类型的错误', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('状态查询失败'));

      try {
        await get_debug_status();
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as Error).message).toBe('状态查询失败');
      }
    });

    it('IpcError 应该包含正确的 command 信息', async () => {
      mockInvoke.mockRejectedValueOnce('测试错误');

      try {
        await get_debug_status();
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as IpcError).command).toBe('get_debug_status');
      }
    });
  });

  describe('force_end_debug', () => {
    it('应该调用 invoke 的 force_end_debug 命令', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await force_end_debug();

      expect(mockInvoke).toHaveBeenCalledTimes(1);
      expect(mockInvoke).toHaveBeenCalledWith('force_end_debug');
    });

    it('应该成功强制结束调试会话', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await expect(force_end_debug()).resolves.not.toThrow();
    });

    it('应该处理字符串类型的错误', async () => {
      mockInvoke.mockRejectedValueOnce('强制结束失败');

      try {
        await force_end_debug();
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as Error).message).toBe('强制结束失败');
      }
    });

    it('应该处理 Error 类型的错误', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('清理状态失败'));

      try {
        await force_end_debug();
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as Error).message).toBe('清理状态失败');
      }
    });

    it('IpcError 应该包含正确的 command 信息', async () => {
      mockInvoke.mockRejectedValueOnce('测试错误');

      try {
        await force_end_debug();
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as IpcError).command).toBe('force_end_debug');
      }
    });
  });

  describe('IpcError 错误类', () => {
    it('应该正确设置错误名称', async () => {
      mockInvoke.mockRejectedValueOnce('错误信息');

      try {
        await endDebug();
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as IpcError).name).toBe('IpcError');
      }
    });

    it('应该保留原始错误作为 cause', async () => {
      const originalError = new Error('原始错误');
      mockInvoke.mockRejectedValueOnce(originalError);

      try {
        await endDebug();
        expect.fail('应该抛出错误');
      } catch (error) {
        expect(error).toBeInstanceOf(IpcError);
        expect((error as IpcError).cause).toBe(originalError);
      }
    });
  });
});
