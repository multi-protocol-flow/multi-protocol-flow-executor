import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  extractVariablesFromScript,
  extractNodeExportedVariables,
  extractFlowLocalVariables,
  loadAndResolveSubFlowVariables,
  clearSubFlowVariablesCache,
} from '@/lib/variableExtraction';
import type { Flow, Node } from '@/types';
import { flowStorage } from '@/services/storage';

vi.mock('@/services/storage', () => ({
  flowStorage: {
    loadFlow: vi.fn(),
  },
}));

describe('variableExtraction', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearSubFlowVariablesCache();
  });

  describe('extractVariablesFromScript', () => {
    it('处理空输入或无效类型', () => {
      expect(extractVariablesFromScript(null)).toEqual([]);
      expect(extractVariablesFromScript(undefined)).toEqual([]);
      expect(extractVariablesFromScript('')).toEqual([]);
    });

    it('支持单引号、双引号与反引号的 fn.variables.set', () => {
      const script = `
        fn.variables.set('token', 'secret123');
        fn.variables.set("user_id", 42);
        fn.variables.set(\`session_key\`, 'abc');
        context.variables.set('legacy_key', 'val');
      `;
      const res = extractVariablesFromScript(script);
      expect(res).toEqual(['token', 'user_id', 'session_key', 'legacy_key']);
    });

    it('支持 fn.variables.setAll 批量设置', () => {
      const script = `
        fn.variables.setAll({
          role: 'admin',
          'access_token': 'xyz',
          "client_id": 999
        });
      `;
      const res = extractVariablesFromScript(script);
      expect(res).toEqual(['role', 'access_token', 'client_id']);
    });

    it('去重重复设置的变量名', () => {
      const script = `
        fn.variables.set('dup_var', 1);
        fn.variables.set('dup_var', 2);
        fn.variables.setAll({ dup_var: 3, other: 4 });
      `;
      const res = extractVariablesFromScript(script);
      expect(res).toEqual(['dup_var', 'other']);
    });
  });

  describe('extractNodeExportedVariables', () => {
    it('提取 variable_extractor 节点的 output_mappings', () => {
      const node: Node = {
        uuid: 'extractor-1',
        name: '提取Token',
        type: 'variable:extractor',
        position: { x: 0, y: 0 },
        data: {
          type: 'variable:extractor',
          output_mappings: [
            { source: 'data.token', target: 'token', description: '用户凭证' },
            { source: 'data.id', target: 'uid' },
          ],
        },
      };

      const res = extractNodeExportedVariables(node, '登录流程');
      expect(res).toHaveLength(2);
      expect(res[0]).toEqual({
        name: 'token',
        value: '{{data.token}}',
        sourceNodeUuid: 'extractor-1',
        description: '用户凭证',
      });
      expect(res[1]).toEqual({
        name: 'uid',
        value: '{{data.id}}',
        sourceNodeUuid: 'extractor-1',
        description: '[登录流程] 提取Token',
      });
    });

    it('提取 script 节点导出的变量', () => {
      const node: Node = {
        uuid: 'script-1',
        name: '预处理',
        type: 'script',
        position: { x: 0, y: 0 },
        data: {
          type: 'script',
          script: "fn.variables.set('sign', 'sig_123');",
          timeout_ms: 1000,
        },
      };

      const res = extractNodeExportedVariables(node);
      expect(res).toHaveLength(1);
      expect(res[0]).toEqual({
        name: 'sign',
        value: undefined,
        sourceNodeUuid: 'script-1',
        description: '预处理',
      });
    });

    it('提取非全部导出子流程的显式 output_mappings', () => {
      const node: Node = {
        uuid: 'subflow-1',
        name: '认证子流程',
        type: 'flow:sub',
        position: { x: 0, y: 0 },
        data: {
          type: 'flow:sub',
          target_flow_id: 'target-1',
          export_all_variables: false,
          output_mappings: [
            { source: 'child_token', target: 'auth_token' },
          ],
        },
      };

      const res = extractNodeExportedVariables(node);
      expect(res).toHaveLength(1);
      expect(res[0]).toEqual({
        name: 'auth_token',
        value: '{{child_token}}',
        sourceNodeUuid: 'subflow-1',
        description: '认证子流程',
      });
    });
  });

  describe('extractFlowLocalVariables', () => {
    it('提取流程的 initial_variables 以及 Setup/Main/Teardown 节点的变量', () => {
      const flow: Flow = {
        uuid: 'flow-main',
        name: '主流程',
        initial_variables: { env_base: 'https://api.test' },
        setup: {
          nodes: [
            {
              uuid: 'setup-script',
              name: 'Setup脚本',
              type: 'script',
              position: { x: 0, y: 0 },
              data: {
                type: 'script',
                script: "fn.variables.set('setup_token', 'abc');",
                timeout_ms: 1000,
              },
            },
          ],
          connections: [],
        },
        nodes: [
          {
            uuid: 'main-script',
            name: 'Main脚本',
            type: 'script',
            position: { x: 0, y: 0 },
            data: {
              type: 'script',
              script: "fn.variables.set('main_res', 123);",
              timeout_ms: 1000,
            },
          },
        ],
        connections: [],
      };

      const res = extractFlowLocalVariables(flow);
      expect(res.map((v) => v.name)).toEqual(['env_base', 'setup_token', 'main_res']);
    });
  });

  describe('loadAndResolveSubFlowVariables', () => {
    it('加载并提取目标流程变量，且防止循环引用', async () => {
      const mockChildFlow: Flow = {
        uuid: 'child-flow-id',
        name: '子流程',
        initial_variables: { child_var: 'val' },
        nodes: [
          {
            uuid: 'child-script',
            name: '子流程脚本',
            type: 'script',
            position: { x: 0, y: 0 },
            data: {
              type: 'script',
              script: "fn.variables.set('child_token', 'xyz');",
              timeout_ms: 1000,
            },
          },
          // 循环引用回父流程或自身
          {
            uuid: 'circular-sub',
            name: '循环子流程',
            type: 'flow:sub',
            position: { x: 0, y: 0 },
            data: {
              type: 'flow:sub',
              target_flow_id: 'child-flow-id',
              export_all_variables: true,
            },
          },
        ],
        connections: [],
      };

      vi.mocked(flowStorage.loadFlow).mockResolvedValue(mockChildFlow);

      const res = await loadAndResolveSubFlowVariables('child-flow-id');
      expect(res.flow).toBe(mockChildFlow);
      expect(res.variables.map((v) => v.name)).toEqual(['child_var', 'child_token']);

      // 再次调用应该命中缓存
      const cachedRes = await loadAndResolveSubFlowVariables('child-flow-id');
      expect(cachedRes.variables.map((v) => v.name)).toEqual(['child_var', 'child_token']);
      expect(flowStorage.loadFlow).toHaveBeenCalledTimes(1);
    });
  });
});
