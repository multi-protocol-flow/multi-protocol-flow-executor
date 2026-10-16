/**
 * flowSearch - 全流程搜索
 * 搜索当前流程所有节点的名称、配置字段、变量引用
 */
import type { Flow } from '@/types/flow';

/** 搜索结果类型 */
export type SearchMatchType = 'name' | 'config' | 'variable';

/** 单条搜索结果 */
export interface SearchResult {
  nodeUuid: string;
  nodeName: string;
  nodeType: string;
  field: string;         // 匹配的字段路径
  value: string;         // 匹配的值（含上下文）
  matchType: SearchMatchType;
}

/** 按节点分组的结果 */
export interface GroupedSearchResult {
  nodeUuid: string;
  nodeName: string;
  nodeType: string;
  matches: SearchResult[];
}

/** 全局搜索（跨流程）结果 */
export interface GlobalSearchResult {
  flowMeta: { uuid: string; name: string };
  matches: SearchResult[];
}

/** 按节点分组的全局搜索结果 */
export interface GroupedGlobalSearchResult {
  flowName: string;
  flowUuid: string;
  groups: GroupedSearchResult[];
}

/**
 * 深度搜索节点配置中的匹配值
 */
function deepSearchConfig(
  data: Record<string, unknown>,
  query: string,
  prefix: string
): SearchResult[] {
  const results: SearchResult[] = [];
  const lowerQuery = query.toLowerCase();

  for (const [key, value] of Object.entries(data)) {
    const fieldPath = prefix ? `${prefix}.${key}` : key;

    if (typeof value === 'string') {
      if (value.toLowerCase().includes(lowerQuery)) {
        results.push({
          nodeUuid: '',
          nodeName: '',
          nodeType: '',
          field: fieldPath,
          value: value.length > 100 ? value.slice(0, 100) + '...' : value,
          matchType: 'config',
        });
      }
    } else if (typeof value === 'object' && value !== null) {
      results.push(...deepSearchConfig(value as Record<string, unknown>, query, fieldPath));
    } else if (typeof value === 'number' || typeof value === 'boolean') {
      if (String(value).toLowerCase().includes(lowerQuery)) {
        results.push({
          nodeUuid: '',
          nodeName: '',
          nodeType: '',
          field: fieldPath,
          value: String(value),
          matchType: 'config',
        });
      }
    }
  }

  return results;
}

/**
 * 从值中提取变量引用 {{var}}
 */
function extractVariableRefs(value: unknown, results: string[] = []): void {
  if (typeof value === 'string') {
    const matches = value.match(/\{\{([^}]+)\}\}/g);
    if (matches) {
      for (const match of matches) {
        const varName = match.slice(2, -2).trim();
        if (!results.includes(varName)) {
          results.push(varName);
        }
      }
    }
  } else if (Array.isArray(value)) {
    for (const item of value) {
      extractVariableRefs(item, results);
    }
  } else if (typeof value === 'object' && value !== null) {
    for (const val of Object.values(value as Record<string, unknown>)) {
      extractVariableRefs(val, results);
    }
  }
}

/**
 * 搜索变量引用匹配
 */
function searchVariableRefs(
  data: Record<string, unknown>,
  query: string
): SearchResult[] {
  const results: SearchResult[] = [];
  const allRefs: string[] = [];
  extractVariableRefs(data, allRefs);

  const lowerQuery = query.toLowerCase();
  for (const ref of allRefs) {
    if (ref.toLowerCase().includes(lowerQuery)) {
      results.push({
        nodeUuid: '',
        nodeName: '',
        nodeType: '',
        field: `{{${ref}}}`,
        value: ref,
        matchType: 'variable',
      });
    }
  }

  return results;
}

/**
 * 搜索当前流程
 * @param flow 流程对象
 * @param query 搜索关键词
 * @returns 搜索结果列表（不含 nodeUuid/nodeName，需填充）
 */
function _searchFlow(flow: Flow, query: string): SearchResult[] {
  const results: SearchResult[] = [];
  const lowerQuery = query.toLowerCase().trim();

  if (!lowerQuery) return results;

  for (const node of flow.nodes) {
    // 1. 匹配节点名称
    if (node.name.toLowerCase().includes(lowerQuery)) {
      results.push({
        nodeUuid: node.uuid,
        nodeName: node.name,
        nodeType: node.type,
        field: 'name',
        value: node.name,
        matchType: 'name',
      });
    }

    // 2. 深度递归匹配节点配置
    const configMatches = deepSearchConfig(node.data as unknown as Record<string, unknown>, query, '');
    for (const m of configMatches) {
      m.nodeUuid = node.uuid;
      m.nodeName = node.name;
      m.nodeType = node.type;
    }
    results.push(...configMatches);

    // 3. 匹配变量引用 {{...}}
    const variableMatches = searchVariableRefs(node.data as unknown as Record<string, unknown>, query);
    for (const m of variableMatches) {
      m.nodeUuid = node.uuid;
      m.nodeName = node.name;
      m.nodeType = node.type;
    }
    results.push(...variableMatches);
  }

  return results;
}

/**
 * 搜索流程并按节点分组
 */
export function searchFlow(flow: Flow, query: string): GroupedSearchResult[] {
  const results = _searchFlow(flow, query);
  return groupByNode(results);
}

