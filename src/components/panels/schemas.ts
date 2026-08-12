/**
 * 节点配置表单校验 Schema
 */
import { z } from 'zod';
import { getLL } from '@/i18n/i18n-lazy';
import type { GrpcFieldInfo, GrpcMessageInfo } from '@/types/node';

// ============================================================================
// HTTP 配置 Schema
// ============================================================================

/**
 * HTTP 方法枚举
 */
export const httpMethodSchema = z.enum(['get', 'post', 'put', 'delete', 'patch', 'head', 'options']);

/**
 * HTTP Body 类型枚举
 */
export const httpBodyTypeSchema = z.enum(['none', 'body', 'form-data', 'x-www-form-urlencoded', 'binary']);

/**
 * Content Type 枚举
 */
export const contentTypeSchema = z.enum(['json', 'text', 'html', 'xml', 'javascript']);

/**
 * Form Field 类型枚举
 */
export const formFieldTypeSchema = z.enum(['text', 'file']);

/**
 * Form Field Schema
 */
export const formFieldSchema = z.object({
  key: z.string(), // 允许空值，保存时过滤空行
  value: z.string(),
  field_type: formFieldTypeSchema.optional(),
  file_path: z.string().optional(),
});

/**
 * 输出映射 Schema
 */
export const outputMappingSchema = z.object({
  source: z.string().min(1, getLL().validation.source_field_required()),
  target: z.string()
    .min(1, getLL().validation.variable_name_required())
    .max(64, getLL().validation.variable_name_max())
    .regex(/^[a-zA-Z0-9_]+$/, getLL().validation.variable_name_regex()),
  description: z.string().optional(),
  enabled: z.boolean().optional(),
});

export type OutputMappingFormData = z.infer<typeof outputMappingSchema>;


/**
 * Header 键值对 Schema
 */
export const headerEntrySchema = z.object({
  key: z.string(),
  value: z.string(),
});

// HTTP 认证 Schema
export const httpAuthTypeSchema = z.enum(['none', 'basic', 'bearer', 'api_key_header', 'api_key_query', 'digest']);

export const httpAuthConfigSchema = z.object({
  auth_type: httpAuthTypeSchema.optional(),
  username: z.string().optional(),
  password: z.string().optional(),
  token: z.string().optional(),
  api_key_name: z.string().optional(),
  api_key_value: z.string().optional(),
});

// Cookie Schema
export const cookieEntrySchema = z.object({
  name: z.string().min(1),
  value: z.string(),
  domain: z.string().optional(),
  path: z.string().optional(),
});

export const httpConfigSchema = z.object({
  url: z.string().min(1, getLL().validation.url_required()),
  method: httpMethodSchema,
  headers: z.array(headerEntrySchema).optional(),
  body_type: httpBodyTypeSchema.optional(),
  content_type: contentTypeSchema.optional(),
  form_fields: z.array(formFieldSchema).optional(),
  file_path: z.string().optional(),
  body: z.string().optional(),
  timeout_ms: z.number().optional(),
  auth: httpAuthConfigSchema.optional(),
  follow_redirects: z.boolean().optional(),
  max_redirects: z.number().min(1).max(20).optional(),
  tls_skip_verify: z.boolean().optional(),
  tls_ca_cert_path: z.string().optional(),
  tls_client_cert_path: z.string().optional(),
  tls_client_key_path: z.string().optional(),
  accept_encoding: z.boolean().optional(),
  enable_cookies: z.boolean().optional(),
  cookie_jar: z.array(cookieEntrySchema).optional(),
  retry_count: z.number().min(0).max(10).optional(),
  retry_delay_ms: z.number().min(0).optional(),
  retry_on_status: z.array(z.number().min(100).max(599)).optional(),
});

export type HttpConfigFormData = z.infer<typeof httpConfigSchema>;
export type HeaderEntry = z.infer<typeof headerEntrySchema>;
export type HttpBodyType = z.infer<typeof httpBodyTypeSchema>;
export type ContentType = z.infer<typeof contentTypeSchema>;
export type FormFieldType = z.infer<typeof formFieldTypeSchema>;
export type FormField = z.infer<typeof formFieldSchema>;

// ============================================================================
// Conditional 配置 Schema
// ============================================================================

/**
 * 条件类型枚举
 */
export const conditionTypeSchema = z.enum(['simple', 'advanced']);

/**
 * 条件操作符枚举
 */
export const conditionOperatorSchema = z.enum([
  'equal',
  'not_equal',
  'greater',
  'greater_or_equal',
  'less',
  'less_or_equal',
  'contains',
  'starts_with',
  'ends_with',
  'exists',
]);

