/**
 * 变量提取与推导工具
 *
 * 负责从流程、节点、脚本代码以及引用的子流程中静态提取导出的变量。
 */
import type { Flow, Node } from '@/types';
import type { Variable } from '@/types/variable';
import { isVariableExtractorConfig, isScriptConfig, isSubFlowConfig } from '@/types/node';
import { flowStorage } from '@/services/storage';
import { getLL } from '@/i18n/i18n-lazy';

/**
 * 从脚本内容中静态分析提取写入的变量名
 *
 * 支持语法：
 * - `fn.variables.set('key', ...)` / `context.variables.set('key', ...)`
 * - `fn.variables.setAll({ key1: ..., 'key2': ... })`
 */
export function extractVariablesFromScript(script?: string | null): string[] {
  if (!script || typeof script !== 'string') return [];
  const vars = new Set<string>();

  // 1. fn.variables.set('key', ...) 或 context.variables.set("key", ...)
  const setRegex = /(?:fn|context)\.variables\.set\s*\(\s*(['"`])([a-zA-Z0-9_.$#@-]+)\1/g;
  let match: RegExpExecArray | null;
  while ((match = setRegex.exec(script)) !== null) {
    const key = match[2]?.trim();
    if (key) {
      vars.add(key);
    }
  }

  // 2. fn.variables.setAll({ ... })
  const setAllRegex = /(?:fn|context)\.variables\.setAll\s*\(\s*\{([^}]+)\}\s*\)/g;
  while ((match = setAllRegex.exec(script)) !== null) {
    const objectContent = match[1];
    // 匹配 key: 或 'key': 或 "key":
    const keyRegex = /(?:['"`]([a-zA-Z0-9_.$#@-]+)['"`]|([a-zA-Z_$][a-zA-Z0-9_.$#@-]*))\s*:/g;
    let keyMatch: RegExpExecArray | null;
    while ((keyMatch = keyRegex.exec(objectContent)) !== null) {
      const key = (keyMatch[1] || keyMatch[2])?.trim();
      if (key) {
        vars.add(key);
      }
    }
  }

  return Array.from(vars);
}

/**
 * 提取单个节点显式或静态声明导出的变量列表（不展开全量导出子流程）
 */
export function extractNodeExportedVariables(node: Node, flowName?: string): Variable[] {
  const vars: Variable[] = [];
  const data = node.data;

  // 1. 变量提取节点
  if (isVariableExtractorConfig(data)) {
    const mappings = data.output_mappings;
    if (mappings && Array.isArray(mappings)) {
      mappings.forEach((om) => {
        vars.push({
          name: om.target,
          value: `{{${om.source}}}`,
          sourceNodeUuid: node.uuid,
          description: om.description || (flowName ? `[${flowName}] ${node.name}` : node.name),
        });
      });
    }
  }

  // 2. 脚本节点 (静态扫描 fn.variables.set)
  if (isScriptConfig(data)) {
    const extractedKeys = extractVariablesFromScript(data.script);
    extractedKeys.forEach((key) => {
      vars.push({
        name: key,
        value: undefined,
        sourceNodeUuid: node.uuid,
        description: flowName ? `[${flowName}] ${node.name}` : node.name,
      });
    });
  }

  // 3. 子流程节点显式配置的输出映射 (export_all_variables 为 false 时)
  if (isSubFlowConfig(data) && !data.export_all_variables) {
    const mappings = data.output_mappings;
    if (mappings && Array.isArray(mappings)) {
      mappings.forEach((om) => {
        vars.push({
          name: om.target,
          value: `{{${om.source}}}`,
          sourceNodeUuid: node.uuid,
          description: flowName ? `[${flowName}] ${node.name}` : node.name,
        });
      });
    }
  }

  return vars;
}

/**
 * 静态提取 Flow 自身（包含 Setup / Main / Teardown）直接产生的所有局部变量
 */
export function extractFlowLocalVariables(flow: Flow): Variable[] {
  const ll = getLL();
  const initialVarLabel = ll.common.flow_initial_variables();
  const vars: Variable[] = [];

  // 1. 流程级别初始变量
  if (flow.initial_variables) {
    Object.entries(flow.initial_variables).forEach(([key, value]) => {
      vars.push({
        name: key,
        value,
        sourceNodeUuid: '__flow__',
        description: flow.name ? `[${flow.name}] ${initialVarLabel}` : initialVarLabel,
      });
    });
  }

  // 2. 收集所有阶段的节点
  const allNodes: Node[] = [];
  if (flow.setup?.nodes) allNodes.push(...flow.setup.nodes);
  if (flow.nodes) allNodes.push(...flow.nodes);
  if (flow.teardown?.nodes) allNodes.push(...flow.teardown.nodes);

  allNodes.forEach((node) => {
    vars.push(...extractNodeExportedVariables(node, flow.name));
  });

  return vars;
}

/**
 * 内存缓存：target (uuid/path) -> 解析后的变量列表及加载时间
 */
interface CachedSubFlow {
  flow: Flow;
  variables: Variable[];
  timestamp: number;
}

const subFlowCache = new Map<string, CachedSubFlow>();

/**
 * 清除子流程变量缓存（特定目标或全部）
 */
export function clearSubFlowVariablesCache(target?: string): void {
  if (target) {
    subFlowCache.delete(target);
  } else {
    subFlowCache.clear();
  }
}

/**
 * 获取或加载子流程对象及解析导出的变量（含防循环引用保护）
 */
export async function loadAndResolveSubFlowVariables(
  target: string,
  visited: Set<string> = new Set()
): Promise<{ flow: Flow | null; variables: Variable[] }> {
  if (!target || visited.has(target)) {
    return { flow: null, variables: [] };
  }

  // 检查缓存（5 秒缓存有效期，避免频繁重复加载与解析）
  const cached = subFlowCache.get(target);
  if (cached && Date.now() - cached.timestamp < 5000) {
    return { flow: cached.flow, variables: cached.variables };
  }

  try {
    const loadedFlow = await flowStorage.loadFlow(target);
    if (!loadedFlow) {
      return { flow: null, variables: [] };
    }

    const currentVisited = new Set(visited);
    currentVisited.add(target);
    if (loadedFlow.uuid) {
      currentVisited.add(loadedFlow.uuid);
    }

    // 1. 提取目标 Flow 本地的变量
    const localVars = extractFlowLocalVariables(loadedFlow);
    const combinedVars: Variable[] = [...localVars];

    // 2. 检查目标 Flow 内部是否嵌套了其他全量导出子流程（单层深搜）
    const childSubFlowNodes = (loadedFlow.nodes || []).filter(
      (n) => isSubFlowConfig(n.data) && n.data.export_all_variables
    );

    for (const childNode of childSubFlowNodes) {
      if (!isSubFlowConfig(childNode.data)) continue;
      const childTarget = childNode.data.target_flow_path || childNode.data.target_flow_id;
      if (childTarget && !currentVisited.has(childTarget)) {
        const nestedResult = await loadAndResolveSubFlowVariables(childTarget, currentVisited);
        combinedVars.push(...nestedResult.variables);
      }
    }

    // 去重（同名变量保留最早出现的）
    const seenNames = new Set<string>();
    const deduplicatedVars: Variable[] = [];
    for (const v of combinedVars) {
      if (!seenNames.has(v.name)) {
        seenNames.add(v.name);
        deduplicatedVars.push(v);
      }
    }

    subFlowCache.set(target, {
      flow: loadedFlow,
      variables: deduplicatedVars,
      timestamp: Date.now(),
    });

    return { flow: loadedFlow, variables: deduplicatedVars };
  } catch (err) {
    // 加载失败时静默降级（文件不存在或格式不匹配）
    return { flow: null, variables: [] };
  }
}
