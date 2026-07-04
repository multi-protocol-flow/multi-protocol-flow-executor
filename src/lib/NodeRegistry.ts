/**
 * 节点注册表服务
 * 从后端动态获取节点元数据，缓存并按分类分组
 */
import type { NodeMetadata } from '@/types/nodeRegistry';
import { i18nObject } from '@/i18n/i18n-util';
import type { TranslationFunctions } from '@/i18n/i18n-types';

// 懒加载 i18n 对象
let LL_cache: TranslationFunctions | null = null;
function getLL() {
  if (!LL_cache) {
    const locale = (localStorage.getItem('locale') as any) || 'en-US';
    LL_cache = i18nObject(locale);
  }
  return LL_cache;
}

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
  get grpc() { return getLL().nodes.category.grpc(); },
  get mcp() { return getLL().nodes.category.mcp(); },
  get mqtt() { return getLL().nodes.category.mqtt(); },
  get kafka() { return getLL().nodes.category.kafka(); },
  get sse() { return getLL().nodes.category.sse(); },
  get graphql() { return getLL().nodes.category.graphql(); },
  get amqp() { return getLL().nodes.category.amqp(); },
  get tcp() { return getLL().nodes.category.tcp(); },
  get udp() { return getLL().nodes.category.udp(); },
  get email() { return getLL().nodes.category.email(); },
  get redis() { return getLL().nodes.category.redis(); },
};

/**
 * 分类排序顺序（越小越靠前）
 * 按一级分类分组，组内间隔 10 方便后续插入
 *
 * 网络: http(0), websocket(1), sse(2), graphql(3)
 * 服务: grpc(10), mcp(11)
 * 消息: mqtt(20), kafka(21), amqp(22)
 * 工具: tools(30), script(31), assertion(32)
 * 其他: database(40), email(41), tcp(42), udp(43)
 */
