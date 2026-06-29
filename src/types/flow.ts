/**
 * 流程类型定义
 * 与后端 src-tauri/crates/flow-engine-core/src/flow.rs 中的 Flow 结构体对齐
 */

import type { Node } from './node';
import type { Connection } from './connection';
import type { FlowHook } from './hook';

/**
 * 环境变量集 — 一组命名变量（如 "dev"、"staging"、"prod"）
 */
export interface EnvironmentSet {
  /** 环境名称（如 "dev"、"staging"、"prod"） */
  name: string;
  /** 环境变量 */
  variables: Record<string, unknown>;
}

/**
 * 流程数据结构
 */
export interface Flow {
  /** 流程唯一标识符（前端生成） */
  uuid: string;
  /** 流程名称 */
  name: string;
  /** 流程描述（Markdown 格式） */
  description?: string;
  /** 流程级别的初始变量 */
  initial_variables?: Record<string, unknown>;
  /** 环境列表（多组命名变量集，如 dev/staging/prod） */
  environments?: EnvironmentSet[];
  /** 当前激活的环境名称 */
  active_environment?: string;
  /** 流程中的节点列表 */
  nodes: Node[];
  /** 流程中的连接列表 */
  connections: Connection[];
  /** 流程钩子列表（流程完成时自动触发） */
  hooks?: FlowHook[];
}

/**
 * 获取当前激活环境的变量
 *
 * 如果设置了 active_environment 且存在匹配的环境，返回该环境的变量；
 * 否则返回 initial_variables。
 */
export function getActiveVariables(flow: Flow): Record<string, unknown> | undefined {
  if (flow.active_environment && flow.environments) {
    const env = flow.environments.find((e) => e.name === flow.active_environment);
    if (env) {
      return env.variables;
    }
  }
  return flow.initial_variables;
}
