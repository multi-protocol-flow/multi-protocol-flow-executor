/**
 * 节点注册表服务
 * 从后端动态获取节点元数据，缓存并按分类分组
 */
import type { NodeMetadata } from '@/types/nodeRegistry';
import { getLL } from '@/i18n/i18n-lazy';

import type { LucideIcon } from 'lucide-react';
import {
  Globe,
  GitBranch,
  Sparkles,
  Clock,
  Puzzle,
} from 'lucide-react';
import { resolveBackendIcon, BACKEND_ICON_MAP } from '@/lib/nodeIcons';
import { getNodeDisplayName, getNodeDescription, ICON_MAP } from '@/lib/nodeMeta';
// 节点元数据缓存
let cachedMetadata: NodeMetadata[] | null = null;

/**
 * 从后端获取所有已注册节点类型的元数据
 *
 * 结果会被缓存，后续调用直接返回缓存数据。
 * 在非 Tauri 环境中返回空数组（Mock 模式）。
 *
 * @returns 节点元数据数组
 */
export async function getNodeMetadata(): Promise<NodeMetadata[]> {
  if (cachedMetadata) return cachedMetadata;
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    cachedMetadata = await invoke<NodeMetadata[]>('get_registered_node_types');
    return cachedMetadata;
  } catch {
    // 非 Tauri 环境返回空数组
    return [];
  }
}

/**
 * 清除节点元数据缓存
 *
 * 用于强制重新获取最新元数据（例如热更新后）。
 */
export function clearNodeMetadataCache(): void {
  cachedMetadata = null;
}

/** 同步获取已缓存的节点元数据列表（未获取过时返回 null） */
export function getCachedNodeMetadataList(): NodeMetadata[] | null {
  return cachedMetadata;
}

/**
 * 按分类对节点元数据分组
 *
 * @param metadata - 节点元数据数组
 * @returns 以分类为键的元数据映射
 */
export function groupByCategory(
  metadata: NodeMetadata[]
): Record<string, NodeMetadata[]> {
  const groups: Record<string, NodeMetadata[]> = {};
  for (const m of metadata) {
    if (!groups[m.category]) groups[m.category] = [];
    groups[m.category].push(m);
  }
  return groups;
}

/**
 * 分类显示名称映射
 */
export const CATEGORY_NAMES: Record<string, string> = {
  get http() { return getLL().nodes.category.http(); },
  get websocket() { return getLL().nodes.category.websocket(); },
  get database() { return getLL().nodes.category.database(); },
  get tools() { return getLL().nodes.category.tools(); },
  get script() { return getLL().nodes.category.script(); },
  get assertion() { return getLL().nodes.category.assertion(); },
  get sse() { return getLL().nodes.category.sse(); },
  get graphql() { return getLL().nodes.category.graphql(); },
  get tcp() { return getLL().nodes.category.tcp(); },
  get udp() { return getLL().nodes.category.udp(); },
  MongoDB: 'MongoDB',
};

/**
 * 分类排序顺序（越小越靠前）
 * 按一级分类分组，组内间隔 10 方便后续插入
 *
 * 网络: http(0), websocket(1), sse(2), graphql(3)
 * 工具: tools(30), script(31), assertion(32)
 * 其他: database(40), tcp(42), udp(43)
 */
export const CATEGORY_ORDER: Record<string, number> = {
  'http:request': 0,
  websocket: 1,
  sse: 2,
  graphql: 3,
  tools: 30,
  script: 31,
  assertion: 32,
  database: 40,
  tcp: 42,
  udp: 43,
  MongoDB: 44,
};

/**
 * 分组内节点排序（越小越靠前）
 * 协议类: 连接 → 调用/发送 → 关闭
 * 数据库: 连接 → 开启事务 → 查询 → 提交 → 回滚 → 关闭
 * 工具类: 条件判断 → 脚本 → 断言
 */
