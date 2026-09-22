/**
 * 节点类型定义
 * 与后端 src-tauri/src/node.rs 和 src-tauri/src/node_config.rs 中的结构体对齐
 */

import type { OutputMapping } from './variable';

import type { Port } from './port';


// ============================================================================
// 节点类型枚举
// ============================================================================

/**
 * 节点类型枚举
 */
export type NodeType =
  | 'entry'
  | 'end'
  | 'flow:sub'
  | 'http:request'
  | 'conditional'
  | 'ws:connect'
  | 'ws:send_collect'
  | 'ws:close'
  | 'assertion'
  | 'script'
  | 'tcp:connect'
  | 'tcp:send'
  | 'tcp:receive'
  | 'tcp:close'
  | 'udp:bind'
  | 'udp:send_to'
  | 'udp:recv_from'
  | 'udp:close'
  | 'sse:connect'
  | 'sse:listen'
  | 'sse:disconnect'
  | 'graphql:connect'
  | 'graphql:query'
  | 'graphql:subscribe'
  | 'graphql:introspect'
  | 'graphql:disconnect'
  | 'variable:extractor'
  | 'proxy'
  | 'http:ingress'
  | 'http:respond';

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
  /** 关联的 CSV 文件路径 */
  dataset_path?: string;
  /** CSV 编码（不设置=自动检测） */
  dataset_encoding?: string;
}

/**
 * End 节点配置
 */
export interface EndConfig {
  type: 'end';
}

export interface HttpConfig {
  type: 'http:request';
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
  type: 'ws:connect';
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
  type: 'ws:send_collect';
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
  type: 'ws:close';
  /** 连接 ID */
  connection_id: string;
  /** 关闭码 */
  close_code?: number;
  /** 关闭原因 */
  close_reason?: string;
  /** 关闭超时（毫秒） */
  close_timeout_ms?: number;
}
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
  type: 'tcp:connect';
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
  type: 'tcp:send';
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
  type: 'tcp:receive';
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
  type: 'tcp:close';
  connection_id: string;
  close_timeout_ms?: number;
}

/**
 * UDP Bind 节点配置
 */
export interface UdpBindConfig {
  type: 'udp:bind';
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
  type: 'udp:send_to';
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
  type: 'udp:recv_from';
  connection_id: string;
  receive_timeout_ms?: number;
  max_size?: number;
}

/**
 * UDP Close 节点配置
 */
export interface UdpCloseConfig {
  type: 'udp:close';
  connection_id: string;
}

/** SSE 连接节点配置 */
export interface SseConnectConfig {
  type: 'sse:connect';
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
  type: 'sse:listen';
  connection_id: string;
  event_filter?: string;
  max_events?: number;
  timeout_ms?: number;
  include_data?: boolean;
  decode_json?: boolean;
}

/** SSE 断开连接节点配置 */
export interface SseDisconnectConfig {
  type: 'sse:disconnect';
  connection_id: string;
}

// ============================================================================
// GraphQL 节点配置
// ============================================================================

/** GraphQL 连接节点配置 */
export interface GraphqlConnectConfig {
  type: 'graphql:connect';
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
  type: 'graphql:query';
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
  type: 'graphql:subscribe';
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
  type: 'graphql:introspect';
  /** 连接 ID */
  connection_id: string;
  /** 超时（毫秒） */
  timeout_ms?: number;
}

/** GraphQL 断开连接节点配置 */
export interface GraphqlDisconnectConfig {
  type: 'graphql:disconnect';
  /** 连接 ID */
  connection_id: string;
}

/**
 * 变量提取节点配置
 */
