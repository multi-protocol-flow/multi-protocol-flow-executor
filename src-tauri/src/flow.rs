// 流程模块 - 完整结构体实现

use crate::connection::Connection;
use crate::node::Node;
use std::collections::HashMap;

/// 流程数据结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Flow {
    /// 流程唯一标识符（前端生成）
    pub uuid: String,
    /// 流程名称
    pub name: String,
    /// 流程级别的初始变量
    #[serde(default)]
    pub initial_variables: Option<HashMap<String, serde_json::Value>>,
    /// 流程中的节点列表
    pub nodes: Vec<Node>,
    /// 流程中的连接列表
    pub connections: Vec<Connection>,
}
