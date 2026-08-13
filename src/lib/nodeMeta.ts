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
  Cable,
  PhoneOutgoing,
  PhoneOff,
  FileSearch,
  Unplug,
  Radio,
  ArrowUpRight,
  ArrowDownLeft,
  Download,
  Eye,
  Search,
  Ear,
  RadioTower,
  Braces,
  Shield,
  Square,
} from 'lucide-react';
import { getCachedDisplayName } from '@/lib/NodeRegistry';
import { getLL } from '@/i18n/i18n-lazy';

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
  grpc_connect: { icon: Cable, iconBg: 'bg-sky-500', labelColor: 'text-sky-600', badge: 'bg-sky-100 text-sky-700' },
  grpc_call: { icon: PhoneOutgoing, iconBg: 'bg-sky-500', labelColor: 'text-sky-600', badge: 'bg-sky-100 text-sky-700' },
  grpc_close: { icon: PhoneOff, iconBg: 'bg-sky-500', labelColor: 'text-sky-600', badge: 'bg-sky-100 text-sky-700' },
  tcp_connect: { icon: Cable, iconBg: 'bg-emerald-500', labelColor: 'text-emerald-600', badge: 'bg-emerald-100 text-emerald-700' },
  tcp_send: { icon: Send, iconBg: 'bg-emerald-500', labelColor: 'text-emerald-600', badge: 'bg-emerald-100 text-emerald-700' },
  tcp_close: { icon: Unplug, iconBg: 'bg-emerald-500', labelColor: 'text-emerald-600', badge: 'bg-emerald-100 text-emerald-700' },
  udp_bind: { icon: Radio, iconBg: 'bg-orange-500', labelColor: 'text-orange-600', badge: 'bg-orange-100 text-orange-700' },
  udp_send_to: { icon: ArrowUpRight, iconBg: 'bg-orange-500', labelColor: 'text-orange-600', badge: 'bg-orange-100 text-orange-700' },
  udp_recv_from: { icon: ArrowDownLeft, iconBg: 'bg-orange-500', labelColor: 'text-orange-600', badge: 'bg-orange-100 text-orange-700' },
  udp_close: { icon: Unplug, iconBg: 'bg-orange-500', labelColor: 'text-orange-600', badge: 'bg-orange-100 text-orange-700' },

  // TCP (补充)
  tcp_receive: { icon: Download, iconBg: 'bg-emerald-500', labelColor: 'text-emerald-600', badge: 'bg-emerald-100 text-emerald-700' },



  // SSE (3个) — rose 色
  sse_connect: { icon: RadioTower, iconBg: 'bg-rose-500', labelColor: 'text-rose-600', badge: 'bg-rose-100 text-rose-700' },
  sse_listen: { icon: Ear, iconBg: 'bg-rose-500', labelColor: 'text-rose-600', badge: 'bg-rose-100 text-rose-700' },
  sse_disconnect: { icon: Unplug, iconBg: 'bg-rose-500', labelColor: 'text-rose-600', badge: 'bg-rose-100 text-rose-700' },

  // GraphQL (6个) — indigo 色
  graphql_connect: { icon: Plug, iconBg: 'bg-indigo-500', labelColor: 'text-indigo-600', badge: 'bg-indigo-100 text-indigo-700' },
  graphql_query: { icon: Search, iconBg: 'bg-indigo-500', labelColor: 'text-indigo-600', badge: 'bg-indigo-100 text-indigo-700' },
  graphql_subscribe: { icon: Eye, iconBg: 'bg-indigo-500', labelColor: 'text-indigo-600', badge: 'bg-indigo-100 text-indigo-700' },
  graphql_introspect: { icon: FileSearch, iconBg: 'bg-indigo-500', labelColor: 'text-indigo-600', badge: 'bg-indigo-100 text-indigo-700' },
  graphql_disconnect: { icon: Unplug, iconBg: 'bg-indigo-500', labelColor: 'text-indigo-600', badge: 'bg-indigo-100 text-indigo-700' },

  // AMQP 0-9-1 (12个) — violet 色



  // 工具
  variable_extractor: { icon: Braces, iconBg: 'bg-amber-500', labelColor: 'text-amber-600', badge: 'bg-amber-100 text-amber-700' },
  proxy: { icon: Shield, iconBg: 'bg-lime-500', labelColor: 'text-lime-600', badge: 'bg-lime-100 text-lime-700' },
  end: { icon: Square, iconBg: 'bg-purple-500', labelColor: 'text-purple-600', badge: 'bg-purple-100 text-purple-700' },
};

/** 静态显示名称映射（通过 getLL() 获取当前 locale 的翻译） */
function getStaticDisplayNames(): Record<string, string> {
  const LL = getLL();
  return {
    entry: LL.nodes.entry(),
    http: LL.nodes.http(),
    conditional: LL.nodes.conditional(),
    script: LL.nodes.script(),
    assertion: LL.nodes.assertion(),
    ws_connect: LL.nodes.ws_connect(),
    ws_send_collect: LL.nodes.ws_send_collect(),
    ws_close: LL.nodes.ws_close(),
    grpc_connect: LL.nodes.grpc_connect(),
    grpc_call: LL.nodes.grpc_call(),
    grpc_close: LL.nodes.grpc_close(),
    tcp_connect: LL.nodes.tcp_connect(),
    tcp_send: LL.nodes.tcp_send(),
    tcp_close: LL.nodes.tcp_close(),
    udp_bind: LL.nodes.udp_bind(),
    udp_send_to: LL.nodes.udp_send_to(),
    udp_recv_from: LL.nodes.udp_recv_from(),
    udp_close: LL.nodes.udp_close(),
    // TCP
    tcp_receive: LL.nodes.tcp_receive(),
    // SSE
    sse_connect: LL.nodes.sse_connect(),
    sse_listen: LL.nodes.sse_listen(),
    sse_disconnect: LL.nodes.sse_disconnect(),
    // GraphQL
    graphql_connect: LL.nodes.graphql_connect(),
    graphql_query: LL.nodes.graphql_query(),
    graphql_subscribe: LL.nodes.graphql_subscribe(),
    graphql_introspect: LL.nodes.graphql_introspect(),
    graphql_disconnect: LL.nodes.graphql_disconnect(),
    // AMQP 0-9-1
    // AMQP 1.0
    // 工具
    variable_extractor: LL.nodes.variable_extractor(),
    proxy: LL.nodes.proxy(),
    end: LL.nodes.end(),
  };
}




/**
 * 获取节点显示名称：优先使用当前语言翻译，支持中/英文动态切换
 *
 * 查找优先级：前端 i18n（当前 locale）→ 后端缓存（兜底）→ typeId
 *
 * @param typeId - 节点类型标识
 * @returns 显示名称，兜底返回 typeId 本身
 */
export function getNodeDisplayName(typeId: string): string {
  // 优先：当前 locale 的 i18n 翻译（支持中/英文动态切换）
  const i18nName = getStaticDisplayNames()[typeId];
  if (i18nName) return i18nName;
  // 兜底：后端缓存（未知类型时使用）
  const cached = getCachedDisplayName(typeId);
  if (cached) return cached;
  return typeId;
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
