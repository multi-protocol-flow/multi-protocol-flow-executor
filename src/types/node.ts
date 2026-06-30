/**
 * 节点类型定义
 * 与后端 src-tauri/src/node.rs 和 src-tauri/src/node_config.rs 中的结构体对齐
 */

import type { OutputMapping, InitialVariable } from './variable';

import type { Port } from './port';

import type { DbFetchMode, DbIsolationLevel } from './database';

import type { FlowHook } from './hook';

import type { SmtpConnectConfig, SmtpSendConfig, SmtpDisconnectConfig } from './smtp';

import type { RedisConnectConfig, RedisCommandConfig, RedisCloseConfig } from './redis';
export type { RedisConnectConfig, RedisCommandConfig, RedisCloseConfig };

import type {
  ImapConnectConfig,
  ImapSelectConfig,
  ImapSearchConfig,
  ImapFetchConfig,
  ImapStoreFlagsConfig,
  ImapCopyConfig,
  ImapMoveConfig,
  ImapExpungeConfig,
  ImapListConfig,
  ImapFolderCreateConfig,
  ImapFolderDeleteConfig,
  ImapIdleConfig,
  ImapCloseConfig,
} from './imap';

// ============================================================================
// 节点类型枚举
// ============================================================================

/**
 * 节点类型枚举
 */
export type NodeType =
  | 'entry'
  | 'end'
  | 'http'
  | 'conditional'
  | 'ws_connect'
  | 'ws_send_collect'
  | 'ws_close'
  | 'assertion'
  | 'db_connect'
  | 'db_query'
  | 'db_begin'
  | 'db_commit'
  | 'db_rollback'
  | 'db_close'
  | 'script'
  | 'grpc_connect'
  | 'grpc_call'
  | 'grpc_close'
  | 'mcp_connect'
  | 'mcp_call_tool'
  | 'mcp_read_resource'
  | 'mcp_get_prompt'
  | 'mcp_close'
  | 'tcp_connect'
  | 'tcp_send'
  | 'tcp_receive'
  | 'tcp_close'
  | 'udp_bind'
  | 'udp_send_to'
  | 'udp_recv_from'
  | 'udp_close'
  | 'mqtt_connect'
  | 'mqtt_publish'
  | 'mqtt_subscribe'
  | 'mqtt_unsubscribe'
  | 'mqtt_disconnect'
  | 'mqtt_wait_message'
  | 'mqtt_request'
  | 'kafka_connect'
  | 'kafka_produce'
  | 'kafka_consume'
  | 'kafka_consume_batch'
  | 'kafka_admin'
  | 'kafka_schema_registry'
  | 'kafka_close'
  | 'sse_connect'
  | 'sse_listen'
  | 'sse_disconnect'
  | 'graphql_connect'
  | 'graphql_query'
  | 'graphql_subscribe'
  | 'graphql_introspect'
  | 'graphql_disconnect'
  | 'amqp_connect'
  | 'amqp_close'
  | 'amqp_exchange_declare'
  | 'amqp_exchange_delete'
  | 'amqp_queue_declare'
  | 'amqp_queue_delete'
  | 'amqp_queue_bind'
  | 'amqp_publish'
  | 'amqp_consume'
  | 'amqp_get'
  | 'amqp_ack'
  | 'amqp_management'
  | 'amqp10_connect'
  | 'amqp10_send'
  | 'amqp10_receive'
  | 'amqp10_accept'
  | 'amqp10_reject'
  | 'amqp10_disconnect'
  | 'smtp_connect'
  | 'smtp_send'
  | 'smtp_disconnect'
  | 'imap_connect'
  | 'imap_select'
  | 'imap_search'
  | 'imap_fetch'
  | 'imap_store_flags'
  | 'imap_copy'
  | 'imap_move'
  | 'imap_expunge'
  | 'imap_list'
  | 'imap_folder_create'
  | 'imap_folder_delete'
  | 'imap_idle'
  | 'imap_close'
  | 'redis_connect'
  | 'redis_command'
  | 'redis_close'
  | 'variable_extractor'
  | 'proxy';

/**
 * 节点位置结构体
 */
export interface NodePosition {
  /** X 坐标 */
  x: number;
  /** Y 坐标 */
  y: number;
}

// ============================================================================
// HTTP 方法枚举
// ============================================================================

/**
 * HTTP 方法枚举
 */
export type HttpMethod = 'get' | 'post' | 'put' | 'delete' | 'patch' | 'head' | 'options';

/**
 * HTTP 请求体类型
 */
export type HttpBodyType = 'none' | 'body' | 'form-data' | 'x-www-form-urlencoded' | 'binary';

/**
 * Content-Type 枚举
 */
export type ContentType = 'json' | 'text' | 'html' | 'xml' | 'javascript';

/**
 * 表单字段类型
 */
export type FormFieldType = 'text' | 'file';

/**
 * 表单字段
 */
export interface FormField {
  /** 字段名 */
  key: string;
  /** 字段值 */
  value: string;
  /** 字段类型 */
  field_type?: FormFieldType;
  /** 文件路径（仅 file 类型使用） */
  file_path?: string;
}

// ============================================================================
// HTTP 认证类型定义
// ============================================================================

/**
 * HTTP 认证类型
 */
export type HttpAuthType = 'none' | 'basic' | 'bearer' | 'api_key_header' | 'api_key_query' | 'digest';

/**
 * HTTP 认证配置
 */
export interface HttpAuthConfig {
  /** 认证类型（默认 none） */
  auth_type?: HttpAuthType;
  /** 用户名（Basic 认证） */
  username?: string;
  /** 密码（Basic 认证） */
  password?: string;
  /** Token（Bearer 认证） */
  token?: string;
  /** API Key 名称（API Key 模式） */
  api_key_name?: string;
  /** API Key 值（API Key 模式） */
  api_key_value?: string;
}

/**
 * Cookie 条目
 */
export interface CookieEntry {
  /** Cookie 名称 */
  name: string;
  /** Cookie 值 */
  value: string;
  /** 域名 */
  domain?: string;
  /** 路径 */
  path?: string;
}

// ============================================================================
// 条件类型枚举
// ============================================================================

/**
 * 条件类型枚举
 */
export type ConditionType = 'simple' | 'advanced';

/**
 * 条件操作符枚举
 */
export type ConditionOperator =
  | 'equal'
  | 'not_equal'
  | 'greater'
  | 'greater_or_equal'
  | 'less'
  | 'less_or_equal'
  | 'contains'
  | 'starts_with'
  | 'ends_with'
  | 'exists';

/**
 * 期望值类型选择器
 */
export type ExpectedValueType = 'number' | 'string' | 'boolean' | 'null';

// ============================================================================
// 节点配置（判别联合类型）
// ============================================================================

/**
 * Entry 节点配置
 */
export interface EntryConfig {
  type: 'entry';
  /** 初始变量列表 */
  initial_variables?: InitialVariable[];
  /** 关联的 CSV 文件路径 */
  dataset_path?: string;
  /** CSV 编码（不设置=自动检测） */
  dataset_encoding?: string;
  /** 流程钩子列表（入口节点配置中可编辑） */
  hooks?: FlowHook[];
}

/**
 * End 节点配置
 */
export interface EndConfig {
  type: 'end';
}

export interface HttpConfig {
  type: 'http';
  /** 请求 URL */
  url: string;
  /** HTTP 方法（默认 get） */
  method?: HttpMethod;
  /** 请求头 */
  headers?: Record<string, string>;
  /** 请求体类型（默认 none） */
  body_type?: HttpBodyType;
  /** Content-Type（默认 json） */
  content_type?: ContentType;
  /** 表单字段 */
  form_fields?: FormField[];
  /** 二进制文件路径 */
  file_path?: string;
  /** 请求体（文本格式） */

  body?: string;
  /** 请求超时时间（毫秒） */
  timeout_ms?: number;
  // === 新增字段 ===
  /** 认证配置 */
  auth?: HttpAuthConfig;
  /** 是否跟随重定向（默认 true） */
  follow_redirects?: boolean;
  /** 最大重定向次数（默认 10） */
  max_redirects?: number;
  /** 是否跳过 TLS 证书验证 */
  tls_skip_verify?: boolean;
  /** TLS CA 证书路径 */
  tls_ca_cert_path?: string;
  /** TLS 客户端证书路径 */
  tls_client_cert_path?: string;
  /** TLS 客户端密钥路径 */
  tls_client_key_path?: string;
  /** 是否接受压缩响应（默认 true） */
  accept_encoding?: boolean;
  /** 是否启用 Cookie 管理（默认 false） */
  enable_cookies?: boolean;
  /** 初始 Cookie 列表 */
  cookie_jar?: CookieEntry[];
  /** 重试次数（默认 0） */
  retry_count?: number;
  /** 重试延迟（毫秒，默认 1000） */
  retry_delay_ms?: number;
  /** 触发重试的状态码列表（如 [429, 502, 503, 504]） */
  retry_on_status?: number[];
}

/**
 * Conditional 节点配置
 */
export interface ConditionalConfig {
  type: 'conditional';
  /** 条件类型（默认 simple） */
  condition_type?: ConditionType;
  /** 字段路径（用于简单条件） */
  field_path?: string;
  /** 操作符 */
  operator?: ConditionOperator;
  /** 期望值 */
  expected_value?: unknown;
  /** 高级表达式（用于复杂条件） */
  advanced_expression?: string;
  /** 期望值类型选择器 */
  expected_value_type?: ExpectedValueType;
  /** 可选，指定读取特定上游节点的输出进行条件判断（UUID）。
   * 为 undefined 时读取 last_node_output（即最近上游节点输出） */
  source_node_id?: string;
}

// ============================================================================
// 断言节点类型定义
// ============================================================================

/**
 * 断言操作符
 */
export type AssertionOperator =
  | 'eq' | 'ne'
  | 'gt' | 'gte' | 'lt' | 'lte'
  | 'contains' | 'not_contains'
  | 'matches'
  | 'exists' | 'not_exists'
  | 'in' | 'not_in'
  | 'type_is'
  | 'between';

/**
 * 断言组合模式
 */
export type AssertionMode = 'all' | 'any';

/**
 * 失败行为
 */