/**
 * 按节点 UUID 分组搜索结果
 */
function groupByNode(results: SearchResult[]): GroupedSearchResult[] {
  const map = new Map<string, SearchResult[]>();

  for (const r of results) {
    if (!map.has(r.nodeUuid)) {
      map.set(r.nodeUuid, []);
    }
    map.get(r.nodeUuid)!.push(r);
  }

  const grouped: GroupedSearchResult[] = [];
  for (const [nodeUuid, matches] of map) {
    const first = matches[0];
    grouped.push({
      nodeUuid,
      nodeName: first.nodeName,
      nodeType: first.nodeType,
      matches,
    });
  }

  return grouped;
}

/**
 * 全局搜索（跨流程）
 * 搜索所有已保存流程中的内容
 * @param loadFlowFn 加载流程的函数（如 flowStorage.loadFlow）
 * @param flowMetas 流程元数据列表
 * @param query 搜索关键词（>=3 字符时搜索内容）
 */
export async function globalSearchFlow(
  loadFlowFn: (uuid: string) => Promise<Flow | null>,
  flowMetas: { uuid: string; name: string }[],
  query: string
): Promise<GroupedGlobalSearchResult[]> {
  const lowerQuery = query.toLowerCase().trim();
  if (!lowerQuery) return [];

  // 搜索流程名称
  const nameMatchedFlows: GroupedGlobalSearchResult[] = [];
  const remainingMetas: { uuid: string; name: string }[] = [];

  for (const meta of flowMetas) {
    if (meta.name.toLowerCase().includes(lowerQuery)) {
      nameMatchedFlows.push({
        flowName: meta.name,
        flowUuid: meta.uuid,
        groups: [],
      });
    } else {
      remainingMetas.push(meta);
    }
  }

  // 仅当搜索词 >= 3 时搜索内容，避免性能问题
  if (query.length < 3) {
    return nameMatchedFlows;
  }

  // 并行加载流程并搜索内容
  const contentResults = await Promise.all(
    remainingMetas.map(async (meta) => {
      try {
        const flow = await loadFlowFn(meta.uuid);
        if (!flow) return null;
        const groups = searchFlow(flow, query);
        if (groups.length === 0) return null;
        return {
          flowName: meta.name,
          flowUuid: meta.uuid,
          groups,
        } satisfies GroupedGlobalSearchResult;
      } catch {
        return null;
      }
    })
  );

  const validContentResults = contentResults.filter((r): r is GroupedGlobalSearchResult => r !== null);
  return [...nameMatchedFlows, ...validContentResults];
}

/**
 * 递归替换对象中的字符串内容
 */
function deepReplaceObject(
  obj: unknown,
  searchStr: string,
  replaceWith: string,
): { newObj: unknown; count: number } {
  if (typeof obj === 'string') {
    if (obj.includes(searchStr)) {
      const parts = obj.split(searchStr);
      const count = parts.length - 1;
      const newStr = parts.join(replaceWith);
      return { newObj: newStr, count };
    }
    return { newObj: obj, count: 0 };
  }
  if (Array.isArray(obj)) {
    let totalCount = 0;
    const newArr = obj.map((item) => {
      const res = deepReplaceObject(item, searchStr, replaceWith);
      totalCount += res.count;
      return res.newObj;
    });
    return { newObj: newArr, count: totalCount };
  }
  if (typeof obj === 'object' && obj !== null) {
    let totalCount = 0;
    const newObj: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(obj)) {
      const res = deepReplaceObject(v, searchStr, replaceWith);
      totalCount += res.count;
      newObj[k] = res.newObj;
    }
    return { newObj, count: totalCount };
  }
  return { newObj: obj, count: 0 };
}

/**
 * 在 Flow 对象中批量替换匹配的文本（覆盖节点名称与配置对象）
 */
export function replaceInFlow(
  flow: Flow,
  searchStr: string,
  replaceWith: string,
): { updatedFlow: Flow; replaceCount: number } {
  if (!searchStr) return { updatedFlow: flow, replaceCount: 0 };

  let totalCount = 0;

  const replaceNode = (node: Flow['nodes'][number]) => {
    let nodeCount = 0;
    let newName = node.name;
    if (newName.includes(searchStr)) {
      const parts = newName.split(searchStr);
      nodeCount += parts.length - 1;
      newName = parts.join(replaceWith);
    }
    const dataRes = deepReplaceObject(node.data, searchStr, replaceWith);
    nodeCount += dataRes.count;
    totalCount += nodeCount;

    if (nodeCount > 0) {
      return {
        ...node,
        name: newName,
        data: dataRes.newObj as typeof node.data,
      };
    }
    return node;
  };

  const newMainNodes = flow.nodes.map(replaceNode);

  const newSetup = flow.setup
    ? {
        ...flow.setup,
        nodes: flow.setup.nodes.map(replaceNode),
      }
    : flow.setup;

  const newTeardown = flow.teardown
    ? {
        ...flow.teardown,
        nodes: flow.teardown.nodes.map(replaceNode),
      }
    : flow.teardown;

  const updatedFlow: Flow = {
    ...flow,
    nodes: newMainNodes,
    setup: newSetup,
    teardown: newTeardown,
  };

  return { updatedFlow, replaceCount: totalCount };
}
