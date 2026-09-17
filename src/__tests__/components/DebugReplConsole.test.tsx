/**
 * DebugReplConsole 单元测试
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import {
  DebugReplConsole,
  evaluateJsonPath,
  evaluateJsExpression,
} from '@/components/execution/DebugReplConsole';
import { useDebugStore } from '@/stores/debug';

describe('DebugReplConsole', () => {
  beforeEach(() => {
    useDebugStore.getState().reset();
    vi.clearAllMocks();
  });

  describe('求值器算法单元测试', () => {
    it('evaluateJsonPath 正确提取深层与数组元素', () => {
      const data = {
        user: {
          id: 1001,
          name: 'Alice',
          roles: ['admin', 'editor'],
        },
        status: 200,
      };

      expect(evaluateJsonPath(data, '$.user.id')).toBe(1001);
      expect(evaluateJsonPath(data, '$.user.name')).toBe('Alice');
      expect(evaluateJsonPath(data, '$.user.roles[0]')).toBe('admin');
      expect(evaluateJsonPath(data, '$.status')).toBe(200);
      expect(evaluateJsonPath(data, '$.nonexistent')).toBeUndefined();
    });

    it('evaluateJsExpression 正确在沙箱中求值', () => {
      const vars = {
        count: 5,
        price: 20,
        status: 'PAID',
        items: [1, 2, 3],
      };

      expect(evaluateJsExpression('count * price', vars)).toBe(100);
      expect(evaluateJsExpression('status === "PAID"', vars)).toBe(true);
      expect(evaluateJsExpression('$.items.length', vars)).toBe(3);
    });
  });

  describe('UI 交互与状态绑定测试', () => {
    it('非暂停状态下显示禁用提示', () => {
      useDebugStore.setState({ status: 'running' });
      render(<DebugReplConsole />);

      expect(screen.getByText('运行中 · 仅在暂停时可求值')).toBeInTheDocument();
    });

    it('暂停状态下输入框激活并可执行求值', async () => {
      useDebugStore.setState({
        status: 'paused',
        variables: { token: 'secret-xyz', user_id: 42 },
      });

      render(<DebugReplConsole />);

      // 点击展开标题栏
      fireEvent.click(screen.getByText('断点即时 REPL 求值器'));

      const input = screen.getByPlaceholderText(/输入表达式/);
      expect(input).not.toBeDisabled();

      // 输入表达式并回车
      fireEvent.change(input, { target: { value: 'user_id * 2' } });
      fireEvent.keyDown(input, { key: 'Enter' });

      // 应输出求值结果 84
      expect(screen.getByText('84')).toBeInTheDocument();
    });

    it('支持输入历史上下键切换', () => {
      useDebugStore.setState({
        status: 'paused',
        variables: { a: 1, b: 2 },
      });

      render(<DebugReplConsole />);
      fireEvent.click(screen.getByText('断点即时 REPL 求值器'));

      const input = screen.getByPlaceholderText(/输入表达式/) as HTMLInputElement;

      // 评估第一个表达式
      fireEvent.change(input, { target: { value: 'a + 10' } });
      fireEvent.keyDown(input, { key: 'Enter' });

      // 评估第二个表达式
      fireEvent.change(input, { target: { value: 'b + 20' } });
      fireEvent.keyDown(input, { key: 'Enter' });

      // 按上键：应调出上一个输入的表达式 'b + 20'
      fireEvent.keyDown(input, { key: 'ArrowUp' });
      expect(input.value).toBe('b + 20');

      // 再次按上键：应调出更早的 'a + 10'
      fireEvent.keyDown(input, { key: 'ArrowUp' });
      expect(input.value).toBe('a + 10');

      // 按下键：应前进到 'b + 20'
      fireEvent.keyDown(input, { key: 'ArrowDown' });
      expect(input.value).toBe('b + 20');
    });
  });
});
