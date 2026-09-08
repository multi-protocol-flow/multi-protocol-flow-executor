/**
 * 流程变量提取 Hook
 * 从所有来源（节点级、流程级、环境变量）合并可用变量列表
 */
import { useMemo, useRef } from 'react';
import { useFlowStore } from '@/stores';
import type { Variable } from '@/types/variable';
import { isVariableExtractorConfig } from '@/types/node';
import { getLL } from '@/i18n/i18n-lazy';
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
 * 比较两个 FlowVariablesResult 内容是否实质相同
 */
function areVariableResultsEqual(a: FlowVariablesResult, b: FlowVariablesResult): boolean {
  if (a.variables.length !== b.variables.length) return false;
  if (a.availableVariableNames.length !== b.availableVariableNames.length) return false;
  const aLabels = Object.entries(a.nodeLabels);
  const bLabels = Object.entries(b.nodeLabels);
  if (aLabels.length !== bLabels.length) return false;

  for (let i = 0; i < a.variables.length; i++) {
    const va = a.variables[i];
    const vb = b.variables[i];
    if (
      va.name !== vb.name ||
      va.value !== vb.value ||
      va.sourceNodeUuid !== vb.sourceNodeUuid ||
      va.description !== vb.description
    ) {
      return false;
    }
  }

  for (let i = 0; i < a.availableVariableNames.length; i++) {
    if (a.availableVariableNames[i] !== b.availableVariableNames[i]) return false;
  }

  for (const [k, v] of aLabels) {
    if (b.nodeLabels[k] !== v) return false;
  }

  return true;
}

/**
 * 从流程中提取所有可用变量
 *
 * 来源：
 * 1. 变量提取节点的 output_mappings
 * 2. 流程级别的 initial_variables
 * 3. 激活环境的变量
 */
export function useFlowVariables(): FlowVariablesResult {
  const flow = useFlowStore((state) => state.flow);
  const prevResultRef = useRef<FlowVariablesResult | null>(null);

  return useMemo(() => {
    if (!flow) {
      const empty = { variables: [], availableVariableNames: [], nodeLabels: {} };
      prevResultRef.current = empty;
      return empty;
    }

    const ll = getLL();
    const vars: Variable[] = [];

    // 1. 变量提取节点的 output_mappings
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

    // 2. 流程级别的 initial_variables
    if (flow.initial_variables) {
      Object.entries(flow.initial_variables).forEach(([key, value]) => {
        vars.push({
          name: key,
          value,
          sourceNodeUuid: '__flow__',
          description: ll.common.flow_initial_variables(),
        });
      });
    }

    // 3. 激活环境的变量
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
            description: ll.common.environment_variable({ name: env.name }),
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
    nodeLabels['__flow__'] = ll.common.label_flow_variables();
    nodeLabels['__environment__'] = ll.common.label_environment_variables();

    const freshResult: FlowVariablesResult = { variables: vars, availableVariableNames, nodeLabels };
    if (prevResultRef.current && areVariableResultsEqual(prevResultRef.current, freshResult)) {
      return prevResultRef.current;
    }
    prevResultRef.current = freshResult;
    return freshResult;
  }, [flow]);
}
