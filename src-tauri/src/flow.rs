// 流程模块 - 完整结构体实现

use crate::connection::Connection;
use crate::node::Node;

/// 流程数据结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Flow {
    /// 流程唯一标识符
    pub id: String,
    /// 流程名称
    pub name: String,
    /// 流程中的节点列表
    pub nodes: Vec<Node>,
    /// 流程中的连接列表
    pub connections: Vec<Connection>,
}
