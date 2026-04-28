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
  EyeOff,
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
  mcp_get_prompt: { icon: MessageSquare, iconBg: 'bg-fuchsia-500', labelColor: 'text-fuchsia-600', badge: 'bg-fuchsia-100 text-fuchsia-700' },
  mcp_close: { icon: Unplug, iconBg: 'bg-fuchsia-500', labelColor: 'text-fuchsia-600', badge: 'bg-fuchsia-100 text-fuchsia-700' },
  tcp_connect: { icon: Cable, iconBg: 'bg-emerald-100', labelColor: 'text-emerald-700', badge: 'bg-emerald-100 text-emerald-700' },
  tcp_send: { icon: Send, iconBg: 'bg-emerald-100', labelColor: 'text-emerald-700', badge: 'bg-emerald-100 text-emerald-700' },
  tcp_close: { icon: Unplug, iconBg: 'bg-emerald-100', labelColor: 'text-emerald-700', badge: 'bg-emerald-100 text-emerald-700' },
  udp_bind: { icon: Radio, iconBg: 'bg-orange-100', labelColor: 'text-orange-700', badge: 'bg-orange-100 text-orange-700' },
  udp_send_to: { icon: ArrowUpRight, iconBg: 'bg-orange-100', labelColor: 'text-orange-700', badge: 'bg-orange-100 text-orange-700' },
  udp_recv_from: { icon: ArrowDownLeft, iconBg: 'bg-orange-100', labelColor: 'text-orange-700', badge: 'bg-orange-100 text-orange-700' },
  udp_close: { icon: Unplug, iconBg: 'bg-orange-100', labelColor: 'text-orange-700', badge: 'bg-orange-100 text-orange-700' },

  // TCP (补充)
  tcp_receive: { icon: Download, iconBg: 'bg-emerald-100', labelColor: 'text-emerald-700', badge: 'bg-emerald-100 text-emerald-700' },

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
  graphql_unsubscribe: { icon: EyeOff, iconBg: 'bg-indigo-500', labelColor: 'text-indigo-600', badge: 'bg-indigo-100 text-indigo-700' },
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
  mcp_get_prompt: 'MCP-获取提示词',
  mcp_close: 'MCP-关闭',
  tcp_connect: 'TCP 连接',
  tcp_send: 'TCP 发送',
  tcp_close: 'TCP 关闭',
  udp_bind: 'UDP 绑定',
  udp_send_to: 'UDP 发送',
  udp_recv_from: 'UDP 接收',
  udp_close: 'UDP 关闭',
  // TCP
  tcp_receive: 'TCP 接收',
  // MQTT
  mqtt_connect: 'MQTT 连接',
  mqtt_publish: 'MQTT 发布',
  mqtt_subscribe: 'MQTT 订阅',
  mqtt_unsubscribe: 'MQTT 取消订阅',
  mqtt_disconnect: 'MQTT 断开',
  mqtt_wait_message: 'MQTT 等待消息',
  mqtt_request: 'MQTT 请求',
  // Kafka
  kafka_connect: 'Kafka 连接',
  kafka_produce: 'Kafka 生产',
  kafka_consume: 'Kafka 消费',
  kafka_consume_batch: 'Kafka 批量消费',
  kafka_admin: 'Kafka 管理',
  kafka_schema_registry: 'Kafka Schema',
  kafka_close: 'Kafka 关闭',
  // SSE
  sse_connect: 'SSE 连接',
  sse_listen: 'SSE 监听',
  sse_disconnect: 'SSE 断开',
  // GraphQL
  graphql_connect: 'GraphQL 连接',
  graphql_query: 'GraphQL 查询',
  graphql_subscribe: 'GraphQL 订阅',
  graphql_unsubscribe: 'GraphQL 取消订阅',
  graphql_introspect: 'GraphQL 内省',
  graphql_disconnect: 'GraphQL 断开',
  // AMQP 0-9-1
  amqp_connect: 'AMQP 连接',
  amqp_exchange_declare: 'AMQP 声明交换器',
  amqp_exchange_delete: 'AMQP 删除交换器',
  amqp_queue_declare: 'AMQP 声明队列',
  amqp_queue_delete: 'AMQP 删除队列',
  amqp_queue_bind: 'AMQP 队列绑定',
  amqp_publish: 'AMQP 发布',
  amqp_consume: 'AMQP 消费',
  amqp_get: 'AMQP 获取',
  amqp_ack: 'AMQP 确认',
  amqp_management: 'AMQP 管理',
  amqp_close: 'AMQP 关闭',
  // AMQP 1.0
  amqp10_connect: 'AMQP1.0 连接',
  amqp10_send: 'AMQP1.0 发送',
  amqp10_receive: 'AMQP1.0 接收',
  amqp10_accept: 'AMQP1.0 接受',
  amqp10_reject: 'AMQP1.0 拒绝',
  amqp10_disconnect: 'AMQP1.0 断开',
  // SMTP
  smtp_connect: 'SMTP 连接',
  smtp_send: 'SMTP 发送',
  smtp_disconnect: 'SMTP 断开',
  // IMAP
  imap_connect: 'IMAP 连接',
  imap_select: 'IMAP 选择',
  imap_search: 'IMAP 搜索',
  imap_fetch: 'IMAP 获取',
  imap_store_flags: 'IMAP 标记',
  imap_copy: 'IMAP 复制',
  imap_move: 'IMAP 移动',
  imap_expunge: 'IMAP 清理',
  imap_list: 'IMAP 列表',
  imap_folder_create: 'IMAP 创建文件夹',
  imap_folder_delete: 'IMAP 删除文件夹',
  imap_idle: 'IMAP 等待',
  imap_close: 'IMAP 关闭',
  // 工具
  variable_extractor: '变量提取',
  proxy: '代理设置',
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