export interface VariableExtractorConfig {
  type: 'variable:extractor';
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
 * 子流程变量映射
 */
export interface VariableMapping {
  /** 源变量名或表达式 */
  source: string;
  /** 目标变量名 */
  target: string;
}

/**
 * 子流程节点配置
 */
export interface SubFlowConfig {
  type: 'flow:sub';
  /** 目标流程 UUID（GUI 引用） */
  target_flow_id?: string;
  /** 目标流程文件路径（CLI / 相对路径） */
  target_flow_path?: string;
  /** 是否继承父流程所有变量（默认 true） */
  inherit_all_variables?: boolean;
  /** 自定义输入变量映射 */
  variable_mappings?: VariableMapping[];
  /** 是否导出子流程所有变量（默认 true） */
  export_all_variables?: boolean;
  /** 指定导出的变量映射（当 export_all_variables 为 false 时使用） */
  output_mappings?: VariableMapping[];
}

/**
 * HTTP Ingress 节点配置（Mock 服务入口）
 */
export interface HttpIngressConfig {
  type: 'http:ingress';
  /** 监听端口 */
  port: number;
  /** 路由路径模式（例如 /webhook/stripe 或 /*） */
  path_pattern: string;
  /** 支持的 HTTP 方法 */
  methods: string[];
}

/**
 * HTTP Respond 节点配置（Mock 服务出口）
 */
export interface HttpRespondConfig {
  type: 'http:respond';
  /** 响应状态码 */
  status_code: number;
  /** 响应头 */
  headers: Record<string, string>;
  /** 响应体类型 */
  body_type: 'json' | 'text' | 'binary' | string;
  /** 响应内容 */
  body: string;
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
  | TcpConnectConfig
  | TcpSendConfig
  | TcpReceiveConfig
  | TcpCloseConfig
  | UdpBindConfig
  | UdpSendToConfig
  | UdpRecvFromConfig
  | UdpCloseConfig
  | ScriptNodeConfig
  | SseConnectConfig
  | SseListenConfig
  | SseDisconnectConfig
  | GraphqlConnectConfig
  | GraphqlQueryConfig
  | GraphqlSubscribeConfig
  | GraphqlIntrospectConfig
  | GraphqlDisconnectConfig
  | VariableExtractorConfig
  | ProxyConfig
  | SubFlowConfig
  | HttpIngressConfig
  | HttpRespondConfig;

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
// 插件节点类型（store/注册表边界宽化）
// 插件节点由后端注册表产生任意字符串 type_id（如 "vendor:my-node"，见 M2a'/M2c）。
// 内置联合类型 NodeType / NodeData 保持闭合，以保留穷尽匹配与类型窄化；
// 以下宽化类型仅用于 store 动作签名与注册表 API 边界。
// ============================================================================

/**
 * 插件节点数据（任意 JSON 配置）
 */
export interface PluginNodeData {
  /** 插件节点类型标识（任意字符串，如 "vendor:my-node"） */
  type: string;
  /** 插件自定义配置（任意 JSON 结构） */
  [key: string]: unknown;
}

/**
 * 节点类型在 store/注册表边界处的宽化类型。
 * 接受任意插件 type_id，同时保留内置节点类型的自动补全与窄化。
 */
export type AnyNodeType = NodeType | (string & {});

/**
 * 节点数据在 store/注册表边界处的宽化类型。
 * 接受插件节点的任意 JSON 配置。
 */
export type AnyNodeData = NodeData | PluginNodeData;

/**
 * 节点在 store 边界处的宽化类型（支持插件节点）。
 */
export type AnyNode = Omit<Node, 'type' | 'data'> & {
  type: AnyNodeType;
  data: AnyNodeData;
};

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
 * 判断是否为 EndConfig
 */
export function isEndConfig(data: NodeData): data is EndConfig {
  return data.type === 'end';
}

/**
 * 判断是否为 HttpConfig
 */
export function isHttpConfig(data: NodeData): data is HttpConfig {
  return data.type === 'http:request';
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
  return data.type === 'ws:connect';
}

/**
 * 判断是否为 WsSendCollectConfig
 */
export function isWsSendCollectConfig(data: NodeData): data is WsSendCollectConfig {
  return data.type === 'ws:send_collect';
}

/**
 * 判断是否为 WsCloseConfig
 */
export function isWsCloseConfig(data: NodeData): data is WsCloseConfig {
  return data.type === 'ws:close';
}

/**
 * 判断是否为 AssertionNodeConfig
 */
export function isAssertionConfig(data: NodeData): data is AssertionNodeConfig {
  return data.type === 'assertion';
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
  return data.type === 'tcp:connect';
}

/**
 * 判断是否为 TcpSendConfig
 */
export function isTcpSendConfig(data: NodeData): data is TcpSendConfig {
  return data.type === 'tcp:send';
}

/**
 * 判断是否为 TcpReceiveConfig
 */
export function isTcpReceiveConfig(data: NodeData): data is TcpReceiveConfig {
  return data.type === 'tcp:receive';
}

/**
 * 判断是否为 TcpCloseConfig
 */
export function isTcpCloseConfig(data: NodeData): data is TcpCloseConfig {
  return data.type === 'tcp:close';
}

/**
 * 判断是否为 UdpBindConfig
 */
export function isUdpBindConfig(data: NodeData): data is UdpBindConfig {
  return data.type === 'udp:bind';
}

/**
 * 判断是否为 UdpSendToConfig
 */
export function isUdpSendToConfig(data: NodeData): data is UdpSendToConfig {
  return data.type === 'udp:send_to';
}

/**
 * 判断是否为 UdpRecvFromConfig
 */
export function isUdpRecvFromConfig(data: NodeData): data is UdpRecvFromConfig {
  return data.type === 'udp:recv_from';
}

/**
 * 判断是否为 UdpCloseConfig
 */
export function isUdpCloseConfig(data: NodeData): data is UdpCloseConfig {
  return data.type === 'udp:close';
}

export function isSseConnectConfig(data: NodeData): data is SseConnectConfig {
  return data.type === 'sse:connect';
}

export function isSseListenConfig(data: NodeData): data is SseListenConfig {
  return data.type === 'sse:listen';
}

export function isSseDisconnectConfig(data: NodeData): data is SseDisconnectConfig {
  return data.type === 'sse:disconnect';
}

export function isGraphqlConnectConfig(data: NodeData): data is GraphqlConnectConfig {
  return data.type === 'graphql:connect';
}

export function isGraphqlQueryConfig(data: NodeData): data is GraphqlQueryConfig {
  return data.type === 'graphql:query';
}

export function isGraphqlSubscribeConfig(data: NodeData): data is GraphqlSubscribeConfig {
  return data.type === 'graphql:subscribe';
}

export function isGraphqlIntrospectConfig(data: NodeData): data is GraphqlIntrospectConfig {
  return data.type === 'graphql:introspect';
}

export function isGraphqlDisconnectConfig(data: NodeData): data is GraphqlDisconnectConfig {
  return data.type === 'graphql:disconnect';
}



/**
 * 判断是否为 VariableExtractorConfig
 */

/**
 * 判断是否为 HttpIngressConfig
 */
export function isHttpIngressConfig(data: NodeData): data is HttpIngressConfig {
  return data.type === 'http:ingress';
}

/**
 * 判断是否为 HttpRespondConfig
 */
export function isHttpRespondConfig(data: NodeData): data is HttpRespondConfig {
  return data.type === 'http:respond';
}
export function isVariableExtractorConfig(data: NodeData): data is VariableExtractorConfig {
  return data.type === 'variable:extractor';
}

/**
 * 判断是否为 ProxyConfig
 */
export function isProxyConfig(data: NodeData): data is ProxyConfig {
  return data.type === 'proxy';
}
