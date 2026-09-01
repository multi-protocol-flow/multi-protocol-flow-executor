import { describe, it, expect, beforeEach } from 'vitest';
import {
  getAllAIToolDefinitions,
  isHighRiskTool,
  buildActionDescription,
  dispatchToolCall,
} from '@/services/ai/toolDispatcher';
import { useFlowStore } from '@/stores/flow';
import type { Flow } from '@/types/flow';

describe('toolDispatcher', () => {
  beforeEach(() => {
    const initialFlow: Flow = {
      uuid: 'test-flow',
      name: 'Test Flow',
      initial_variables: {},
      environments: [],
      active_environment: undefined,
      nodes: [],
      connections: [],
    };
    useFlowStore.getState().setFlow(initialFlow);
  });

  it('获取所有工具定义', () => {
    const tools = getAllAIToolDefinitions();
    expect(tools.length).toBeGreaterThan(10);
    const names = tools.map((t) => t.function.name);
    expect(names).toContain('add_node');
    expect(names).toContain('update_node_config');
    expect(names).toContain('connect_nodes');
    expect(names).toContain('generate_or_update_script');
    expect(names).toContain('auto_layout');
    expect(names).toContain('delete_nodes');
    expect(names).toContain('clear_canvas');
  });

  it('正确识别高风险操作', () => {
    expect(isHighRiskTool('clear_canvas')).toBe(true);
    expect(isHighRiskTool('delete_nodes')).toBe(true);
    expect(isHighRiskTool('install_marketplace_plugin')).toBe(true);
    expect(isHighRiskTool('add_node')).toBe(false);
    expect(isHighRiskTool('auto_layout')).toBe(false);
  });

  it('构造高风险操作的展示描述', () => {
    const descClear = buildActionDescription('clear_canvas', {});
    expect(descClear.description).toContain('清空当前流程画布');

    const descDelete = buildActionDescription('delete_nodes', { node_uuids: ['n1', 'n2'] });
    expect(descDelete.description).toContain('删除 2 个节点');
  });

  it('成功分发并执行 add_node 和 connect_nodes 工具', async () => {
    const node1Res = (await dispatchToolCall('add_node', {
      type: 'http',
      name: 'HTTP_1',
    })) as { success: boolean; node_uuid: string };

    expect(node1Res.success).toBe(true);
    expect(node1Res.node_uuid).toBeDefined();

    const node2Res = (await dispatchToolCall('add_node', {
      type: 'script',
      name: 'SCRIPT_1',
    })) as { success: boolean; node_uuid: string };

    expect(node2Res.success).toBe(true);

    const connRes = (await dispatchToolCall('connect_nodes', {
      source_node_uuid: node1Res.node_uuid,
      target_node_uuid: node2Res.node_uuid,
    })) as { success: boolean; connection_id: string };

    expect(connRes.success).toBe(true);

    const currentFlow = useFlowStore.getState().flow;
    expect(currentFlow?.nodes).toHaveLength(2);
    expect(currentFlow?.connections).toHaveLength(1);
  });
});
