/**
 * 节点类型定义
 * 与后端 src-tauri/src/node.rs 和 src-tauri/src/node_config.rs 中的结构体对齐
 */

import type { OutputMapping, InitialVariable } from './variable';

import type { Port } from './port';

// ============================================================================
// 节点类型枚举
// ============================================================================

/**
 * 节点类型枚举
 */
export type NodeType = 'entry' | 'http' | 'conditional' | 'ws_connect' | 'ws_send_wait' | 'ws_close';

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
// WebSocket 类型定义
// ============================================================================

/**
 * WebSocket 等待模式
 */
export type WsWaitMode = 'none' | 'first_message' | 'condition' | 'duration';

/**
 * WebSocket 匹配类型
 */
export type WsMatchType = 'json_path' | 'regex' | 'expression';

/**
 * WebSocket 消息类型
 */
export type WsMessageType = 'text' | 'binary';

/**
 * WebSocket 等待条件
 */
export interface WsWaitCondition {
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
  /** 是否等待消息 */
  wait_for_message?: boolean;
  /** 等待模式 */
  wait_mode?: WsWaitMode;
  /** 等待条件 */
  wait_condition?: WsWaitCondition;
  /** 消息超时（毫秒） */
  message_timeout_ms?: number;
  /** 输出变量映射 */
  output_mappings?: OutputMapping[];
}

/**
 * WebSocket Send+Wait 节点配置
 */
export interface WsSendWaitConfig {
  type: 'ws_send_wait';
  /** 连接 ID */
  connection_id: string;
  /** 消息内容 */
  message: string;
  /** 消息类型 */
  message_type?: WsMessageType;
  /** 发送超时（毫秒） */
  send_timeout_ms?: number;
  /** 是否等待响应 */
  wait_for_response?: boolean;
  /** 等待模式 */
  wait_mode?: WsWaitMode;
  /** 等待条件 */
  wait_condition?: WsWaitCondition;
  /** 消息超时（毫秒） */
  message_timeout_ms?: number;
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
  /** 等待关闭超时（毫秒） */
  wait_close_timeout_ms?: number;
}

/**
 * 节点数据配置（判别联合类型）
 */
export type NodeData = EntryConfig | HttpConfig | ConditionalConfig | WsConnectConfig | WsSendWaitConfig | WsCloseConfig;

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
 * 判断是否为 WsSendWaitConfig
 */
export function isWsSendWaitConfig(data: NodeData): data is WsSendWaitConfig {
  return data.type === 'ws_send_wait';
}

/**
 * 判断是否为 WsCloseConfig
 */
export function isWsCloseConfig(data: NodeData): data is WsCloseConfig {
  return data.type === 'ws_close';
}