export type OnFailureMode = 'route_false' | 'throw';

/**
 * 节点执行失败处理策略
 */
export type ExecutionErrorStrategy = 'route_to_false' | 'abort_flow' | 'ignore';

/**
 * 单条断言配置
 */
export interface Assertion {
  /** 断言唯一标识 */
  id: string;
  /** 断言描述 */
  description?: string;
  /** 断言目标 (JSONPath) */
  target: string;
  /** 操作符 */
  operator: AssertionOperator;
  /** 期望值 */
  expected: unknown;
  /** 是否启用，默认 true */
  enabled?: boolean;
}

/**
 * 断言节点配置
 */
export interface AssertionNodeConfig {
  type: 'assertion';
  /** 断言列表 */
  assertions: Assertion[];
  /** 组合模式：all=全部通过, any=任一通过 */
  mode: AssertionMode;
  /** 失败行为：route_false=路由到false分支, throw=抛出异常 */
  on_failure: OnFailureMode;
  /** 失败后是否继续执行后续断言（用于收集所有失败） */
  continue_on_failure: boolean;
  /** 预设模板名称（可选） */
  preset?: string;
  /** 可选，指定读取特定上游节点的输出进行断言判断（UUID）。
   * 为 undefined 时读取 last_node_output（即最近上游节点输出） */
  source_node_id?: string;
}

/**
 * 单条断言执行结果
 */
export interface AssertionResult {
  /** 断言ID */
  id: string;
  /** 断言描述 */
  description?: string;
  /** 是否通过 */
  passed: boolean;
  /** 断言目标 */
  target: string;
  /** 操作符 */
  operator: string;
  /** 期望值 */
  expected: unknown;
  /** 实际值 */
  actual: unknown;
  /** 错误信息（如解析失败） */
  error?: string;
}

/**
 * 断言节点输出
 */
export interface AssertionOutput {
  /** 整体是否通过 */
  passed: boolean;
  /** 断言总数 */
  total: number;
  /** 通过数量 */
  passed_count: number;
  /** 失败数量 */
  failed_count: number;
  /** 每条断言的详情 */
  results: AssertionResult[];
  /** 失败的断言列表 */
  failed_assertions: AssertionResult[];
  /** 透传上游输入 */
  input: Record<string, unknown>;
}

// ============================================================================
// WebSocket 类型定义
// ============================================================================

/**
 * WebSocket 收集模式
 */
export type WsCollectMode = 'first_match' | 'condition_match' | 'duration_collect';

/**
 * WebSocket 匹配类型
 */
export type WsMatchType = 'json_path' | 'regex' | 'expression';

/**
 * WebSocket 消息类型
 */
export type WsMessageType = 'text' | 'binary';

/**
 * WebSocket 收集条件
 */
export interface WsCollectCondition {
  /** 匹配类型 */
  match_type: WsMatchType;
  /** 匹配表达式 */
  expression: string;
}

/**
 * WebSocket Connect 节点配置
 */
export interface WsConnectConfig {
  type: 'ws_connect';
  /** WebSocket URL */
  url: string;
  /** 请求头 */
  headers?: FormField[];
  /** 子协议 */
  subprotocols?: string[];
  /** 连接超时（毫秒） */
  connect_timeout_ms?: number;
  /** 是否收集消息 */
  collect_message?: boolean;
  /** 收集模式 */
  collect_mode?: WsCollectMode;
  /** 收集条件 */
  collect_condition?: WsCollectCondition;
  /** 收集超时（毫秒） */
  collect_timeout_ms?: number;
  /** 是否使用 TLS */
  use_tls?: boolean;
  /** 是否跳过 TLS 证书验证 */
  tls_skip_verify?: boolean;
  /** Ping 间隔（毫秒） */
  ping_interval_ms?: number;
  /** Ping 超时（毫秒） */
  ping_timeout_ms?: number;
}

/**
 * WebSocket Send+Collect 节点配置
 */
export interface WsSendCollectConfig {
  type: 'ws_send_collect';
  /** 连接 ID */
  connection_id: string;
  /** 消息内容 */
  message: string;
  /** 消息类型 */
  message_type?: WsMessageType;
  /** 二进制消息输入模式（仅前端 UI 使用，不传给后端执行器）
   *  hex    - 十六进制字符串，后端直接解析
   *  base64 - Base64 字符串，前端转换为 hex
   *  text   - 普通文本，前端转换为 hex（UTF-8 字节）
   */
  binary_input_mode?: 'hex' | 'base64' | 'text';
  /** 发送超时（毫秒） */
  send_timeout_ms?: number;
  /** 是否收集响应 */
  collect_response: boolean;
  /** 收集模式 */
  collect_mode?: WsCollectMode;
  /** 收集条件 */
  collect_condition?: WsCollectCondition;
  /** 收集超时（毫秒） */
  collect_timeout_ms?: number;
}

/**
 * WebSocket Close 节点配置
 */
export interface WsCloseConfig {
  type: 'ws_close';
  /** 连接 ID */
  connection_id: string;
  /** 关闭码 */
  close_code?: number;
  /** 关闭原因 */
  close_reason?: string;
  /** 关闭超时（毫秒） */
  close_timeout_ms?: number;
}

// ============================================================================
// gRPC 节点类型定义
// ============================================================================

/**
 * Proto 文件描述
 */
export interface ProtoFile {
  /** 文件路径/名称 */
  path: string;
  /** 文件内容 */
  content: string;
}

/**
 * gRPC Metadata 条目
 */
export interface GrpcMetadataEntry {
  /** 键 */
  key: string;
  /** 值 */
  value: string;
}

/**
 * gRPC 通信模式
 */
export type GrpcPattern = 'unary' | 'server_streaming' | 'client_streaming' | 'bidi_streaming';

/**
 * gRPC 流式消息
 */
export interface StreamMessage {
  /** 是否启用 */
  enabled: boolean;
  /** 消息内容（JSON 字符串） */
  content: string;
}

/**
 * gRPC 计时信息
 */
export interface GrpcTimingInfo {
  /** 总耗时（毫秒） */
  total_ms: number;
  /** 首个响应耗时（毫秒） */
  first_response_ms?: number;
}

/**
 * gRPC 流式结果
 */
export interface GrpcStreamingResult {
  /** gRPC 状态码 */
  status: number;
  /** 响应列表 */
  responses: Array<{ index: number; data: unknown; received_at_ms: number }>;
  /** 已发送消息数 */
  sent_count: number;
  /** 已接收消息数 */
  received_count: number;
  /** Trailers metadata */
  trailers: Record<string, string>;
  /** 计时信息 */
  timing: GrpcTimingInfo;
}

/**
 * gRPC 流式消息（用于实时流式传递）
 */
export interface GrpcStreamMessage {
  /** 消息序号（从 1 开始） */
  index: number;
  /** 消息数据 */
  data: unknown;
  /** 接收时间戳（毫秒） */
  received_at_ms: number;
  /** 消息大小（字节） */
  size_bytes: number;
}

/**
 * gRPC 健康检查响应
 */
export interface HealthCheckResponse {
  /** 服务状态："UNKNOWN", "SERVING", "NOT_SERVING", "NOT_IMPLEMENTED" */
  status: string;
}

/**
 * gRPC 错误详情
 */
export interface GrpcErrorDetail {
  /** 错误详情类型 URL */
  type_url: string;
  /** 解码后的错误详情 */
  decoded?: unknown;
  /** 原始十六进制数据 */
  raw_hex?: string;
}

/**
 * gRPC 方法信息（从 Server Reflection 或 Proto 解析得到）
 */
export interface GrpcMethodInfo {
  /** 方法名 */
  method_name: string;
  /** 输入类型全名 */
  input_type: string;
  /** 输出类型全名 */
  output_type: string;
  /** 是否为服务端流式 */
  is_server_streaming: boolean;
  /** 是否为客户端流式 */
  is_client_streaming?: boolean;
}

/**
 * gRPC 服务信息（从 Server Reflection 或 Proto 解析得到）
 */
export interface GrpcServiceInfo {
  /** 服务全名（如 "package.ServiceName"） */
  service_name: string;
  /** 方法列表 */
  methods: GrpcMethodInfo[];
}

/**
 * gRPC Connect 节点配置
 */
export interface GrpcConnectConfig {
  type: 'grpc_connect';
  /** gRPC 服务器地址 */
  url: string;
  /** 是否使用 TLS */
  use_tls?: boolean;
  /** 是否跳过 TLS 证书验证 */
  tls_skip_verify?: boolean;
  /** 是否启用 Server Reflection */
  enable_reflection?: boolean;
  /** Proto 文件列表 */
  proto_files?: ProtoFile[];
  /** 默认 metadata */
  default_metadata?: GrpcMetadataEntry[];
  /** 连接超时（毫秒） */
  connect_timeout_ms?: number;
  /** 通过 Server Reflection 或 Proto 解析发现的服务列表（设计时缓存） */
  discovered_services?: GrpcServiceInfo[];
  // === TLS 增强字段 ===
  /** TLS CA 证书（PEM 格式内容） */
  tls_ca_cert?: string;
  /** TLS 客户端证书（PEM 格式内容） */
  tls_client_cert?: string;
  /** TLS 客户端私钥（PEM 格式内容） */
  tls_client_key?: string;
  /** TLS 服务器名称覆盖 */
  tls_server_name_override?: string;
  // === 压缩配置 ===
  /** 压缩编码："gzip" 或 "none" */
  compression_encoding?: string;
  // === Keepalive 配置 ===
  /** Keepalive 间隔（毫秒） */
  keepalive_time_ms?: number;
  /** Keepalive 超时（毫秒） */
  keepalive_timeout_ms?: number;
  /** 是否允许无流时发送 Keepalive */
  keepalive_permit_without_streams?: boolean;
  // === Reflection 增强配置 ===
  /** Reflection 请求附加 metadata */
  reflection_metadata?: GrpcMetadataEntry[];
  // === 健康检查配置 ===
  /** 健康检查服务名称 */
  health_check_service?: string;
  // === 负载均衡配置 ===
  /** 多端点列表（用于负载均衡） */
  endpoints?: string[];
  // === 重试配置 ===
  /** 最大重试次数 */
  max_retries?: number;
  /** 初始退避时间（毫秒） */
  initial_backoff_ms?: number;
  /** 最大退避时间（毫秒） */
  max_backoff_ms?: number;
  /** 可重试的 gRPC 状态码列表 */
  retryable_status_codes?: string[];
}

