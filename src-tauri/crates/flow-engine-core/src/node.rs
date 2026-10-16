// 节点模块 - 完整结构体实现

use crate::node_config::NodeData;

/// 节点位置结构体
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Port {
    /// 端口唯一标识符
    pub id: String,
    /// 端口类型
    #[serde(rename = "type")]
    pub port_type: PortType,
    /// 端口标签（可选）
    pub label: Option<String>,
}

/// 执行Error策略
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionErrorStrategy {
    /// 路由到 false 端口（默认行为）
    #[default]
    RouteToFalse,
    /// 忽略Error，继续执行（如同Success，路由到 true 端口）
    Ignore,
    /// 中止整个流程
    AbortFlow,
    /// 临时旁路/禁用节点（不执行网络动作，输入直接透传到输出）
    Bypass,
}

/// 节点数据结构
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Node {
    /// 节点唯一标识符（前端生成UUID，用于连接关联）
    pub uuid: String,
    /// 节点类型（字符串形式，使用 `node_types` 模块中的常量进行比较）
    #[serde(rename = "type")]
    pub node_type: String,
    /// 节点名称
    pub name: String,
    /// 节点位置
    pub position: NodePosition,
    /// 节点特定数据（强类型配置）
    pub data: NodeData,
    /// 端口数组（可选）
    pub ports: Option<Vec<Port>>,
    /// 是否设置断点（可选，默认 false）
    #[serde(rename = "hasBreakpoint", default, skip_serializing_if = "Option::is_none")]
    pub has_breakpoint: Option<bool>,
    /// 执行Error策略（可选，默认 `RouteToFalse`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_error: Option<ExecutionErrorStrategy>,
}

impl<'de> serde::Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[allow(non_snake_case)]
        struct NodeHelper {
            pub uuid: String,
            #[serde(rename = "type")]
            pub node_type: String,
            pub name: String,
            #[serde(default)]
            pub position: NodePosition,
            pub data: NodeData,
            #[serde(default)]
            pub ports: Option<Vec<Port>>,
            #[serde(default)]
            pub has_breakpoint: Option<bool>,
            #[serde(default)]
            pub hasBreakpoint: Option<bool>,
            #[serde(default)]
            pub on_error: Option<ExecutionErrorStrategy>,
            #[serde(default)]
            pub disabled: Option<bool>,
        }

        let helper = NodeHelper::deserialize(deserializer)?;
        let has_breakpoint = helper.hasBreakpoint.or(helper.has_breakpoint);
        let on_error = if helper.disabled == Some(true) {
            Some(ExecutionErrorStrategy::Bypass)
        } else {
            helper.on_error
        };

        Ok(Node {
            uuid: helper.uuid,
            node_type: helper.node_type,
            name: helper.name,
            position: helper.position,
            data: helper.data,
            ports: helper.ports,
            has_breakpoint,
            on_error,
        })
    }
}

impl Default for Node {
    fn default() -> Self {
        Self {
            uuid: String::new(),
            node_type: crate::node_types::ENTRY.to_string(),
            name: String::new(),
            position: NodePosition::default(),
            data: NodeData::default(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }
}

impl Node {
    /// 节点是否被临时旁路/禁用
    pub fn is_bypassed(&self) -> bool {
        self.on_error == Some(ExecutionErrorStrategy::Bypass)
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
            node_type: "entry".to_string(),
            name: "Entry Node".to_string(),
            position: NodePosition { x: 100.0, y: 200.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
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
            node_type: "http:request".to_string(),
            name: "HTTP Request".to_string(),
            position: NodePosition { x: 150.0, y: 250.0 },
            data: NodeData::http("https://api.example.com"),
            ports: None,
            has_breakpoint: Some(true),
            on_error: None,
        };

        let json = serde_json::to_string(&node).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("node-uuid-456"));
        assert!(json.contains("https://api.example.com"));
        assert!(json.contains("hasBreakpoint"));

        // 反序列化测试
        let deserialized: Node = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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

        let json = serde_json::to_string(&port).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("port-789"));
        assert!(json.contains("input"));

        // 反序列化测试
        let deserialized: Port = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
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
            node_type: "conditional".to_string(),
            name: "Conditional Node".to_string(),
            position: NodePosition { x: 50.0, y: 75.0 },
            data: NodeData::conditional_simple("field", ConditionOperator::Equal, json!(1)),
            ports: Some(ports),
            has_breakpoint: Some(false),
            on_error: None,
        };

        assert_eq!(node.uuid, "node-uuid-999");
        assert!(node.ports.is_some());
        let ports_ref = node
            .ports
            .as_ref()
            .unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(ports_ref.len(), 2);
        assert_eq!(ports_ref[0].id, "port-1");
        assert_eq!(ports_ref[1].id, "port-2");
    }

    #[test]
    fn test_node_has_breakpoint_camel_case_alias() {
        let json_str = r#"{
            "uuid": "node-bp",
            "type": "entry",
            "name": "Entry",
            "position": {"x": 0.0, "y": 0.0},
            "data": {},
            "hasBreakpoint": true
        }"#;
        let node: Node = serde_json::from_str(json_str).expect("deserialize node with hasBreakpoint");
        assert_eq!(node.has_breakpoint, Some(true));
    }

    #[test]
    fn test_node_duplicate_breakpoint_keys_no_conflict() {
        let json_str = r#"{
            "uuid": "node-duplicate",
            "type": "entry",
            "name": "Entry",
            "position": {"x": 0.0, "y": 0.0},
            "data": {},
            "has_breakpoint": null,
            "hasBreakpoint": true
        }"#;
        let node: Node = serde_json::from_str(json_str).expect("deserialize node with both keys");
        assert_eq!(node.has_breakpoint, Some(true));
    }

    #[test]
    fn test_node_serialize_has_breakpoint() {
        let node = Node {
            uuid: "node-1".to_string(),
            node_type: "entry".to_string(),
            name: "Entry".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: Some(true),
            on_error: None,
        };
        let serialized = serde_json::to_string(&node).expect("serialize node");
        assert!(serialized.contains(r#""hasBreakpoint":true"#));
        assert!(!serialized.contains("has_breakpoint"));
    }
}