const NODE_TYPE_ORDER: Record<string, number> = {
  // HTTP & Mock Ingress
  'http:request': 0,
  'http:ingress': 1,
  'http:respond': 2,

  // WebSocket: 连接 → 发送采集 → 关闭
  'ws:connect': 0,
  'ws:send_collect': 1,
  'ws:close': 2,

  // SSE: 连接 → 监听 → 断开
  'sse:connect': 0,
  'sse:listen': 1,
  'sse:disconnect': 2,

  // GraphQL: 连接 → 查询 → 订阅 → 内省 → 断开
  'graphql:connect': 0,
  'graphql:query': 1,
  'graphql:subscribe': 2,
  'graphql:introspect': 3,
  'graphql:disconnect': 4,


  // TCP: 连接 → 发送 → 接收 → 关闭
  'tcp:connect': 0,
  'tcp:send': 1,
  'tcp:receive': 2,
  'tcp:close': 3,

  // UDP: 绑定 → 发送 → 接收 → 关闭
  'udp:bind': 0,
  'udp:send_to': 1,
  'udp:recv_from': 2,
  'udp:close': 3,


  // 工具: 条件判断 → 脚本 → 断言 → 变量提取 → 代理 → 结束
  conditional: 0,
  script: 1,
  assertion: 2,
  'variable:extractor': 3,
  proxy: 4,
  end: 5,
};

/**
 * 根据后端颜色生成 Tailwind badge class
 *
 * 后端返回十六进制颜色，前端需要映射到 Tailwind 类名。
 * 保留原有硬编码映射以确保一致性。
 *
 * @param typeId - 节点类型标识
 * @returns Tailwind badge class 字符串
 */
export function getBadgeClassForType(typeId: string): string {
  const badgeMap: Record<string, string> = {
    entry: 'bg-emerald-100 text-emerald-700',
    'http:request': 'bg-blue-100 text-blue-700',
    conditional: 'bg-orange-100 text-orange-700',
    script: 'bg-indigo-100 text-indigo-700',
    assertion: 'bg-purple-100 text-purple-700',
    'ws:connect': 'bg-amber-100 text-amber-700',
    'ws:send_collect': 'bg-teal-100 text-teal-700',
    'ws:close': 'bg-rose-100 text-rose-700',
    'variable:extractor': 'bg-amber-100 text-amber-700',
    proxy: 'bg-lime-100 text-lime-700',
    end: 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300 border-purple-200 dark:border-purple-800',
    'tcp:connect': 'bg-emerald-100 text-emerald-700',
    'tcp:send': 'bg-emerald-100 text-emerald-700',
    'tcp:receive': 'bg-emerald-100 text-emerald-700',
    'tcp:close': 'bg-emerald-100 text-emerald-700',
    'udp:bind': 'bg-orange-100 text-orange-700',
    'udp:send_to': 'bg-orange-100 text-orange-700',
    'udp:recv_from': 'bg-orange-100 text-orange-700',
    'udp:close': 'bg-orange-100 text-orange-700',
    'sse:connect': 'bg-rose-100 text-rose-700',
    'sse:listen': 'bg-rose-100 text-rose-700',
    'sse:disconnect': 'bg-rose-100 text-rose-700',
    'graphql:connect': 'bg-indigo-100 text-indigo-700',
    'graphql:query': 'bg-indigo-100 text-indigo-700',
    'graphql:subscribe': 'bg-indigo-100 text-indigo-700',
    'graphql:introspect': 'bg-indigo-100 text-indigo-700',
    'graphql:disconnect': 'bg-indigo-100 text-indigo-700',
  };
  return badgeMap[typeId] ?? 'bg-slate-100 text-slate-700';
}


/**
 * 获取分类的排序权重
 */
export function getCategoryOrder(category: string): number {
  return CATEGORY_ORDER[category] ?? 99;
}

/**
 * 获取节点类型在分组内的排序权重
 */
export function getNodeTypeOrder(typeId: string): number {
  return NODE_TYPE_ORDER[typeId] ?? 99;
}

// ============================================================================
// 同步缓存访问方法（供 nodeMeta / utils 等同步上下文使用）
// ============================================================================

/**
 * 获取已缓存的节点元数据（同步）
 * 如果缓存未加载则返回 null
 */
export function getCachedMetadata(): NodeMetadata[] | null {
  return cachedMetadata;
}