/**
 * gRPC Call 节点配置
 */
export interface GrpcCallConfig {
  type: 'grpc_call';
  /** 连接 ID */
  connection_id: string;
  /** 服务名称 */
  service_name: string;
  /** 方法名称 */
  method_name: string;
  /** 请求 JSON */
  request_json: string;
  /** 调用超时（毫秒） */
  timeout_ms?: number;
  /** 调用级 metadata */
  metadata?: GrpcMetadataEntry[];
  /** 流式消息列表（Client/Bidi Streaming 模式） */
  request_messages?: StreamMessage[];
  // === 压缩覆盖配置 ===
  /** 调用级压缩编码（覆盖连接级配置）："gzip" 或 "none" */
  compression_encoding?: string;
  // === 消息大小限制 ===
  /** 最大接收消息大小（字节） */
  max_receive_message_size?: number;
  /** 最大发送消息大小（字节） */
  max_send_message_size?: number;
  // === 重试配置（调用级覆盖，None 时使用连接级默认值）===
  /** 最大重试次数（0 = 不重试，覆盖连接级配置） */
  max_retries?: number;
  /** 初始退避时间（毫秒，覆盖连接级配置） */
  initial_backoff_ms?: number;
}

/**
 * gRPC Close 节点配置
 */
export interface GrpcCloseConfig {
  type: 'grpc_close';
  /** 连接 ID */
  connection_id: string;
}

// ============================================================================
// MCP 节点类型定义
// ============================================================================

/**
 * MCP 工具信息（从 MCP 服务器发现）
 */
export interface McpToolInfo {
  /** 工具名称 */
  name: string;
  /** 工具描述 */
  description?: string;
  /** 工具参数 JSON Schema */
  input_schema?: Record<string, unknown>;
}

/**
 * MCP 资源信息（从 MCP 服务器发现）
 */
export interface McpResourceInfo {
  /** 资源 URI */
  uri: string;
  /** 资源名称 */
  name: string;
  /** 资源描述 */
  description?: string;
  /** MIME 类型 */
  mime_type?: string;
}

/**
 * MCP 提示词参数信息
 */
export interface McpPromptArgumentInfo {
  /** 参数名称 */
  name: string;
  /** 参数描述 */
  description?: string;
  /** 是否必填 */
  required?: boolean;
}

/**
 * MCP 提示词信息（从 MCP 服务器发现）
 */
export interface McpPromptInfo {
  /** 提示词名称 */
  name: string;
  /** 提示词描述 */
  description?: string;
  /** 提示词参数 */
  arguments?: McpPromptArgumentInfo[];
}

/**
 * MCP Connect 节点配置
 */
export interface McpConnectConfig {
  type: 'mcp_connect';
  /** 传输模式：streamable_http (默认) | stdio */
  transport?: 'streamable_http' | 'stdio';
  /** MCP 服务器 URL（Streamable HTTP 模式） */
  url: string;
  /** 请求头（Streamable HTTP 模式） */
  headers?: Record<string, string>;
  /** 协议版本（默认 "2025-11-25"） */
  protocol_version?: string;
  /** 自定义请求头（Streamable HTTP 模式） */
  custom_headers?: Record<string, string>;
  /** Stdio 模式：可执行文件路径（如 npx, uvx, node） */
  command?: string;
  /** Stdio 模式：命令参数 */
  args?: string[];
  /** Stdio 模式：环境变量 */
  env?: Record<string, string>;
  /** 设计时发现缓存 - 工具列表 */
  discovered_tools?: McpToolInfo[];
  /** 设计时发现缓存 - 资源列表 */
  discovered_resources?: McpResourceInfo[];
  /** 设计时发现缓存 - 提示词列表 */
  discovered_prompts?: McpPromptInfo[];
}

/**
 * MCP Call Tool 节点配置
 */
export interface McpCallToolConfig {
  type: 'mcp_call_tool';
  /** 连接 ID（引用 mcp_connect 节点的 UUID） */
  connection_id: string;
  /** 工具名称 */
  tool_name: string;
  /** 工具参数（JSON） */
  arguments?: Record<string, unknown>;
}

/**
 * MCP Read Resource 节点配置
 */
export interface McpReadResourceConfig {
  type: 'mcp_read_resource';
  /** 连接 ID（引用 mcp_connect 节点的 UUID） */
  connection_id: string;
  /** 资源 URI */
  resource_uri: string;
}

/**
 * MCP Close 节点配置
 */
export interface McpCloseConfig {
  type: 'mcp_close';
  /** 连接 ID（引用 mcp_connect 节点的 UUID） */
  connection_id: string;
}

/**
 * MCP Get Prompt 节点配置
 */
export interface McpGetPromptConfig {
  type: 'mcp_get_prompt';
  /** 连接 ID（引用 mcp_connect 节点的 UUID） */
  connection_id: string;
  /** 提示词名称 */
  prompt_name: string;
  /** 提示词参数 */
  arguments?: Record<string, string>;
}

// ============================================================================
// TCP/UDP 节点类型定义
// ============================================================================

/**
 * TCP 帧定界模式
 */
export type TcpFramingMode =
  | { type: 'length_prefix'; prefix_bytes: number }
  | { type: 'delimiter'; delimiter: string }
  | { type: 'fixed_length'; length: number }
  | { type: 'timeout'; timeout_ms: number }
  | { type: 'manual' };

/**
 * TCP 帧配置
 */
export interface FramingConfig {
  mode: TcpFramingMode;
  max_frame_size?: number;
  include_framing_in_data?: boolean;
}

/**
 * TCP Connect 节点配置
 */
export interface TcpConnectConfig {
  type: 'tcp_connect';
  host: string;
  port: number;
  use_tls?: boolean;
  tls_skip_verify?: boolean;
  connect_timeout_ms?: number;
  framing_mode?: TcpFramingMode;
  framing_config?: FramingConfig;
  /** 是否禁用 Nagle 算法 */
  no_delay?: boolean;
  /** TCP Keepalive 配置 */
  keepalive?: { idle_secs: number; interval_secs: number; probe_count: number };
  /** TLS CA 证书路径 */
  tls_ca_cert?: string;
}

/**
 * TCP Send 节点配置
 */
export interface TcpSendConfig {
  type: 'tcp_send';
  connection_id: string;
  data: string;
  data_encoding?: string;
  framing_mode?: TcpFramingMode;
  framing_config?: FramingConfig;
  send_timeout_ms?: number;
  /** 解析器配置（透传 JSON） */
  parser_config?: Record<string, unknown>;
}

/**
 * TCP Receive 节点配置
 */
export interface TcpReceiveConfig {
  type: 'tcp_receive';
  /** 连接 ID */
  connection_id: string;
  /** 接收超时（毫秒） */
  receive_timeout_ms: number;
  /** 帧定界模式 */
  framing_mode: string;
  /** 帧配置 */
  framing_config: Record<string, unknown>;
}

/**
 * TCP Close 节点配置
 */
export interface TcpCloseConfig {
  type: 'tcp_close';
  connection_id: string;
  close_timeout_ms?: number;
}

/**
 * UDP Bind 节点配置
 */
export interface UdpBindConfig {
  type: 'udp_bind';
  local_addr?: string;
  local_port: number;
  reuse_addr?: boolean;
  /** 是否启用广播 */
  broadcast?: boolean;
  /** 多播组地址 */
  multicast_group?: string;
  /** 多播 TTL */
  multicast_ttl?: number;
  /** 是否启用多播环回 */
  multicast_loop?: boolean;
  /** 多播网络接口 */
  multicast_interface?: string;
}

/**
 * UDP SendTo 节点配置
 */
export interface UdpSendToConfig {
  type: 'udp_send_to';
  connection_id: string;
  data: string;
  data_encoding?: string;
  target_addr: string;
  target_port: number;
  send_timeout_ms?: number;
}

/**
 * UDP RecvFrom 节点配置
 */
export interface UdpRecvFromConfig {
  type: 'udp_recv_from';
  connection_id: string;
  receive_timeout_ms?: number;
  max_size?: number;
}

/**
 * UDP Close 节点配置
 */
export interface UdpCloseConfig {
  type: 'udp_close';
  connection_id: string;
}

// ============================================================================
// MQTT 节点类型定义
// ============================================================================

/**
 * MQTT 协议版本
 */
export type MqttVersion = '3.1.1' | '5.0';

/**
 * MQTT QoS 等级
 */
export type MqttQoS = 0 | 1 | 2;

/**
 * MQTT 载荷格式
 */
export type MqttPayloadFormat = 'text' | 'json' | 'hex' | 'base64';

/**
 * MQTT 保留消息处理策略（MQTT 5.0）
 */
export type MqttRetainHandling = 0 | 1 | 2;

/**
 * MQTT 输出模式
 */
export type MqttOutputMode = 'first' | 'all';

/**
 * MQTT Connect 节点配置
 */
export interface MqttConnectConfig {
  type: 'mqtt_connect';
  /** MQTT Broker 地址 */
  url?: string;
  /** 客户端 ID */
  client_id?: string;
  /** 用户名 */
  username?: string;
  /** 密码 */
  password?: string;
  /** MQTT 协议版本（默认 5.0） */
  mqtt_version?: MqttVersion;
  /** 清除会话（默认 true） */
  clean_session?: boolean;
  /** 心跳保活间隔（秒，默认 60） */
  keep_alive_secs?: number;
  /** 连接超时（毫秒，默认 5000） */
  connect_timeout_ms?: number;
  /** 遗嘱主题 */
  will_topic?: string;
  /** 遗嘱载荷 */
  will_payload?: string;
  /** 遗嘱 QoS */
  will_qos?: number;
  /** 遗嘱保留标志 */
  will_retain?: boolean;
  /** 是否使用 TLS */
  use_tls?: boolean;
  /** 是否跳过 TLS 证书验证 */
  tls_skip_verify?: boolean;
  /** 会话过期间隔（MQTT 5.0） */
  session_expiry_interval?: number;
  /** 遗嘱延迟间隔（MQTT 5.0） */
  will_delay_interval?: number;
  /** 接收最大值（MQTT 5.0） */
  receive_maximum?: number;
  /** 主题别名最大值（MQTT 5.0） */
  topic_alias_maximum?: number;
  /** 最大报文大小（MQTT 5.0） */
  max_packet_size?: number;
  /** 用户属性（MQTT 5.0） */
  user_properties?: [string, string][];
}

