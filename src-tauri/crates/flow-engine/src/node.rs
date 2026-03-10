// 节点模块 - 完整结构体实现

use crate::node_config::NodeData;

/// 节点类型枚举
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    /// 入口节点
    Entry,
    /// HTTP 请求节点
    Http,
    /// 条件分支节点
    Conditional,
    /// WebSocket 连接节点
    #[serde(rename = "ws_connect")]
    WsConnect,
    /// WebSocket 发送等待节点
    #[serde(rename = "ws_send_wait")]
    WsSendWait,
    /// WebSocket 关闭节点
    #[serde(rename = "ws_close")]
    WsClose,
}

/// 节点位置结构体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodePosition {
    /// X 坐标
    pub x: f64,
    /// Y 坐标
    pub y: f64,
}

impl Default for NodePosition {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// 端口类型枚举
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// 是否设置断点（可选，默认 false）
    #[serde(default)]
    pub has_breakpoint: Option<bool>,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            uuid: String::new(),
            node_type: NodeType::Entry,
            name: String::new(),
            position: NodePosition::default(),
            data: NodeData::default(),
            ports: None,
            has_breakpoint: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_config::{ConditionOperator, NodeData};
    use serde_json::json;

    #[test]
    fn test_node_creation() {
        let node = Node {
            uuid: "node-uuid-123".to_string(),
            node_type: NodeType::Entry,
            name: "Entry Node".to_string(),
            position: NodePosition { x: 100.0, y: 200.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        };

        assert_eq!(node.uuid, "node-uuid-123");
        assert_eq!(node.name, "Entry Node");
        assert_eq!(node.position.x, 100.0);
        assert_eq!(node.position.y, 200.0);
        assert_eq!(node.has_breakpoint, None);
    }

    #[test]
    fn test_node_serialization() {
        let node = Node {
            uuid: "node-uuid-456".to_string(),
            node_type: NodeType::Http,
            name: "HTTP Request".to_string(),
            position: NodePosition { x: 150.0, y: 250.0 },
            data: NodeData::http("https://api.example.com"),
            ports: None,
            has_breakpoint: Some(true),
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("node-uuid-456"));
        assert!(json.contains("https://api.example.com"));
        assert!(json.contains("has_breakpoint"));

        // 反序列化测试
        let deserialized: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uuid, "node-uuid-456");
        assert_eq!(deserialized.name, "HTTP Request");
        assert_eq!(deserialized.has_breakpoint, Some(true));
    }

    #[test]
    fn test_port_creation() {
        let port = Port {
            id: "port-123".to_string(),
            port_type: PortType::Input,
            label: Some("Input Port".to_string()),
        };

        assert_eq!(port.id, "port-123");
        assert_eq!(port.port_type, PortType::Input);
        assert_eq!(port.label, Some("Input Port".to_string()));
    }

    #[test]
    fn test_port_id_field_unchanged() {
        // 确认 Port 结构保持 id 字段，没有改为 uuid
        let port = Port {
            id: "port-id-unchanged".to_string(),
            port_type: PortType::Output,
            label: None,
        };

        assert_eq!(port.id, "port-id-unchanged");
    }

    #[test]
    fn test_port_serialization() {
        let port = Port {
            id: "port-789".to_string(),
            port_type: PortType::Input,
            label: Some("Test Port".to_string()),
        };

        let json = serde_json::to_string(&port).unwrap();
        assert!(json.contains("port-789"));
        assert!(json.contains("input"));

        // 反序列化测试
        let deserialized: Port = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "port-789");
        assert_eq!(deserialized.port_type, PortType::Input);
    }

    #[test]
    fn test_node_with_ports() {
        let ports = vec![
            Port {
                id: "port-1".to_string(),
                port_type: PortType::Input,
                label: Some("Input".to_string()),
            },
            Port {
                id: "port-2".to_string(),
                port_type: PortType::Output,
                label: Some("Output".to_string()),
            },
        ];

        let node = Node {
            uuid: "node-uuid-999".to_string(),
            node_type: NodeType::Conditional,
            name: "Conditional Node".to_string(),
            position: NodePosition { x: 50.0, y: 75.0 },
            data: NodeData::conditional_simple("field", ConditionOperator::Equal, json!(1)),
            ports: Some(ports),
            has_breakpoint: Some(false),
        };

        assert_eq!(node.uuid, "node-uuid-999");
        assert!(node.ports.is_some());
        let ports_ref = node.ports.as_ref().unwrap();
        assert_eq!(ports_ref.len(), 2);
        assert_eq!(ports_ref[0].id, "port-1");
        assert_eq!(ports_ref[1].id, "port-2");
    }

    #[test]
    fn test_node_type_enum() {
        assert_eq!(
            serde_json::to_string(&NodeType::Entry).unwrap(),
            r#""entry""#
        );
        assert_eq!(serde_json::to_string(&NodeType::Http).unwrap(), r#""http""#);
        assert_eq!(
            serde_json::to_string(&NodeType::Conditional).unwrap(),
            r#""conditional""#
        );
    }
}