/**
 * 从缓存中获取指定类型的默认配置（同步）
 */
export function getCachedDefaultConfig(typeId: string): Record<string, unknown> | null {
  if (!cachedMetadata) return null;
  const meta = cachedMetadata.find(m => m.type_id === typeId);
  return meta?.default_config ?? null;
}

/**
 * 从缓存中获取指定类型的显示名称（同步）
 */
export function getCachedDisplayName(typeId: string): string | null {
  if (!cachedMetadata) return null;
  const meta = cachedMetadata.find(m => m.type_id === typeId);
  return meta?.display_name ?? null;
}

/**
 * 从缓存中获取指定类型的完整元数据（同步）
 */
export function getCachedNodeMeta(typeId: string): NodeMetadata | null {
  if (!cachedMetadata) return null;
  return cachedMetadata.find(m => m.type_id === typeId) ?? null;
}

// ============================================================================
// 插件化双栏面板目录结构与构建器
// ============================================================================

/** 节点算子目录条目 */
export interface CatalogItem {
  /** 节点类型标识，如 "http:request", "redis:command" */
  typeId: string;
  /** 显示名称 */
  name: string;
  /** 功能简述 */
  description: string;
  /** 图标组件 */
  icon: LucideIcon;
  /** 分组标识 */
  category: string;
  /** 所属插件标识（如 redis, kafka, db） */
  pluginId?: string;
  /** 所属插件显示名 */
  pluginName?: string;
  /** 是否为插件节点 */
  isPlugin: boolean;
  /** 端口定义 */
  ports?: Array<{ id: string; label: string; position: string }>;
}

/** 节点分组定义 */
export interface CatalogGroup {
  /** 分组唯一 ID */
  id: string;
  /** 分组显示名称 */
  name: string;
  /** 图标 */
  icon: LucideIcon;
  /** 分组类型 */
  kind: 'all' | 'recent' | 'core' | 'plugin';
  /** 插件 ID（当 kind === 'plugin'） */
  pluginId?: string;
  /** 插件版本（可选） */
  pluginVersion?: string;
  /** 包含节点总数 */
  count: number;
  /** 节点条目 */
  items: CatalogItem[];
}

/** 内置逻辑节点类型列表 */
const CORE_LOGIC_TYPES = [
  'conditional',
  'script',
  'assertion',
  'variable:extractor',
  'proxy',
  'end',
];

/** 内置网络协议节点类型列表 */
const CORE_NETWORK_TYPES = [
  'http:request',
  'http:ingress',
  'http:respond',
  'ws:connect',
  'ws:send_collect',
  'ws:close',
  'sse:connect',
  'sse:listen',
  'sse:disconnect',
  'graphql:connect',
  'graphql:query',
  'graphql:subscribe',
  'graphql:introspect',
  'graphql:disconnect',
  'tcp:connect',
  'tcp:send',
  'tcp:receive',
  'tcp:close',
  'udp:bind',
  'udp:send_to',
  'udp:recv_from',
  'udp:close',
];