/**
 * MQTT Publish 节点配置
 */
export interface MqttPublishConfig {
  type: 'mqtt_publish';
  /** 连接 ID（引用 mqtt_connect 节点的 UUID） */
  connection_id: string;
  /** 发布主题 */
  topic: string;
  /** 发布载荷 */
  payload?: string;
  /** QoS 等级（默认 0） */
  qos?: number;
  /** 保留标志 */
  retain?: boolean;
  /** 载荷格式 */
  payload_format?: MqttPayloadFormat;
  /** 内容类型（MQTT 5.0） */
  content_type?: string;
  /** 消息过期间隔（MQTT 5.0） */
  message_expiry_interval?: number;
  /** 响应主题（MQTT 5.0） */
  response_topic?: string;
  /** 关联数据（MQTT 5.0） */
  correlation_data?: string;
  /** 主题别名（MQTT 5.0） */
  topic_alias?: number;
  /** 用户属性（MQTT 5.0） */
  user_properties?: [string, string][];
}

/**
 * MQTT Subscribe 节点配置
 */
export interface MqttSubscribeConfig {
  type: 'mqtt_subscribe';
  /** 连接 ID（引用 mqtt_connect 节点的 UUID） */
  connection_id: string;
  /** 订阅主题 */
  topic: string;
  /** QoS 等级（默认 0） */
  qos?: number;
  /** 最大接收消息数 */
  max_messages?: number;
  /** 采集超时（毫秒） */
  collect_timeout_ms?: number;
  /** 输出模式（默认 first） */
  output_mode?: MqttOutputMode;
  /** 不转发自身发布的消息（MQTT 5.0） */
  no_local?: boolean;
  /** 保留消息按原样发布（MQTT 5.0） */
  retain_as_published?: boolean;
  /** 保留消息处理策略（MQTT 5.0: 0=发送, 1=仅新, 2=不发送） */
  retain_handling?: MqttRetainHandling;
  /** 订阅标识符（MQTT 5.0） */
  subscription_identifier?: number;
  /** 共享订阅组名 */
  share_group?: string;
}

/**
 * MQTT Unsubscribe 节点配置
 */
export interface MqttUnsubscribeConfig {
  type: 'mqtt_unsubscribe';
  /** 连接 ID（引用 mqtt_connect 节点的 UUID） */
  connection_id: string;
  /** 取消订阅主题 */
  topic: string;
}

/**
 * MQTT Disconnect 节点配置
 */
export interface MqttDisconnectConfig {
  type: 'mqtt_disconnect';
  /** 连接 ID（引用 mqtt_connect 节点的 UUID） */
  connection_id: string;
  /** 断开原因码 */
  reason_code?: number;
  /** 会话过期间隔（MQTT 5.0） */
  session_expiry_interval?: number;
}

/**
 * MQTT WaitMessage 节点配置
 */
export interface MqttWaitMessageConfig {
  type: 'mqtt_wait_message';
  /** 连接 ID（引用 mqtt_connect 节点的 UUID） */
  connection_id: string;
  /** 等待主题 */
  topic: string;
  /** 匹配条件 */
  match_condition?: Record<string, unknown>;
  /** 等待超时（毫秒，默认 30000） */
  timeout_ms?: number;
  /** 最大等待消息数 */
  max_messages?: number;
}

/**
 * MQTT Request 节点配置（请求-响应模式）
 */
export interface MqttRequestConfig {
  type: 'mqtt_request';
  /** 连接 ID（引用 mqtt_connect 节点的 UUID） */
  connection_id: string;
  /** 请求主题 */
  request_topic: string;
  /** 响应主题 */
  response_topic: string;
  /** 请求载荷 */
  payload?: string;
  /** QoS 等级 */
  qos?: number;
  /** 关联数据 */
  correlation_data?: string;
  /** 响应等待超时（毫秒，默认 30000） */
  timeout_ms?: number;
}

// ============================================================================
// Kafka 节点类型定义
// ============================================================================

/**
 * Kafka 安全协议
 */
export type KafkaSecurityProtocol = 'plaintext' | 'ssl' | 'sasl_plaintext' | 'sasl_ssl';

/**
 * Kafka 序列化格式
 */
export type KafkaSerializationFormat = 'raw' | 'json' | 'avro' | 'protobuf';

/**
 * Kafka 压缩算法
 */
export type KafkaCompressionType = 'none' | 'gzip' | 'snappy' | 'lz4' | 'zstd';

/**
 * Kafka Admin 操作类型
 */
export type KafkaAdminOperation =
  | 'topic_create' | 'topic_delete' | 'topic_describe' | 'topic_list' | 'topic_alter_config'
  | 'consumer_group_list' | 'consumer_group_describe' | 'consumer_group_reset_offset'
  | 'broker_list' | 'broker_describe_config'
  | 'acl_create' | 'acl_delete' | 'acl_list';

/**
 * Kafka Schema Registry 操作类型
 */
export type KafkaSchemaOperation =
  | 'list_subjects' | 'get_subject_versions' | 'get_schema_by_version'
  | 'check_compatibility' | 'register_schema';

/**
 * Kafka 批量消费策略
 */
export type KafkaBatchStrategy = 'count_timeout' | 'count_only' | 'timeout_only';

/**
 * Kafka Connect 节点配置
 */
export interface KafkaConnectConfig {
  type: 'kafka_connect';
  /** Kafka Bootstrap Servers（逗号分隔） */
  bootstrap_servers?: string;
  /** 客户端 ID */
  client_id?: string;
  /** 安全协议 */
  security_protocol?: KafkaSecurityProtocol;
  /** SASL 机制 */
  sasl_mechanism?: string;
  /** SASL 用户名 */
  sasl_username?: string;
  /** SASL 密码 */
  sasl_password?: string;
  /** SASL OAuth Token */
  sasl_oauth_token?: string;
  /** SSL CA 证书路径 */
  ssl_ca_cert_path?: string;
  /** SSL 客户端证书路径 */
  ssl_client_cert_path?: string;
  /** SSL 客户端密钥路径 */
  ssl_client_key_path?: string;
  /** 是否跳过 SSL 证书验证 */
  ssl_skip_verify?: boolean;
  /** Schema Registry URL */
  schema_registry_url?: string;
  /** Schema Registry 用户名 */
  schema_registry_username?: string;
  /** Schema Registry 密码 */
  schema_registry_password?: string;
  /** 连接超时（毫秒） */
  connect_timeout_ms?: number;
}

/**
 * Kafka Produce 节点配置
 */
export interface KafkaProduceConfig {
  type: 'kafka_produce';
  /** 连接 ID */
  connection_id: string;
  /** 目标主题 */
  topic: string;
  /** 消息键 */
  key?: string;
  /** 消息值 */
  value?: string;
  /** 消息头 */
  headers?: [string, string][];
  /** 目标分区 */
  partition?: number;
  /** 时间戳 */
  timestamp?: number;
  /** 序列化格式 */
  serialization_format?: KafkaSerializationFormat;
  /** Schema 主题 */
  schema_subject?: string;
  /** 确认超时（毫秒） */
  ack_timeout_ms?: number;
  /** 压缩算法 */
  compression?: KafkaCompressionType;
}

/**
 * Kafka Consume 节点配置
 */
export interface KafkaConsumeConfig {
  type: 'kafka_consume';
  /** 连接 ID */
  connection_id: string;
  /** 订阅主题 */
  topic: string;
  /** 指定分区 */
  partition?: number;
  /** 起始偏移量 */
  offset?: number;
  /** 偏移重置策略 */
  offset_reset?: string;
  /** 消费者组 ID */
  group_id?: string;
  /** 是否启用自动提交 */
  enable_auto_commit?: boolean;
  /** 最大消费消息数 */
  max_messages?: number;
  /** 拉取超时（毫秒） */
  poll_timeout_ms?: number;
  /** 反序列化格式 */
  deserialization_format?: KafkaSerializationFormat;
  /** Schema 主题 */
  schema_subject?: string;
  /** 最小拉取字节数 */
  fetch_min_bytes?: number;
  /** 最大拉取等待时间（毫秒） */
  fetch_max_wait_ms?: number;
}

/**
 * Kafka ConsumeBatch 节点配置
 */
export interface KafkaConsumeBatchConfig {
  type: 'kafka_consume_batch';
  /** 连接 ID */
  connection_id: string;
  /** 订阅主题 */
  topic: string;
  /** 指定分区 */
  partition?: number;
  /** 起始偏移量 */
  offset?: number;
  /** 偏移重置策略 */
  offset_reset?: string;
  /** 消费者组 ID */
  group_id?: string;
  /** 是否启用自动提交 */
  enable_auto_commit?: boolean;
  /** 最大消费消息数 */
  max_messages?: number;
  /** 拉取超时（毫秒） */
  poll_timeout_ms?: number;
  /** 反序列化格式 */
  deserialization_format?: KafkaSerializationFormat;
  /** Schema 主题 */
  schema_subject?: string;
  /** 批量策略 */
  batch_strategy?: KafkaBatchStrategy;
}

/**
 * Kafka Admin 节点配置
 */
export interface KafkaAdminConfig {
  type: 'kafka_admin';
  /** 连接 ID */
  connection_id: string;
  /** 管理操作 */
  operation: KafkaAdminOperation;
  /** 主题名称 */
  topic_name?: string;
  /** 分区数 */
  partitions?: number;
  /** 副本因子 */
  replication_factor?: number;
  /** 主题配置 */
  topic_config?: Record<string, string>;
  /** 消费者组 ID */
  consumer_group_id?: string;
  /** ACL 主体 */
  acl_principal?: string;
  /** ACL 主机 */
  acl_host?: string;
  /** ACL 操作 */
  acl_operation?: string;
  /** ACL 权限类型 */
  acl_permission_type?: string;
  /** ACL 资源类型 */
  acl_resource_type?: string;
  /** ACL 资源名称 */
  acl_resource_name?: string;
  /** 是否试运行 */
  dry_run?: boolean;
}