/**
 * 期望值类型枚举
 */
export const expectedValueTypeSchema = z.enum(['number', 'string', 'boolean', 'null']);

/**
 * Conditional 配置表单 Schema
 */
export const conditionalConfigSchema = z.object({
  condition_type: conditionTypeSchema,
  field_path: z.string().optional(),
  operator: conditionOperatorSchema.optional(),
  expected_value: z.string().optional(),
  advanced_expression: z.string().optional(),
  expected_value_type: expectedValueTypeSchema.optional(),
  source_node_id: z.string().optional(),
});

export type ConditionalConfigFormData = z.infer<typeof conditionalConfigSchema>;

// ============================================================================
// 操作符选项
// ============================================================================

export const OPERATOR_OPTIONS = [
  { value: 'equal', label: getLL().validation.operator_equal() },
  { value: 'not_equal', label: getLL().validation.operator_not_equal() },
  { value: 'greater', label: getLL().validation.operator_greater() },
  { value: 'greater_or_equal', label: getLL().validation.operator_greater_or_equal() },
  { value: 'less', label: getLL().validation.operator_less() },
  { value: 'less_or_equal', label: getLL().validation.operator_less_or_equal() },
  { value: 'contains', label: getLL().validation.operator_contains() },
  { value: 'starts_with', label: getLL().validation.operator_starts_with() },
  { value: 'ends_with', label: getLL().validation.operator_ends_with() },
  { value: 'exists', label: getLL().validation.operator_exists() },
] as const;

// ============================================================================
// Entry 配置 Schema
// ============================================================================
// Hook 钩子 Schema
// ============================================================================

/** 钩子触发条件 schema */
const hookTriggerSchema = z.enum(['on_success', 'on_failure', 'on_cancel', 'on_complete']);

/** Webhook 动作 schema */
const webhookHookActionSchema = z.object({
  type: z.literal('webhook'),
  url: z.string().min(1, getLL().validation.url_required()),
  method: z.string().min(1, getLL().validation.webhook_method_required()),
  headers: z.record(z.string(), z.string()),
  body_template: z.string().optional(),
  timeout_ms: z.number().min(1000).max(300000).optional(),
});

/** Script 动作 schema */
const scriptHookActionSchema = z.object({
  type: z.literal('script'),
  code: z.string().min(1, getLL().validation.script_code_required()),
  timeout_ms: z.number().min(1000).max(300000).optional(),
});

/** Notify 动作 schema */
const notifyHookActionSchema = z.object({
  type: z.literal('notify'),
  title_template: z.string().min(1, getLL().validation.notify_title_required()),
  message_template: z.string().min(1, getLL().validation.notify_message_required()),
});

/** 钩子动作 discriminated union */
const hookActionSchema = z.discriminatedUnion('type', [
  webhookHookActionSchema,
  scriptHookActionSchema,
  notifyHookActionSchema,
]);

/** 单个钩子 schema */
const flowHookSchema = z.object({
  uuid: z.string(),
  name: z.string().min(1, getLL().validation.hook_name_required()),
  enabled: z.boolean(),
  trigger: hookTriggerSchema,
  action: hookActionSchema,
});

/** 钩子列表 schema */
const hooksSchema = z.array(flowHookSchema).max(10, getLL().validation.hooks_max()).optional();

/**
 * Entry 配置表单 Schema
 */
export const entryConfigSchema = z.object({
  type: z.literal('entry'),
  /** 关联的 CSV 文件路径 */
  dataset_path: z.string().optional(),
  /** CSV 编码（不设置=自动检测） */
  dataset_encoding: z.string().optional(),
  /** 流程钩子配置 */
  hooks: hooksSchema,
});

export type EntryConfigFormData = z.infer<typeof entryConfigSchema>;

// ============================================================================
// WebSocket 配置 Schema
// ============================================================================

/**
 * WebSocket 收集模式枚举
 */
export const wsCollectModeSchema = z.enum(['first_match', 'condition_match', 'duration_collect']);
export type WsCollectMode = z.infer<typeof wsCollectModeSchema>;

/**
 * WebSocket 匹配类型枚举
 */
export const wsMatchTypeSchema = z.enum(['json_path', 'regex', 'expression']);
export type WsMatchType = z.infer<typeof wsMatchTypeSchema>;

/**
 * WebSocket 消息类型枚举
 */
