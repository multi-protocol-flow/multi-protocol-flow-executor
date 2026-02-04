// 流程验证器模块 - FlowValidator 实现

use crate::flow::Flow;
use crate::node::{Node, NodeType};

/// 验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 验证是否通过
    pub valid: bool,
    /// 错误信息列表
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// 创建成功的验证结果
    pub fn success() -> Self {
        ValidationResult {
            valid: true,
            errors: Vec::new(),
        }
    }

    /// 创建失败的验证结果
    pub fn error(message: impl Into<String>) -> Self {
        ValidationResult {
            valid: false,
            errors: vec![message.into()],
        }
    }

    /// 创建多个错误的验证结果
    pub fn errors(messages: Vec<String>) -> Self {
        ValidationResult {
            valid: false,
            errors: messages,
        }
    }
}

/// 流程验证器
///
/// 验证流程的基本规则：
/// - 必须有且仅有一个开始节点
/// - 开始节点必须有输出连接
pub struct FlowValidator;

impl FlowValidator {
    /// 创建新的流程验证器
    pub fn new() -> Self {
        FlowValidator
    }

    /// 验证流程
    ///
    /// # 参数
    /// - `flow`: 要验证的流程
    ///
    /// # 返回
    /// - `ValidationResult`: 验证结果
    pub fn validate(&self, flow: &Flow) -> ValidationResult {
        let mut errors = Vec::new();

        // 规则 1: 必须有且仅有一个开始节点
        let start_nodes: Vec<&Node> = flow
            .nodes
            .iter()
            .filter(|node| matches!(node.node_type, NodeType::Start))
            .collect();

        if start_nodes.is_empty() {
            errors.push("流程缺少开始节点".to_string());
        } else if start_nodes.len() > 1 {
            errors.push(format!(
                "流程包含多个开始节点（{} 个），必须仅有一个",
                start_nodes.len()
            ));
        }

        // 规则 2: 开始节点必须有输出连接
        if start_nodes.len() == 1 {
            let start_node = start_nodes[0];
            let has_output = flow
                .connections
                .iter()
                .any(|conn| conn.source_node_id == start_node.id);

            if !has_output {
                errors.push(format!("开始节点 '{}' 没有输出连接", start_node.name));
            }
        }

        if errors.is_empty() {
            ValidationResult::success()
        } else {
            ValidationResult::errors(errors)
        }
    }

    /// 执行验证（别名方法）
    ///
    /// # 参数
    /// - `flow`: 要验证的流程
    ///
    /// # 返回
    /// - `ValidationResult`: 验证结果
    pub fn execute(&self, flow: &Flow) -> ValidationResult {
        self.validate(flow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Connection;
    use crate::node::{Node, NodePosition, NodeType};
    use serde_json::json;
    use uuid::Uuid;

    /// 创建测试用的开始节点
    fn create_start_node() -> Node {
        Node {
            id: Uuid::new_v4().to_string(),
            node_type: NodeType::Start,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: json!({}),
            ports: None,
        }
    }

    /// 创建测试用的结束节点
    fn create_end_node() -> Node {
        Node {
            id: Uuid::new_v4().to_string(),
            node_type: NodeType::End,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: json!({}),
            ports: None,
        }
    }

    /// 创建连接
    fn create_connection(source_id: &str, target_id: &str) -> Connection {
        Connection {
            id: Uuid::new_v4().to_string(),
            source_node_id: source_id.to_string(),
            target_node_id: target_id.to_string(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        }
    }

    #[test]
    fn test_validator_no_start_node() {
        // 创建没有开始节点的流程
        let flow = Flow {
            id: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            nodes: vec![create_end_node()],
            connections: vec![],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到错误
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("缺少开始节点"));
    }

    #[test]
    fn test_validator_multiple_start_nodes() {
        // 创建包含多个开始节点的流程
        let start_node1 = create_start_node();
        let start_node2 = create_start_node();

        let flow = Flow {
            id: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            nodes: vec![start_node1, start_node2],
            connections: vec![],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到错误
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("多个开始节点"));
        assert!(result.errors[0].contains("2"));
    }

    #[test]
    fn test_validator_no_output_connection() {
        // 创建没有输出连接的开始节点
        let start_node = create_start_node();

        let flow = Flow {
            id: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到错误
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("没有输出连接"));
    }

    #[test]
    fn test_validator_valid_flow() {
        // 创建有效的流程
        let start_node = create_start_node();
        let end_node = create_end_node();
        let connection = create_connection(&start_node.id, &end_node.id);

        let flow = Flow {
            id: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证通过
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validator_multiple_errors() {
        // 创建包含多个错误的流程
        let start_node1 = create_start_node();
        let start_node2 = create_start_node();

        let flow = Flow {
            id: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            nodes: vec![start_node1, start_node2],
            connections: vec![],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到多个错误
        assert!(!result.valid);
        // 应该有多个错误（多个开始节点，且开始节点没有输出连接）
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validator_execute_method() {
        // 测试 execute 方法
        let start_node = create_start_node();
        let end_node = create_end_node();
        let connection = create_connection(&start_node.id, &end_node.id);

        let flow = Flow {
            id: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let validator = FlowValidator::new();
        let result = validator.execute(&flow);

        // 验证结果正确
        assert!(result.valid);
    }

    #[test]
    fn test_validator_empty_flow() {
        // 创建空流程
        let flow = Flow {
            id: Uuid::new_v4().to_string(),
            name: "空流程".to_string(),
            nodes: vec![],
            connections: vec![],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到错误
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("缺少开始节点"));
    }

    #[test]
    fn test_validator_start_node_with_other_nodes() {
        // 创建包含其他节点的有效流程
        let start_node = create_start_node();
        let http_node = Node {
            id: Uuid::new_v4().to_string(),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: json!({ "url": "https://example.com" }),
            ports: None,
        };
        let end_node = create_end_node();

        let connection1 = create_connection(&start_node.id, &http_node.id);
        let connection2 = create_connection(&http_node.id, &end_node.id);

        let flow = Flow {
            id: Uuid::new_v4().to_string(),
            name: "复杂测试流程".to_string(),
            nodes: vec![start_node, http_node, end_node],
            connections: vec![connection1, connection2],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证通过
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validator_connection_from_non_start() {
        // 创建从非开始节点发出的连接的流程
        let start_node = create_start_node();
        let http_node = Node {
            id: Uuid::new_v4().to_string(),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: json!({}),
            ports: None,
        };
        let end_node = create_end_node();

        // 只从 HTTP 节点发出连接
        let connection = create_connection(&http_node.id, &end_node.id);

        let flow = Flow {
            id: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            nodes: vec![start_node, http_node, end_node],
            connections: vec![connection],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到错误
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("没有输出连接"));
    }
}
