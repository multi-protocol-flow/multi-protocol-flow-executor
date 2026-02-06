// 节点模块 - 完整结构体实现

use crate::node_config::NodeData;

/// 节点类型枚举
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    /// 开始节点
    Start,
    /// HTTP 请求节点
    Http,
    /// 条件分支节点
    Conditional,
    /// 结束节点
    End,
}

/// 节点位置结构体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodePosition {
    /// X 坐标
    pub x: f64,
    /// Y 坐标
    pub y: f64,
}

/// 端口类型枚举
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortType {
    /// 输入端口
    Input,
    /// 输出端口
    Output,
}

/// 端口结构体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Port {
    /// 端口唯一标识符
    pub id: String,
    /// 端口类型
    #[serde(rename = "type")]
    pub port_type: PortType,
    /// 端口标签（可选）
    pub label: Option<String>,
}

/// 节点数据结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    /// 节点唯一标识符（前端生成UUID，用于连接关联）
    pub uuid: String,
    /// 节点类型
    #[serde(rename = "type")]
    pub node_type: NodeType,
    /// 节点名称
    pub name: String,
    /// 节点位置
    pub position: NodePosition,
    /// 节点特定数据（强类型配置）
    pub data: NodeData,
    /// 端口数组（可选）
    pub ports: Option<Vec<Port>>,
}
