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

/**
 * Entry 配置表单 Schema
 */
export const entryConfigSchema = z.object({
  type: z.literal('entry'),
  initial_variables: z.array(initialVariableSchema).optional(),
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
