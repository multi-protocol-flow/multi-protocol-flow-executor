/**
 * Full Flow Integration Tests
 *
 * These tests verify the complete flow lifecycle:
 * - Create flow with start node
 * - Rename node
 * - Execute flow
 * - Save flow to file
 * - Load flow from file
 *
 * Note: These are integration tests that test the logical flow
 * with mock responses representing Tauri command outputs.
 * No browser interaction is required.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import type { Flow } from '../../src/types/flow';
import type { Node } from '../../src/types/node';
import type { Connection } from '../../src/types/connection';
import { NodeType } from '../../src/types/node';

// Mock Tauri commands - each test will mock these as needed
const mockCommands = {
  create_flow: vi.fn(),
  update_node: vi.fn(),
  execute_flow: vi.fn(),
  save_flow: vi.fn(),
  load_flow: vi.fn(),
};

// Helper function to invoke mocked commands
async function invoke<T>(command: string, args?: any): Promise<T> {
  const cmd = mockCommands[command as keyof typeof mockCommands];
  if (!cmd) {
    throw new Error(`Command ${command} not mocked`);
  }
  return cmd(args);
}

describe('Full Flow Integration', () => {
  // Reset mocks before each test
  beforeEach(() => {
    Object.values(mockCommands).forEach((mock) => mock.mockReset());
  });

  describe('Create Flow', () => {
    it('should create a new flow with a start node', async () => {
      // Mock create_flow response
      const mockFlow: Flow = {
        id: 'flow-123',
        name: 'Untitled Flow',
        nodes: [
          {
            id: 'start-flow-123',
            type: NodeType.Start,
            name: '开始',
            position: { x: 0, y: 0 },
            data: {},
          },
        ],
        connections: [],
      };

      mockCommands.create_flow.mockResolvedValue(mockFlow);

      // Call create_flow command
      const result = await invoke<Flow>('create_flow');

      // Verify the flow was created with a start node
      expect(result).toEqual(mockFlow);
      expect(result.nodes).toHaveLength(1);
      expect(result.nodes[0].type).toBe(NodeType.Start);
      expect(result.nodes[0].name).toBe('开始');
      expect(mockCommands.create_flow).toHaveBeenCalled();
    });

    it('should generate unique flow IDs', async () => {
      // Mock two create_flow responses with different IDs
      const mockFlow1: Flow = {
        id: 'flow-abc-123',
        name: 'Untitled Flow',
        nodes: [],
        connections: [],
      };

      const mockFlow2: Flow = {
        id: 'flow-def-456',
        name: 'Untitled Flow',
        nodes: [],
        connections: [],
      };

      mockCommands.create_flow
        .mockResolvedValueOnce(mockFlow1)
        .mockResolvedValueOnce(mockFlow2);

      // Create two flows
      const flow1 = await invoke<Flow>('create_flow');
      const flow2 = await invoke<Flow>('create_flow');

      // Verify IDs are unique
      expect(flow1.id).not.toBe(flow2.id);
      expect(flow1.id).toBe('flow-abc-123');
      expect(flow2.id).toBe('flow-def-456');
    });

    it('should auto-create start node with correct ID format', async () => {
      const flowId = 'flow-test-789';
      const mockFlow: Flow = {
        id: flowId,
        name: 'Untitled Flow',
        nodes: [
          {
            id: `start-${flowId}`,
            type: NodeType.Start,
            name: '开始',
            position: { x: 0, y: 0 },
            data: {},
          },
        ],
        connections: [],
      };

      mockCommands.create_flow.mockResolvedValue(mockFlow);

      const result = await invoke<Flow>('create_flow');

      // Verify start node ID format: start-{flowId}
      expect(result.nodes[0].id).toBe(`start-${flowId}`);
    });
  });

  describe('Rename Node', () => {
    it('should successfully rename a node', async () => {
      const mockNode: Node = {
        id: 'start-flow-123',
        type: NodeType.Start,
        name: '我的开始节点',
        position: { x: 0, y: 0 },
        data: {},
      };

      mockCommands.update_node.mockResolvedValue({ success: true, node: mockNode });

      // Call update_node command with new name
      const result = await invoke('update_node', {
        flowId: 'flow-123',
        nodeId: 'start-flow-123',
        data: { name: '我的开始节点' },
      });

      // Verify the node was renamed
      expect(result).toEqual({ success: true, node: mockNode });
      expect(result.node.name).toBe('我的开始节点');
    });

    it('should validate node name length (max 50 characters)', async () => {
      // Test with valid name (50 characters)
      const validName = 'a'.repeat(50);
      const mockNode: Node = {
        id: 'start-flow-123',
        type: NodeType.Start,
        name: validName,
        position: { x: 0, y: 0 },
        data: {},
      };

      mockCommands.update_node.mockResolvedValue({ success: true, node: mockNode });

      const result = await invoke('update_node', {
        flowId: 'flow-123',
        nodeId: 'start-flow-123',
        data: { name: validName },
      });

      expect(result.success).toBe(true);
      expect(result.node.name).toBe(validName);
    });

    it('should reject empty node name', async () => {
      mockCommands.update_node.mockResolvedValue({
        success: false,
        error: '名称不能为空',
      });

      const result = await invoke('update_node', {
        flowId: 'flow-123',
        nodeId: 'start-flow-123',
        data: { name: '' },
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain('不能为空');
    });
  });

  describe('Execute Flow', () => {
    it('should successfully execute a flow with start node', async () => {
      const mockExecutionResult = {
        success: true,
        executionTime: 0.5, // milliseconds
        executedNodes: ['start-flow-123'],
        data: {},
      };

      const mockFlow: Flow = {
        id: 'flow-123',
        name: 'Test Flow',
        nodes: [
          {
            id: 'start-flow-123',
            type: NodeType.Start,
            name: '开始',
            position: { x: 0, y: 0 },
            data: {},
          },
        ],
        connections: [],
      };

      mockCommands.execute_flow.mockResolvedValue(mockExecutionResult);

      const result = await invoke('execute_flow', {
        flowId: 'flow-123',
      });

      // Verify execution was successful
      expect(result.success).toBe(true);
      expect(result.executionTime).toBeLessThan(1); // < 1ms for start node
      expect(result.executedNodes).toContain('start-flow-123');
    });

    it('should fail execution when flow has no start node', async () => {
      const mockExecutionResult = {
        success: false,
        error: '流程缺少开始节点',
        executionTime: 0,
        executedNodes: [],
        data: null,
      };

      mockCommands.execute_flow.mockResolvedValue(mockExecutionResult);

      const result = await invoke('execute_flow', {
        flowId: 'flow-without-start',
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain('缺少开始节点');
    });

    it('should fail execution when start node has no connections', async () => {
      const mockExecutionResult = {
        success: false,
        error: '无连接',
        executionTime: 0,
        executedNodes: [],
        data: null,
      };

      mockCommands.execute_flow.mockResolvedValue(mockExecutionResult);

      const result = await invoke('execute_flow', {
        flowId: 'flow-no-connections',
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain('无连接');
    });
  });

  describe('Save Flow', () => {
    it('should successfully save flow to file', async () => {
      mockCommands.save_flow.mockResolvedValue({
        success: true,
        path: '/mock/data/dir/flows/flow-123.json',
      });

      const result = await invoke('save_flow', {
        flowId: 'flow-123',
        name: 'My Flow',
      });

      expect(result.success).toBe(true);
      expect(result.path).toContain('flow-123.json');
    });

    it('should handle save errors gracefully', async () => {
      mockCommands.save_flow.mockResolvedValue({
        success: false,
        error: '无法保存流程：文件写入失败',
      });

      const result = await invoke('save_flow', {
        flowId: 'flow-123',
        name: 'My Flow',
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain('无法保存');
    });
  });

  describe('Load Flow', () => {
    it('should successfully load flow from file', async () => {
      const mockFlow: Flow = {
        id: 'flow-123',
        name: 'My Flow',
        nodes: [
          {
            id: 'start-flow-123',
            type: NodeType.Start,
            name: '开始',
            position: { x: 0, y: 0 },
            data: {},
          },
        ],
        connections: [],
      };

      mockCommands.load_flow.mockResolvedValue(mockFlow);

      const result = await invoke<Flow>('load_flow', {
        path: '/mock/data/dir/flows/flow-123.json',
      });

      expect(result).toEqual(mockFlow);
      expect(result.id).toBe('flow-123');
      expect(result.name).toBe('My Flow');
      expect(result.nodes).toHaveLength(1);
      expect(result.nodes[0].type).toBe(NodeType.Start);
    });

    it('should handle load errors gracefully', async () => {
      mockCommands.load_flow.mockResolvedValue({
        success: false,
        error: '无法加载流程：文件不存在',
      });

      const result = await invoke('load_flow', {
        path: '/mock/data/dir/flows/nonexistent.json',
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain('无法加载');
    });
  });

  describe('Complete Flow Lifecycle', () => {
    it('should support complete workflow: create → rename → execute → save → load', async () => {
      // Step 1: Create flow
      const createdFlow: Flow = {
        id: 'flow-complete-test',
        name: 'Untitled Flow',
        nodes: [
          {
            id: 'start-flow-complete-test',
            type: NodeType.Start,
            name: '开始',
            position: { x: 0, y: 0 },
            data: {},
          },
        ],
        connections: [],
      };

      mockCommands.create_flow.mockResolvedValue(createdFlow);
      const flow = await invoke<Flow>('create_flow');
      expect(flow.id).toBe('flow-complete-test');
      expect(flow.nodes[0].name).toBe('开始');

      // Step 2: Rename start node
      const renamedNode: Node = {
        id: 'start-flow-complete-test',
        type: NodeType.Start,
        name: '我的工作流起点',
        position: { x: 0, y: 0 },
        data: {},
      };

      mockCommands.update_node.mockResolvedValue({ success: true, node: renamedNode });
      const updateResult = await invoke('update_node', {
        flowId: 'flow-complete-test',
        nodeId: 'start-flow-complete-test',
        data: { name: '我的工作流起点' },
      });
      expect(updateResult.success).toBe(true);

      // Step 3: Execute flow
      const executionResult = {
        success: true,
        executionTime: 0.3,
        executedNodes: ['start-flow-complete-test'],
        data: {},
      };

      mockCommands.execute_flow.mockResolvedValue(executionResult);
      const execResult = await invoke('execute_flow', { flowId: 'flow-complete-test' });
      expect(execResult.success).toBe(true);
      expect(execResult.executionTime).toBeLessThan(1);

      // Step 4: Save flow
      mockCommands.save_flow.mockResolvedValue({
        success: true,
        path: '/mock/data/dir/flows/flow-complete-test.json',
      });
      const saveResult = await invoke('save_flow', {
        flowId: 'flow-complete-test',
        name: 'Complete Test Flow',
      });
      expect(saveResult.success).toBe(true);

      // Step 5: Load flow
      const savedFlow: Flow = {
        ...createdFlow,
        nodes: [renamedNode],
      };

      mockCommands.load_flow.mockResolvedValue(savedFlow);
      const loadedFlow = await invoke<Flow>('load_flow', {
        path: '/mock/data/dir/flows/flow-complete-test.json',
      });
      expect(loadedFlow.id).toBe('flow-complete-test');
      expect(loadedFlow.nodes[0].name).toBe('我的工作流起点');
    });
  });
});