export const CATEGORY_ORDER: Record<string, number> = {
  http: 0,
  websocket: 1,
  sse: 2,
  graphql: 3,
  grpc: 10,
  mcp: 11,
  mqtt: 20,
  kafka: 21,
  amqp: 22,
  tools: 30,
  script: 31,
  assertion: 32,
  database: 40,
  email: 41,
  tcp: 42,
  udp: 43,
  redis: 23,
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

  // MQTT: 连接 → 发布 → 订阅 → 取消订阅 → 断开 → 等待消息 → 请求
  mqtt_connect: 0,
  mqtt_publish: 1,
  mqtt_subscribe: 2,
  mqtt_unsubscribe: 3,
  mqtt_disconnect: 4,
  mqtt_wait_message: 5,
  mqtt_request: 6,

  // Kafka: 连接 → 生产 → 消费 → 批量消费 → 管理 → Schema Registry → 关闭
  kafka_connect: 0,
  kafka_produce: 1,
  kafka_consume: 2,
  kafka_consume_batch: 3,
  kafka_admin: 4,
  kafka_schema_registry: 5,
  kafka_close: 6,

  // SSE: 连接 → 监听 → 断开
  sse_connect: 0,
  sse_listen: 1,
  sse_disconnect: 2,

  // GraphQL: 连接 → 查询 → 订阅 → 内省 → 断开
  graphql_connect: 0,
  graphql_query: 1,
  graphql_subscribe: 2,
  graphql_introspect: 3,
  graphql_disconnect: 4,

  // AMQP: 连接 → 交换器声明 → 交换器删除 → 队列声明 → 队列删除 → 队列绑定 → 发布 → 消费 → 获取 → 确认 → 管理 → 关闭 → AMQP1.0连接 → 发送 → 接收 → 确认 → 拒绝 → 断开
  amqp_connect: 0,
  amqp_exchange_declare: 1,
  amqp_exchange_delete: 2,
  amqp_queue_declare: 3,
  amqp_queue_delete: 4,
  amqp_queue_bind: 5,
  amqp_publish: 6,
  amqp_consume: 7,
  amqp_get: 8,
  amqp_ack: 9,
  amqp_management: 10,
  amqp_close: 11,
  amqp10_connect: 12,
  amqp10_send: 13,
  amqp10_receive: 14,
  amqp10_accept: 15,
  amqp10_reject: 16,
  amqp10_disconnect: 17,

  // TCP: 连接 → 发送 → 接收 → 关闭
  tcp_connect: 0,
  tcp_send: 1,
  tcp_receive: 2,
  tcp_close: 3,

  // UDP: 绑定 → 发送 → 接收 → 关闭
  udp_bind: 0,
  udp_send_to: 1,
  udp_recv_from: 2,
  udp_close: 3,

  // Email - SMTP: 连接 → 发送 → 断开
  smtp_connect: 0,
  smtp_send: 1,
  smtp_disconnect: 2,

  // Email - IMAP: 连接 → 选择 → 搜索 → 获取 → 标记 → 复制 → 移动 → 清理 → 列表 → 创建文件夹 → 删除文件夹 → 等待 → 关闭
  imap_connect: 0,
  imap_select: 1,
  imap_search: 2,
  imap_fetch: 3,
  imap_store_flags: 4,
  imap_copy: 5,
  imap_move: 6,
  imap_expunge: 7,
  imap_list: 8,
  imap_folder_create: 9,
  imap_folder_delete: 10,
  imap_idle: 11,
  imap_close: 12,

  // Redis: 连接 → 命令 → 关闭
  redis_connect: 0,
  redis_command: 1,
  redis_close: 2,

  // 数据库: 连接 → 开启事务 → 查询 → 提交 → 回滚 → 关闭
  db_connect: 0,
  db_begin: 1,
  db_query: 2,
  db_commit: 3,
  db_rollback: 4,
  db_close: 5,

  // 工具: 条件判断 → 脚本 → 断言 → 变量提取 → 代理 → 结束
  conditional: 0,
  script: 1,
  assertion: 2,
  variable_extractor: 3,
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
    mqtt_connect: 'bg-cyan-100 text-cyan-700',
    mqtt_publish: 'bg-cyan-100 text-cyan-700',
    mqtt_subscribe: 'bg-cyan-100 text-cyan-700',
    mqtt_unsubscribe: 'bg-cyan-100 text-cyan-700',
    mqtt_disconnect: 'bg-cyan-100 text-cyan-700',
    mqtt_wait_message: 'bg-cyan-100 text-cyan-700',
    mqtt_request: 'bg-cyan-100 text-cyan-700',
    kafka_connect: 'bg-rose-100 text-rose-700',
    kafka_produce: 'bg-rose-100 text-rose-700',
    kafka_consume: 'bg-rose-100 text-rose-700',
    kafka_consume_batch: 'bg-rose-100 text-rose-700',
    kafka_admin: 'bg-rose-100 text-rose-700',
    kafka_schema_registry: 'bg-rose-100 text-rose-700',
    kafka_close: 'bg-rose-100 text-rose-700',
    variable_extractor: 'bg-amber-100 text-amber-700',
    proxy: 'bg-lime-100 text-lime-700',
    end: 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300 border-purple-200 dark:border-purple-800',
    tcp_connect: 'bg-emerald-100 text-emerald-700',
    tcp_send: 'bg-emerald-100 text-emerald-700',
    tcp_receive: 'bg-emerald-100 text-emerald-700',
    tcp_close: 'bg-emerald-100 text-emerald-700',
    udp_bind: 'bg-orange-100 text-orange-700',
    udp_send_to: 'bg-orange-100 text-orange-700',
    udp_recv_from: 'bg-orange-100 text-orange-700',
    udp_close: 'bg-orange-100 text-orange-700',
    sse_connect: 'bg-rose-100 text-rose-700',
    sse_listen: 'bg-rose-100 text-rose-700',
    sse_disconnect: 'bg-rose-100 text-rose-700',
    graphql_connect: 'bg-indigo-100 text-indigo-700',
    graphql_query: 'bg-indigo-100 text-indigo-700',
    graphql_subscribe: 'bg-indigo-100 text-indigo-700',
    graphql_introspect: 'bg-indigo-100 text-indigo-700',
    graphql_disconnect: 'bg-indigo-100 text-indigo-700',
    amqp_connect: 'bg-violet-100 text-violet-700',
    amqp_close: 'bg-violet-100 text-violet-700',
    amqp_exchange_declare: 'bg-violet-100 text-violet-700',
    amqp_exchange_delete: 'bg-violet-100 text-violet-700',
    amqp_queue_declare: 'bg-violet-100 text-violet-700',
    amqp_queue_delete: 'bg-violet-100 text-violet-700',
    amqp_queue_bind: 'bg-violet-100 text-violet-700',
    amqp_publish: 'bg-violet-100 text-violet-700',
    amqp_consume: 'bg-violet-100 text-violet-700',
    amqp_get: 'bg-violet-100 text-violet-700',
    amqp_ack: 'bg-violet-100 text-violet-700',
    amqp_management: 'bg-violet-100 text-violet-700',
    amqp10_connect: 'bg-purple-100 text-purple-700',
    amqp10_send: 'bg-purple-100 text-purple-700',
    amqp10_receive: 'bg-purple-100 text-purple-700',
    amqp10_accept: 'bg-purple-100 text-purple-700',
    amqp10_reject: 'bg-purple-100 text-purple-700',
    amqp10_disconnect: 'bg-purple-100 text-purple-700',
    // Email - SMTP (橙色)
    smtp_connect: 'bg-orange-100 text-orange-700',
    smtp_send: 'bg-orange-100 text-orange-700',
    smtp_disconnect: 'bg-orange-100 text-orange-700',
    // Email - IMAP (蓝色)
    imap_connect: 'bg-blue-100 text-blue-700',
    imap_select: 'bg-blue-100 text-blue-700',
    imap_search: 'bg-blue-100 text-blue-700',
    imap_fetch: 'bg-blue-100 text-blue-700',
    imap_store_flags: 'bg-blue-100 text-blue-700',
    imap_copy: 'bg-blue-100 text-blue-700',
    imap_move: 'bg-blue-100 text-blue-700',
    imap_expunge: 'bg-blue-100 text-blue-700',
    imap_list: 'bg-blue-100 text-blue-700',
    imap_folder_create: 'bg-blue-100 text-blue-700',
    imap_folder_delete: 'bg-blue-100 text-blue-700',
    imap_idle: 'bg-blue-100 text-blue-700',
    imap_close: 'bg-blue-100 text-blue-700',
    // Redis (红色)
    redis_connect: 'bg-red-100 text-red-700',
    redis_command: 'bg-red-100 text-red-700',
    redis_close: 'bg-red-100 text-red-700',
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
