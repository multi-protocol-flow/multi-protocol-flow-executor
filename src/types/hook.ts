/**
 * 钩子类型定义
 * 与后端 src-tauri/crates/flow-engine-core/src/hook.rs 中的类型严格对齐
 */

/** 钩子触发条件 */
export type HookTrigger = 'on_success' | 'on_failure' | 'on_cancel' | 'on_complete';

/** Webhook 动作配置 */
export interface WebhookHookAction {
  /** 动作类型标识 */
  type: 'webhook';
  /** 请求 URL */
  url: string;
  /** HTTP 方法 */
  method: string;
  /** 请求头 */
  headers: Record<string, string>;
  /** 请求体模板，支持 {{variable}} 占位符 */
  body_template?: string;
  /** 超时时间（毫秒） */
  timeout_ms?: number;
}

/** 脚本动作配置 */
export interface ScriptHookAction {
  /** 动作类型标识 */
  type: 'script';
  /** 脚本代码（QuickJS 运行时） */
  code: string;
  /** 超时时间（毫秒） */
  timeout_ms?: number;
}

/** 通知动作配置 */
export interface NotifyHookAction {
  /** 动作类型标识 */
  type: 'notify';
  /** 通知标题模板 */
  title_template: string;
  /** 通知消息模板 */
  message_template: string;
}

/** 钩子动作（判别联合，通过 type 字段区分） */
export type HookAction = WebhookHookAction | ScriptHookAction | NotifyHookAction;

/** 单个钩子定义 */
export interface FlowHook {
  /** 钩子唯一标识 */
  uuid: string;
  /** 钩子名称 */
  name: string;
  /** 是否启用 */
  enabled: boolean;
  /** 触发条件 */
  trigger: HookTrigger;
  /** 执行动作 */
  action: HookAction;
}
