// 连接模块 - 连接结构体实现

/// 连接数据结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Connection {
    /// 连接唯一标识符
    pub id: String,
    /// 源节点的 UUID（引用 node.uuid）
    pub source_node_uuid: String,
    /// 目标节点的 UUID（引用 node.uuid）
    pub target_node_uuid: String,
    /// 源端口 ID
    pub source_port_id: String,
    /// 目标端口 ID
    pub target_port_id: String,
}
