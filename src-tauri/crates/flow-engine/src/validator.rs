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

impl Default for FlowValidator {
    fn default() -> Self {
        Self::new()
    }
}

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

        // 规则 1: 必须有至少一个 Entry 节点（流程入口）
        let entry_nodes: Vec<&Node> = flow
            .nodes
            .iter()
            .filter(|node| matches!(node.node_type, NodeType::Entry))
            .collect();

        if entry_nodes.is_empty() {
            errors.push("流程缺少 Entry 节点（流程入口）".to_string());
        }
        // 注意：不再检查 Entry 节点数量限制或输出连接要求
        // 新架构允许：
        // 1. 多个 Entry 节点（Entry 节点可以用于流程入口和终止点）
        // 2. Entry 节点可以没有输出连接（正常流程终止）

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
    use crate::flow::Flow;
    use crate::node::{Node, NodePosition, NodeType};
    use crate::node_config::NodeData;
    use uuid::Uuid;

    /// 创建测试用的入口节点
    fn create_entry_node() -> Node {
        Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: NodeType::Entry,
            name: "入口".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        }
    }

    /// 创建测试用的结束节点（使用 Http 节点类型，因为 Entry 被视为开始节点）
    fn create_end_node() -> Node {
        Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: NodeType::Http,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com/end"),
            ports: None,
        }
    }

    /// 创建连接
    fn create_connection(source_uuid: &str, target_uuid: &str) -> Connection {
        Connection {
            id: Uuid::new_v4().to_string(),
            source_node_uuid: source_uuid.to_string(),
            target_node_uuid: target_uuid.to_string(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        }
    }

    #[test]
    fn test_validator_no_start_node() {
        // 创建没有开始节点的流程
        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            initial_variables: None,
            nodes: vec![create_end_node()],
            connections: vec![],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到错误
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        eprintln!("Error: {}", result.errors[0]);
        assert!(
            result.errors[0].contains("缺少开始节点")
                || result.errors[0].contains("缺少 Entry 节点")
        );
    }

    #[test]
    fn test_validator_multiple_start_nodes() {
        // 创建包含多个开始节点的流程
        let start_node1 = create_entry_node();
        let start_node2 = create_entry_node();

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            initial_variables: None,
            nodes: vec![start_node1, start_node2],
            connections: vec![],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 新架构：多个 Entry 节点是有效的
        assert!(result.valid);
    }

    #[test]
    fn test_validator_no_output_connection() {
        // 创建没有 Entry 节点的流程（新架构只检查是否有 Entry 节点）
        let http_node = Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: NodeType::Http,
            name: "HTTP 节点".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
        };

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            initial_variables: None,
            nodes: vec![http_node],
            connections: vec![],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到错误（缺少 Entry 节点）
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        eprintln!("Error: {}", result.errors[0]);
        assert!(result.errors[0].contains("缺少 Entry 节点"));
    }

    #[test]
    fn test_validator_valid_flow() {
        // 创建有效的流程
        let start_node = create_entry_node();
        let end_node = create_end_node();
        let connection = create_connection(&start_node.uuid, &end_node.uuid);

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            initial_variables: None,
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
        // 新架构：只有一个错误类型（缺少 Entry 节点）
        // 创建没有 Entry 节点的流程
        let http_node1 = Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: NodeType::Http,
            name: "HTTP 节点 1".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::http("https://example.com/1"),
            ports: None,
        };
        let http_node2 = Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: NodeType::Http,
            name: "HTTP 节点 2".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com/2"),
            ports: None,
        };

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            initial_variables: None,
            nodes: vec![http_node1, http_node2],
            connections: vec![],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到错误（缺少 Entry 节点）
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("缺少 Entry 节点"));
    }

    #[test]
    fn test_validator_execute_method() {
        // 测试 execute 方法
        let start_node = create_entry_node();
        let end_node = create_end_node();
        let connection = create_connection(&start_node.uuid, &end_node.uuid);

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            initial_variables: None,
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
            uuid: Uuid::new_v4().to_string(),
            name: "空流程".to_string(),
            initial_variables: None,
            nodes: vec![],
            connections: vec![],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到错误（缺少 Entry 节点）
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        eprintln!("Empty flow error: {}", result.errors[0]);
        assert!(result.errors[0].contains("缺少 Entry 节点"));
    }

    #[test]
    fn test_validator_start_node_with_other_nodes() {
        // 创建包含其他节点的有效流程
        let start_node = create_entry_node();
        let http_node = Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
        };
        let end_node = create_end_node();

        let connection1 = create_connection(&start_node.uuid, &http_node.uuid);
        let connection2 = create_connection(&http_node.uuid, &end_node.uuid);

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "复杂测试流程".to_string(),
            initial_variables: None,
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
        // 新架构：只检查是否有 Entry 节点，不检查连接来源
        // 创建有 Entry 节点的流程（应该通过验证）
        let start_node = create_entry_node();
        let http_node = Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
        };
        let end_node = create_end_node();

        // 只从 HTTP 节点发出连接
        let connection = create_connection(&http_node.uuid, &end_node.uuid);

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            initial_variables: None,
            nodes: vec![start_node, http_node, end_node],
            connections: vec![connection],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 新架构：只要 Entry 节点存在就通过验证
        assert!(result.valid);
    }

    #[test]
    fn test_validator_uses_source_node_uuid() {
        // 测试开始节点输出连接检查使用 conn.source_node_uuid
        let start_node = create_entry_node();
        let end_node = create_end_node();

        // 连接使用 source_node_uuid 字段
        let connection = Connection {
            id: "test-conn-id".to_string(),
            source_node_uuid: start_node.uuid.clone(), // 明确使用 source_node_uuid
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            initial_variables: None,
            nodes: vec![start_node.clone(), end_node],
            connections: vec![connection],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证通过（因为开始节点有输出连接）
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validator_uses_start_node_uuid() {
        // 新架构：只检查是否有 Entry 节点，不检查连接来源
        // 创建有 Entry 节点的流程（应该通过验证）
        let start_node = Node {
            uuid: "start-uuid-test".to_string(),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };
        let end_node = create_end_node();

        // 创建不匹配的连接（source 不是 start_node.uuid）
        let other_node = Node {
            uuid: "other-node-uuid".to_string(),
            node_type: NodeType::Http,
            name: "其他节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::http("https://example.com/other"),
            ports: None,
        };

        let connection = Connection {
            id: "test-conn-id".to_string(),
            source_node_uuid: other_node.uuid.clone(), // 不是 start_node.uuid
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            initial_variables: None,
            nodes: vec![start_node, other_node, end_node],
            connections: vec![connection],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 新架构：只要 Entry 节点存在就通过验证
        assert!(result.valid);
    }

    #[test]
    fn test_validator_create_connection_helper() {
        // 测试 create_connection 辅助函数使用 source_node_uuid 和 target_node_uuid
        let source_uuid = "source-node-test-uuid";
        let target_uuid = "target-node-test-uuid";

        let connection = create_connection(source_uuid, target_uuid);

        // 验证连接正确使用了 source_node_uuid 和 target_node_uuid
        assert_eq!(connection.source_node_uuid, source_uuid);
        assert_eq!(connection.target_node_uuid, target_uuid);
    }

    #[test]
    fn test_validator_error_messages_correct() {
        // 新架构：测试缺少 Entry 节点的错误消息
        let http_node = Node {
            uuid: "error-test-http".to_string(),
            node_type: NodeType::Http,
            name: "HTTP 节点".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
        };

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "错误测试流程".to_string(),
            initial_variables: None,
            nodes: vec![http_node],
            connections: vec![],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证错误消息正确
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        let error_msg = &result.errors[0];
        eprintln!("Error message: {}", error_msg);
        assert!(error_msg.contains("缺少 Entry 节点"));
    }

    #[test]
    fn test_validator_multiple_connections_same_source() {
        // 测试多个连接从同一个源节点发出
        let start_node = create_entry_node();
        let end_node_1 = create_end_node();
        let end_node_2 = create_end_node();

        let connection_1 = create_connection(&start_node.uuid, &end_node_1.uuid);
        let connection_2 = create_connection(&start_node.uuid, &end_node_2.uuid);

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            initial_variables: None,
            nodes: vec![start_node, end_node_1, end_node_2],
            connections: vec![connection_1, connection_2],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证通过（开始节点有输出连接）
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validator_connection_with_specific_uuids() {
        // 测试使用特定 UUID 的连接
        let start_node = Node {
            uuid: "specific-start-uuid-001".to_string(),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let end_node = Node {
            uuid: "specific-end-uuid-002".to_string(),
            node_type: NodeType::Http,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com/end"),
            ports: None,
        };

        // 使用特定 UUID 创建连接
        let connection = Connection {
            id: "specific-conn-uuid-003".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "测试流程".to_string(),
            initial_variables: None,
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证通过
        assert!(result.valid);
    }
}
