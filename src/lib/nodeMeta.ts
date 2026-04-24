/**
 * 节点类型元数据
 * 提供 18 种节点类型的图标、颜色映射，以及动态显示名称获取
 *
 * - ICON_MAP: 前端独有数据（图标组件），不可从后端获取
 * - getNodeDisplayName(): 优先从 NodeRegistry 缓存获取动态名称
 * - getNodeMeta(): 合并动态名称 + 前端图标数据
 */
import type { LucideIcon } from 'lucide-react';
import {
  Play,
  Globe,
  GitBranch,
  Code,
  CheckCircle,
  Wifi,
  Send,
  Plug,
  Database,
  CirclePlay,
  Check,
  RotateCcw,
  Cable,
  PhoneOutgoing,
  PhoneOff,
  Puzzle,
  Wrench,
  FileSearch,
  Unplug,
} from 'lucide-react';
import { getCachedDisplayName } from '@/lib/NodeRegistry';

/** 节点类型元数据 */
export interface NodeTypeMeta {
  /** 显示名称 */
  name: string;
  /** lucide-react 图标组件 */
  icon: LucideIcon;
  /** 图标背景色 class (如 'bg-blue-500') */
  iconBg: string;
  /** 标签颜色 class (如 'text-blue-600') */
  labelColor: string;
  /** 徽章颜色 class (如 'bg-blue-100 text-blue-700') */
  badge: string;
}

/** 节点图标映射（前端独有，不可从后端获取） */
export const ICON_MAP: Record<string, { icon: LucideIcon; iconBg: string; labelColor: string; badge: string }> = {
  entry: { icon: Play, iconBg: 'bg-emerald-500', labelColor: 'text-emerald-600', badge: 'bg-emerald-100 text-emerald-700' },
  http: { icon: Globe, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },
  conditional: { icon: GitBranch, iconBg: 'bg-orange-500', labelColor: 'text-orange-600', badge: 'bg-orange-100 text-orange-700' },
  script: { icon: Code, iconBg: 'bg-indigo-500', labelColor: 'text-indigo-600', badge: 'bg-indigo-100 text-indigo-700' },
  assertion: { icon: CheckCircle, iconBg: 'bg-purple-500', labelColor: 'text-purple-600', badge: 'bg-purple-100 text-purple-700' },
  ws_connect: { icon: Wifi, iconBg: 'bg-amber-500', labelColor: 'text-amber-600', badge: 'bg-amber-100 text-amber-700' },
  ws_send_collect: { icon: Send, iconBg: 'bg-teal-500', labelColor: 'text-teal-600', badge: 'bg-teal-100 text-teal-700' },
  ws_close: { icon: Plug, iconBg: 'bg-rose-500', labelColor: 'text-rose-600', badge: 'bg-rose-100 text-rose-700' },
  db_connect: { icon: Database, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  db_query: { icon: Database, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  db_begin: { icon: CirclePlay, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  db_commit: { icon: Check, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  db_rollback: { icon: RotateCcw, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  db_close: { icon: Plug, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  grpc_connect: { icon: Cable, iconBg: 'bg-sky-500', labelColor: 'text-sky-600', badge: 'bg-sky-100 text-sky-700' },
  grpc_call: { icon: PhoneOutgoing, iconBg: 'bg-sky-500', labelColor: 'text-sky-600', badge: 'bg-sky-100 text-sky-700' },
  grpc_close: { icon: PhoneOff, iconBg: 'bg-sky-500', labelColor: 'text-sky-600', badge: 'bg-sky-100 text-sky-700' },
  mcp_connect: { icon: Puzzle, iconBg: 'bg-fuchsia-500', labelColor: 'text-fuchsia-600', badge: 'bg-fuchsia-100 text-fuchsia-700' },
  mcp_call_tool: { icon: Wrench, iconBg: 'bg-fuchsia-500', labelColor: 'text-fuchsia-600', badge: 'bg-fuchsia-100 text-fuchsia-700' },
  mcp_read_resource: { icon: FileSearch, iconBg: 'bg-fuchsia-500', labelColor: 'text-fuchsia-600', badge: 'bg-fuchsia-100 text-fuchsia-700' },
  mcp_close: { icon: Unplug, iconBg: 'bg-fuchsia-500', labelColor: 'text-fuchsia-600', badge: 'bg-fuchsia-100 text-fuchsia-700' },
};

/** 静态显示名称兜底映射 */
const STATIC_DISPLAY_NAMES: Record<string, string> = {
  entry: '入口',
  http: 'HTTP 请求',
  conditional: '条件判断',
  script: '脚本',
  assertion: '断言',
  ws_connect: 'WS-连接',
  ws_send_collect: 'WS-发送采集',
  ws_close: 'WS-关闭',
  db_connect: 'DB-连接',
  db_query: 'DB-查询',
  db_begin: 'DB-开始事务',
  db_commit: 'DB-提交',
  db_rollback: 'DB-回滚',
  db_close: 'DB-关闭',
  grpc_connect: 'gRPC-连接',
  grpc_call: 'gRPC-调用',
  grpc_close: 'gRPC-关闭',
  mcp_connect: 'MCP-连接',
  mcp_call_tool: 'MCP-调用工具',
  mcp_read_resource: 'MCP-读取资源',
  mcp_close: 'MCP-关闭',
};

/**
 * 获取节点显示名称：优先从后端缓存获取
 *
 * @param typeId - 节点类型标识
 * @returns 显示名称，兜底返回 typeId 本身
 */
export function getNodeDisplayName(typeId: string): string {
  const cached = getCachedDisplayName(typeId);
  if (cached) return cached;
  return STATIC_DISPLAY_NAMES[typeId] ?? typeId;
}

/**
 * 获取节点元数据：合并后端动态名称 + 前端图标
 *
 * @param typeId - 节点类型标识
 * @returns 合并后的元数据，无匹配时返回 null
 */
export function getNodeMeta(typeId: string): NodeTypeMeta | null {
  const iconData = ICON_MAP[typeId];
  if (!iconData) return null;
  return {
    name: getNodeDisplayName(typeId),
    ...iconData,
  };
}

/**
 * 向后兼容：保留 NODE_META 导出，内部通过 getNodeMeta 动态构建
 *
 * @deprecated 优先使用 getNodeMeta(typeId) 或 getNodeDisplayName(typeId)
 */
export const NODE_META: Record<string, NodeTypeMeta> = new Proxy({} as Record<string, NodeTypeMeta>, {
  get(_target, prop: string) {
    return getNodeMeta(prop);
  },
});