/** 内置节点默认兜底列表（用于 Mock 或离线模式） */
export const BUILTIN_FALLBACK_METADATA: NodeMetadata[] = [
  { type_id: 'http:request', display_name: 'HTTP 请求', category: 'http', icon: 'globe', color: '#3B82F6', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }, { id: 'false', label: 'False', position: 'bottom' }], default_config: {} },
  { type_id: 'http:ingress', display_name: 'HTTP Ingress 监听', category: 'network', icon: 'log-in', color: '#6366F1', ports: [{ id: 'out', label: 'Out', position: 'right' }], default_config: { port: 9527, path_pattern: '/*', methods: ['GET', 'POST'] } },
  { type_id: 'http:respond', display_name: 'HTTP Respond 响应', category: 'network', icon: 'log-out', color: '#14B8A6', ports: [{ id: 'in', label: 'In', position: 'left' }, { id: 'out', label: 'Out', position: 'right' }], default_config: { status_code: 200, headers: {}, body_type: 'json', body: '{"success": true}' } },
  { type_id: 'conditional', display_name: '条件判断', category: 'logic', icon: 'git-branch', color: '#F97316', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }, { id: 'false', label: 'False', position: 'bottom' }], default_config: {} },
  { type_id: 'script', display_name: '脚本', category: 'script', icon: 'code', color: '#6366F1', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'assertion', display_name: '断言', category: 'assertion', icon: 'check-circle', color: '#A855F7', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }, { id: 'false', label: 'False', position: 'bottom' }], default_config: {} },
  { type_id: 'variable:extractor', display_name: '变量提取', category: 'tools', icon: 'braces', color: '#F59E0B', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'proxy', display_name: '代理设置', category: 'tools', icon: 'shield', color: '#84CC16', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'end', display_name: '终止', category: 'tools', icon: 'square', color: '#8B5CF6', ports: [{ id: 'flow_in', label: 'In', position: 'top' }], default_config: {} },
  { type_id: 'ws:connect', display_name: 'WS-连接', category: 'websocket', icon: 'wifi', color: '#F59E0B', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'ws:send_collect', display_name: 'WS-发送采集', category: 'websocket', icon: 'send', color: '#14B8A6', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'ws:close', display_name: 'WS-关闭', category: 'websocket', icon: 'plug', color: '#F43F5E', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'sse:connect', display_name: 'SSE 连接', category: 'sse', icon: 'radio-tower', color: '#F43F5E', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'sse:listen', display_name: 'SSE 监听', category: 'sse', icon: 'ear', color: '#F43F5E', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'sse:disconnect', display_name: 'SSE 断开', category: 'sse', icon: 'unplug', color: '#F43F5E', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'graphql:connect', display_name: 'GraphQL 连接', category: 'graphql', icon: 'plug', color: '#6366F1', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'graphql:query', display_name: 'GraphQL 查询', category: 'graphql', icon: 'search', color: '#6366F1', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'graphql:subscribe', display_name: 'GraphQL 订阅', category: 'graphql', icon: 'eye', color: '#6366F1', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'graphql:introspect', display_name: 'GraphQL 内省', category: 'graphql', icon: 'file-search', color: '#6366F1', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'graphql:disconnect', display_name: 'GraphQL 断开', category: 'graphql', icon: 'unplug', color: '#6366F1', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'tcp:connect', display_name: 'TCP 连接', category: 'tcp', icon: 'cable', color: '#10B981', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'tcp:send', display_name: 'TCP 发送', category: 'tcp', icon: 'send', color: '#10B981', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'tcp:receive', display_name: 'TCP 接收', category: 'tcp', icon: 'download', color: '#10B981', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'tcp:close', display_name: 'TCP 关闭', category: 'tcp', icon: 'unplug', color: '#10B981', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'udp:bind', display_name: 'UDP 绑定', category: 'udp', icon: 'radio', color: '#F97316', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'udp:send_to', display_name: 'UDP 发送', category: 'udp', icon: 'arrow-up-right', color: '#F97316', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'udp:recv_from', display_name: 'UDP 接收', category: 'udp', icon: 'arrow-down-left', color: '#F97316', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
  { type_id: 'udp:close', display_name: 'UDP 关闭', category: 'udp', icon: 'unplug', color: '#F97316', ports: [{ id: 'flow_in', label: 'In', position: 'top' }, { id: 'true', label: 'True', position: 'right' }], default_config: {} },
];

/**
 * 将 NodeMetadata 转换为 CatalogItem
 */
export function convertToCatalogItem(meta: NodeMetadata): CatalogItem {
  // 插件判定只看「是否属于内置目录」：type_id 里的 `:` 是命名空间分隔符，
  // 内置节点 `http:ingress` / `http:respond` 同样带 `:`，不能据此判为插件
  // （否则会在选择器里被标成插件并显示插件徽章）。
  const isPlugin =
    !CORE_LOGIC_TYPES.includes(meta.type_id) && !CORE_NETWORK_TYPES.includes(meta.type_id);
  let pluginId: string | undefined;
  let pluginName: string | undefined;

  if (isPlugin) {
    if (meta.type_id.includes(':')) {
      pluginId = meta.type_id.split(':')[0];
      pluginName = pluginId.charAt(0).toUpperCase() + pluginId.slice(1);
    } else {
      pluginId = meta.category;
      pluginName = meta.category;
    }
  }

  const icon = ICON_MAP[meta.type_id]?.icon ?? resolveBackendIcon(meta.icon) ?? BACKEND_ICON_MAP[meta.icon] ?? Puzzle;

  return {
    typeId: meta.type_id,
    name: getNodeDisplayName(meta.type_id) || meta.display_name,
    description: getNodeDescription(meta.type_id),
    icon,
    category: meta.category,
    pluginId,
    pluginName,
    isPlugin,
    ports: meta.ports,
  };
}

