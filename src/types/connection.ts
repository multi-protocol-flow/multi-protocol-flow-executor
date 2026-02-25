/**
 * 连接类型定义
 * 与后端 src-tauri/src/connection.rs 中的 Connection 结构体对齐
 */

/**
 * 连接数据结构
 */
export interface Connection {
  /** 连接唯一标识符 */
  id: string;
  /** 源节点的 UUID（引用 node.uuid） */
  source_node_uuid: string;
  /** 目标节点的 UUID（引用 node.uuid） */
  target_node_uuid: string;
  /** 源端口 ID */
  source_port_id: string;
  /** 目标端口 ID */
  target_port_id: string;
}
