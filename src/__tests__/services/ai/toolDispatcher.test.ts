import { describe, it, expect, beforeEach } from 'vitest';
import {
  getAllAIToolDefinitions,
  isHighRiskTool,
  buildActionDescription,
  dispatchToolCall,
} from '@/services/ai/toolDispatcher';
import { isSafeTerminalCommand } from '@/services/ai/tools/terminalTools';
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
    expect(names).toContain('get_node_detail');
    expect(names).toContain('get_node_type_schema');
    expect(names).toContain('get_flow_nodes_config');
    expect(names).toContain('diagnose_flow_errors');
    expect(names).toContain('run_terminal_command');
  });

  it('正确识别高风险操作', () => {
    expect(isHighRiskTool('clear_canvas')).toBe(true);
    expect(isHighRiskTool('delete_nodes')).toBe(true);
    expect(isHighRiskTool('install_marketplace_plugin')).toBe(true);
    expect(isHighRiskTool('add_node')).toBe(false);
    expect(isHighRiskTool('auto_layout')).toBe(false);

    // 终端命令动态判定：只读/白名单命令放行 (false)，修改/危险命令拦截 (true)
    expect(isHighRiskTool('run_terminal_command', { command: 'git status' })).toBe(false);
    expect(isHighRiskTool('run_terminal_command', { command: 'node -v' })).toBe(false);
    expect(isHighRiskTool('run_terminal_command', { command: 'npm list' })).toBe(false);
    expect(isHighRiskTool('run_terminal_command', { command: 'ls -la' })).toBe(false);
    expect(isHighRiskTool('run_terminal_command', { command: 'rm -rf ./dist' })).toBe(true);
    expect(isHighRiskTool('run_terminal_command', { command: 'npm install axios' })).toBe(true);
    expect(isHighRiskTool('run_terminal_command', { command: 'git push origin main' })).toBe(true);
  });

  it('白名单规则准确判定安全命令与高危命令', () => {
    // 白名单放行
    expect(isSafeTerminalCommand('git status')).toBe(true);
    expect(isSafeTerminalCommand('git log -n 5')).toBe(true);
    expect(isSafeTerminalCommand('cargo --version')).toBe(true);
    expect(isSafeTerminalCommand('cargo tree')).toBe(true);
    expect(isSafeTerminalCommand('Get-Location')).toBe(true);
    expect(isSafeTerminalCommand('pwd')).toBe(true);
    expect(isSafeTerminalCommand('type package.json')).toBe(true);

    // 复合管道/重定向/高危操作拦截
    expect(isSafeTerminalCommand('echo dangerous > file.txt')).toBe(false);
    expect(isSafeTerminalCommand('npm install')).toBe(false);
    expect(isSafeTerminalCommand('git commit -m "feat"')).toBe(false);
    expect(isSafeTerminalCommand('Remove-Item -Recurse ./node_modules')).toBe(false);
    expect(isSafeTerminalCommand('')).toBe(false);
  });

  it('构造高风险操作的展示描述', () => {
    const descClear = buildActionDescription('clear_canvas', {});
    expect(descClear.description).toContain('清空当前流程画布');

    const descDelete = buildActionDescription('delete_nodes', { node_uuids: ['n1', 'n2'] });
    expect(descDelete.description).toContain('删除 2 个节点');

    const descTerm = buildActionDescription('run_terminal_command', {
      command: 'npm install express',
      cwd: '/workspace/project',
    });
    expect(descTerm.description).toContain('npm install express');
    expect(descTerm.diffSummary).toContain('/workspace/project');
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

  it('分发并执行 get_node_type_schema 与 diagnose_flow_errors', async () => {
    const schemaRes = (await dispatchToolCall('get_node_type_schema', {
      node_type: 'http',
    })) as { success: boolean; node_type: string; builtin_schema: unknown };
    expect(schemaRes.success).toBe(true);
    expect(schemaRes.node_type).toBe('http');
    expect(schemaRes.builtin_schema).toBeDefined();

    const diagRes = (await dispatchToolCall('diagnose_flow_errors', {})) as {
      success: boolean;
      has_execution_report: boolean;
      instruction_for_assistant: string;
    };
    expect(diagRes.success).toBe(true);
    expect(diagRes.instruction_for_assistant).toContain('propose the suggested repair directions');
  });
});
