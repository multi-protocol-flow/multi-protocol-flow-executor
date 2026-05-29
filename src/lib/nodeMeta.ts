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
  MessageSquare,
  Unplug,
  Radio,
  ArrowUpRight,
  ArrowDownLeft,
  Download,
  Timer,
  Eye,
  Search,
  Mail,
  Layers,
  Settings,
  FileText,
  Ear,
  RadioTower,
  Shuffle,
  Trash2,
  List,
  Trash,
  Link2,
  Inbox,
  XCircle,
  Braces,
  Copy,
  FolderPlus,
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
  mcp_get_prompt: { icon: MessageSquare, iconBg: 'bg-fuchsia-500', labelColor: 'text-fuchsia-600', badge: 'bg-fuchsia-100 text-fuchsia-700' },
  mcp_close: { icon: Unplug, iconBg: 'bg-fuchsia-500', labelColor: 'text-fuchsia-600', badge: 'bg-fuchsia-100 text-fuchsia-700' },
  tcp_connect: { icon: Cable, iconBg: 'bg-emerald-500', labelColor: 'text-emerald-600', badge: 'bg-emerald-100 text-emerald-700' },
  tcp_send: { icon: Send, iconBg: 'bg-emerald-500', labelColor: 'text-emerald-600', badge: 'bg-emerald-100 text-emerald-700' },
  tcp_close: { icon: Unplug, iconBg: 'bg-emerald-500', labelColor: 'text-emerald-600', badge: 'bg-emerald-100 text-emerald-700' },
  udp_bind: { icon: Radio, iconBg: 'bg-orange-500', labelColor: 'text-orange-600', badge: 'bg-orange-100 text-orange-700' },
  udp_send_to: { icon: ArrowUpRight, iconBg: 'bg-orange-500', labelColor: 'text-orange-600', badge: 'bg-orange-100 text-orange-700' },
  udp_recv_from: { icon: ArrowDownLeft, iconBg: 'bg-orange-500', labelColor: 'text-orange-600', badge: 'bg-orange-100 text-orange-700' },
  udp_close: { icon: Unplug, iconBg: 'bg-orange-500', labelColor: 'text-orange-600', badge: 'bg-orange-100 text-orange-700' },

  // TCP (补充)
  tcp_receive: { icon: Download, iconBg: 'bg-emerald-500', labelColor: 'text-emerald-600', badge: 'bg-emerald-100 text-emerald-700' },

  // MQTT (7个) — cyan 色
  mqtt_connect: { icon: Radio, iconBg: 'bg-cyan-500', labelColor: 'text-cyan-600', badge: 'bg-cyan-100 text-cyan-700' },
  mqtt_publish: { icon: Send, iconBg: 'bg-cyan-500', labelColor: 'text-cyan-600', badge: 'bg-cyan-100 text-cyan-700' },
  mqtt_subscribe: { icon: ArrowDownLeft, iconBg: 'bg-cyan-500', labelColor: 'text-cyan-600', badge: 'bg-cyan-100 text-cyan-700' },
  mqtt_unsubscribe: { icon: XCircle, iconBg: 'bg-cyan-500', labelColor: 'text-cyan-600', badge: 'bg-cyan-100 text-cyan-700' },
  mqtt_disconnect: { icon: Unplug, iconBg: 'bg-cyan-500', labelColor: 'text-cyan-600', badge: 'bg-cyan-100 text-cyan-700' },
  mqtt_wait_message: { icon: Timer, iconBg: 'bg-cyan-500', labelColor: 'text-cyan-600', badge: 'bg-cyan-100 text-cyan-700' },
  mqtt_request: { icon: MessageSquare, iconBg: 'bg-cyan-500', labelColor: 'text-cyan-600', badge: 'bg-cyan-100 text-cyan-700' },

  // Kafka (7个) — rose 色
  kafka_connect: { icon: Radio, iconBg: 'bg-rose-500', labelColor: 'text-rose-600', badge: 'bg-rose-100 text-rose-700' },
  kafka_produce: { icon: Send, iconBg: 'bg-rose-500', labelColor: 'text-rose-600', badge: 'bg-rose-100 text-rose-700' },
  kafka_consume: { icon: Download, iconBg: 'bg-rose-500', labelColor: 'text-rose-600', badge: 'bg-rose-100 text-rose-700' },
  kafka_consume_batch: { icon: Layers, iconBg: 'bg-rose-500', labelColor: 'text-rose-600', badge: 'bg-rose-100 text-rose-700' },
  kafka_admin: { icon: Settings, iconBg: 'bg-rose-500', labelColor: 'text-rose-600', badge: 'bg-rose-100 text-rose-700' },
  kafka_schema_registry: { icon: FileText, iconBg: 'bg-rose-500', labelColor: 'text-rose-600', badge: 'bg-rose-100 text-rose-700' },
  kafka_close: { icon: XCircle, iconBg: 'bg-rose-500', labelColor: 'text-rose-600', badge: 'bg-rose-100 text-rose-700' },

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
  amqp_connect: { icon: RadioTower, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  amqp_exchange_declare: { icon: Shuffle, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  amqp_exchange_delete: { icon: Trash2, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  amqp_queue_declare: { icon: List, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  amqp_queue_delete: { icon: Trash, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  amqp_queue_bind: { icon: Link2, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  amqp_publish: { icon: Send, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  amqp_consume: { icon: Inbox, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  amqp_get: { icon: Download, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  amqp_ack: { icon: CheckCircle, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  amqp_management: { icon: Settings, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },
  amqp_close: { icon: Unplug, iconBg: 'bg-violet-500', labelColor: 'text-violet-600', badge: 'bg-violet-100 text-violet-700' },

  // AMQP 1.0 (6个) — purple 色
  amqp10_connect: { icon: RadioTower, iconBg: 'bg-purple-500', labelColor: 'text-purple-600', badge: 'bg-purple-100 text-purple-700' },
  amqp10_send: { icon: Send, iconBg: 'bg-purple-500', labelColor: 'text-purple-600', badge: 'bg-purple-100 text-purple-700' },
  amqp10_receive: { icon: Inbox, iconBg: 'bg-purple-500', labelColor: 'text-purple-600', badge: 'bg-purple-100 text-purple-700' },
  amqp10_accept: { icon: CheckCircle, iconBg: 'bg-purple-500', labelColor: 'text-purple-600', badge: 'bg-purple-100 text-purple-700' },
  amqp10_reject: { icon: XCircle, iconBg: 'bg-purple-500', labelColor: 'text-purple-600', badge: 'bg-purple-100 text-purple-700' },
  amqp10_disconnect: { icon: Unplug, iconBg: 'bg-purple-500', labelColor: 'text-purple-600', badge: 'bg-purple-100 text-purple-700' },

  // SMTP (3个) — orange 色
  smtp_connect: { icon: Mail, iconBg: 'bg-orange-500', labelColor: 'text-orange-600', badge: 'bg-orange-100 text-orange-700' },
  smtp_send: { icon: Send, iconBg: 'bg-orange-500', labelColor: 'text-orange-600', badge: 'bg-orange-100 text-orange-700' },
  smtp_disconnect: { icon: Mail, iconBg: 'bg-orange-500', labelColor: 'text-orange-600', badge: 'bg-orange-100 text-orange-700' },

  // IMAP (13个) — blue 色
  imap_connect: { icon: Mail, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },
  imap_select: { icon: Mail, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },
  imap_search: { icon: Search, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },
  imap_fetch: { icon: Download, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },
  imap_store_flags: { icon: CheckCircle, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },
  imap_copy: { icon: Copy, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },
  imap_move: { icon: ArrowUpRight, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },
  imap_expunge: { icon: Trash2, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },
  imap_list: { icon: List, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },
  imap_folder_create: { icon: FolderPlus, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },
  imap_folder_delete: { icon: Trash, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },
  imap_idle: { icon: Timer, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },
  imap_close: { icon: Mail, iconBg: 'bg-blue-500', labelColor: 'text-blue-600', badge: 'bg-blue-100 text-blue-700' },

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
    db_connect: LL.nodes.db_connect(),
    db_query: LL.nodes.db_query(),
    db_begin: LL.nodes.db_begin(),
    db_commit: LL.nodes.db_commit(),
    db_rollback: LL.nodes.db_rollback(),
    db_close: LL.nodes.db_close(),
    grpc_connect: LL.nodes.grpc_connect(),
    grpc_call: LL.nodes.grpc_call(),
    grpc_close: LL.nodes.grpc_close(),
    mcp_connect: LL.nodes.mcp_connect(),
    mcp_call_tool: LL.nodes.mcp_call_tool(),
    mcp_read_resource: LL.nodes.mcp_read_resource(),
    mcp_get_prompt: LL.nodes.mcp_get_prompt(),
    mcp_close: LL.nodes.mcp_close(),
    tcp_connect: LL.nodes.tcp_connect(),
    tcp_send: LL.nodes.tcp_send(),
    tcp_close: LL.nodes.tcp_close(),
    udp_bind: LL.nodes.udp_bind(),
    udp_send_to: LL.nodes.udp_send_to(),
    udp_recv_from: LL.nodes.udp_recv_from(),
    udp_close: LL.nodes.udp_close(),
    // TCP
    tcp_receive: LL.nodes.tcp_receive(),
    // MQTT
    mqtt_connect: LL.nodes.mqtt_connect(),
    mqtt_publish: LL.nodes.mqtt_publish(),
    mqtt_subscribe: LL.nodes.mqtt_subscribe(),
    mqtt_unsubscribe: LL.nodes.mqtt_unsubscribe(),
    mqtt_disconnect: LL.nodes.mqtt_disconnect(),
    mqtt_wait_message: LL.nodes.mqtt_wait_message(),
    mqtt_request: LL.nodes.mqtt_request(),
    // Kafka
    kafka_connect: LL.nodes.kafka_connect(),
    kafka_produce: LL.nodes.kafka_produce(),
    kafka_consume: LL.nodes.kafka_consume(),
    kafka_consume_batch: LL.nodes.kafka_consume_batch(),
    kafka_admin: LL.nodes.kafka_admin(),
    kafka_schema_registry: LL.nodes.kafka_schema_registry(),
    kafka_close: LL.nodes.kafka_close(),
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
    amqp_connect: LL.nodes.amqp_connect(),
    amqp_exchange_declare: LL.nodes.amqp_exchange_declare(),
    amqp_exchange_delete: LL.nodes.amqp_exchange_delete(),
    amqp_queue_declare: LL.nodes.amqp_queue_declare(),
    amqp_queue_delete: LL.nodes.amqp_queue_delete(),
    amqp_queue_bind: LL.nodes.amqp_queue_bind(),
    amqp_publish: LL.nodes.amqp_publish(),
    amqp_consume: LL.nodes.amqp_consume(),
    amqp_get: LL.nodes.amqp_get(),
    amqp_ack: LL.nodes.amqp_ack(),
    amqp_management: LL.nodes.amqp_management(),
    amqp_close: LL.nodes.amqp_close(),
    // AMQP 1.0
    amqp10_connect: LL.nodes.amqp10_connect(),
    amqp10_send: LL.nodes.amqp10_send(),
    amqp10_receive: LL.nodes.amqp10_receive(),
    amqp10_accept: LL.nodes.amqp10_accept(),
    amqp10_reject: LL.nodes.amqp10_reject(),
    amqp10_disconnect: LL.nodes.amqp10_disconnect(),
    // SMTP
    smtp_connect: LL.nodes.smtp_connect(),
    smtp_send: LL.nodes.smtp_send(),
    smtp_disconnect: LL.nodes.smtp_disconnect(),
    // IMAP
    imap_connect: LL.nodes.imap_connect(),
    imap_select: LL.nodes.imap_select(),
    imap_search: LL.nodes.imap_search(),
    imap_fetch: LL.nodes.imap_fetch(),
    imap_store_flags: LL.nodes.imap_store_flags(),
    imap_copy: LL.nodes.imap_copy(),
    imap_move: LL.nodes.imap_move(),
    imap_expunge: LL.nodes.imap_expunge(),
    imap_list: LL.nodes.imap_list(),
    imap_folder_create: LL.nodes.imap_folder_create(),
    imap_folder_delete: LL.nodes.imap_folder_delete(),
    imap_idle: LL.nodes.imap_idle(),
    imap_close: LL.nodes.imap_close(),
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
