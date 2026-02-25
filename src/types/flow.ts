/**
 * 流程类型定义
 * 与后端 src-tauri/src/flow.rs 中的 Flow 结构体对齐
 */

import type { Node } from './node';
import type { Connection } from './connection';

/**
 * 流程数据结构
 */
export interface Flow {
  /** 流程唯一标识符（前端生成） */
  uuid: string;
  /** 流程名称 */
  name: string;
  /** 流程级别的初始变量 */
  initial_variables?: Record<string, unknown>;
  /** 流程中的节点列表 */
  nodes: Node[];
  /** 流程中的连接列表 */
  connections: Connection[];
}
