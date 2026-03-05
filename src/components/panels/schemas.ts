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
 * WebSocket 等待模式枚举
 */
export const wsWaitModeSchema = z.enum(['none', 'first_message', 'condition', 'duration']);
export type WsWaitMode = z.infer<typeof wsWaitModeSchema>;

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
 * WebSocket 等待条件 Schema
 */
export const wsWaitConditionSchema = z.object({
  match_type: wsMatchTypeSchema,
  expression: z.string().min(1, '匹配表达式不能为空'),
});

export type WsWaitConditionFormData = z.infer<typeof wsWaitConditionSchema>;

/**
 * WebSocket Connect 配置表单 Schema
 */
export const wsConnectConfigSchema = z.object({
  type: z.literal('ws_connect'),
  url: z.string().min(1, 'URL 不能为空'),
  headers: z.array(formFieldSchema).optional(),
  subprotocols: z.array(z.string()).optional(),
  connect_timeout_ms: z.number().min(1000).max(300000).optional(),
  wait_for_message: z.boolean().optional(),
  wait_mode: wsWaitModeSchema.optional(),
  wait_condition: wsWaitConditionSchema.optional(),
  message_timeout_ms: z.number().min(1000).max(300000).optional(),
  output_mappings: z.array(outputMappingSchema).optional(),
});

export type WsConnectConfigFormData = z.infer<typeof wsConnectConfigSchema>;

/**
 * WebSocket Send+Wait 配置表单 Schema
 */
export const wsSendWaitConfigSchema = z.object({
  type: z.literal('ws_send_wait'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  message: z.string().min(1, '消息内容不能为空'),
  message_type: wsMessageTypeSchema.optional(),
  send_timeout_ms: z.number().min(1000).max(300000).optional(),
  wait_for_response: z.boolean().optional(),
  wait_mode: wsWaitModeSchema.optional(),
  wait_condition: wsWaitConditionSchema.optional(),
  message_timeout_ms: z.number().min(1000).max(300000).optional(),
  output_mappings: z.array(outputMappingSchema).optional(),
});

export type WsSendWaitConfigFormData = z.infer<typeof wsSendWaitConfigSchema>;

/**
 * WebSocket Close 配置表单 Schema
 */
export const wsCloseConfigSchema = z.object({
  type: z.literal('ws_close'),
  connection_id: z.string().min(1, '连接 ID 不能为空'),
  close_code: z.number().optional(),
  close_reason: z.string().optional(),
  wait_close_timeout_ms: z.number().min(1000).max(300000).optional(),
});

export type WsCloseConfigFormData = z.infer<typeof wsCloseConfigSchema>;
