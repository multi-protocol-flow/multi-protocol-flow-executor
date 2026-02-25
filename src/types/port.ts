/**
 * 端口类型定义
 * 与后端 src-tauri/src/node.rs 中的 Port 结构体对齐
 */

/**
 * 端口类型枚举
 */
export type PortType = 'input' | 'output';

/**
 * 端口结构体
 */
export interface Port {
  /** 端口唯一标识符 */
  id: string;
  /** 端口类型 */
  type: PortType;
  /** 端口标签（可选） */
  label?: string;
}