/**
 * Kafka Schema Registry 节点配置
 */
export interface KafkaSchemaRegistryConfig {
  type: 'kafka_schema_registry';
  /** 连接 ID */
  connection_id: string;
  /** 操作类型 */
  operation: KafkaSchemaOperation;
  /** Schema 主题 */
  subject?: string;
  /** 版本号 */
  version?: number;
  /** Schema 定义 */
  schema_definition?: string;
  /** Schema 类型 */
  schema_type?: string;
  /** 兼容性级别 */
  compatibility_level?: string;
}

/**
 * Kafka Close 节点配置
 */
export interface KafkaCloseConfig {
  type: 'kafka_close';
  /** 连接 ID */
  connection_id: string;
}

/** SSE 连接节点配置 */
export interface SseConnectConfig {
  type: 'sse_connect';
  url: string;
  headers?: Record<string, string>;
  last_event_id?: string;
  timeout_ms?: number;
  connection_id?: string;
  auto_reconnect?: boolean;
  reconnect_interval_ms?: number;
  max_reconnect_attempts?: number;
  tls_mode?: 'none' | 'standard' | 'skip-verify';
}

/** SSE 监听节点配置 */
export interface SseListenConfig {
  type: 'sse_listen';
  connection_id: string;
  event_filter?: string;
  max_events?: number;
  timeout_ms?: number;
  include_data?: boolean;
  decode_json?: boolean;
}

/** SSE 断开连接节点配置 */
export interface SseDisconnectConfig {
  type: 'sse_disconnect';
  connection_id: string;
}

// ============================================================================
// GraphQL 节点配置
// ============================================================================

/** GraphQL 连接节点配置 */
export interface GraphqlConnectConfig {
  type: 'graphql_connect';
  /** HTTP 端点 URL */
  endpoint: string;
  /** WebSocket 端点 URL（用于订阅） */
  ws_endpoint?: string;
  /** 请求头 */
  headers?: Record<string, string>;
  /** 连接超时（毫秒） */
  connection_timeout_ms?: number;
  /** 连接 ID */
  connection_id?: string;
  /** 设计时内省发现的 Schema（JSON） */
  discovered_schema?: Record<string, unknown>;
  /** 设计时内省发现的查询模板 */
  discovered_queries?: Array<{
    type_name: string;
    operation: string;
    template: string;
  }>;
}

/** GraphQL 查询/变更节点配置 */
export interface GraphqlQueryConfig {
  type: 'graphql_query';
  /** 连接 ID */
  connection_id: string;
  /** GraphQL 查询字符串 */
  query: string;
  /** 查询变量 */
  variables?: Record<string, unknown>;
  /** 操作名称 */
  operation_name?: string;
  /** 文件上传列表 */
  files?: Array<{ field_name: string; file_path: string }>;
  /** 超时（毫秒） */
  timeout_ms?: number;
}

/** GraphQL 订阅节点配置 */
export interface GraphqlSubscribeConfig {
  type: 'graphql_subscribe';
  /** 连接 ID */
  connection_id: string;
  /** GraphQL 订阅字符串 */
  subscription: string;
  /** 查询变量 */
  variables?: Record<string, unknown>;
  /** 操作名称 */
  operation_name?: string;
  /** 最大事件数 */
  max_events?: number;
  /** 超时（毫秒） */
  timeout_ms?: number;
}

/** GraphQL 内省节点配置 */
export interface GraphqlIntrospectConfig {
  type: 'graphql_introspect';
  /** 连接 ID */
  connection_id: string;
  /** 超时（毫秒） */
  timeout_ms?: number;
}

/** GraphQL 断开连接节点配置 */
export interface GraphqlDisconnectConfig {
  type: 'graphql_disconnect';
  /** 连接 ID */
  connection_id: string;
}

// ============================================================================
// AMQP 0-9-1 节点类型定义
// ============================================================================

/**
 * AMQP TLS 模式
 */
export type AmqpTlsMode = 'none' | 'standard' | 'skip-verify';

/**
 * AMQP 认证机制
 */
export type AmqpAuthMechanism = 'PLAIN' | 'AMQPLAIN' | 'EXTERNAL' | 'RABBIT-CR-DEMO';

/**
 * AMQP Exchange 类型
 */
export type AmqpExchangeType = 'direct' | 'fanout' | 'topic' | 'headers' | 'custom';

/**
 * AMQP Body 类型
 */
export type AmqpBodyType = 'text' | 'json' | 'binary';

/**
 * AMQP 确认模式
 */
export type AmqpConfirmMode = 'none' | 'publisher-confirms' | 'transaction';

/**
 * AMQP 消费确认模式
 */
export type AmqpAckMode = 'auto' | 'manual';

/**
 * AMQP Queue 操作类型
 */
export type AmqpQueueAction = 'delete' | 'purge';

/**
 * AMQP Ack 操作类型
 */
export type AmqpAckAction = 'ack' | 'nack' | 'reject';

/**
 * AMQP Management 操作类型
 */
export type AmqpManagementAction =
  | 'overview'
  | 'list-queues'
  | 'list-connections'
  | 'list-exchanges'
  | 'list-channels';

/** AMQP Connect 节点配置 */
export interface AmqpConnectConfig {
  type: 'amqp_connect';
  /** AMQP Broker URL */
  url?: string;
  /** 连接 ID（留空自动生成） */
  connection_id?: string;
  /** 心跳间隔（秒） */
  heartbeat?: number;
  /** 最大通道数 */
  channel_max?: number;
  /** 最大帧大小 */
  frame_max?: number;
  /** 连接超时（毫秒） */
  connection_timeout_ms?: number;
  /** TLS 模式 */
  tls_mode?: AmqpTlsMode;
  /** 认证机制 */
  auth_mechanism?: AmqpAuthMechanism;
  /** 自动恢复 */
  auto_recover?: boolean;
  /** 最大重连次数 */
  reconnect_max_retries?: number;
}

/** AMQP Close 节点配置 */
export interface AmqpCloseConfig {
  type: 'amqp_close';
  /** 连接 ID */
  connection_id: string;
  /** 关闭回复码 */
  reply_code?: number;
  /** 关闭回复文本 */
  reply_text?: string;
}

/** AMQP Exchange Declare 节点配置 */
export interface AmqpExchangeDeclareConfig {
  type: 'amqp_exchange_declare';
  /** 连接 ID */
  connection_id: string;
  /** Exchange 名称 */
  exchange: string;
  /** Exchange 类型 */
  exchange_type: AmqpExchangeType;
  /** 自定义类型（exchange_type=custom 时使用） */
  custom_type?: string;
  /** 被动声明（仅检查是否存在） */
  passive?: boolean;
  /** 持久化 */
  durable?: boolean;
  /** 自动删除 */
  auto_delete?: boolean;
  /** 内部 Exchange */
  internal?: boolean;
  /** 额外参数 */
  arguments?: Record<string, unknown>;
}

/** AMQP Exchange Delete 节点配置 */
export interface AmqpExchangeDeleteConfig {
  type: 'amqp_exchange_delete';
  /** 连接 ID */
  connection_id: string;
  /** Exchange 名称 */
  exchange: string;
  /** 仅在未被使用时删除 */
  if_unused?: boolean;
}

/** AMQP Queue Declare 节点配置 */
export interface AmqpQueueDeclareConfig {
  type: 'amqp_queue_declare';
  /** 连接 ID */
  connection_id: string;
  /** 队列名称（留空自动生成） */
  queue?: string;
  /** 被动声明（仅检查是否存在） */
  passive?: boolean;
  /** 持久化 */
  durable?: boolean;
  /** 独占队列 */
  exclusive?: boolean;
  /** 自动删除 */
  auto_delete?: boolean;
  /** 额外参数 */
  arguments?: Record<string, unknown>;
}

/** AMQP Queue Delete 节点配置 */
export interface AmqpQueueDeleteConfig {
  type: 'amqp_queue_delete';
  /** 连接 ID */
  connection_id: string;
  /** 队列名称 */
  queue: string;
  /** 操作类型 */
  action?: AmqpQueueAction;
  /** 仅在未被使用时删除 */
  if_unused?: boolean;
  /** 仅在为空时删除 */
  if_empty?: boolean;
}

/** AMQP Queue Bind 节点配置 */
export interface AmqpQueueBindConfig {
  type: 'amqp_queue_bind';
  /** 连接 ID */
  connection_id: string;
  /** 队列名称 */
  queue: string;
  /** Exchange 名称 */
  exchange: string;
  /** 路由键 */
  routing_key?: string;
  /** 额外参数 */
  arguments?: Record<string, unknown>;
}

/** AMQP Publish 节点配置 */
export interface AmqpPublishConfig {
  type: 'amqp_publish';
  /** 连接 ID */
  connection_id: string;
  /** Exchange 名称 */
  exchange?: string;
  /** 路由键 */
  routing_key?: string;
  /** 消息体 */
  body?: string;
  /** 消息体类型 */
  body_type?: AmqpBodyType;
  /** Content-Type */
  content_type?: string;
  /** 内容编码 */
  content_encoding?: string;
  /** 投递模式（1=非持久化, 2=持久化） */
  delivery_mode?: number;
  /** 优先级（0-9） */
  priority?: number;
  /** 消息 ID */
  message_id?: string;
  /** 关联 ID */
  correlation_id?: string;
  /** 回复队列 */
  reply_to?: string;
  /** 过期时间 */
  expiration?: string;
  /** 时间戳 */
  timestamp?: number;
  /** 消息类型 */
  message_type?: string;
  /** 用户 ID */
  user_id?: string;
  /** 应用 ID */
  app_id?: string;
  /** 集群 ID */
  cluster_id?: string;
  /** 消息头 */
  headers?: Record<string, unknown>;
  /** Mandatory 标志 */
  mandatory?: boolean;
  /** 确认模式 */
  confirm_mode?: AmqpConfirmMode;
}

