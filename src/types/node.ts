/**
 * 节点类型定义
 * 与后端 src-tauri/src/node.rs 和 src-tauri/src/node_config.rs 中的结构体对齐
 */

import type { OutputMapping, InitialVariable } from './variable';

import type { Port } from './port';

import type { DbFetchMode, DbIsolationLevel } from './database';

// ============================================================================
// 节点类型枚举
// ============================================================================

/**
 * 节点类型枚举
 */
export type NodeType =
  | 'entry'
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
  | 'mcp_close';

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
export type HttpMethod = 'get' | 'post' | 'put' | 'delete' | 'patch';

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
  type?: FormFieldType;
  /** 文件路径（仅 file 类型使用） */
  file_path?: string;
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
  /** 输出变量映射 */
  output_mappings?: OutputMapping[];
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
  /** 输出变量映射 */
  output_mappings?: OutputMapping[];
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
  /** 输出变量映射 */
  output_mappings?: OutputMapping[];
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
  /** 输出变量映射 */
  output_mappings?: OutputMapping[];
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
  /** 断言失败时的行为 */
  on_assertion_fail?: AssertionFailStrategy;
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
  | DbConnectConfig
  | DbQueryConfig
  | DbBeginConfig
  | DbCommitConfig
  | DbRollbackConfig
  | DbCloseConfig
  | ScriptNodeConfig;

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
