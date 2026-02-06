import { describe, it, expect } from 'vitest';
import type { Connection, Edge } from '@xyflow/react';

/**
 * isValidConnection 函数测试
 *
 * 这些测试验证节点连接验证逻辑：
 * - 拒绝自连接（节点连接到自身）
 * - 拒绝重复连接（相同的 source-target 已存在）
 * - 接受有效连接
 */

/**
 * isValidConnection 实现（从 App.tsx 复制）
 *
 * @param connection - 要创建的连接
 * @param edges - 现有的边列表
 * @returns 是否允许连接
 */
function isValidConnection(
  connection: Connection,
  edges: Edge[]
): boolean {
  // 防止自连接
  if (connection.source === connection.target) {
    return false;
  }

  // 防止重复连接
  const duplicateExists = edges.some(
    (edge) => edge.source === connection.source && edge.target === connection.target
  );

  if (duplicateExists) {
    return false;
  }

  return true;
}

describe('isValidConnection', () => {
  describe('应该拒绝自连接', () => {
    it('当 source 等于 target 时返回 false', () => {
      const connection: Connection = {
        source: 'node-1',
        target: 'node-1',
        sourceHandle: null,
        targetHandle: null,
      };

      const edges: Edge[] = [];

      expect(isValidConnection(connection, edges)).toBe(false);
    });

    it('即使 edges 为空也拒绝自连接', () => {
      const connection: Connection = {
        source: 'node-1',
        target: 'node-1',
        sourceHandle: null,
        targetHandle: null,
      };

      const edges: Edge[] = [];

      expect(isValidConnection(connection, edges)).toBe(false);
    });
  });

  describe('应该拒绝重复连接', () => {
    it('当相同的 source-target 已存在时返回 false', () => {
      const connection: Connection = {
        source: 'node-1',
        target: 'node-2',
        sourceHandle: null,
        targetHandle: null,
      };

      const edges: Edge[] = [
        {
          id: 'edge-1',
          source: 'node-1',
          target: 'node-2',
          type: 'smoothstep',
        },
      ];

      expect(isValidConnection(connection, edges)).toBe(false);
    });

    it('当有多条边时检测重复连接', () => {
      const connection: Connection = {
        source: 'node-2',
        target: 'node-3',
        sourceHandle: null,
        targetHandle: null,
      };

      const edges: Edge[] = [
        {
          id: 'edge-1',
          source: 'node-1',
          target: 'node-2',
          type: 'smoothstep',
        },
        {
          id: 'edge-2',
          source: 'node-2',
          target: 'node-3',
          type: 'smoothstep',
        },
        {
          id: 'edge-3',
          source: 'node-3',
          target: 'node-4',
          type: 'smoothstep',
        },
      ];

      expect(isValidConnection(connection, edges)).toBe(false);
    });
  });

  describe('应该接受有效连接', () => {
    it('当 edges 为空且非自连接时返回 true', () => {
      const connection: Connection = {
        source: 'node-1',
        target: 'node-2',
        sourceHandle: null,
        targetHandle: null,
      };

      const edges: Edge[] = [];

      expect(isValidConnection(connection, edges)).toBe(true);
    });

    it('当 source-target 不存在时返回 true', () => {
      const connection: Connection = {
        source: 'node-1',
        target: 'node-2',
        sourceHandle: null,
        targetHandle: null,
      };

      const edges: Edge[] = [
        {
          id: 'edge-1',
          source: 'node-2',
          target: 'node-3',
          type: 'smoothstep',
        },
      ];

      expect(isValidConnection(connection, edges)).toBe(true);
    });

    it('当有多个边但都不匹配时返回 true', () => {
      const connection: Connection = {
        source: 'node-4',
        target: 'node-5',
        sourceHandle: null,
        targetHandle: null,
      };

      const edges: Edge[] = [
        {
          id: 'edge-1',
          source: 'node-1',
          target: 'node-2',
          type: 'smoothstep',
        },
        {
          id: 'edge-2',
          source: 'node-2',
          target: 'node-3',
          type: 'smoothstep',
        },
        {
          id: 'edge-3',
          source: 'node-3',
          target: 'node-4',
          type: 'smoothstep',
        },
      ];

      expect(isValidConnection(connection, edges)).toBe(true);
    });

    it('允许相反方向的连接（target-source）', () => {
      // 测试反向连接是否被允许
      // 根据当前实现，只检查 exact match (source, target)
      // 所以反向连接应该被允许

      const connection1: Connection = {
        source: 'node-1',
        target: 'node-2',
        sourceHandle: null,
        targetHandle: null,
      };

      const connection2: Connection = {
        source: 'node-2',
        target: 'node-1',
        sourceHandle: null,
        targetHandle: null,
      };

      const edges: Edge[] = [
        {
          id: 'edge-1',
          source: 'node-1',
          target: 'node-2',
          type: 'smoothstep',
        },
      ];

      // 第一个连接被拒绝（重复）
      expect(isValidConnection(connection1, edges)).toBe(false);

      // 第二个连接被允许（反向）
      expect(isValidConnection(connection2, edges)).toBe(true);
    });
  });

  describe('边界情况', () => {
    it('处理空字符串 ID', () => {
      const connection: Connection = {
        source: '',
        target: '',
        sourceHandle: null,
        targetHandle: null,
      };

      const edges: Edge[] = [];

      // 自连接（空字符串 === 空字符串）
      expect(isValidConnection(connection, edges)).toBe(false);
    });

    it('处理带有特殊字符的 ID', () => {
      const connection: Connection = {
        source: 'node-1-with-dash',
        target: 'node-2_with_underscore',
        sourceHandle: null,
        targetHandle: null,
      };

      const edges: Edge[] = [];

      expect(isValidConnection(connection, edges)).toBe(true);
    });
  });
});