/** AMQP Consume 节点配置 */
export interface AmqpConsumeConfig {
  type: 'amqp_consume';
  /** 连接 ID */
  connection_id: string;
  /** 队列名称 */
  queue: string;
  /** 消费者标签 */
  consumer_tag?: string;
  /** 确认模式 */
  ack_mode?: AmqpAckMode;
  /** 独占消费 */
  exclusive?: boolean;
  /** 不接收自身发布的消息 */
  no_local?: boolean;
  /** 预取数量 */
  prefetch_count?: number;
  /** 最大消费消息数 */
  max_messages?: number;
  /** 超时（毫秒） */
  timeout_ms?: number;
  /** 成功时自动确认 */
  auto_ack_on_success?: boolean;
  /** 失败时自动拒绝 */
  auto_reject_on_failure?: boolean;
  /** 拒绝时重新入队 */
  reject_requeue?: boolean;
}

/** AMQP Get 节点配置 */
export interface AmqpGetConfig {
  type: 'amqp_get';
  /** 连接 ID */
  connection_id: string;
  /** 队列名称 */
  queue: string;
  /** 不确认模式 */
  no_ack?: boolean;
}

/** AMQP Ack 节点配置 */
export interface AmqpAckConfig {
  type: 'amqp_ack';
  /** 连接 ID */
  connection_id: string;
  /** 操作类型 */
  action: AmqpAckAction;
  /** 投递标签 */
  delivery_tag: number;
  /** 批量确认 */
  multiple?: boolean;
  /** 重新入队（nack/reject 时） */
  requeue?: boolean;
}

/** AMQP Management 节点配置 */
export interface AmqpManagementConfig {
  type: 'amqp_management';
  /** Management API URL */
  management_url?: string;
  /** 用户名 */
  username?: string;
  /** 密码 */
  password?: string;
  /** 操作类型 */
  action: AmqpManagementAction;
  /** VHost */
  vhost?: string;
  /** 队列名称（部分操作需要） */
  queue_name?: string;
  /** 连接名称（部分操作需要） */
  connection_id?: string;
}

// ============================================================================
// AMQP 1.0 节点类型定义
// ============================================================================

/**
 * AMQP 1.0 认证机制
 */
export type Amqp10AuthMechanism = 'PLAIN' | 'ANONYMOUS' | 'EXTERNAL';

/** AMQP 1.0 Connect 节点配置 */
export interface Amqp10ConnectConfig {
  type: 'amqp10_connect';
  /** AMQP 1.0 服务器 URL */
  url?: string;
  /** 连接 ID（留空自动生成） */
  connection_id?: string;
  /** TLS 模式 */
  tls_mode?: AmqpTlsMode;
  /** 认证机制 */
  auth_mechanism?: Amqp10AuthMechanism;
  /** 容器 ID */
  container_id?: string;
}

/** AMQP 1.0 消息属性 */
export interface Amqp10MessageProperties {
  /** 内容类型 */
  content_type?: string;
  /** 内容编码 */
  content_encoding?: string;
  /** 关联 ID */
  correlation_id?: string;
  /** 消息 ID */
  message_id?: string;
  /** 回复地址 */
  reply_to?: string;
  /** 主题 */
  subject?: string;
  /** 目标 */
  to?: string;
  /** 绝对过期时间（Unix 毫秒） */
  absolute_expiry_time?: number;
  /** 创建时间（Unix 毫秒） */
  creation_time?: number;
}

/** AMQP 1.0 Send 节点配置 */
export interface Amqp10SendConfig {
  type: 'amqp10_send';
  /** 连接 ID */
  connection_id: string;
  /** 目标地址 */
  address: string;
  /** 消息体 */
  message_body?: string;
  /** 消息属性 */
  message_properties?: Amqp10MessageProperties;
  /** 消息注解 */
  message_annotations?: Record<string, unknown>;
}

/** AMQP 1.0 Receive 节点配置 */
export interface Amqp10ReceiveConfig {
  type: 'amqp10_receive';
  /** 连接 ID */
  connection_id: string;
  /** 源地址 */
  address: string;
  /** 信用额度 */
  credit?: number;
  /** 最大接收消息数 */
  max_messages?: number;
  /** 超时（毫秒） */
  timeout_ms?: number;
  /** 自动接受 */
  auto_accept?: boolean;
}

/** AMQP 1.0 Accept 节点配置 */
export interface Amqp10AcceptConfig {
  type: 'amqp10_accept';
  /** 连接 ID */
  connection_id: string;
  /** 投递 ID */
  delivery_id: number;
}

/** AMQP 1.0 Reject 节点配置 */
export interface Amqp10RejectConfig {
  type: 'amqp10_reject';
  /** 连接 ID */
  connection_id: string;
  /** 投递 ID */
  delivery_id: number;
  /** 错误条件 */
  error_condition?: string;
  /** 错误描述 */
  error_description?: string;
}

/** AMQP 1.0 Disconnect 节点配置 */
export interface Amqp10DisconnectConfig {
  type: 'amqp10_disconnect';
  /** 连接 ID */
  connection_id: string;
}

/**
 * 变量提取节点配置
 */
export interface VariableExtractorConfig {
  type: 'variable_extractor';
  /** 输出变量映射列表 */
  output_mappings?: OutputMapping[];
  /** 可选，指定读取特定上游节点的输出（UUID）。为 undefined 时读取 last_node_output */
  source_node_id?: string;
}

// ============================================================================
// 代理节点类型定义
// ============================================================================

/** 代理模式 */
export type ProxyMode = 'clear' | 'http' | 'socks5';

/** 代理设置节点配置 */
export interface ProxyConfig {
  type: 'proxy';
  /** 代理模式 */
  mode: ProxyMode;
  /** 代理地址（仅 http/socks5 模式） */
  proxy_url?: string;
  /** 代理用户名 */
  username?: string;
  /** 代理密码 */
  password?: string;
}

// ============================================================================
// 协议模板与解析类型定义
// ============================================================================

/**
 * 协议模板
 */
export interface ProtocolTemplate {
  /** 模板 ID */
  id: string;
  /** 模板名称 */
  name: string;
  /** 模板描述 */
  description: string;
  /** 解析器配置 */
  parser_config: Record<string, unknown>;
}

/**
 * 解析后的消息
 */
export interface ParsedMessage {
  /** 原始数据 */
  raw_data: string;
  /** 解析字段列表 */
  fields: ParsedField[];
  /** 解析时间（ISO 格式） */
  parsed_at: string;
}

/**
 * 解析后的字段
 */
export interface ParsedField {
  /** 字段名称 */
  name: string;
  /** 字段值 */
  value: string;
  /** 数据偏移量 */
  offset: number;
  /** 数据长度 */
  length: number;
}

// ============================================================================
// TCP/UDP 录制类型定义
// ============================================================================

/**
 * TCP/UDP 录制条目方向
 */
export type RecordingDirection = 'send' | 'receive';

/**
 * TCP/UDP 录制条目
 */
export interface RecordingEntry {
  /** 时间戳（毫秒，相对录制开始时间） */
  timestamp_ms: number;
  /** 方向（发送/接收） */
  direction: RecordingDirection;
  /** 连接 ID */
  connection_id: string;
  /** 数据（Base64 编码） */
  data_base64: string;
  /** 数据大小（字节） */
  data_size: number;
  /** 附加元数据 */
  metadata: Record<string, unknown>;
  /** 外部 .bin 文件路径（大数据模式），内嵌时为 undefined */
  external_bin_path?: string;
}

/**
 * TCP/UDP 完整录制
 */
export interface TcpUdpRecording {
  /** 录制唯一标识 */
  id: string;
  /** 录制名称 */
  name: string;
  /** 创建时间（Unix 时间戳，毫秒） */
  created_at: number;
  /** 录制条目列表 */
  entries: RecordingEntry[];
  /** 录制总时长（毫秒） */
  duration_ms: number;
}

// ============================================================================
// 数据库节点类型定义
// ============================================================================

/**
 * DB Connect 节点配置
 */
export interface DbConnectConfig {
  type: 'db_connect';
  /** 数据库驱动类型: "postgres" | "mysql" | "sqlite" | "redis" */
  driver?: string;
  /** 连接字符串（SQLite 使用文件路径或 :memory:） */
  connection_string?: string;
  /** 主机地址（PostgreSQL/MySQL） */
  host?: string;
  /** 端口号（PostgreSQL/MySQL） */
  port?: number;
  /** 数据库名 */
  database?: string;
  /** 用户名 */
  username?: string;
  /** 密码 */
  password?: string;
  /** 最小连接数 */
  min_connections?: number;
  /** 最大连接数 */
  max_connections?: number;
  /** 测试查询语句 */
  test_query?: string;
  /** 连接超时（毫秒） */
  connect_timeout_ms?: number;
}

/**
 * DB Query 节点配置
 */
export interface DbQueryConfig {
  type: 'db_query';
  /** 连接 UUID */
  connection_uuid: string;
  /** SQL 查询语句 */
  sql?: string;
  /** SQL 参数 */
  params?: string[];
  /** Redis 命令 */
  redis_command?: string;
  /** Redis 键名 */
  redis_key?: string;
  /** Redis 值 */
  redis_value?: string;
  /** Redis TTL（秒） */
  redis_ttl?: number;
  /** 获取模式 */
  fetch_mode?: DbFetchMode;
  /** 结果存储变量名 */
  result_to?: string;
  /** 查询超时（毫秒） */
  timeout_ms?: number;
}

/**
 * DB Begin 节点配置
 */
export interface DbBeginConfig {
  type: 'db_begin';
  /** 连接 UUID */
  connection_uuid: string;
  /** 事务隔离级别 */
  isolation_level?: DbIsolationLevel;
}

/**
 * DB Commit 节点配置
 */
export interface DbCommitConfig {
  type: 'db_commit';
  /** 连接 UUID */
  connection_uuid: string;
}

/**
 * DB Rollback 节点配置
 */
export interface DbRollbackConfig {
  type: 'db_rollback';
  /** 连接 UUID */
  connection_uuid: string;
}

/**
 * DB Close 节点配置
 */
export interface DbCloseConfig {
  type: 'db_close';
  /** 连接 UUID */
  connection_uuid: string;
}

// ============================================================================
// 脚本节点类型定义
// ============================================================================

