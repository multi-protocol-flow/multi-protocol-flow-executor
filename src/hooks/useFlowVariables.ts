/**
 * 流程变量提取 Hook
 * 从所有来源（节点级、流程级、环境变量）合并可用变量列表
 */
import { useMemo } from 'react';
import { useFlowStore } from '@/stores';
import type { Variable } from '@/types/variable';
import {
  isEntryConfig,
  isVariableExtractorConfig,
} from '@/types/node';
/** 流程变量提取结果 */
export interface FlowVariablesResult {
  /** 所有可用变量（合并所有来源） */
  variables: Variable[];
  /** 变量名列表（去重） */
  availableVariableNames: string[];
  /** 节点 UUID -> 名称映射 */
  nodeLabels: Record<string, string>;
}

/**
 * 从流程中提取所有可用变量
 *
 * 来源：
 * 1. Entry 节点的 initial_variables
 * 2. 协议节点的 output_mappings（HTTP、WS、DB、变量提取）
 * 3. 流程级别的 initial_variables
 * 4. 激活环境的变量
 */
export function useFlowVariables(): FlowVariablesResult {
  const flow = useFlowStore((state) => state.flow);

  return useMemo(() => {
    if (!flow) {
      return { variables: [], availableVariableNames: [], nodeLabels: {} };
    }

    const vars: Variable[] = [];

    // 1. Entry 节点的 initial_variables
    flow.nodes.forEach((node) => {
      if (isEntryConfig(node.data) && node.data.initial_variables) {
        node.data.initial_variables.forEach((iv) => {
          vars.push({
            name: iv.name,
            value: iv.value,
            sourceNodeUuid: node.uuid,
            description: iv.description,
          });
        });
      }
    });

    // 2. 变量提取节点的 output_mappings
    flow.nodes.forEach((node) => {
      const data = node.data;
      const mappings = isVariableExtractorConfig(data) ? data.output_mappings : null;

      if (mappings) {
        mappings.forEach((om) => {
          vars.push({
            name: om.target,
            value: `{{${om.source}}}`,
            sourceNodeUuid: node.uuid,
            description: om.description,
          });
        });
      }
    });

    // 3. 流程级别的 initial_variables
    if (flow.initial_variables) {
      Object.entries(flow.initial_variables).forEach(([key, value]) => {
        vars.push({
          name: key,
          value,
          sourceNodeUuid: '__flow__',
          description: '流程初始变量',
        });
      });
    }

    // 4. 激活环境的变量
    if (flow.active_environment && flow.environments) {
      const env = flow.environments.find(
        (e) => e.name === flow.active_environment
      );
      if (env) {
        Object.entries(env.variables).forEach(([key, value]) => {
          vars.push({
            name: key,
            value,
            sourceNodeUuid: '__environment__',
            description: `环境变量 (${env.name})`,
          });
        });
      }
    }

    // 去重变量名
    const availableVariableNames = [...new Set(vars.map((v) => v.name))];

    // 节点标签映射（含虚拟来源）
    const nodeLabels: Record<string, string> = {};
    flow.nodes.forEach((node) => {
      nodeLabels[node.uuid] = node.name;
    });
    nodeLabels['__flow__'] = '流程变量';
    nodeLabels['__environment__'] = '环境变量';

    return { variables: vars, availableVariableNames, nodeLabels };
  }, [flow]);
}
