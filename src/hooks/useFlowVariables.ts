/**
 * 流程变量提取 Hook
 * 从所有来源（节点级、脚本级、子流程级、流程级、环境变量）合并可用变量列表
 */
import { useEffect, useMemo, useRef, useState } from 'react';
import { useFlowStore } from '@/stores';
import { useEnvironmentStore } from '@/stores/environment';
import type { Variable } from '@/types/variable';
import { isVariableExtractorConfig, isScriptConfig, isSubFlowConfig } from '@/types/node';
import { getLL } from '@/i18n/i18n-lazy';
import {
  extractVariablesFromScript,
  loadAndResolveSubFlowVariables,
} from '@/lib/variableExtraction';

/** 流程变量提取结果 */
export interface FlowVariablesResult {
  /** 所有可用变量（合并所有来源） */
  variables: Variable[];
  /** 变量名列表（去重） */
  availableVariableNames: string[];
  /** 节点 UUID -> 名称映射 */
  nodeLabels: Record<string, string>;
  /** 是否存在开启全量导出的子流程节点 */
  hasOpenEndedSubflow: boolean;
}

/**
 * 比较两个 FlowVariablesResult 内容是否实质相同
 */
function areVariableResultsEqual(a: FlowVariablesResult, b: FlowVariablesResult): boolean {
  if (a.hasOpenEndedSubflow !== b.hasOpenEndedSubflow) return false;
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
 * 2. 脚本节点 (script) 中静态分析的 fn.variables.set / setAll
 * 3. 子流程节点 (flow:sub) 显式指定的 output_mappings，或全量导出时的推导变量
 * 4. 流程级别的 initial_variables
 * 5. 激活环境的变量
 */
export function useFlowVariables(): FlowVariablesResult {
  const flow = useFlowStore((state) => state.flow);
  const activeEnvData = useEnvironmentStore((state) => state.activeEnvironmentData);
  const prevResultRef = useRef<FlowVariablesResult | null>(null);

  // 存储各全量导出子流程节点解析到的变量: nodeUuid -> Variable[]
  const [subflowVarsMap, setSubflowVarsMap] = useState<Record<string, Variable[]>>({});

  // 异步加载目标子流程的静态变量
  useEffect(() => {
    if (!flow || !flow.nodes) {
      setSubflowVarsMap({});
      return;
    }

    let isCancelled = false;

    // 筛选所有开启全量导出的子流程节点
    const subflowNodes = flow.nodes.filter(
      (n) => isSubFlowConfig(n.data) && n.data.export_all_variables
    );

    if (subflowNodes.length === 0) {
      setSubflowVarsMap((prev) => (Object.keys(prev).length === 0 ? prev : {}));
      return;
    }

    const loadPromises = subflowNodes.map(async (node) => {
      if (!isSubFlowConfig(node.data)) return { nodeUuid: node.uuid, vars: [] };
      const target = node.data.target_flow_path || node.data.target_flow_id;
      if (!target) return { nodeUuid: node.uuid, vars: [] };

      // 避免引用当前流程自身导致死循环
      if (node.data.target_flow_id && node.data.target_flow_id === flow.uuid) {
        return { nodeUuid: node.uuid, vars: [] };
      }

      const res = await loadAndResolveSubFlowVariables(target);
      return { nodeUuid: node.uuid, vars: res.variables };
    });

    Promise.all(loadPromises).then((results) => {
      if (isCancelled) return;
      const nextMap: Record<string, Variable[]> = {};
      results.forEach(({ nodeUuid, vars }) => {
        nextMap[nodeUuid] = vars;
      });

      setSubflowVarsMap((prev) => {
        const prevKeys = Object.keys(prev);
        const nextKeys = Object.keys(nextMap);
        if (prevKeys.length !== nextKeys.length) return nextMap;
        let isSame = true;
        for (const k of nextKeys) {
          const prevList = prev[k] || [];
          const nextList = nextMap[k] || [];
          if (prevList.length !== nextList.length) {
            isSame = false;
            break;
          }
          for (let i = 0; i < prevList.length; i++) {
            if (prevList[i].name !== nextList[i].name) {
              isSame = false;
              break;
            }
          }
          if (!isSame) break;
        }
        return isSame ? prev : nextMap;
      });
    });

    return () => {
      isCancelled = true;
    };
  }, [flow]);

  return useMemo(() => {
    if (!flow) {
      const empty: FlowVariablesResult = {
        variables: [],
        availableVariableNames: [],
        nodeLabels: {},
        hasOpenEndedSubflow: false,
      };
      prevResultRef.current = empty;
      return empty;
    }

    const ll = getLL();
    const vars: Variable[] = [];
    let hasOpenEndedSubflow = false;

    // 1. 遍历当前流程节点提取变量
    flow.nodes.forEach((node) => {
      const data = node.data;

      // 1.1 变量提取节点的 output_mappings
      if (isVariableExtractorConfig(data)) {
        const mappings = data.output_mappings;
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
      }

      // 1.2 脚本节点的静态变量 (fn.variables.set / setAll)
      if (isScriptConfig(data)) {
        const scriptKeys = extractVariablesFromScript(data.script);
        scriptKeys.forEach((key) => {
          vars.push({
            name: key,
            value: undefined,
            sourceNodeUuid: node.uuid,
            description: node.name,
          });
        });
      }

      // 1.3 子流程节点变量
      if (isSubFlowConfig(data)) {
        if (data.export_all_variables) {
          hasOpenEndedSubflow = true;
          // 若已加载并解析到子流程导出的变量，合并进入变量池
          const childVars = subflowVarsMap[node.uuid];
          if (childVars && childVars.length > 0) {
            childVars.forEach((cv) => {
              vars.push({
                name: cv.name,
                value: cv.value,
                sourceNodeUuid: node.uuid,
                description: cv.description
                  ? `${node.name}: ${cv.description}`
                  : `${node.name}`,
              });
            });
          }
        } else {
          // 显式导出映射
          const mappings = data.output_mappings;
          if (mappings) {
            mappings.forEach((om) => {
              vars.push({
                name: om.target,
                value: `{{${om.source}}}`,
                sourceNodeUuid: node.uuid,
                description: node.name,
              });
            });
          }
        }
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
    if (activeEnvData) {
      Object.entries(activeEnvData.variables).forEach(([key, value]) => {
        vars.push({
          name: key,
          value,
          sourceNodeUuid: '__environment__',
          description: ll.common.environment_variable({ name: activeEnvData.name }),
        });
      });
    }

    // 去重变量名
    const availableVariableNames = [...new Set(vars.map((v) => v.name))];

    // 节点标签映射（含虚拟来源与子流程标识）
    const nodeLabels: Record<string, string> = {};
    flow.nodes.forEach((node) => {
      if (node.type === 'flow:sub') {
        nodeLabels[node.uuid] = `[${ll.panels.subflow.target_flow()}] ${node.name}`;
      } else {
        nodeLabels[node.uuid] = node.name;
      }
    });
    nodeLabels['__flow__'] = ll.common.label_flow_variables();
    nodeLabels['__environment__'] = ll.common.label_environment_variables();

    const freshResult: FlowVariablesResult = {
      variables: vars,
      availableVariableNames,
      nodeLabels,
      hasOpenEndedSubflow,
    };

    if (prevResultRef.current && areVariableResultsEqual(prevResultRef.current, freshResult)) {
      return prevResultRef.current;
    }
    prevResultRef.current = freshResult;
    return freshResult;
  }, [flow, activeEnvData, subflowVarsMap]);
}