/**
 * 脚本执行时机
 */
export type ScriptExecutionPhase = 'post_request' | 'pre_request' | 'standalone';

/**
 * 脚本错误处理策略
 */
export type ScriptErrorStrategy = 'recoverable' | 'fatal';

/**
 * 断言失败处理策略
 */
export type AssertionFailStrategy = 'route_false' | 'ignore' | 'abort';

/**
 * 脚本节点配置
 */
export interface ScriptNodeConfig {
  type: 'script';
  /** 脚本内容（JavaScript 代码） */
  script: string;
  /** 超时时间（毫秒），默认 5000 */
  timeout_ms?: number;
  /** 内存限制（MB），默认 10 */
  memory_limit_mb?: number;
  /** 脚本执行时机 */
  execution_phase?: ScriptExecutionPhase;
  /** 错误处理策略 */
  on_error?: ScriptErrorStrategy;
  /**
   * @deprecated on_error (Node level) unified by OnErrorSection with 'ignore' option.
   * Kept for backward compat with existing flow files.
   */
  // on_assertion_fail?: AssertionFailStrategy;
}

// ============================================================================
// 脚本节点输出类型定义
// ============================================================================

/**
 * 脚本日志级别
 */
export type ScriptLogLevel = 'log' | 'info' | 'warn' | 'error' | 'debug';

/**
 * 脚本日志条目
 */
export interface ScriptLogEntry {
  /** 日志级别 */
  level: ScriptLogLevel;
  /** 日志消息 */
  message: string;
  /** 时间戳（毫秒） */
  timestamp: number;
}

/**
 * 脚本断言结果
 */
export interface ScriptAssertionResult {
  /** 断言名称 */
  name: string;
  /** 是否通过 */
  passed: boolean;
  /** 错误信息（可选） */
  error?: string;
}

/**
 * 脚本节点输出
 */
export interface ScriptNodeOutput {
  /** 脚本执行是否成功 */
  success: boolean;
  /** 修改的变量列表 */
  variables_modified: Record<string, unknown>;
  /** 断言结果列表 */
  assertion_results: ScriptAssertionResult[];
  /** 日志输出列表 */
  logs: ScriptLogEntry[];
  /** 是否调用了 skip() */
  should_skip: boolean;
  /** 是否调用了 abort() */
  should_abort: boolean;
  /** abort 消息（可选） */
  abort_message?: string;
  /** 是否调用了 stop() */
  should_stop: boolean;
  /** 错误信息（执行失败时） */
  error?: string;
}

/**
 * 节点数据配置（判别联合类型）
 */
export type NodeData =
  | EntryConfig
  | EndConfig
  | HttpConfig
  | ConditionalConfig
  | WsConnectConfig
  | WsSendCollectConfig
  | WsCloseConfig
  | AssertionNodeConfig
  | GrpcConnectConfig
  | GrpcCallConfig
  | GrpcCloseConfig
  | McpConnectConfig
  | McpCallToolConfig
  | McpReadResourceConfig
  | McpGetPromptConfig
  | McpCloseConfig
  | TcpConnectConfig
  | TcpSendConfig
  | TcpReceiveConfig
  | TcpCloseConfig
  | UdpBindConfig
  | UdpSendToConfig
  | UdpRecvFromConfig
  | UdpCloseConfig
  | MqttConnectConfig
  | MqttPublishConfig
  | MqttSubscribeConfig
  | MqttUnsubscribeConfig
  | MqttDisconnectConfig
  | MqttWaitMessageConfig
  | MqttRequestConfig
  | KafkaConnectConfig
  | KafkaProduceConfig
  | KafkaConsumeConfig
  | KafkaConsumeBatchConfig
  | KafkaAdminConfig
  | KafkaSchemaRegistryConfig
  | KafkaCloseConfig
  | DbConnectConfig
  | DbQueryConfig
  | DbBeginConfig
  | DbCommitConfig
  | DbRollbackConfig
  | DbCloseConfig
  | ScriptNodeConfig
  | SseConnectConfig
  | SseListenConfig
  | SseDisconnectConfig
  | GraphqlConnectConfig
  | GraphqlQueryConfig
  | GraphqlSubscribeConfig
  | GraphqlIntrospectConfig
  | GraphqlDisconnectConfig
  | AmqpConnectConfig
  | AmqpCloseConfig
  | AmqpExchangeDeclareConfig
  | AmqpExchangeDeleteConfig
  | AmqpQueueDeclareConfig
  | AmqpQueueDeleteConfig
  | AmqpQueueBindConfig
  | AmqpPublishConfig
  | AmqpConsumeConfig
  | AmqpGetConfig
  | AmqpAckConfig
  | AmqpManagementConfig
  | Amqp10ConnectConfig
  | Amqp10SendConfig
  | Amqp10ReceiveConfig
  | Amqp10AcceptConfig
  | Amqp10RejectConfig
  | Amqp10DisconnectConfig
  | SmtpConnectConfig
  | SmtpSendConfig
  | SmtpDisconnectConfig
  | ImapConnectConfig
  | ImapSelectConfig
  | ImapSearchConfig
  | ImapFetchConfig
  | ImapStoreFlagsConfig
  | ImapCopyConfig
  | ImapMoveConfig
  | ImapExpungeConfig
  | ImapListConfig
  | ImapFolderCreateConfig
  | ImapFolderDeleteConfig
  | ImapIdleConfig
  | ImapCloseConfig
  | RedisConnectConfig
  | RedisCommandConfig
  | RedisCloseConfig
  | VariableExtractorConfig
  | ProxyConfig;

// ============================================================================
// 节点结构体
// ============================================================================

/**
 * 节点结构体
 */
export interface Node {
  /** 节点唯一标识符（前端生成 UUID，用于连接关联） */
  uuid: string;
  /** 节点类型 */
  type: NodeType;
  /** 节点名称 */
  name: string;
  /** 节点位置 */
  position: NodePosition;
  /** 节点特定数据（强类型配置） */
  data: NodeData;
  /** 端口数组（可选） */
  ports?: Port[];
  /** 是否设置断点（可选，默认 false） */
  hasBreakpoint?: boolean;
  /** 节点执行失败处理策略（可选） */
  on_error?: ExecutionErrorStrategy;
}

// ============================================================================
// 类型守卫函数
// ============================================================================

/**
 * 判断是否为 EntryConfig
 */
export function isEntryConfig(data: NodeData): data is EntryConfig {
  return data.type === 'entry';
}

/**
 * 判断是否为 HttpConfig
 */
export function isHttpConfig(data: NodeData): data is HttpConfig {
  return data.type === 'http';
}

/**
 * 判断是否为 ConditionalConfig
 */
export function isConditionalConfig(data: NodeData): data is ConditionalConfig {
  return data.type === 'conditional';
}

/**
 * 判断是否为 WsConnectConfig
 */
export function isWsConnectConfig(data: NodeData): data is WsConnectConfig {
  return data.type === 'ws_connect';
}

/**
 * 判断是否为 WsSendCollectConfig
 */
export function isWsSendCollectConfig(data: NodeData): data is WsSendCollectConfig {
  return data.type === 'ws_send_collect';
}

/**
 * 判断是否为 WsCloseConfig
 */
export function isWsCloseConfig(data: NodeData): data is WsCloseConfig {
  return data.type === 'ws_close';
}

/**
 * 判断是否为 GrpcConnectConfig
 */
export function isGrpcConnectConfig(data: NodeData): data is GrpcConnectConfig {
  return data.type === 'grpc_connect';
}

/**
 * 判断是否为 GrpcCallConfig
 */
export function isGrpcCallConfig(data: NodeData): data is GrpcCallConfig {
  return data.type === 'grpc_call';
}

/**
 * 判断是否为 GrpcCloseConfig
 */
export function isGrpcCloseConfig(data: NodeData): data is GrpcCloseConfig {
  return data.type === 'grpc_close';
}

/**
 * 判断是否为 McpConnectConfig
 */
export function isMcpConnectConfig(data: NodeData): data is McpConnectConfig {
  return data.type === 'mcp_connect';
}

/**
 * 判断是否为 McpCallToolConfig
 */
export function isMcpCallToolConfig(data: NodeData): data is McpCallToolConfig {
  return data.type === 'mcp_call_tool';
}

/**
 * 判断是否为 McpReadResourceConfig
 */
export function isMcpReadResourceConfig(data: NodeData): data is McpReadResourceConfig {
  return data.type === 'mcp_read_resource';
}

/**
 * 判断是否为 McpCloseConfig
 */
export function isMcpCloseConfig(data: NodeData): data is McpCloseConfig {
  return data.type === 'mcp_close';
}

/**
 * 判断是否为 McpGetPromptConfig
 */
export function isMcpGetPromptConfig(data: NodeData): data is McpGetPromptConfig {
  return data.type === 'mcp_get_prompt';
}

/**
 * 判断是否为 AssertionNodeConfig
 */
export function isAssertionConfig(data: NodeData): data is AssertionNodeConfig {
  return data.type === 'assertion';
}

/**
 * 判断是否为 DbConnectConfig
 */
export function isDbConnectConfig(data: NodeData): data is DbConnectConfig {
  return data.type === 'db_connect';
}

/**
 * 判断是否为 DbQueryConfig
 */
export function isDbQueryConfig(data: NodeData): data is DbQueryConfig {
  return data.type === 'db_query';
}

/**
 * 判断是否为 DbBeginConfig
 */
export function isDbBeginConfig(data: NodeData): data is DbBeginConfig {
  return data.type === 'db_begin';
}

/**
 * 判断是否为 DbCommitConfig
 */
export function isDbCommitConfig(data: NodeData): data is DbCommitConfig {
  return data.type === 'db_commit';
}

/**
 * 判断是否为 DbRollbackConfig
 */
export function isDbRollbackConfig(data: NodeData): data is DbRollbackConfig {
  return data.type === 'db_rollback';
}

/**
 * 判断是否为 DbCloseConfig
 */
export function isDbCloseConfig(data: NodeData): data is DbCloseConfig {
  return data.type === 'db_close';
}

/**
 * 判断是否为 ScriptNodeConfig
 */
export function isScriptConfig(data: NodeData): data is ScriptNodeConfig {
  return data.type === 'script';
}

