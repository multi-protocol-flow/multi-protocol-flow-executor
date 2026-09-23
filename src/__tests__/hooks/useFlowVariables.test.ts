/**
 * useFlowVariables Hook 测试
 * 覆盖：无流程空结果、三类来源合并（变量提取节点/流程初始变量/激活环境变量）、
 * 变量名去重、节点标签映射、流程切换后结果更新
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { act, renderHook, waitFor } from '@testing-library/react';
import { useFlowVariables } from '@/hooks/useFlowVariables';
import { useFlowStore, useEnvironmentStore } from '@/stores';
import { createFlow, createNode } from '@/test-utils';
import { flowStorage } from '@/services/storage';
import type { Flow } from '@/types';

vi.mock('@/services/storage', () => ({
  flowStorage: {
    loadFlow: vi.fn(),
  },
}));
describe('useFlowVariables', () => {
  beforeEach(() => {
    useFlowStore.getState().reset();
    useEnvironmentStore.getState().reset();
  });

  it('无流程时返回空结果', () => {
    const { result } = renderHook(() => useFlowVariables());
    expect(result.current.variables).toEqual([]);
    expect(result.current.availableVariableNames).toEqual([]);
    expect(result.current.nodeLabels).toEqual({});
  });

  it('合并变量提取节点、流程初始变量与激活环境变量', () => {
    const extractor = createNode({
      uuid: 'node-extractor',
      type: 'variable:extractor',
      name: '变量提取',
      data: {
        type: 'variable:extractor',
        output_mappings: [
          { source: 'body.id', target: 'user_id', description: '用户 ID' },
          { source: 'body.name', target: 'user_name', description: '用户姓名' },
        ],
      },
    });

    act(() => {
      useFlowStore.setState({
        flow: createFlow({
          nodes: [extractor],
          initial_variables: { api_key: 'flow-secret' },
        }),
      });
      useEnvironmentStore.setState({
        activeEnvironmentName: 'dev',
        activeEnvironmentData: {
          name: 'dev',
          variables: { api_key: 'dev-key', debug: true },
        },
      });
    });

    const { result } = renderHook(() => useFlowVariables());
    const { variables, availableVariableNames, nodeLabels } = result.current;

    // 变量提取节点的 output_mappings
    expect(variables).toContainEqual({
      name: 'user_id',
      value: '{{body.id}}',
      sourceNodeUuid: 'node-extractor',
      description: '用户 ID',
    });
    expect(variables).toContainEqual({
      name: 'user_name',
      value: '{{body.name}}',
      sourceNodeUuid: 'node-extractor',
      description: '用户姓名',
    });

    // 流程级别初始变量
    expect(variables).toContainEqual({
      name: 'api_key',
      value: 'flow-secret',
      sourceNodeUuid: '__flow__',
      description: '流程初始变量',
    });

    // 激活环境变量（与流程变量同名时保留各自来源）
    expect(variables).toContainEqual({
      name: 'api_key',
      value: 'dev-key',
      sourceNodeUuid: '__environment__',
      description: '环境变量 (dev)',
    });
    expect(variables).toContainEqual({
      name: 'debug',
      value: true,
      sourceNodeUuid: '__environment__',
      description: '环境变量 (dev)',
    });

    // 变量名去重（api_key 只出现一次）
    expect(availableVariableNames).toEqual(['user_id', 'user_name', 'api_key', 'debug']);

    // 节点标签映射（含虚拟来源标签）
    expect(nodeLabels).toEqual({
      'node-extractor': '变量提取',
      __flow__: '流程变量',
      __environment__: '环境变量',
    });
  });

  it('激活环境不存在时不包含环境变量', () => {
    act(() => {
      useFlowStore.setState({
        flow: createFlow({
          initial_variables: { base: 'v1' },
        }),
      });
      useEnvironmentStore.setState({
        activeEnvironmentName: null,
        activeEnvironmentData: null,
      });
    });

    const { result } = renderHook(() => useFlowVariables());
    expect(result.current.variables).toHaveLength(1);
    expect(result.current.availableVariableNames).toEqual(['base']);
  });

  it('流程切换后变量结果随之更新', () => {
    const { result, rerender } = renderHook(() => useFlowVariables());
    // 初始无流程
    expect(result.current.variables).toEqual([]);

    act(() => {
      useFlowStore.setState({
        flow: createFlow({
          initial_variables: { a: '1' },
          nodes: [
            createNode({
              uuid: 'n1',
              type: 'variable:extractor',
              name: '提取',
              data: {
                type: 'variable:extractor',
                output_mappings: [{ source: 'resp.data', target: 'resp_data' }],
              },
            }),
          ],
        }),
      });
    });
    rerender();

    expect(result.current.availableVariableNames).toEqual(['resp_data', 'a']);
    expect(result.current.nodeLabels['n1']).toBe('提取');
  });

  it('节点仅位置改变（变量无变化）时保持返回相同的对象引用', () => {
    const initialFlow = createFlow({
      initial_variables: { a: '1' },
      nodes: [
        createNode({
          uuid: 'n1',
          type: 'http:request',
          name: 'HTTP请求',
          position: { x: 100, y: 100 },
        }),
      ],
    });

    act(() => {
      useFlowStore.setState({ flow: initialFlow });
    });

    const { result, rerender } = renderHook(() => useFlowVariables());
    const firstRef = result.current;

    // 仅移动节点坐标，创建新 flow 实例
    act(() => {
      useFlowStore.setState({
        flow: {
          ...initialFlow,
          nodes: [
            {
              ...initialFlow.nodes[0],
              position: { x: 250, y: 350 },
            },
          ],
        },
      });
    });
    rerender();

    expect(result.current).toBe(firstRef);
  });

  it('提取 script 节点的静态变量', () => {
    const scriptNode = createNode({
      uuid: 'script-1',
      name: '签名生成',
      type: 'script',
      data: {
        type: 'script',
        script: "fn.variables.set('auth_sign', 'xxx'); fn.variables.setAll({ app_id: '100' });",
        timeout_ms: 1000,
      },
    });

    act(() => {
      useFlowStore.setState({
        flow: createFlow({
          uuid: 'flow-script',
          name: '主流程',
          nodes: [scriptNode],
        }),
      });
    });

    const { result } = renderHook(() => useFlowVariables());
    expect(result.current.availableVariableNames).toEqual(['auth_sign', 'app_id']);
    expect(result.current.variables).toContainEqual(
      expect.objectContaining({ name: 'auth_sign', sourceNodeUuid: 'script-1' })
    );
  });

  it('提取显式子流程节点的 output_mappings', () => {
    const subflowNode = createNode({
      uuid: 'subflow-1',
      name: '认证子流程',
      type: 'flow:sub',
      data: {
        type: 'flow:sub',
        target_flow_id: 'child-1',
        export_all_variables: false,
        output_mappings: [
          { source: 'child_token', target: 'parent_token' },
        ],
      },
    });

    act(() => {
      useFlowStore.setState({
        flow: createFlow({
          uuid: 'flow-subflow-explicit',
          name: '主流程',
          nodes: [subflowNode],
        }),
      });
    });

    const { result } = renderHook(() => useFlowVariables());
    expect(result.current.hasOpenEndedSubflow).toBe(false);
    expect(result.current.availableVariableNames).toEqual(['parent_token']);
    expect(result.current.variables[0]).toEqual({
      name: 'parent_token',
      value: '{{child_token}}',
      sourceNodeUuid: 'subflow-1',
      description: '认证子流程',
    });
  });

  it('全量导出子流程异步推导并合并其内部变量', async () => {
    const targetChildFlow: Flow = {
      uuid: 'child-flow-uuid',
      name: '登录子流程',
      initial_variables: { client_code: 'TEST' },
      nodes: [
        createNode({
          uuid: 'child-script',
          name: '内部脚本',
          type: 'script',
          data: {
            type: 'script',
            script: "fn.variables.set('session_id', 'sess_123');",
            timeout_ms: 1000,
          },
        }),
      ],
      connections: [],
    };

    vi.mocked(flowStorage.loadFlow).mockResolvedValue(targetChildFlow);

    const subflowNode = createNode({
      uuid: 'subflow-auto',
      name: '登录流程调用',
      type: 'flow:sub',
      data: {
        type: 'flow:sub',
        target_flow_id: 'child-flow-uuid',
        export_all_variables: true,
      },
    });

    act(() => {
      useFlowStore.setState({
        flow: createFlow({
          uuid: 'flow-main-auto',
          name: '主流程',
          nodes: [subflowNode],
        }),
      });
    });

    const { result } = renderHook(() => useFlowVariables());
    expect(result.current.hasOpenEndedSubflow).toBe(true);

    // 等待异步解析子流程变量并合并
    await waitFor(() => {
      expect(result.current.availableVariableNames).toContain('client_code');
      expect(result.current.availableVariableNames).toContain('session_id');
    });

    expect(result.current.variables).toContainEqual(
      expect.objectContaining({
        name: 'client_code',
        sourceNodeUuid: 'subflow-auto',
      })
    );
    expect(result.current.variables).toContainEqual(
      expect.objectContaining({
        name: 'session_id',
        sourceNodeUuid: 'subflow-auto',
      })
    );
  });
});