export const wsMessageTypeSchema = z.enum(['text', 'binary']);
export type WsMessageType = z.infer<typeof wsMessageTypeSchema>;

/**
 * 二进制消息输入模式枚举
 * hex    - 十六进制字符串（如 010203），直接传给后端
 * base64 - Base64 编码字符串，前端转换为 hex 后传给后端
 * text   - 普通文本，前端转换为 hex 后传给后端（每字节取 charCodeAt）
 */
export const wsBinaryInputModeSchema = z.enum(['hex', 'base64', 'text']);
export type WsBinaryInputMode = z.infer<typeof wsBinaryInputModeSchema>;

/**
 * WebSocket 收集条件 Schema
 */
export const wsCollectConditionSchema = z.object({
  match_type: wsMatchTypeSchema,
  expression: z.string().min(1, getLL().validation.match_expression_required()),
});

export type WsCollectConditionFormData = z.infer<typeof wsCollectConditionSchema>;

/**
 * WebSocket Connect 配置表单 Schema
 */
export const wsConnectConfigSchema = z.object({
  type: z.literal('ws_connect'),
  url: z.string().min(1, getLL().validation.url_required()),
  headers: z.array(formFieldSchema).optional(),
  subprotocols: z.array(z.string()).optional(),
  connect_timeout_ms: z.number().min(1000).max(300000).optional(),
  collect_message: z.boolean().optional(),
  collect_mode: wsCollectModeSchema.optional(),
  collect_condition: wsCollectConditionSchema.optional(),
  collect_timeout_ms: z.number().min(1000).max(300000).optional(),
  /** 是否使用 TLS */
  use_tls: z.boolean().optional(),
  /** 是否跳过 TLS 证书验证 */
  tls_skip_verify: z.boolean().optional(),
  /** Ping 间隔（毫秒） */
  ping_interval_ms: z.number().optional(),
  /** Ping 超时（毫秒） */
  ping_timeout_ms: z.number().optional(),
  // 注意：collect_mode 和 collect_condition 在 collect_message=false 时的处理
  // 由 WsConnectConfig 的 fromFormData 函数负责，这里不做额外验证
});

export type WsConnectConfigFormData = z.infer<typeof wsConnectConfigSchema>;

/**
 * WebSocket Send+Collect 配置表单 Schema
 */