/**
 * 判断是否为 TcpConnectConfig
 */
export function isTcpConnectConfig(data: NodeData): data is TcpConnectConfig {
  return data.type === 'tcp_connect';
}

/**
 * 判断是否为 TcpSendConfig
 */
export function isTcpSendConfig(data: NodeData): data is TcpSendConfig {
  return data.type === 'tcp_send';
}

/**
 * 判断是否为 TcpReceiveConfig
 */
export function isTcpReceiveConfig(data: NodeData): data is TcpReceiveConfig {
  return data.type === 'tcp_receive';
}

/**
 * 判断是否为 TcpCloseConfig
 */
export function isTcpCloseConfig(data: NodeData): data is TcpCloseConfig {
  return data.type === 'tcp_close';
}

/**
 * 判断是否为 UdpBindConfig
 */
export function isUdpBindConfig(data: NodeData): data is UdpBindConfig {
  return data.type === 'udp_bind';
}

/**
 * 判断是否为 UdpSendToConfig
 */
export function isUdpSendToConfig(data: NodeData): data is UdpSendToConfig {
  return data.type === 'udp_send_to';
}

/**
 * 判断是否为 UdpRecvFromConfig
 */
export function isUdpRecvFromConfig(data: NodeData): data is UdpRecvFromConfig {
  return data.type === 'udp_recv_from';
}

/**
 * 判断是否为 UdpCloseConfig
 */
export function isUdpCloseConfig(data: NodeData): data is UdpCloseConfig {
  return data.type === 'udp_close';
}

/**
 * 判断是否为 MqttConnectConfig
 */
export function isMqttConnectConfig(data: NodeData): data is MqttConnectConfig {
  return data.type === 'mqtt_connect';
}

/**
 * 判断是否为 MqttPublishConfig
 */
export function isMqttPublishConfig(data: NodeData): data is MqttPublishConfig {
  return data.type === 'mqtt_publish';
}

/**
 * 判断是否为 MqttSubscribeConfig
 */
export function isMqttSubscribeConfig(data: NodeData): data is MqttSubscribeConfig {
  return data.type === 'mqtt_subscribe';
}

/**
 * 判断是否为 MqttUnsubscribeConfig
 */
export function isMqttUnsubscribeConfig(data: NodeData): data is MqttUnsubscribeConfig {
  return data.type === 'mqtt_unsubscribe';
}

/**
 * 判断是否为 MqttDisconnectConfig
 */
export function isMqttDisconnectConfig(data: NodeData): data is MqttDisconnectConfig {
  return data.type === 'mqtt_disconnect';
}

/**
 * 判断是否为 MqttWaitMessageConfig
 */
export function isMqttWaitMessageConfig(data: NodeData): data is MqttWaitMessageConfig {
  return data.type === 'mqtt_wait_message';
}

/**
 * 判断是否为 MqttRequestConfig
 */
export function isMqttRequestConfig(data: NodeData): data is MqttRequestConfig {
  return data.type === 'mqtt_request';
}

export function isKafkaConnectConfig(data: NodeData): data is KafkaConnectConfig {
  return data.type === 'kafka_connect';
}
export function isKafkaProduceConfig(data: NodeData): data is KafkaProduceConfig {
  return data.type === 'kafka_produce';
}
export function isKafkaConsumeConfig(data: NodeData): data is KafkaConsumeConfig {
  return data.type === 'kafka_consume';
}
export function isKafkaConsumeBatchConfig(data: NodeData): data is KafkaConsumeBatchConfig {
  return data.type === 'kafka_consume_batch';
}
export function isKafkaAdminConfig(data: NodeData): data is KafkaAdminConfig {
  return data.type === 'kafka_admin';
}
export function isKafkaSchemaRegistryConfig(data: NodeData): data is KafkaSchemaRegistryConfig {
  return data.type === 'kafka_schema_registry';
}
export function isKafkaCloseConfig(data: NodeData): data is KafkaCloseConfig {
  return data.type === 'kafka_close';
}

export function isSseConnectConfig(data: NodeData): data is SseConnectConfig {
  return data.type === 'sse_connect';
}

export function isSseListenConfig(data: NodeData): data is SseListenConfig {
  return data.type === 'sse_listen';
}

export function isSseDisconnectConfig(data: NodeData): data is SseDisconnectConfig {
  return data.type === 'sse_disconnect';
}

export function isGraphqlConnectConfig(data: NodeData): data is GraphqlConnectConfig {
  return data.type === 'graphql_connect';
}

export function isGraphqlQueryConfig(data: NodeData): data is GraphqlQueryConfig {
  return data.type === 'graphql_query';
}

export function isGraphqlSubscribeConfig(data: NodeData): data is GraphqlSubscribeConfig {
  return data.type === 'graphql_subscribe';
}

export function isGraphqlIntrospectConfig(data: NodeData): data is GraphqlIntrospectConfig {
  return data.type === 'graphql_introspect';
}

export function isGraphqlDisconnectConfig(data: NodeData): data is GraphqlDisconnectConfig {
  return data.type === 'graphql_disconnect';
}

export function isAmqpConnectConfig(data: NodeData): data is AmqpConnectConfig { return data.type === 'amqp_connect'; }
export function isAmqpCloseConfig(data: NodeData): data is AmqpCloseConfig { return data.type === 'amqp_close'; }
export function isAmqpExchangeDeclareConfig(data: NodeData): data is AmqpExchangeDeclareConfig { return data.type === 'amqp_exchange_declare'; }
export function isAmqpExchangeDeleteConfig(data: NodeData): data is AmqpExchangeDeleteConfig { return data.type === 'amqp_exchange_delete'; }
export function isAmqpQueueDeclareConfig(data: NodeData): data is AmqpQueueDeclareConfig { return data.type === 'amqp_queue_declare'; }
export function isAmqpQueueDeleteConfig(data: NodeData): data is AmqpQueueDeleteConfig { return data.type === 'amqp_queue_delete'; }
export function isAmqpQueueBindConfig(data: NodeData): data is AmqpQueueBindConfig { return data.type === 'amqp_queue_bind'; }
export function isAmqpPublishConfig(data: NodeData): data is AmqpPublishConfig { return data.type === 'amqp_publish'; }
export function isAmqpConsumeConfig(data: NodeData): data is AmqpConsumeConfig { return data.type === 'amqp_consume'; }
export function isAmqpGetConfig(data: NodeData): data is AmqpGetConfig { return data.type === 'amqp_get'; }
export function isAmqpAckConfig(data: NodeData): data is AmqpAckConfig { return data.type === 'amqp_ack'; }
export function isAmqpManagementConfig(data: NodeData): data is AmqpManagementConfig { return data.type === 'amqp_management'; }
export function isAmqp10ConnectConfig(data: NodeData): data is Amqp10ConnectConfig { return data.type === 'amqp10_connect'; }
export function isAmqp10SendConfig(data: NodeData): data is Amqp10SendConfig { return data.type === 'amqp10_send'; }
export function isAmqp10ReceiveConfig(data: NodeData): data is Amqp10ReceiveConfig { return data.type === 'amqp10_receive'; }
export function isAmqp10AcceptConfig(data: NodeData): data is Amqp10AcceptConfig { return data.type === 'amqp10_accept'; }
export function isAmqp10RejectConfig(data: NodeData): data is Amqp10RejectConfig { return data.type === 'amqp10_reject'; }
export function isAmqp10DisconnectConfig(data: NodeData): data is Amqp10DisconnectConfig { return data.type === 'amqp10_disconnect'; }

export function isSmtpConnectConfig(data: NodeData): data is SmtpConnectConfig { return data.type === 'smtp_connect'; }
export function isSmtpSendConfig(data: NodeData): data is SmtpSendConfig { return data.type === 'smtp_send'; }
export function isSmtpDisconnectConfig(data: NodeData): data is SmtpDisconnectConfig { return data.type === 'smtp_disconnect'; }
export function isImapConnectConfig(data: NodeData): data is ImapConnectConfig { return data.type === 'imap_connect'; }
export function isImapSelectConfig(data: NodeData): data is ImapSelectConfig { return data.type === 'imap_select'; }
export function isImapSearchConfig(data: NodeData): data is ImapSearchConfig { return data.type === 'imap_search'; }
export function isImapFetchConfig(data: NodeData): data is ImapFetchConfig { return data.type === 'imap_fetch'; }
export function isImapStoreFlagsConfig(data: NodeData): data is ImapStoreFlagsConfig { return data.type === 'imap_store_flags'; }
export function isImapCopyConfig(data: NodeData): data is ImapCopyConfig { return data.type === 'imap_copy'; }
export function isImapMoveConfig(data: NodeData): data is ImapMoveConfig { return data.type === 'imap_move'; }
export function isImapExpungeConfig(data: NodeData): data is ImapExpungeConfig { return data.type === 'imap_expunge'; }
export function isImapListConfig(data: NodeData): data is ImapListConfig { return data.type === 'imap_list'; }
export function isImapFolderCreateConfig(data: NodeData): data is ImapFolderCreateConfig { return data.type === 'imap_folder_create'; }
export function isImapFolderDeleteConfig(data: NodeData): data is ImapFolderDeleteConfig { return data.type === 'imap_folder_delete'; }
export function isImapIdleConfig(data: NodeData): data is ImapIdleConfig { return data.type === 'imap_idle'; }
export function isImapCloseConfig(data: NodeData): data is ImapCloseConfig { return data.type === 'imap_close'; }

export function isRedisConnectConfig(data: NodeData): data is RedisConnectConfig { return data.type === 'redis_connect'; }
export function isRedisCommandConfig(data: NodeData): data is RedisCommandConfig { return data.type === 'redis_command'; }
export function isRedisCloseConfig(data: NodeData): data is RedisCloseConfig { return data.type === 'redis_close'; }

/**
 * 判断是否为 VariableExtractorConfig
 */
export function isVariableExtractorConfig(data: NodeData): data is VariableExtractorConfig {
  return data.type === 'variable_extractor';
}

/**
 * 判断是否为 ProxyConfig
 */
export function isProxyConfig(data: NodeData): data is ProxyConfig {
  return data.type === 'proxy';
}
