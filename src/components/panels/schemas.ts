/**
 * 节点配置表单校验 Schema
 */
import { z } from 'zod';

// ============================================================================
// HTTP 配置 Schema
// ============================================================================

/**
 * HTTP 方法枚举
 */
export const httpMethodSchema = z.enum(['get', 'post', 'put', 'delete', 'patch']);

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
  type: formFieldTypeSchema.optional(),
  file_path: z.string().optional(),
});

/**
 * 输出映射 Schema
 */
export const outputMappingSchema = z.object({
  source: z.string().min(1, '源字段不能为空'),
  target: z.string()
    .min(1, '变量名不能为空')
    .max(64, '变量名不能超过64个字符')
    .regex(/^[a-zA-Z0-9_]+$/, '变量名只能包含字母、数字和下划线'),
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

export const httpConfigSchema = z.object({
  url: z.string().url('请输入有效的 URL').min(1, 'URL 不能为空'),
  method: httpMethodSchema,
  headers: z.array(headerEntrySchema).optional(),
  body_type: httpBodyTypeSchema.optional(),
  content_type: contentTypeSchema.optional(),
  form_fields: z.array(formFieldSchema).optional(),
  file_path: z.string().optional(),
  body: z.string().optional(),
  timeout_ms: z.number().optional(),
  output_mappings: z.array(outputMappingSchema).optional(),
}).refine(
  (data) => {
    if (!data.output_mappings) return true;
    const targets = data.output_mappings.map(m => m.target);
    const uniqueTargets = new Set(targets);
    return targets.length === uniqueTargets.size;
  },
  {
    message: '变量名必须唯一',
    path: ['output_mappings'],
  }
);

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
});

export type ConditionalConfigFormData = z.infer<typeof conditionalConfigSchema>;

// ============================================================================
// 操作符选项
// ============================================================================

export const OPERATOR_OPTIONS = [
  { value: 'equal', label: '等于 (=)' },
  { value: 'not_equal', label: '不等于 (!=)' },
  { value: 'greater', label: '大于 (>)' },
  { value: 'greater_or_equal', label: '大于等于 (>=)' },
  { value: 'less', label: '小于 (<)' },
  { value: 'less_or_equal', label: '小于等于 (<=)' },
  { value: 'contains', label: '包含' },
  { value: 'starts_with', label: '以...开始' },
  { value: 'ends_with', label: '以...结束' },
  { value: 'exists', label: '存在' },
] as const;

// ============================================================================
// Entry 配置 Schema
// ============================================================================

/**
 * 初始变量 Schema
 */
export const initialVariableSchema = z.object({
  name: z.string()
    .min(1, '变量名不能为空')
    .max(64, '变量名不能超过64个字符')
    .regex(/^[a-zA-Z0-9_]+$/, '变量名只能包含字母、数字和下划线'),
  value: z.unknown(),
  description: z.string().optional(),
});

export type InitialVariableFormData = z.infer<typeof initialVariableSchema>;

// ============================================================================
// Hook 钩子 Schema
// ============================================================================

/** 钩子触发条件 schema */
const hookTriggerSchema = z.enum(['on_success', 'on_failure', 'on_cancel', 'on_complete']);

/** Webhook 动作 schema */
const webhookHookActionSchema = z.object({
  type: z.literal('webhook'),
  url: z.string().min(1, 'URL 不能为空'),
  method: z.string().min(1, '方法不能为空'),
  headers: z.record(z.string(), z.string()),
  body_template: z.string().optional(),
  timeout_ms: z.number().min(1000).max(300000).optional(),
});

/** Script 动作 schema */
const scriptHookActionSchema = z.object({
  type: z.literal('script'),
  code: z.string().min(1, '代码不能为空'),
  timeout_ms: z.number().min(1000).max(300000).optional(),
});

/** Notify 动作 schema */
const notifyHookActionSchema = z.object({
  type: z.literal('notify'),
  title_template: z.string().min(1, '标题模板不能为空'),
  message_template: z.string().min(1, '消息模板不能为空'),
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
  name: z.string().min(1, '名称不能为空'),
  enabled: z.boolean(),
  trigger: hookTriggerSchema,
  action: hookActionSchema,
});

/** 钩子列表 schema */
const hooksSchema = z.array(flowHookSchema).max(10, '最多 10 个钩子').optional();

/**
 * Entry 配置表单 Schema
 */
export const entryConfigSchema = z.object({
  type: z.literal('entry'),
  initial_variables: z.array(initialVariableSchema).optional(),
  /** 关联的 CSV 文件路径 */
  dataset_path: z.string().optional(),
  /** CSV 编码（不设置=自动检测） */
  dataset_encoding: z.string().optional(),
  /** 流程钩子配置 */
  hooks: hooksSchema,
}).refine(
  (data) => {
    if (!data.initial_variables) return true;
    const names = data.initial_variables.map(v => v.name);
    const uniqueNames = new Set(names);
    return names.length === uniqueNames.size;
  },
  {
    message: '变量名必须唯一',
    path: ['initial_variables'],
  }
);

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
 * WebSocket 收集条件 Schema
 */