export const wsSendCollectConfigSchema = z.object({
  type: z.literal('ws_send_collect'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
  message: z.string().min(1, getLL().validation.ws_message_required()),
  message_type: wsMessageTypeSchema.optional(),
  /** 二进制消息输入模式，仅当 message_type === 'binary' 时生效 */
  binary_input_mode: wsBinaryInputModeSchema,
  send_timeout_ms: z.number().min(1000).max(300000).optional(),
  collect_response: z.boolean(),
  collect_mode: wsCollectModeSchema.optional(),
  collect_condition: wsCollectConditionSchema.optional(),
  collect_timeout_ms: z.number().min(1000).max(300000).optional(),
  // 注意：collect_mode 和 collect_condition 在 collect_response=false 时的处理
  // 由 fromFormData 函数负责，这里不做额外验证
});

export type WsSendCollectConfigFormData = z.infer<typeof wsSendCollectConfigSchema>;

/**
 * WebSocket Close 配置表单 Schema
 */
export const wsCloseConfigSchema = z.object({
  type: z.literal('ws_close'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
  close_code: z.number().optional(),
  close_reason: z.string().optional(),
  close_timeout_ms: z.number().min(1000).max(300000).optional(),
});

export type WsCloseConfigFormData = z.infer<typeof wsCloseConfigSchema>;

// ============================================================================
// Assertion 配置 Schema
// ============================================================================

/**
 * 断言操作符枚举
 */
export const assertionOperatorSchema = z.enum([
  'equal',
  'not_equal',
  'greater',
  'greater_or_equal',
  'less',
  'less_or_equal',
  'contains',
  'not_contains',
  'starts_with',
  'ends_with',
  'exists',
  'not_exists',
  'is_empty',
  'is_not_empty',
  'matches_regex',
]);

export type AssertionOperator = z.infer<typeof assertionOperatorSchema>;

/**
 * 断言模式枚举
 */
export const assertionModeSchema = z.enum(['all', 'any']);
export type AssertionMode = z.infer<typeof assertionModeSchema>;

/**
 * 节点执行失败处理策略枚举
 */
export const executionErrorStrategySchema = z.enum(['route_to_false', 'abort_flow', 'ignore']);
export type ExecutionErrorStrategy = z.infer<typeof executionErrorStrategySchema>;

/**
 * 失败处理模式枚举
 */
export const onFailureModeSchema = z.enum(['route_false', 'throw']);
export type OnFailureMode = z.infer<typeof onFailureModeSchema>;

/**
 * 单条断言 Schema
 */
export const assertionSchema = z.object({
  target: z.string().min(1, getLL().validation.target_field_required()),
  operator: assertionOperatorSchema,
  expected: z.string(),
  enabled: z.boolean().default(true),
  description: z.string().optional(),
});

export type AssertionFormData = z.infer<typeof assertionSchema>;

/**
 * Assertion 配置表单 Schema
 */
export const assertionConfigSchema = z.object({
  type: z.literal('assertion'),
  preset: z.string().optional(),
  mode: assertionModeSchema,
  on_failure: onFailureModeSchema,
  continue_on_failure: z.boolean().default(false),
  assertions: z.array(assertionSchema),
  source_node_id: z.string().optional(),
});

export type AssertionConfigFormData = z.infer<typeof assertionConfigSchema>;

// ============================================================================
// Script 配置 Schema
// ============================================================================

/**
 * 脚本执行时机枚举
 */
export const scriptExecutionPhaseSchema = z.enum(['post_request', 'pre_request', 'standalone']);
export type ScriptExecutionPhaseFormData = z.infer<typeof scriptExecutionPhaseSchema>;

/**
 * 断言失败处理策略枚举
 */
export const assertionFailStrategySchema = z.enum(['route_false', 'ignore', 'abort']);
export type AssertionFailStrategyFormData = z.infer<typeof assertionFailStrategySchema>;

/**
 * Script 配置表单 Schema
 *
 * NOTE: `on_error` (ScriptErrorStrategy) 和 `on_assertion_fail` 均已废弃，
 *       由 Node.on_error (ExecutionErrorStrategy) 统一替代。
 *       `assertionFailStrategySchema` type definition kept for backward compat.
 */
export const scriptConfigSchema = z.object({
  type: z.literal('script'),
  script: z.string(),
  timeout_ms: z.number().min(100).max(60000).optional(),
  memory_limit_mb: z.number().min(1).max(100).optional(),
  execution_phase: scriptExecutionPhaseSchema.optional(),
});

export type ScriptConfigFormData = z.infer<typeof scriptConfigSchema>;

// ============================================================================
// gRPC 配置 Schema
// ============================================================================

/**
 * Proto 文件 Schema
 */
export const protoFileSchema = z.object({
  path: z.string().min(1, getLL().validation.proto_file_path_required()),
  content: z.string().min(1, getLL().validation.proto_file_content_required()),
});

export type ProtoFileFormData = z.infer<typeof protoFileSchema>;

/**
 * gRPC Metadata 条目 Schema
 */
export const grpcMetadataEntrySchema = z.object({
  key: z.string(),
  value: z.string(),
});

/**
 * gRPC 方法信息 Schema
 */
export const grpcMethodInfoSchema = z.object({
  method_name: z.string(),
  input_type: z.string(),
  output_type: z.string(),
  is_server_streaming: z.boolean(),
  is_client_streaming: z.boolean().optional(),
});

// ============================================================================
// gRPC Message Definition Schemas
// ============================================================================

/**
 * gRPC 枚举值 Schema
 */
export const grpcEnumValueSchema = z.object({
  name: z.string(),
  number: z.number(),
});

/**
 * gRPC oneof 分组 Schema
 */
export const grpcOneofInfoSchema = z.object({
  name: z.string(),
  field_numbers: z.array(z.number()),
});

/**
 * gRPC 字段信息 Schema
 * （递归类型：nested_message 引用 GrpcMessageInfo，
 *  使用 z.lazy() 包裹整个 schema 以支持递归引用）
 */
export const grpcFieldInfoSchema: z.ZodType<GrpcFieldInfo> = z.lazy(() =>
  z.object({
    name: z.string(),
    number: z.number(),
    type_kind: z.string(),
    type_display: z.string(),
    type_full_name: z.string().optional(),
    label: z.string(),
    is_map: z.boolean(),
    map_key_type: z.string().optional(),
    map_value_type: z.string().optional(),
    enum_values: z.array(grpcEnumValueSchema).default([]),
    nested_message: grpcMessageInfoSchema.optional(),
  })
);

/**
 * gRPC 消息定义 Schema
 * （递归类型：fields 中的 GrpcFieldInfo 可能引用其它 GrpcMessageInfo，
 *  使用 z.lazy() 包裹整个 schema 以支持递归引用）
 */
export const grpcMessageInfoSchema: z.ZodType<GrpcMessageInfo> = z.lazy(() =>
  z.object({
    full_name: z.string(),
    fields: z.array(grpcFieldInfoSchema),
    oneof_groups: z.array(grpcOneofInfoSchema),
    is_wkt: z.boolean(),
    reserved_ranges: z.array(z.tuple([z.number(), z.number()])).optional(),
    reserved_names: z.array(z.string()).optional(),
  })
);

/**
 * gRPC 服务信息 Schema
 */
export const grpcServiceInfoSchema = z.object({
  service_name: z.string(),
  methods: z.array(grpcMethodInfoSchema),
  message_definitions: z.custom<Record<string, GrpcMessageInfo>>().optional(),
});

/**
 * gRPC Connect 配置表单 Schema
 */
export const grpcConnectConfigSchema = z.object({
  type: z.literal('grpc_connect'),
  url: z.string().min(1, getLL().validation.grpc_server_url_required()),
  use_tls: z.boolean().optional(),
  tls_skip_verify: z.boolean().optional(),
  enable_reflection: z.boolean().optional(),
  proto_files: z.array(protoFileSchema).optional(),
  default_metadata: z.array(grpcMetadataEntrySchema).optional(),
  connect_timeout_ms: z.number().min(1000).max(300000).optional(),
  discovered_services: z.array(grpcServiceInfoSchema).optional(),
  // TLS 增强字段
  tls_ca_cert: z.string().optional(),
  tls_client_cert: z.string().optional(),
  tls_client_key: z.string().optional(),
  tls_server_name_override: z.string().optional(),
  // 压缩配置
  compression_encoding: z.string().optional(),
  // Keepalive 配置
  keepalive_time_ms: z.number().optional(),
  keepalive_timeout_ms: z.number().optional(),
  keepalive_permit_without_streams: z.boolean().optional(),
  // Reflection 增强配置
  reflection_metadata: z.array(grpcMetadataEntrySchema).optional(),
  // 负载均衡配置
  endpoints: z.array(z.string()).optional(),
  // Channelz 内省（EXPERIMENTAL）— 仅前端 UI 状态，不保存到节点数据
  channelz_connection_id: z.string().optional(),
});

export type GrpcConnectConfigFormData = z.infer<typeof grpcConnectConfigSchema>;

/**
 * gRPC 流式消息 Schema
 */
export const streamMessageSchema = z.object({
  enabled: z.boolean(),
  content: z.string(),
});

export type StreamMessageFormData = z.infer<typeof streamMessageSchema>;

/**
 * gRPC Call 配置表单 Schema
 */
export const grpcCallConfigSchema = z.object({
  type: z.literal('grpc_call'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
  service_name: z.string().min(1, getLL().validation.grpc_service_name_required()),
  method_name: z.string().min(1, getLL().validation.grpc_method_name_required()),
  request_json: z.string(),
  timeout_ms: z.number().min(1000).max(300000).optional(),
  metadata: z.array(grpcMetadataEntrySchema).optional(),
  request_messages: z.array(streamMessageSchema).optional(),
  // 压缩覆盖配置
  compression_encoding: z.string().optional(),
  // 消息大小限制
  max_receive_message_size: z.number().optional(),
  max_send_message_size: z.number().optional(),
});

export type GrpcCallConfigFormData = z.infer<typeof grpcCallConfigSchema>;

/**
 * gRPC Close 配置表单 Schema
 */
export const grpcCloseConfigSchema = z.object({
  type: z.literal('grpc_close'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
});

export type GrpcCloseConfigFormData = z.infer<typeof grpcCloseConfigSchema>;

// ============================================================================
// MCP 配置 Schema
// ============================================================================

/**
 * MCP Connect 配置表单 Schema
 *
 * 条件验证：
 * - streamable_http 模式下 url 必填
 * - stdio 模式下 command 必填
 */
export const mcpConnectConfigSchema = z.object({
  type: z.literal('mcp_connect'),
  transport: z.enum(['streamable_http', 'stdio']).optional(),
  url: z.string().optional(),
  protocol_version: z.string().optional(),
  custom_headers: z.array(z.object({
    key: z.string(),
    value: z.string(),
  })).optional(),
  command: z.string().optional(),
  args: z.array(z.string()).optional(),
  env: z.array(z.object({
    key: z.string(),
    value: z.string(),
  })).optional(),
}).refine(
  (data) => {
    const transport = data.transport || 'streamable_http';
    if (transport === 'streamable_http') {
      return !!data.url && data.url.trim().length > 0;
    }
    return true;
  },
  { message: getLL().validation.mcp_server_url_required(), path: ['url'] },
).refine(
  (data) => {
    const transport = data.transport || 'streamable_http';
    if (transport === 'stdio') {
      return !!data.command && data.command.trim().length > 0;
    }
    return true;
  },
  { message: getLL().validation.mcp_command_required(), path: ['command'] },
);

export type McpConnectConfigFormData = z.infer<typeof mcpConnectConfigSchema>;

/**
 * MCP Call Tool 配置表单 Schema
 */
export const mcpCallToolConfigSchema = z.object({
  type: z.literal('mcp_call_tool'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
  tool_name: z.string().min(1, getLL().validation.mcp_tool_name_required()),
  arguments: z.any().optional(),
});

export type McpCallToolConfigFormData = z.infer<typeof mcpCallToolConfigSchema>;

/**
 * MCP Read Resource 配置表单 Schema
 */
export const mcpReadResourceConfigSchema = z.object({
  type: z.literal('mcp_read_resource'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
  resource_uri: z.string().min(1, getLL().validation.mcp_resource_uri_required()),
});

export type McpReadResourceConfigFormData = z.infer<typeof mcpReadResourceConfigSchema>;

/**
 * MCP Close 配置表单 Schema
 */
export const mcpCloseConfigSchema = z.object({
  type: z.literal('mcp_close'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
});

export type McpCloseConfigFormData = z.infer<typeof mcpCloseConfigSchema>;

/**
 * MCP Get Prompt 配置表单 Schema
 */
export const mcpGetPromptConfigSchema = z.object({
  type: z.literal('mcp_get_prompt'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
  prompt_name: z.string().min(1, getLL().validation.mcp_prompt_name_required()),
  arguments: z.record(z.string(), z.string()).optional(),
});

export type McpGetPromptConfigFormData = z.infer<typeof mcpGetPromptConfigSchema>;

// ============================================================================
// TCP/UDP 配置 Schema
// ============================================================================

/**
 * TCP/UDP 成帧配置 Schema
 */
export const tcpFramingConfigSchema = z.object({
  mode: z.discriminatedUnion('type', [
    z.object({ type: z.literal('length_prefix'), prefix_bytes: z.number().int().min(1).max(8) }),
    z.object({ type: z.literal('delimiter'), delimiter: z.string().min(1) }),
    z.object({ type: z.literal('fixed_length'), length: z.number().int().positive() }),
    z.object({ type: z.literal('timeout'), timeout_ms: z.number().int().positive() }),
    z.object({ type: z.literal('manual') }),
  ]),
  max_frame_size: z.number().int().positive().optional(),
  include_framing_in_data: z.boolean().optional(),
});

export type TcpFramingConfigFormData = z.infer<typeof tcpFramingConfigSchema>;

/**
 * TCP Connect 配置表单 Schema
 */
export const tcpConnectConfigSchema = z.object({
  type: z.literal('tcp_connect'),
  host: z.string().min(1, getLL().validation.host_required()),
  port: z.number().int().min(1).max(65535, getLL().validation.port_range()),
  use_tls: z.boolean().optional(),
  tls_skip_verify: z.boolean().optional(),
  connect_timeout_ms: z.number().int().positive().optional(),
  framing_mode: tcpFramingConfigSchema.shape.mode.optional(),
  framing_config: tcpFramingConfigSchema.optional(),
  /** 是否禁用 Nagle 算法 */
  no_delay: z.boolean().optional(),
  /** TCP Keepalive 配置 */
  keepalive: z.object({
    idle_secs: z.number(),
    interval_secs: z.number(),
    probe_count: z.number(),
  }).optional(),
  /** TLS CA 证书路径 */
  tls_ca_cert: z.string().optional(),
});

export type TcpConnectConfigFormData = z.infer<typeof tcpConnectConfigSchema>;

/**
 * TCP Send 配置表单 Schema
 */
export const tcpSendConfigSchema = z.object({
  type: z.literal('tcp_send'),
  connection_id: z.string().min(1, getLL().validation.connection_select_required()),
  data: z.string(),
  data_encoding: z.enum(['text', 'hex', 'base64']).optional(),
  framing_mode: tcpFramingConfigSchema.shape.mode.optional(),
  framing_config: tcpFramingConfigSchema.optional(),
  send_timeout_ms: z.number().int().positive().optional(),
  /** 解析器配置（透传 JSON） */
  parser_config: z.record(z.string(), z.unknown()).optional(),
});

export type TcpSendConfigFormData = z.infer<typeof tcpSendConfigSchema>;

/**
 * TCP Close 配置表单 Schema
 */
export const tcpCloseConfigSchema = z.object({
  type: z.literal('tcp_close'),
  connection_id: z.string().min(1, getLL().validation.connection_select_required()),
  close_timeout_ms: z.number().int().positive().optional(),
});

export type TcpCloseConfigFormData = z.infer<typeof tcpCloseConfigSchema>;

/**
 * TCP Receive 配置表单 Schema
 */
export const tcpReceiveConfigSchema = z.object({
  type: z.literal('tcp_receive'),
  connection_id: z.string().min(1, getLL().validation.connection_select_required()),
  receive_timeout_ms: z.number().int().positive(),
  framing_mode: tcpFramingConfigSchema.shape.mode.optional(),
  framing_config: tcpFramingConfigSchema.optional(),
});

export type TcpReceiveConfigFormData = z.infer<typeof tcpReceiveConfigSchema>;

/**
 * UDP Bind 配置表单 Schema
 */
export const udpBindConfigSchema = z.object({
  type: z.literal('udp_bind'),
  local_addr: z.string().optional(),
  local_port: z.number().int().min(0).max(65535),
  reuse_addr: z.boolean().optional(),
  /** 是否启用广播 */
  broadcast: z.boolean().optional(),
  /** 多播组地址 */
  multicast_group: z.string().optional(),
  /** 多播 TTL */
  multicast_ttl: z.number().optional(),
  /** 是否启用多播环回 */
  multicast_loop: z.boolean().optional(),
  /** 多播网络接口 */
  multicast_interface: z.string().optional(),
});

export type UdpBindConfigFormData = z.infer<typeof udpBindConfigSchema>;

/**
 * UDP SendTo 配置表单 Schema
 */
export const udpSendToConfigSchema = z.object({
  type: z.literal('udp_send_to'),
  connection_id: z.string().min(1, getLL().validation.connection_select_required()),
  data: z.string(),
  data_encoding: z.enum(['text', 'hex', 'base64']).optional(),
  target_addr: z.string().min(1, getLL().validation.target_addr_required()),
  target_port: z.number().int().min(1).max(65535, getLL().validation.port_range()),
  send_timeout_ms: z.number().int().positive().optional(),
});

export type UdpSendToConfigFormData = z.infer<typeof udpSendToConfigSchema>;

/**
 * UDP RecvFrom 配置表单 Schema
 */
export const udpRecvFromConfigSchema = z.object({
  type: z.literal('udp_recv_from'),
  connection_id: z.string().min(1, getLL().validation.connection_select_required()),
  receive_timeout_ms: z.number().int().positive().optional(),
  max_size: z.number().int().positive().optional(),
});

export type UdpRecvFromConfigFormData = z.infer<typeof udpRecvFromConfigSchema>;

/**
 * UDP Close 配置表单 Schema
 */
export const udpCloseConfigSchema = z.object({
  type: z.literal('udp_close'),
  connection_id: z.string().min(1, getLL().validation.connection_select_required()),
});

export type UdpCloseConfigFormData = z.infer<typeof udpCloseConfigSchema>;

// ============================================================================
// SSE 配置 Schema
// ============================================================================

/**
 * SSE Connect 配置表单 Schema
 */
export const sseConnectConfigSchema = z.object({
  type: z.literal('sse_connect'),
  url: z.string().min(1, getLL().validation.url_required()),
  headers: z.record(z.string(), z.string()).optional(),
  last_event_id: z.string().optional(),
  timeout_ms: z.number().min(1000).max(300000).optional(),
  auto_reconnect: z.boolean().optional(),
  reconnect_interval_ms: z.number().optional(),
  max_reconnect_attempts: z.number().optional(),
  tls_mode: z.enum(['none', 'standard', 'skip-verify']).optional(),
});

export type SseConnectConfigFormData = z.infer<typeof sseConnectConfigSchema>;

/**
 * SSE Listen 配置表单 Schema
 */
export const sseListenConfigSchema = z.object({
  type: z.literal('sse_listen'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
  event_filter: z.string().optional(),
  max_events: z.number().min(0).optional(),
  timeout_ms: z.number().min(0).optional(),
  include_data: z.boolean().optional(),
  decode_json: z.boolean().optional(),
});

export type SseListenConfigFormData = z.infer<typeof sseListenConfigSchema>;

/**
 * SSE Disconnect 配置表单 Schema
 */
export const sseDisconnectConfigSchema = z.object({
  type: z.literal('sse_disconnect'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
});

export type SseDisconnectConfigFormData = z.infer<typeof sseDisconnectConfigSchema>;

// ============================================================================
// GraphQL 配置 Schema
// ============================================================================

/**
 * GraphQL Connect 配置表单 Schema
 */
export const graphqlConnectConfigSchema = z.object({
  type: z.literal('graphql_connect'),
  endpoint: z.string().min(1, getLL().validation.graphql_endpoint_required()),
  ws_endpoint: z.string().optional(),
  headers: z.record(z.string(), z.string()).optional(),
  connection_timeout_ms: z.number().min(1000).max(300000).optional(),
  connection_id: z.string().optional(),
});

export type GraphqlConnectConfigFormData = z.infer<typeof graphqlConnectConfigSchema>;

/**
 * GraphQL Query 配置表单 Schema
 */
export const graphqlQueryConfigSchema = z.object({
  type: z.literal('graphql_query'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
  query: z.string().min(1, getLL().validation.graphql_query_required()),
  variables: z.string().optional(),
  operation_name: z.string().optional(),
  files: z.array(z.object({
    field_name: z.string(),
    file_path: z.string(),
  })).optional(),
  timeout_ms: z.number().min(1000).max(300000).optional(),
});

export type GraphqlQueryConfigFormData = z.infer<typeof graphqlQueryConfigSchema>;

/**
 * GraphQL Subscribe 配置表单 Schema
 */
export const graphqlSubscribeConfigSchema = z.object({
  type: z.literal('graphql_subscribe'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
  subscription: z.string().min(1, getLL().validation.graphql_subscription_required()),
  variables: z.string().optional(),
  operation_name: z.string().optional(),
  max_events: z.number().min(0).optional(),
  timeout_ms: z.number().min(0).optional(),
});

export type GraphqlSubscribeConfigFormData = z.infer<typeof graphqlSubscribeConfigSchema>;

/**
 * GraphQL Introspect 配置表单 Schema
 */
export const graphqlIntrospectConfigSchema = z.object({
  type: z.literal('graphql_introspect'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
  timeout_ms: z.number().min(1000).max(300000).optional(),
});

export type GraphqlIntrospectConfigFormData = z.infer<typeof graphqlIntrospectConfigSchema>;

/**
 * GraphQL Disconnect 配置表单 Schema
 */
export const graphqlDisconnectConfigSchema = z.object({
  type: z.literal('graphql_disconnect'),
  connection_id: z.string().min(1, getLL().validation.connection_id_required()),
});

export type GraphqlDisconnectConfigFormData = z.infer<typeof graphqlDisconnectConfigSchema>;

// ============================================================================
// Variable Extractor 配置 Schema
// ============================================================================

/**
 * Variable Extractor 配置表单 Schema
 */
export const variableExtractorConfigSchema = z.object({
  type: z.literal('variable_extractor'),
  output_mappings: z.array(outputMappingSchema).optional(),
  source_node_id: z.string().optional(),
}).refine(data => {
  const targets = (data.output_mappings || []).map(m => m.target).filter(Boolean);
  return new Set(targets).size === targets.length;
}, { message: getLL().validation.variable_name_duplicate(), path: ['output_mappings'] });

export type VariableExtractorConfigFormData = z.infer<typeof variableExtractorConfigSchema>;

// ============================================================================
// Proxy 配置 Schema
// ============================================================================

/**
 * 代理模式枚举
 */
export const proxyModeSchema = z.enum(['clear', 'http', 'socks5']);
export type ProxyModeFormData = z.infer<typeof proxyModeSchema>;

/**
 * Proxy 配置表单 Schema
 */
export const proxyConfigSchema = z.object({
  type: z.literal('proxy'),
  mode: proxyModeSchema,
  proxy_url: z.string().optional(),
  username: z.string().optional(),
  password: z.string().optional(),
});

export type ProxyConfigFormData = z.infer<typeof proxyConfigSchema>;


// ============================================================================
// 未知节点类型 Fallback Schema
// ============================================================================

/**
 * 未知节点类型（插件/动态注册）的通用配置 Schema
 * 接受任意 type 字符串 + passthrough 属性，用于 SchemaConfigPanel 渲染
 */
export const unknownNodeConfigSchema = z.object({
  type: z.string(),
}).passthrough();

export type UnknownNodeConfigFormData = z.infer<typeof unknownNodeConfigSchema>;
