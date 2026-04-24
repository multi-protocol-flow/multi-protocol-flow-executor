/**
 * 节点注册表服务
 * 从后端动态获取节点元数据，缓存并按分类分组
 */
import { invoke } from '@tauri-apps/api/core';
import type { NodeMetadata } from '@/types/nodeRegistry';

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
  http: 'HTTP',
  websocket: 'WS',
  database: 'DB',
  tools: 'Tool',
  script: '脚本',
  assertion: '断言',
  grpc: 'gRPC',
  mcp: 'MCP',
};

/**
 * 分类排序顺序（越小越靠前）
 * http → ws → grpc → mcp → db → tool
 */
export const CATEGORY_ORDER: Record<string, number> = {
  http: 0,
  websocket: 1,
  grpc: 2,
  mcp: 3,
  database: 4,
  tools: 5,
  script: 6,
  assertion: 7,
};

/**
 * 分组内节点排序（越小越靠前）
 * 协议类: 连接 → 调用/发送 → 关闭
 * 数据库: 连接 → 开启事务 → 查询 → 提交 → 回滚 → 关闭
 * 工具类: 条件判断 → 脚本 → 断言
 */
const NODE_TYPE_ORDER: Record<string, number> = {
  // HTTP (单项，无排序需求)
  http: 0,

  // WebSocket: 连接 → 发送采集 → 关闭
  ws_connect: 0,
  ws_send_collect: 1,
  ws_close: 2,

  // gRPC: 连接 → 调用 → 关闭
  grpc_connect: 0,
  grpc_call: 1,
  grpc_close: 2,

  // MCP: 连接 → 调用工具 → 读取资源 → 获取提示 → 关闭
  mcp_connect: 0,
  mcp_call_tool: 1,
  mcp_read_resource: 2,
  mcp_get_prompt: 3,
  mcp_close: 4,

  // 数据库: 连接 → 开启事务 → 查询 → 提交 → 回滚 → 关闭
  db_connect: 0,
  db_begin: 1,
  db_query: 2,
  db_commit: 3,
  db_rollback: 4,
  db_close: 5,

  // 工具: 条件判断 → 脚本 → 断言
  conditional: 0,
  script: 1,
  assertion: 2,
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
    http: 'bg-blue-100 text-blue-700',
    conditional: 'bg-orange-100 text-orange-700',
    script: 'bg-indigo-100 text-indigo-700',
    assertion: 'bg-purple-100 text-purple-700',
    ws_connect: 'bg-amber-100 text-amber-700',
    ws_send_collect: 'bg-teal-100 text-teal-700',
    ws_close: 'bg-rose-100 text-rose-700',
    db_connect: 'bg-violet-100 text-violet-700',
    db_query: 'bg-violet-100 text-violet-700',
    db_begin: 'bg-violet-100 text-violet-700',
    db_commit: 'bg-violet-100 text-violet-700',
    db_rollback: 'bg-violet-100 text-violet-700',
    db_close: 'bg-violet-100 text-violet-700',
    grpc_connect: 'bg-sky-100 text-sky-700',
    grpc_call: 'bg-sky-100 text-sky-700',
    grpc_close: 'bg-sky-100 text-sky-700',
    mcp_connect: 'bg-amber-100 text-amber-700',
    mcp_call_tool: 'bg-amber-100 text-amber-700',
    mcp_read_resource: 'bg-amber-100 text-amber-700',
    mcp_get_prompt: 'bg-amber-100 text-amber-700',
    mcp_close: 'bg-amber-100 text-amber-700',
  };
  return badgeMap[typeId] ?? 'bg-slate-100 text-slate-700';
}

/**
 * 从元数据构建节点类型 → 颜色映射
 *
 * @param metadata - 节点元数据数组
 * @returns type_id → 十六进制颜色 的映射
 */
export function buildTypeColorMap(
  metadata: NodeMetadata[]
): Record<string, string> {
  const map: Record<string, string> = {};
  for (const m of metadata) {
    map[m.type_id] = m.color;
  }
  return map;
}

/**
 * 从元数据构建节点类型 → 显示名称映射
 *
 * @param metadata - 节点元数据数组
 * @returns type_id → display_name 的映射
 */
export function buildTypeNameMap(
  metadata: NodeMetadata[]
): Record<string, string> {
  const map: Record<string, string> = {};
  for (const m of metadata) {
    map[m.type_id] = m.display_name;
  }
  return map;
}

/**
 * 从元数据构建节点类型 → 默认配置映射
 *
 * @param metadata - 节点元数据数组
 * @returns type_id → default_config 的映射
 */
export function buildDefaultConfigMap(
  metadata: NodeMetadata[]
): Record<string, Record<string, unknown>> {
  const map: Record<string, Record<string, unknown>> = {};
  for (const m of metadata) {
    map[m.type_id] = m.default_config;
  }
  return map;
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