export const wsCollectConditionSchema = z.object({
  match_type: wsMatchTypeSchema,
  expression: z.string().min(1, '匹配表达式不能为空'),
});

export type WsCollectConditionFormData = z.infer<typeof wsCollectConditionSchema>;

/**
 * WebSocket Connect 配置表单 Schema
 */
export const wsConnectConfigSchema = z.object({
  type: z.literal('ws_connect'),
  url: z.string().min(1, 'URL 不能为空'),
  headers: z.array(formFieldSchema).optional(),
  subprotocols: z.array(z.string()).optional(),
  connect_timeout_ms: z.number().min(1000).max(300000).optional(),
  collect_message: z.boolean().optional(),
  collect_mode: wsCollectModeSchema.optional(),
  collect_condition: wsCollectConditionSchema.optional(),
  collect_timeout_ms: z.number().min(1000).max(300000).optional(),
  output_mappings: z.array(outputMappingSchema).optional(),
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
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  message: z.string().min(1, '消息内容不能为空'),
  message_type: wsMessageTypeSchema.optional(),
  send_timeout_ms: z.number().min(1000).max(300000).optional(),
  collect_response: z.boolean(),
  collect_mode: wsCollectModeSchema.optional(),
  collect_condition: wsCollectConditionSchema.optional(),
  collect_timeout_ms: z.number().min(1000).max(300000).optional(),
  output_mappings: z.array(outputMappingSchema).optional(),
  // 注意：collect_mode 和 collect_condition 在 collect_response=false 时的处理
  // 由 fromFormData 函数负责，这里不做额外验证
});

export type WsSendCollectConfigFormData = z.infer<typeof wsSendCollectConfigSchema>;

/**
 * WebSocket Close 配置表单 Schema
 */
