/**
 * useFlowVariables Hook 测试
 * 覆盖：无流程空结果、三类来源合并（变量提取节点/流程初始变量/激活环境变量）、
 * 变量名去重、节点标签映射、流程切换后结果更新
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { useFlowVariables } from '@/hooks/useFlowVariables';
import { useFlowStore } from '@/stores';
import { createFlow, createNode } from '@/test-utils';

describe('useFlowVariables', () => {
  beforeEach(() => {
    useFlowStore.getState().reset();
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
      type: 'variable_extractor',
      name: '变量提取',
      data: {
        type: 'variable_extractor',
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
          environments: [
            { name: 'dev', variables: { api_key: 'dev-key', debug: true } },
          ],
          active_environment: 'dev',
        }),
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
          environments: [{ name: 'dev', variables: { dev_only: 'x' } }],
          active_environment: 'staging',
        }),
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
              type: 'variable_extractor',
              name: '提取',
              data: {
                type: 'variable_extractor',
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
});