/**
 * 构建面向插件化双栏面板的分组列表
 *
 * @param metadata - 后端返回的节点元数据
 * @param recentTypeIds - 最近使用的节点 type_id 数组
 * @returns 结构化的分组数组
 */
export function buildCatalogGroups(
  metadata: NodeMetadata[],
  recentTypeIds: string[] = []
): CatalogGroup[] {
  const source = metadata.length > 0 ? metadata : BUILTIN_FALLBACK_METADATA;
  const filtered = source.filter((m) => m.type_id !== 'entry');
  const allItems = filtered.map(convertToCatalogItem);

  const groups: CatalogGroup[] = [];

  // 1. 全部节点
  groups.push({
    id: 'all',
    name: getLL().nodes.palette.all_nodes(),
    icon: Sparkles,
    kind: 'all',
    count: allItems.length,
    items: allItems,
  });

  // 2. 最近常用（如果存在）
  if (recentTypeIds.length > 0) {
    const recentItems = recentTypeIds
      .map((id) => allItems.find((item) => item.typeId === id))
      .filter((item): item is CatalogItem => Boolean(item));
    if (recentItems.length > 0) {
      groups.push({
        id: 'recent',
        name: getLL().nodes.palette.recent_nodes(),
        icon: Clock,
        kind: 'recent',
        count: recentItems.length,
        items: recentItems,
      });
    }
  }

  // 3. 核心控制流 (Logic)
  const logicItems = allItems.filter((item) => CORE_LOGIC_TYPES.includes(item.typeId));
  if (logicItems.length > 0) {
    logicItems.sort((a, b) => getNodeTypeOrder(a.typeId) - getNodeTypeOrder(b.typeId));
    groups.push({
      id: 'core_logic',
      name: getLL().nodes.palette.core_logic(),
      icon: GitBranch,
      kind: 'core',
      count: logicItems.length,
      items: logicItems,
    });
  }

  // 4. 网络通信协议 (Network)
  const networkItems = allItems.filter((item) => CORE_NETWORK_TYPES.includes(item.typeId));
  if (networkItems.length > 0) {
    networkItems.sort((a, b) => {
      const catA = getCategoryOrder(a.category);
      const catB = getCategoryOrder(b.category);
      if (catA !== catB) return catA - catB;
      return getNodeTypeOrder(a.typeId) - getNodeTypeOrder(b.typeId);
    });
    groups.push({
      id: 'core_network',
      name: getLL().nodes.palette.core_network(),
      icon: Globe,
      kind: 'core',
      count: networkItems.length,
      items: networkItems,
    });
  }

  // 5. 动态插件分组 (Plugin Groups)
  const pluginMap = new Map<string, CatalogItem[]>();
  for (const item of allItems) {
    if (item.isPlugin && item.pluginId) {
      if (!pluginMap.has(item.pluginId)) {
        pluginMap.set(item.pluginId, []);
      }
      pluginMap.get(item.pluginId)!.push(item);
    }
  }

  for (const [pluginId, items] of pluginMap.entries()) {
    const pluginDisplayName = items[0]?.pluginName || pluginId;
    const sampleIcon = items[0]?.icon || Puzzle;
    groups.push({
      id: `plugin:${pluginId}`,
      name: getLL().nodes.palette.plugin_suffix({ name: pluginDisplayName }),
      icon: sampleIcon,
      kind: 'plugin',
      pluginId,
      count: items.length,
      items,
    });
  }

  return groups;
}