export const wsCloseConfigSchema = z.object({
  type: z.literal('ws_close'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
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
 * 失败处理模式枚举
 */
export const onFailureModeSchema = z.enum(['route_false', 'throw']);
export type OnFailureMode = z.infer<typeof onFailureModeSchema>;

/**
 * 单条断言 Schema
 */
export const assertionSchema = z.object({
  target: z.string().min(1, '目标字段不能为空'),
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
});

export type AssertionConfigFormData = z.infer<typeof assertionConfigSchema>;

// ============================================================================
// Database 配置 Schema
// ============================================================================

/**
 * 数据库获取模式枚举
 */
export const dbFetchModeSchema = z.enum(['all', 'one', 'scalar']);
export type DbFetchModeFormData = z.infer<typeof dbFetchModeSchema>;

/**
 * 数据库事务隔离级别枚举
 */
export const dbIsolationLevelSchema = z.enum([
  'default',
  'read_uncommitted',
  'read_committed',
  'repeatable_read',
  'serializable',
]);
export type DbIsolationLevelFormData = z.infer<typeof dbIsolationLevelSchema>;

/**
 * DB Connect 配置表单 Schema
 */
export const dbConnectConfigSchema = z.object({
  type: z.literal('db_connect'),
  driver: z.string().optional(),
  connection_string: z.string().optional(),
  host: z.string().optional(),
  port: z.number().optional(),
  database: z.string().optional(),
  username: z.string().optional(),
  password: z.string().optional(),
  min_connections: z.number().optional(),
  max_connections: z.number().optional(),
  test_query: z.string().optional(),
  connect_timeout_ms: z.number().min(1000).max(300000).optional(),
});

export type DbConnectConfigFormData = z.infer<typeof dbConnectConfigSchema>;

/**
 * DB Query 配置表单 Schema
 */
export const dbQueryConfigSchema = z.object({
  type: z.literal('db_query'),
  connection_uuid: z.string().min(1, '连接 UUID 不能为空'),
  sql: z.string().optional(),
  params: z.array(z.string()).optional(),
  redis_command: z.string().optional(),
  redis_key: z.string().optional(),
  redis_value: z.string().optional(),
  redis_ttl: z.number().min(0).optional(),
  fetch_mode: dbFetchModeSchema.optional(),
  result_to: z.string().optional(),
  output_mappings: z.array(outputMappingSchema).optional(),
  timeout_ms: z.number().min(1000).max(300000).optional(),
});

export type DbQueryConfigFormData = z.infer<typeof dbQueryConfigSchema>;

/**
 * DB Begin 配置表单 Schema
 */
export const dbBeginConfigSchema = z.object({
  type: z.literal('db_begin'),
  connection_uuid: z.string().min(1, '连接 UUID 不能为空'),
  isolation_level: dbIsolationLevelSchema.optional(),
});

export type DbBeginConfigFormData = z.infer<typeof dbBeginConfigSchema>;

/**
 * DB Commit 配置表单 Schema
 */
export const dbCommitConfigSchema = z.object({
  type: z.literal('db_commit'),
  connection_uuid: z.string().min(1, '连接 UUID 不能为空'),
});

export type DbCommitConfigFormData = z.infer<typeof dbCommitConfigSchema>;

/**
 * DB Rollback 配置表单 Schema
 */
export const dbRollbackConfigSchema = z.object({
  type: z.literal('db_rollback'),
  connection_uuid: z.string().min(1, '连接 UUID 不能为空'),
});

export type DbRollbackConfigFormData = z.infer<typeof dbRollbackConfigSchema>;

/**
 * DB Close 配置表单 Schema
 */
export const dbCloseConfigSchema = z.object({
  type: z.literal('db_close'),
  connection_uuid: z.string().min(1, '连接 UUID 不能为空'),
});

export type DbCloseConfigFormData = z.infer<typeof dbCloseConfigSchema>;

// ============================================================================
// Script 配置 Schema
// ============================================================================

/**
 * 脚本执行时机枚举
 */
export const scriptExecutionPhaseSchema = z.enum(['post_request', 'pre_request', 'standalone']);
export type ScriptExecutionPhaseFormData = z.infer<typeof scriptExecutionPhaseSchema>;

/**
 * 脚本错误处理策略枚举
 */
export const scriptErrorStrategySchema = z.enum(['recoverable', 'fatal']);
export type ScriptErrorStrategyFormData = z.infer<typeof scriptErrorStrategySchema>;

/**
 * 断言失败处理策略枚举
 */
export const assertionFailStrategySchema = z.enum(['route_false', 'ignore', 'abort']);
export type AssertionFailStrategyFormData = z.infer<typeof assertionFailStrategySchema>;

/**
 * Script 配置表单 Schema
 */
export const scriptConfigSchema = z.object({
  type: z.literal('script'),
  script: z.string(),
  timeout_ms: z.number().min(100).max(60000).optional(),
  memory_limit_mb: z.number().min(1).max(100).optional(),
  execution_phase: scriptExecutionPhaseSchema.optional(),
  on_error: scriptErrorStrategySchema.optional(),
  on_assertion_fail: assertionFailStrategySchema.optional(),
});

export type ScriptConfigFormData = z.infer<typeof scriptConfigSchema>;

// ============================================================================
// gRPC 配置 Schema
// ============================================================================

/**
 * Proto 文件 Schema
 */
export const protoFileSchema = z.object({
  path: z.string().min(1, '文件路径不能为空'),
  content: z.string().min(1, '文件内容不能为空'),
});

export type ProtoFileFormData = z.infer<typeof protoFileSchema>;

/**
 * gRPC Metadata 条目 Schema
 */
export const grpcMetadataEntrySchema = z.object({
  key: z.string(),
  value: z.string(),
});

export type GrpcMetadataEntryFormData = z.infer<typeof grpcMetadataEntrySchema>;

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

export type GrpcMethodInfoFormData = z.infer<typeof grpcMethodInfoSchema>;

/**
 * gRPC 服务信息 Schema
 */
export const grpcServiceInfoSchema = z.object({
  service_name: z.string(),
  methods: z.array(grpcMethodInfoSchema),
});

export type GrpcServiceInfoFormData = z.infer<typeof grpcServiceInfoSchema>;

/**
 * gRPC Connect 配置表单 Schema
 */
export const grpcConnectConfigSchema = z.object({
  type: z.literal('grpc_connect'),
  url: z.string().min(1, 'gRPC 服务器地址不能为空'),
  use_tls: z.boolean().optional(),
  tls_skip_verify: z.boolean().optional(),
  enable_reflection: z.boolean().optional(),
  proto_files: z.array(protoFileSchema).optional(),
  default_metadata: z.array(grpcMetadataEntrySchema).optional(),
  connect_timeout_ms: z.number().min(1000).max(300000).optional(),
  discovered_services: z.array(grpcServiceInfoSchema).optional(),
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
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  service_name: z.string().min(1, '服务名称不能为空'),
  method_name: z.string().min(1, '方法名称不能为空'),
  request_json: z.string(),
  timeout_ms: z.number().min(1000).max(300000).optional(),
  metadata: z.array(grpcMetadataEntrySchema).optional(),
  request_messages: z.array(streamMessageSchema).optional(),
});

export type GrpcCallConfigFormData = z.infer<typeof grpcCallConfigSchema>;

/**
 * gRPC Close 配置表单 Schema
 */
export const grpcCloseConfigSchema = z.object({
  type: z.literal('grpc_close'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
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
  { message: 'Streamable HTTP 模式下 MCP 服务器地址不能为空', path: ['url'] },
).refine(
  (data) => {
    const transport = data.transport || 'streamable_http';
    if (transport === 'stdio') {
      return !!data.command && data.command.trim().length > 0;
    }
    return true;
  },
  { message: 'Stdio 模式下命令不能为空', path: ['command'] },
);

export type McpConnectConfigFormData = z.infer<typeof mcpConnectConfigSchema>;

/**
 * MCP Call Tool 配置表单 Schema
 */
export const mcpCallToolConfigSchema = z.object({
  type: z.literal('mcp_call_tool'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  tool_name: z.string().min(1, '工具名称不能为空'),
  arguments: z.any().optional(),
});

export type McpCallToolConfigFormData = z.infer<typeof mcpCallToolConfigSchema>;

/**
 * MCP Read Resource 配置表单 Schema
 */
export const mcpReadResourceConfigSchema = z.object({
  type: z.literal('mcp_read_resource'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  resource_uri: z.string().min(1, '资源 URI 不能为空'),
});

export type McpReadResourceConfigFormData = z.infer<typeof mcpReadResourceConfigSchema>;

/**
 * MCP Close 配置表单 Schema
 */
export const mcpCloseConfigSchema = z.object({
  type: z.literal('mcp_close'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
});

export type McpCloseConfigFormData = z.infer<typeof mcpCloseConfigSchema>;

/**
 * MCP Get Prompt 配置表单 Schema
 */
export const mcpGetPromptConfigSchema = z.object({
  type: z.literal('mcp_get_prompt'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  prompt_name: z.string().min(1, '提示词名称不能为空'),
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
  host: z.string().min(1, '请输入主机地址'),
  port: z.number().int().min(1).max(65535, '端口范围 1-65535'),
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
  connection_id: z.string().min(1, '请选择连接'),
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
  connection_id: z.string().min(1, '请选择连接'),
  close_timeout_ms: z.number().int().positive().optional(),
});

export type TcpCloseConfigFormData = z.infer<typeof tcpCloseConfigSchema>;

/**
 * TCP Receive 配置表单 Schema
 */
export const tcpReceiveConfigSchema = z.object({
  type: z.literal('tcp_receive'),
  connection_id: z.string().min(1, '请选择连接'),
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
  connection_id: z.string().min(1, '请选择连接'),
  data: z.string(),
  data_encoding: z.enum(['text', 'hex', 'base64']).optional(),
  target_addr: z.string().min(1, '请输入目标地址'),
  target_port: z.number().int().min(1).max(65535, '端口范围 1-65535'),
  send_timeout_ms: z.number().int().positive().optional(),
});

export type UdpSendToConfigFormData = z.infer<typeof udpSendToConfigSchema>;

/**
 * UDP RecvFrom 配置表单 Schema
 */
export const udpRecvFromConfigSchema = z.object({
  type: z.literal('udp_recv_from'),
  connection_id: z.string().min(1, '请选择连接'),
  receive_timeout_ms: z.number().int().positive().optional(),
  max_size: z.number().int().positive().optional(),
});

export type UdpRecvFromConfigFormData = z.infer<typeof udpRecvFromConfigSchema>;

/**
 * UDP Close 配置表单 Schema
 */
export const udpCloseConfigSchema = z.object({
  type: z.literal('udp_close'),
  connection_id: z.string().min(1, '请选择连接'),
});

export type UdpCloseConfigFormData = z.infer<typeof udpCloseConfigSchema>;

// ============================================================================
// MQTT 配置 Schema
// ============================================================================

/**
 * MQTT 协议版本枚举
 */
export const mqttVersionSchema = z.enum(['3.1.1', '5.0']);
export type MqttVersionFormData = z.infer<typeof mqttVersionSchema>;

/**
 * MQTT QoS 等级枚举
 */
export const mqttQosSchema = z.union([z.literal(0), z.literal(1), z.literal(2)]);
export type MqttQoSFormData = z.infer<typeof mqttQosSchema>;

/**
 * MQTT 载荷格式枚举
 */
export const mqttPayloadFormatSchema = z.enum(['text', 'json', 'hex', 'base64']);
export type MqttPayloadFormatFormData = z.infer<typeof mqttPayloadFormatSchema>;

/**
 * MQTT 保留消息处理策略枚举
 */
export const mqttRetainHandlingSchema = z.union([z.literal(0), z.literal(1), z.literal(2)]);
export type MqttRetainHandlingFormData = z.infer<typeof mqttRetainHandlingSchema>;

/**
 * MQTT 输出模式枚举
 */
export const mqttOutputModeSchema = z.enum(['first', 'all']);
export type MqttOutputModeFormData = z.infer<typeof mqttOutputModeSchema>;

/**
 * MQTT Connect 配置表单 Schema
 */
export const mqttConnectConfigSchema = z.object({
  type: z.literal('mqtt_connect'),
  url: z.string().optional(),
  client_id: z.string().optional(),
  username: z.string().optional(),
  password: z.string().optional(),
  mqtt_version: mqttVersionSchema.optional(),
  clean_session: z.boolean().optional(),
  keep_alive_secs: z.number().int().positive().optional(),
  connect_timeout_ms: z.number().int().positive().optional(),
  will_topic: z.string().optional(),
  will_payload: z.string().optional(),
  will_qos: z.number().int().min(0).max(2).optional(),
  will_retain: z.boolean().optional(),
  use_tls: z.boolean().optional(),
  tls_skip_verify: z.boolean().optional(),
  session_expiry_interval: z.number().int().positive().optional(),
  will_delay_interval: z.number().int().positive().optional(),
  receive_maximum: z.number().int().positive().optional(),
  topic_alias_maximum: z.number().int().positive().optional(),
  max_packet_size: z.number().int().positive().optional(),
  user_properties: z.array(z.tuple([z.string(), z.string()])).optional(),
});

export type MqttConnectConfigFormData = z.infer<typeof mqttConnectConfigSchema>;

/**
 * MQTT Publish 配置表单 Schema
 */
export const mqttPublishConfigSchema = z.object({
  type: z.literal('mqtt_publish'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  topic: z.string().min(1, '主题不能为空'),
  payload: z.string().optional(),
  qos: z.number().int().min(0).max(2).optional(),
  retain: z.boolean().optional(),
  payload_format: mqttPayloadFormatSchema.optional(),
  content_type: z.string().optional(),
  message_expiry_interval: z.number().int().positive().optional(),
  response_topic: z.string().optional(),
  correlation_data: z.string().optional(),
  topic_alias: z.number().int().positive().optional(),
  user_properties: z.array(z.tuple([z.string(), z.string()])).optional(),
});

export type MqttPublishConfigFormData = z.infer<typeof mqttPublishConfigSchema>;

/**
 * MQTT Subscribe 配置表单 Schema
 */
export const mqttSubscribeConfigSchema = z.object({
  type: z.literal('mqtt_subscribe'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  topic: z.string().min(1, '主题不能为空'),
  qos: z.number().int().min(0).max(2).optional(),
  max_messages: z.number().int().positive().optional(),
  collect_timeout_ms: z.number().int().positive().optional(),
  output_mode: mqttOutputModeSchema.optional(),
  no_local: z.boolean().optional(),
  retain_as_published: z.boolean().optional(),
  retain_handling: mqttRetainHandlingSchema.optional(),
  subscription_identifier: z.number().int().positive().optional(),
  share_group: z.string().optional(),
});

export type MqttSubscribeConfigFormData = z.infer<typeof mqttSubscribeConfigSchema>;

/**
 * MQTT Unsubscribe 配置表单 Schema
 */
export const mqttUnsubscribeConfigSchema = z.object({
  type: z.literal('mqtt_unsubscribe'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  topic: z.string().min(1, '主题不能为空'),
});

export type MqttUnsubscribeConfigFormData = z.infer<typeof mqttUnsubscribeConfigSchema>;

/**
 * MQTT Disconnect 配置表单 Schema
 */
export const mqttDisconnectConfigSchema = z.object({
  type: z.literal('mqtt_disconnect'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  reason_code: z.number().int().optional(),
  session_expiry_interval: z.number().int().positive().optional(),
});

export type MqttDisconnectConfigFormData = z.infer<typeof mqttDisconnectConfigSchema>;

/**
 * MQTT WaitMessage 配置表单 Schema
 */
export const mqttWaitMessageConfigSchema = z.object({
  type: z.literal('mqtt_wait_message'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  topic: z.string().min(1, '主题不能为空'),
  match_condition: z.any().optional(),
  timeout_ms: z.number().int().positive().optional(),
  max_messages: z.number().int().positive().optional(),
});

export type MqttWaitMessageConfigFormData = z.infer<typeof mqttWaitMessageConfigSchema>;

/**
 * MQTT Request 配置表单 Schema
 */
export const mqttRequestConfigSchema = z.object({
  type: z.literal('mqtt_request'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  request_topic: z.string().min(1, '请求主题不能为空'),
  response_topic: z.string().min(1, '响应主题不能为空'),
  payload: z.string().optional(),
  qos: z.number().int().min(0).max(2).optional(),
  correlation_data: z.string().optional(),
  timeout_ms: z.number().int().positive().optional(),
});

export type MqttRequestConfigFormData = z.infer<typeof mqttRequestConfigSchema>;

// ============================================================================
// SSE 配置 Schema
// ============================================================================

/**
 * SSE Connect 配置表单 Schema
 */
export const sseConnectConfigSchema = z.object({
  type: z.literal('sse_connect'),
  url: z.string().min(1, 'URL 不能为空'),
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
  connection_id: z.string().min(1, '连接 ID 不能为空'),
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
  connection_id: z.string().min(1, '连接 ID 不能为空'),
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
  endpoint: z.string().min(1, '端点 URL 不能为空'),
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
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  query: z.string().min(1, '查询不能为空'),
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
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  subscription: z.string().min(1, '订阅不能为空'),
  variables: z.string().optional(),
  operation_name: z.string().optional(),
  max_events: z.number().min(0).optional(),
  timeout_ms: z.number().min(0).optional(),
});

export type GraphqlSubscribeConfigFormData = z.infer<typeof graphqlSubscribeConfigSchema>;

/**
 * GraphQL Unsubscribe 配置表单 Schema
 */
export const graphqlUnsubscribeConfigSchema = z.object({
  type: z.literal('graphql_unsubscribe'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  subscription_id: z.string().min(1, '订阅 ID 不能为空'),
});

export type GraphqlUnsubscribeConfigFormData = z.infer<typeof graphqlUnsubscribeConfigSchema>;

/**
 * GraphQL Introspect 配置表单 Schema
 */
export const graphqlIntrospectConfigSchema = z.object({
  type: z.literal('graphql_introspect'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  timeout_ms: z.number().min(1000).max(300000).optional(),
});

export type GraphqlIntrospectConfigFormData = z.infer<typeof graphqlIntrospectConfigSchema>;

/**
 * GraphQL Disconnect 配置表单 Schema
 */
export const graphqlDisconnectConfigSchema = z.object({
  type: z.literal('graphql_disconnect'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
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
}, { message: '变量名不能重复', path: ['output_mappings'] });

export type VariableExtractorConfigFormData = z.infer<typeof variableExtractorConfigSchema>;

// ============================================================================
// AMQP 0-9-1 配置 Schema
// ============================================================================

/**
 * AMQP TLS 模式枚举
 */
export const amqpTlsModeSchema = z.enum(['none', 'standard', 'skip-verify']);
export type AmqpTlsModeFormData = z.infer<typeof amqpTlsModeSchema>;

/**
 * AMQP 认证机制枚举
 */
export const amqpAuthMechanismSchema = z.enum(['PLAIN', 'AMQPLAIN', 'EXTERNAL', 'RABBIT-CR-DEMO']);
export type AmqpAuthMechanismFormData = z.infer<typeof amqpAuthMechanismSchema>;

/**
 * AMQP Exchange 类型枚举
 */
export const amqpExchangeTypeSchema = z.enum(['direct', 'fanout', 'topic', 'headers', 'custom']);
export type AmqpExchangeTypeFormData = z.infer<typeof amqpExchangeTypeSchema>;

/**
 * AMQP Body 类型枚举
 */
export const amqpBodyTypeSchema = z.enum(['text', 'json', 'binary']);
export type AmqpBodyTypeFormData = z.infer<typeof amqpBodyTypeSchema>;

/**
 * AMQP 确认模式枚举
 */
export const amqpConfirmModeSchema = z.enum(['none', 'publisher-confirms', 'transaction']);
export type AmqpConfirmModeFormData = z.infer<typeof amqpConfirmModeSchema>;

/**
 * AMQP 消费确认模式枚举
 */
export const amqpAckModeSchema = z.enum(['auto', 'manual']);
export type AmqpAckModeFormData = z.infer<typeof amqpAckModeSchema>;

/**
 * AMQP Queue 操作枚举
 */
export const amqpQueueActionSchema = z.enum(['delete', 'purge']);
export type AmqpQueueActionFormData = z.infer<typeof amqpQueueActionSchema>;

/**
 * AMQP Ack 操作枚举
 */
export const amqpAckActionSchema = z.enum(['ack', 'nack', 'reject']);
export type AmqpAckActionFormData = z.infer<typeof amqpAckActionSchema>;

/**
 * AMQP Management 操作枚举
 */
export const amqpManagementActionSchema = z.enum([
  'overview',
  'list-queues',
  'list-connections',
  'list-exchanges',
  'list-channels',
]);
export type AmqpManagementActionFormData = z.infer<typeof amqpManagementActionSchema>;

/**
 * AMQP Connect 配置表单 Schema
 */
export const amqpConnectConfigSchema = z.object({
  type: z.literal('amqp_connect'),
  url: z.string().optional(),
  connection_id: z.string().optional(),
  heartbeat: z.number().int().positive().optional(),
  channel_max: z.number().int().positive().optional(),
  frame_max: z.number().int().positive().optional(),
  connection_timeout_ms: z.number().int().positive().optional(),
  tls_mode: amqpTlsModeSchema.optional(),
  auth_mechanism: amqpAuthMechanismSchema.optional(),
  auto_recover: z.boolean().optional(),
  reconnect_max_retries: z.number().int().positive().optional(),
});

export type AmqpConnectConfigFormData = z.infer<typeof amqpConnectConfigSchema>;

/**
 * AMQP Close 配置表单 Schema
 */
export const amqpCloseConfigSchema = z.object({
  type: z.literal('amqp_close'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  reply_code: z.number().int().optional(),
  reply_text: z.string().optional(),
});

export type AmqpCloseConfigFormData = z.infer<typeof amqpCloseConfigSchema>;

/**
 * AMQP Exchange Declare 配置表单 Schema
 */
export const amqpExchangeDeclareConfigSchema = z.object({
  type: z.literal('amqp_exchange_declare'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  exchange: z.string().min(1, 'Exchange 名称不能为空'),
  exchange_type: amqpExchangeTypeSchema,
  custom_type: z.string().optional(),
  passive: z.boolean().optional(),
  durable: z.boolean().optional(),
  auto_delete: z.boolean().optional(),
  internal: z.boolean().optional(),
  arguments: z.string().optional(),
});

export type AmqpExchangeDeclareConfigFormData = z.infer<typeof amqpExchangeDeclareConfigSchema>;

/**
 * AMQP Exchange Delete 配置表单 Schema
 */
export const amqpExchangeDeleteConfigSchema = z.object({
  type: z.literal('amqp_exchange_delete'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  exchange: z.string().min(1, 'Exchange 名称不能为空'),
  if_unused: z.boolean().optional(),
});

export type AmqpExchangeDeleteConfigFormData = z.infer<typeof amqpExchangeDeleteConfigSchema>;

/**
 * AMQP Queue Declare 配置表单 Schema
 */
export const amqpQueueDeclareConfigSchema = z.object({
  type: z.literal('amqp_queue_declare'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  queue: z.string().optional(),
  passive: z.boolean().optional(),
  durable: z.boolean().optional(),
  exclusive: z.boolean().optional(),
  auto_delete: z.boolean().optional(),
  arguments: z.string().optional(),
});

export type AmqpQueueDeclareConfigFormData = z.infer<typeof amqpQueueDeclareConfigSchema>;

/**
 * AMQP Queue Delete 配置表单 Schema
 */
export const amqpQueueDeleteConfigSchema = z.object({
  type: z.literal('amqp_queue_delete'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  queue: z.string().min(1, '队列名称不能为空'),
  action: amqpQueueActionSchema.optional(),
  if_unused: z.boolean().optional(),
  if_empty: z.boolean().optional(),
});

export type AmqpQueueDeleteConfigFormData = z.infer<typeof amqpQueueDeleteConfigSchema>;

/**
 * AMQP Queue Bind 配置表单 Schema
 */
export const amqpQueueBindConfigSchema = z.object({
  type: z.literal('amqp_queue_bind'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  queue: z.string().min(1, '队列名称不能为空'),
  exchange: z.string().min(1, 'Exchange 名称不能为空'),
  routing_key: z.string().optional(),
  arguments: z.string().optional(),
});

export type AmqpQueueBindConfigFormData = z.infer<typeof amqpQueueBindConfigSchema>;

/**
 * AMQP Publish 配置表单 Schema
 */
export const amqpPublishConfigSchema = z.object({
  type: z.literal('amqp_publish'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  exchange: z.string().optional(),
  routing_key: z.string().optional(),
  body: z.string().optional(),
  body_type: amqpBodyTypeSchema.optional(),
  content_type: z.string().optional(),
  content_encoding: z.string().optional(),
  delivery_mode: z.number().int().min(1).max(2).optional(),
  priority: z.number().int().min(0).max(9).optional(),
  message_id: z.string().optional(),
  correlation_id: z.string().optional(),
  reply_to: z.string().optional(),
  expiration: z.string().optional(),
  timestamp: z.number().int().positive().optional(),
  message_type: z.string().optional(),
  user_id: z.string().optional(),
  app_id: z.string().optional(),
  cluster_id: z.string().optional(),
  headers: z.string().optional(),
  mandatory: z.boolean().optional(),
  confirm_mode: amqpConfirmModeSchema.optional(),
});

export type AmqpPublishConfigFormData = z.infer<typeof amqpPublishConfigSchema>;

/**
 * AMQP Consume 配置表单 Schema
 */
export const amqpConsumeConfigSchema = z.object({
  type: z.literal('amqp_consume'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  queue: z.string().min(1, '队列名称不能为空'),
  consumer_tag: z.string().optional(),
  ack_mode: amqpAckModeSchema.optional(),
  exclusive: z.boolean().optional(),
  no_local: z.boolean().optional(),
  prefetch_count: z.number().int().positive().optional(),
  max_messages: z.number().int().positive().optional(),
  timeout_ms: z.number().int().positive().optional(),
  auto_ack_on_success: z.boolean().optional(),
  auto_reject_on_failure: z.boolean().optional(),
  reject_requeue: z.boolean().optional(),
});

export type AmqpConsumeConfigFormData = z.infer<typeof amqpConsumeConfigSchema>;

/**
 * AMQP Get 配置表单 Schema
 */
export const amqpGetConfigSchema = z.object({
  type: z.literal('amqp_get'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  queue: z.string().min(1, '队列名称不能为空'),
  no_ack: z.boolean().optional(),
});

export type AmqpGetConfigFormData = z.infer<typeof amqpGetConfigSchema>;

/**
 * AMQP Ack 配置表单 Schema
 */
export const amqpAckConfigSchema = z.object({
  type: z.literal('amqp_ack'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  action: amqpAckActionSchema,
  delivery_tag: z.number().int().positive(),
  multiple: z.boolean().optional(),
  requeue: z.boolean().optional(),
});

export type AmqpAckConfigFormData = z.infer<typeof amqpAckConfigSchema>;

/**
 * AMQP Management 配置表单 Schema
 */
export const amqpManagementConfigSchema = z.object({
  type: z.literal('amqp_management'),
  management_url: z.string().optional(),
  username: z.string().optional(),
  password: z.string().optional(),
  action: amqpManagementActionSchema,
  vhost: z.string().optional(),
  queue_name: z.string().optional(),
  connection_id: z.string().optional(),
});

export type AmqpManagementConfigFormData = z.infer<typeof amqpManagementConfigSchema>;

// ============================================================================
// AMQP 1.0 配置 Schema
// ============================================================================

/**
 * AMQP 1.0 认证机制枚举
 */
export const amqp10AuthMechanismSchema = z.enum(['PLAIN', 'ANONYMOUS', 'EXTERNAL']);
export type Amqp10AuthMechanismFormData = z.infer<typeof amqp10AuthMechanismSchema>;

/**
 * AMQP 1.0 消息属性 Schema
 */
export const amqp10MessagePropertiesSchema = z.object({
  content_type: z.string().optional(),
  content_encoding: z.string().optional(),
  correlation_id: z.string().optional(),
  message_id: z.string().optional(),
  reply_to: z.string().optional(),
  subject: z.string().optional(),
  to: z.string().optional(),
  absolute_expiry_time: z.number().int().positive().optional(),
  creation_time: z.number().int().positive().optional(),
});

export type Amqp10MessagePropertiesFormData = z.infer<typeof amqp10MessagePropertiesSchema>;

/**
 * AMQP 1.0 Connect 配置表单 Schema
 */
export const amqp10ConnectConfigSchema = z.object({
  type: z.literal('amqp10_connect'),
  url: z.string().optional(),
  connection_id: z.string().optional(),
  tls_mode: amqpTlsModeSchema.optional(),
  auth_mechanism: amqp10AuthMechanismSchema.optional(),
  container_id: z.string().optional(),
});

export type Amqp10ConnectConfigFormData = z.infer<typeof amqp10ConnectConfigSchema>;

/**
 * AMQP 1.0 Send 配置表单 Schema
 */
export const amqp10SendConfigSchema = z.object({
  type: z.literal('amqp10_send'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  address: z.string().min(1, '目标地址不能为空'),
  message_body: z.string().optional(),
  message_properties: amqp10MessagePropertiesSchema.optional(),
  message_annotations: z.string().optional(),
});

export type Amqp10SendConfigFormData = z.infer<typeof amqp10SendConfigSchema>;

/**
 * AMQP 1.0 Receive 配置表单 Schema
 */
export const amqp10ReceiveConfigSchema = z.object({
  type: z.literal('amqp10_receive'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  address: z.string().min(1, '源地址不能为空'),
  credit: z.number().int().positive().optional(),
  max_messages: z.number().int().positive().optional(),
  timeout_ms: z.number().int().positive().optional(),
  auto_accept: z.boolean().optional(),
});

export type Amqp10ReceiveConfigFormData = z.infer<typeof amqp10ReceiveConfigSchema>;

/**
 * AMQP 1.0 Accept 配置表单 Schema
 */
export const amqp10AcceptConfigSchema = z.object({
  type: z.literal('amqp10_accept'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  delivery_id: z.number().int().positive(),
});

export type Amqp10AcceptConfigFormData = z.infer<typeof amqp10AcceptConfigSchema>;

/**
 * AMQP 1.0 Reject 配置表单 Schema
 */
export const amqp10RejectConfigSchema = z.object({
  type: z.literal('amqp10_reject'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  delivery_id: z.number().int().positive(),
  error_condition: z.string().optional(),
  error_description: z.string().optional(),
});

export type Amqp10RejectConfigFormData = z.infer<typeof amqp10RejectConfigSchema>;

/**
 * AMQP 1.0 Disconnect 配置表单 Schema
 */
export const amqp10DisconnectConfigSchema = z.object({
  type: z.literal('amqp10_disconnect'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
});

export type Amqp10DisconnectConfigFormData = z.infer<typeof amqp10DisconnectConfigSchema>;

// ============================================================================
// SMTP 配置 Schema
// ============================================================================

/**
 * SMTP Connect 配置表单 Schema
 */
export const smtpConnectConfigSchema = z.object({
  type: z.literal('smtp_connect'),
  connection_id: z.string().optional(),
  host: z.string().min(1, '服务器地址不能为空'),
  port: z.number().int().min(1).max(65535),
  tls_mode: z.enum(['none', 'starttls', 'implicit']),
  auth_type: z.enum(['none', 'plain', 'login', 'xoauth2']).optional(),
  username: z.string().optional(),
  password: z.string().optional(),
  oauth2_access_token: z.string().optional(),
  oauth2_refresh_token: z.string().optional(),
  oauth2_expires_at: z.string().optional(),
  oauth2_provider: z.string().optional(),
});

export type SmtpConnectConfigFormData = z.infer<typeof smtpConnectConfigSchema>;

/**
 * SMTP Send 配置表单 Schema
 */
export const smtpSendConfigSchema = z.object({
  type: z.literal('smtp_send'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  from: z.string().min(1, '发件人不能为空'),
  to: z.string(), // 逗号分隔的邮箱地址，解析为数组
  cc: z.string().optional(),
  bcc: z.string().optional(),
  subject: z.string().min(1, '主题不能为空'),
  body: z.string(),
  body_type: z.enum(['text', 'html']),
  reply_to: z.string().optional(),
  attachments_json: z.string().optional(), // JSON 字符串
  headers_json: z.string().optional(), // JSON 字符串
});

export type SmtpSendConfigFormData = z.infer<typeof smtpSendConfigSchema>;

/**
 * SMTP Disconnect 配置表单 Schema
 */
export const smtpDisconnectConfigSchema = z.object({
  type: z.literal('smtp_disconnect'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
});

export type SmtpDisconnectConfigFormData = z.infer<typeof smtpDisconnectConfigSchema>;

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
