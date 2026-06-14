// 流程验证器模块 - FlowValidator 实现

use flow_engine_core::error::ErrorCode;
use flow_engine_core::flow::Flow;
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::node_types;

/// 结构化验证错误条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationEntry {
    /// 错误代码（如 E_VAL_MISSING_ENTRY）
    pub code: ErrorCode,
    /// 人类可读的错误描述
    pub message: String,
    /// 相关的字段或属性名（可选）
    pub field: Option<String>,
}

/// 验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 验证是否通过
    pub valid: bool,
    /// 结构化错误信息列表
    pub errors: Vec<ValidationEntry>,
    /// 警告信息列表（不影响 valid 状态）
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// 创建成功的验证结果
    pub fn success() -> Self {
        ValidationResult {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// 创建包含单个结构化错误的验证结果
    pub fn error(code: ErrorCode, message: impl Into<String>, field: Option<String>) -> Self {
        ValidationResult {
            valid: false,
            errors: vec![ValidationEntry {
                code,
                message: message.into(),
                field,
            }],
            warnings: Vec::new(),
        }
    }

    /// 创建多个错误的验证结果
    pub fn errors(errors: Vec<ValidationEntry>) -> Self {
        ValidationResult {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// 创建带警告的验证结果（valid 仍为 true）
    pub fn warning(message: impl Into<String>) -> Self {
        ValidationResult {
            valid: true,
            errors: Vec::new(),
            warnings: vec![message.into()],
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
        let mut errors: Vec<ValidationEntry> = Vec::new();
        let mut warnings = Vec::new();

        // 规则 1: 必须有至少一个 Entry 节点（流程入口）
        let entry_nodes: Vec<&Node> = flow
            .nodes
            .iter()
            .filter(|node| node.node_type == node_types::ENTRY)
            .collect();

        if entry_nodes.is_empty() {
            errors.push(ValidationEntry {
                code: ErrorCode::E_VAL_MISSING_ENTRY,
                message: "Flow is missing an Entry node".to_string(),
                field: None,
            });
        }
        // 注意：不再检查 Entry 节点数量限制或输出连接要求
        // 新架构允许：
        // 1. 多个 Entry 节点（Entry 节点可以用于流程入口和终止点）
        // 2. Entry 节点可以没有输出连接（正常流程终止）

        // 规则 2: 验证输出端口唯一性（每个输出端口只能有一条出边）
        let port_validation = self.validate_output_port_uniqueness(flow);
        if !port_validation.valid {
            errors.extend(port_validation.errors);
        }

        // 规则 3: 验证无循环（DAG 约束）
        let cycle_validation = self.validate_no_cycles(flow);
        if !cycle_validation.valid {
            errors.extend(cycle_validation.errors);
        }

        // 规则 4: 验证 WebSocket 节点配置
        let ws_validation = self.validate_ws_nodes(flow);
        if !ws_validation.valid {
            errors.extend(ws_validation.errors);
        }

        // 规则 5: 检查 End 节点
        let end_nodes: Vec<&Node> = flow
            .nodes
            .iter()
            .filter(|node| node.node_type == node_types::END)
            .collect();

        if end_nodes.is_empty() {
            // 仅在流程有 Entry 节点（非空流程）时发出警告
            let has_entry = flow
                .nodes
                .iter()
                .any(|n| n.node_type == node_types::ENTRY);
            if has_entry {
                warnings.push(
                    "Flow has no End node; execution status will remain 'success' on completion. Consider adding an End node to explicitly terminate the flow."
                        .to_string(),
                );
            }
        }

        if errors.is_empty() {
            if warnings.is_empty() {
                ValidationResult::success()
            } else {
                ValidationResult {
                    valid: true,
                    errors: vec![],
                    warnings,
                }
            }
        } else {
            ValidationResult {
                valid: false,
                errors,
                warnings,
            }
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

    /// 验证 WebSocket 节点配置
    ///
    /// # 参数
    /// - `flow`: 要验证的流程
    ///
    /// # 返回
    /// - `ValidationResult`: 验证结果
    pub fn validate_ws_nodes(&self, flow: &Flow) -> ValidationResult {
        let mut errors: Vec<ValidationEntry> = Vec::new();

        for node in &flow.nodes {
            match &node.data {
                NodeData::WsConnect(config) => {
                    // 验证 URL 不为空
                    if config.url.is_empty() {
                        errors.push(ValidationEntry {
                            code: ErrorCode::E_HTTP_INVALID_URL,
                            message: format!("Node '{}' URL cannot be empty", node.name),
                            field: Some("url".to_string()),
                        });
                    }

                    // 验证 URL 格式
                    if !config.url.is_empty()
                        && !config.url.starts_with("ws://")
                        && !config.url.starts_with("wss://")
                    {
                        errors.push(ValidationEntry {
                            code: ErrorCode::E_HTTP_INVALID_URL,
                            message: format!(
                                "Node '{}' URL format is invalid (must start with ws:// or wss://)",
                                node.name
                            ),
                            field: Some("url".to_string()),
                        });
                    }

                    // 验证超时配置
                    if config.connect_timeout_ms < 1000 || config.connect_timeout_ms > 60000 {
                        errors.push(ValidationEntry {
                            code: ErrorCode::E_CONFIG_ERROR,
                            message: format!(
                                "Node '{}' connect timeout must be between 1000-60000ms",
                                node.name
                            ),
                            field: Some("connect_timeout_ms".to_string()),
                        });
                    }

                    if config.collect_timeout_ms < 1000 || config.collect_timeout_ms > 300000 {
                        errors.push(ValidationEntry {
                            code: ErrorCode::E_CONFIG_ERROR,
                            message: format!(
                                "Node '{}' collect timeout must be between 1000-300000ms",
                                node.name
                            ),
                            field: Some("collect_timeout_ms".to_string()),
                        });
                    }
                }
                NodeData::WsSendCollect(config) => {
                    // 验证 connection_id 不为空
                    if config.connection_id.is_empty() {
                        errors.push(ValidationEntry {
                            code: ErrorCode::E_CONFIG_ERROR,
                            message: format!("Node '{}' connection_id cannot be empty", node.name),
                            field: Some("connection_id".to_string()),
                        });
                    }

                    // 验证消息不为空
                    if config.message.is_empty() {
                        errors.push(ValidationEntry {
                            code: ErrorCode::E_CONFIG_ERROR,
                            message: format!("Node '{}' message cannot be empty", node.name),
                            field: Some("message".to_string()),
                        });
                    }

                    // 验证超时配置
                    if config.send_timeout_ms < 1000 || config.send_timeout_ms > 30000 {
                        errors.push(ValidationEntry {
                            code: ErrorCode::E_CONFIG_ERROR,
                            message: format!(
                                "Node '{}' send timeout must be between 1000-30000ms",
                                node.name
                            ),
                            field: Some("send_timeout_ms".to_string()),
                        });
                    }

                    if config.collect_timeout_ms < 1000 || config.collect_timeout_ms > 300000 {
                        errors.push(ValidationEntry {
                            code: ErrorCode::E_CONFIG_ERROR,
                            message: format!(
                                "Node '{}' collect timeout must be between 1000-300000ms",
                                node.name
                            ),
                            field: Some("collect_timeout_ms".to_string()),
                        });
                    }
                }
                NodeData::WsClose(config) => {
                    // 验证 connection_id 不为空
                    if config.connection_id.is_empty() {
                        errors.push(ValidationEntry {
                            code: ErrorCode::E_CONFIG_ERROR,
                            message: format!("Node '{}' connection_id cannot be empty", node.name),
                            field: Some("connection_id".to_string()),
                        });
                    }

                    // 验证关闭码
                    if config.close_code < 1000 || config.close_code > 4999 {
                        errors.push(ValidationEntry {
                            code: ErrorCode::E_CONFIG_ERROR,
                            message: format!(
                                "Node '{}' close code must be between 1000-4999",
                                node.name
                            ),
                            field: Some("close_code".to_string()),
                        });
                    }

                    // 验证关闭超时
                    if config.close_timeout_ms < 1000 || config.close_timeout_ms > 30000 {
                        errors.push(ValidationEntry {
                            code: ErrorCode::E_CONFIG_ERROR,
                            message: format!(
                                "Node '{}' close timeout must be between 1000-30000ms",
                                node.name
                            ),
                            field: Some("close_timeout_ms".to_string()),
                        });
                    }
                }
                _ => {}
            }
        }

        if errors.is_empty() {
            ValidationResult::success()
        } else {
            ValidationResult::errors(errors)
        }
    }

    /// 验证输出端口唯一性
    ///
    /// 每个节点的每个输出端口（source_port_id）只能有一条出边。
    /// 一个节点可以有多个输出端口（如 conditional 的 "true"/"false"），
    /// 但每个端口最多只能连接到一个下游节点。
    /// 输入端口（target_port_id）不受此限制，可以接收多条入边。
    pub fn validate_output_port_uniqueness(&self, flow: &Flow) -> ValidationResult {
        let mut errors = Vec::new();

        // 检查是否有重复的 (source_node_uuid, source_port_id) 组合
        let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
        for conn in &flow.connections {
            let key = (conn.source_node_uuid.clone(), conn.source_port_id.clone());
            if !seen.insert(key.clone()) {
                // 查找节点名称以便提供有意义的错误信息
                let node_name = flow
                    .nodes
                    .iter()
                    .find(|n| n.uuid == conn.source_node_uuid)
                    .map(|n| n.name.as_str())
                    .unwrap_or("unknown node");
                errors.push(ValidationEntry {
                    code: ErrorCode::E_VAL_PORT_DUPLICATE,
                    message: format!(
                        "Node '{}' port '{}' can only connect to one downstream node, duplicate connection detected",
                        node_name, conn.source_port_id
                    ),
                    field: Some(conn.source_port_id.clone()),
                });
            }
        }

        if errors.is_empty() {
            ValidationResult::success()
        } else {
            ValidationResult::errors(errors)
        }
    }

    /// 验证无循环（DAG 约束）
    ///
    /// 使用 DFS 染色法检测有向图中是否存在环。
    /// 白色（未访问）、灰色（正在访问）、黑色（已访问完成）。
    /// 如果在遍历过程中遇到灰色节点，说明存在环。
    pub fn validate_no_cycles(&self, flow: &Flow) -> ValidationResult {
        let mut errors: Vec<ValidationEntry> = Vec::new();

        if flow.nodes.is_empty() || flow.connections.is_empty() {
            return ValidationResult::success();
        }

        // 构建邻接表：source_node_uuid -> Vec<target_node_uuid>
        let mut adj: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for conn in &flow.connections {
            adj.entry(conn.source_node_uuid.clone())
                .or_default()
                .push(conn.target_node_uuid.clone());
        }

        // 找到所有节点 UUID 集合
        let all_uuids: Vec<String> = flow
            .nodes
            .iter()
            .map(|n| n.uuid.clone())
            .collect();

        // DFS 染色法检测环
        // 使用 index 映射避免引用生命周期问题
        // 0 = 未访问 (白色), 1 = 正在访问 (灰色), 2 = 已访问完成 (黑色)
        let idx_of: std::collections::HashMap<String, usize> = all_uuids
            .iter()
            .enumerate()
            .map(|(i, u)| (u.clone(), i))
            .collect();
        let n = all_uuids.len();
        let mut color = vec![0u8; n];

        // 构建邻接索引表
        let mut adj_idx: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (src, targets) in &adj {
            if let Some(&si) = idx_of.get(src) {
                for t in targets {
                    if let Some(&ti) = idx_of.get(t) {
                        adj_idx[si].push(ti);
                    }
                }
            }
        }

        // 构建节点名称映射（按索引）
        let node_names: Vec<&str> = flow.nodes.iter().map(|n| n.name.as_str()).collect();

        fn dfs_idx(
            u: usize,
            adj: &[Vec<usize>],
            color: &mut Vec<u8>,
            node_names: &[&str],
            path: &mut Vec<usize>,
        ) -> Option<ValidationEntry> {
            color[u] = 1; // 灰色：正在访问
            path.push(u);

            for &v in &adj[u] {
                match color[v] {
                    1 => {
                        // 找到环
                        let cycle_start = path.iter().position(|&n| n == v).unwrap_or(0);
                        let cycle_path: Vec<&str> = path[cycle_start..]
                            .iter()
                            .map(|&idx| node_names[idx])
                            .collect();
                        let cycle_str = cycle_path.join(" → ");
                        return Some(ValidationEntry {
                            code: ErrorCode::E_VAL_CYCLE_DETECTED,
                            message: format!("Flow contains a cycle: {}", cycle_str),
                            field: None,
                        });
                    }
                    0 => {
                        if let Some(err) = dfs_idx(v, adj, color, node_names, path) {
                            return Some(err);
                        }
                    }
                    _ => {}
                }
            }

            color[u] = 2; // 黑色：已访问完成
            path.pop();
            None
        }

        // 对所有节点执行 DFS
        for i in 0..n {
            if color[i] == 0 {
                let mut path = Vec::new();
                if let Some(err) = dfs_idx(i, &adj_idx, &mut color, &node_names, &mut path) {
                    errors.push(err);
                }
            }
        }

        if errors.is_empty() {
            ValidationResult::success()
        } else {
            ValidationResult::errors(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::connection::Connection;
    use flow_engine_core::flow::Flow;
    use flow_engine_core::node::{Node, NodePosition};
    use flow_engine_core::node_config::NodeData;
    use uuid::Uuid;

    /// 创建测试用的入口节点
    fn create_entry_node() -> Node {
        Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: "entry".to_string(),
            name: "入口".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        }
    }

    /// 创建测试用的结束节点
    fn create_end_node() -> Node {
        Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: "end".to_string(),
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::end(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
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
        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "测试流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![create_end_node()], connections: vec![], hooks: None };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到错误
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        eprintln!("Error: {}", result.errors[0].message);
        assert!(
            result.errors[0].message.contains("missing an Entry node")
        );
    }

    #[test]
    fn test_validator_multiple_start_nodes() {
        // 创建包含多个开始节点的流程
        let start_node1 = create_entry_node();
        let start_node2 = create_entry_node();

        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "测试流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![start_node1, start_node2], connections: vec![], hooks: None };

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
            node_type: "http".to_string(),
            name: "HTTP 节点".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "测试流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![http_node], connections: vec![], hooks: None };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到错误（缺少 Entry 节点）
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        eprintln!("Error: {}", result.errors[0].message);
        assert!(result.errors[0].message.contains("missing an Entry node"));
    }

    #[test]
    fn test_validator_valid_flow() {
        // 创建有效的流程
        let start_node = create_entry_node();
        let end_node = create_end_node();
        let connection = create_connection(&start_node.uuid, &end_node.uuid);

        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "测试流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![start_node, end_node], connections: vec![connection], hooks: None };

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
            node_type: "http".to_string(),
            name: "HTTP 节点 1".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::http("https://example.com/1"),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };
        let http_node2 = Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: "http".to_string(),
            name: "HTTP 节点 2".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com/2"),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "测试流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![http_node1, http_node2], connections: vec![], hooks: None };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到错误（缺少 Entry 节点）
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("missing an Entry node"));
    }

    #[test]
    fn test_validator_execute_method() {
        // 测试 execute 方法
        let start_node = create_entry_node();
        let end_node = create_end_node();
        let connection = create_connection(&start_node.uuid, &end_node.uuid);

        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "测试流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![start_node, end_node], connections: vec![connection], hooks: None };

        let validator = FlowValidator::new();
        let result = validator.execute(&flow);

        // 验证结果正确
        assert!(result.valid);
    }

    #[test]
    fn test_validator_empty_flow() {
        // 创建空流程
        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "空流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![], connections: vec![], hooks: None };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证检测到错误（缺少 Entry 节点）
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        eprintln!("Empty flow error: {}", result.errors[0].message);
        assert!(result.errors[0].message.contains("missing an Entry node"));
    }

    #[test]
    fn test_validator_start_node_with_other_nodes() {
        // 创建包含其他节点的有效流程
        let start_node = create_entry_node();
        let http_node = Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: "http".to_string(),
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };
        let end_node = create_end_node();

        let connection1 = create_connection(&start_node.uuid, &http_node.uuid);
        let connection2 = create_connection(&http_node.uuid, &end_node.uuid);

        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "复杂测试流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![start_node, http_node, end_node], connections: vec![connection1, connection2], hooks: None };

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
            node_type: "http".to_string(),
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };
        let end_node = create_end_node();

        // 只从 HTTP 节点发出连接
        let connection = create_connection(&http_node.uuid, &end_node.uuid);

        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "测试流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![start_node, http_node, end_node], connections: vec![connection], hooks: None };

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

        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "测试流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![start_node.clone(), end_node], connections: vec![connection], hooks: None };

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
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };
        let end_node = create_end_node();

        // 创建不匹配的连接（source 不是 start_node.uuid）
        let other_node = Node {
            uuid: "other-node-uuid".to_string(),
            node_type: "http".to_string(),
            name: "其他节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::http("https://example.com/other"),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let connection = Connection {
            id: "test-conn-id".to_string(),
            source_node_uuid: other_node.uuid.clone(), // 不是 start_node.uuid
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "测试流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![start_node, other_node, end_node], connections: vec![connection], hooks: None };

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
            node_type: "http".to_string(),
            name: "HTTP 节点".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "错误测试流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![http_node], connections: vec![], hooks: None };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证错误消息正确
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        let error_entry = &result.errors[0];
        eprintln!("Error message: {}", error_entry.message);
        assert_eq!(error_entry.code, ErrorCode::E_VAL_MISSING_ENTRY);
        assert!(error_entry.message.contains("missing an Entry node"));
    }

    #[test]
    fn test_validator_multiple_connections_same_source() {
        // 测试多个连接从同一个源节点同一个端口发出 → 应失败
        let start_node = create_entry_node();
        let end_node_1 = create_end_node();
        let end_node_2 = create_end_node();

        let connection_1 = create_connection(&start_node.uuid, &end_node_1.uuid);
        let connection_2 = create_connection(&start_node.uuid, &end_node_2.uuid);

        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "测试流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![start_node, end_node_1, end_node_2], connections: vec![connection_1, connection_2], hooks: None };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 新规则：同一输出端口只能有一条出边
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("can only connect to one downstream node")));
    }

    #[test]
    fn test_validator_connection_with_specific_uuids() {
        // 测试使用特定 UUID 的连接
        let start_node = Node {
            uuid: "specific-start-uuid-001".to_string(),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let end_node = Node {
            uuid: "specific-end-uuid-002".to_string(),
            node_type: "end".to_string(),
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::end(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        // 使用特定 UUID 创建连接
        let connection = Connection {
            id: "specific-conn-uuid-003".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow { uuid: Uuid::new_v4().to_string(), name: "测试流程".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![start_node, end_node], connections: vec![connection], hooks: None };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证通过
        assert!(result.valid);
    }

    // ========================================================================
    // End 节点警告测试
    // ========================================================================

    #[test]
    fn test_validator_no_end_node_warning() {
        // 只有 Entry 节点，没有 End 节点 → 产生警告
        let start_node = create_entry_node();
        let http_node = Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: "http".to_string(),
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "无 End 节点流程".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            nodes: vec![start_node, http_node],
            connections: vec![],
            hooks: None,
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证通过但有警告
        assert!(result.valid);
        assert!(result.errors.is_empty());
        assert!(!result.warnings.is_empty());
        eprintln!("Warning: {}", result.warnings[0]);
        assert!(result.warnings[0].contains("End node"));
    }

    #[test]
    fn test_validator_with_end_node_no_warning() {
        // 有 Entry 节点和 End 节点 → 没有警告
        let start_node = create_entry_node();
        let end_node = create_end_node();
        let connection = create_connection(&start_node.uuid, &end_node.uuid);

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "有 End 节点流程".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            nodes: vec![start_node, end_node],
            connections: vec![connection],
            hooks: None,
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证通过且无警告
        assert!(result.valid);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validator_warnings_do_not_affect_valid() {
        // 验证 warnings 存在时 valid 仍然为 true
        let start_node = create_entry_node();
        let http_node = Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: "http".to_string(),
            name: "HTTP 节点".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "警告测试".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            nodes: vec![start_node, http_node],
            connections: vec![],
            hooks: None,
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 验证 warnings 不影响 valid
        assert!(result.valid);
        assert!(!result.warnings.is_empty());
    }

    // ========================================================================
    // 输出端口唯一性测试
    // ========================================================================

    #[test]
    fn test_output_port_uniqueness_duplicate() {
        // 同一个输出端口连接两个不同节点 → 应该失败
        let entry = create_entry_node();
        let end1 = create_end_node();
        let end2 = create_end_node();

        let conn1 = create_connection(&entry.uuid, &end1.uuid);
        let conn2 = Connection {
            id: "dup-conn".to_string(),
            source_node_uuid: entry.uuid.clone(),
            target_node_uuid: end2.uuid.clone(),
            source_port_id: "output".to_string(), // 同一个端口
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "重复端口测试".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            nodes: vec![entry, end1, end2],
            connections: vec![conn1, conn2],
            hooks: None,
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("can only connect to one downstream node")));
    }

    #[test]
    fn test_output_port_uniqueness_different_ports() {
        // 条件节点的 true 和 false 端口各自连接不同节点 → 应该通过
        let entry = create_entry_node();
        let conditional = Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: "conditional".to_string(),
            name: "条件分支".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::conditional_simple("status", flow_engine_core::node_config::ConditionOperator::Equal, serde_json::json!("ok")),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };
        let http_node = Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: "http".to_string(),
            name: "HTTP 成功".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com/success"),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };
        let end_node = create_end_node();

        let conn_true = Connection {
            id: "conn-true".to_string(),
            source_node_uuid: conditional.uuid.clone(),
            target_node_uuid: http_node.uuid.clone(),
            source_port_id: "true".to_string(),
            target_port_id: "input".to_string(),
        };
        let conn_false = Connection {
            id: "conn-false".to_string(),
            source_node_uuid: conditional.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "false".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "多端口测试".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            nodes: vec![entry, conditional, http_node, end_node],
            connections: vec![conn_true, conn_false],
            hooks: None,
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        // 应该通过（不同端口各自出边是允许的）
        assert!(result.valid, "Different ports should each allow one edge, but errors: {:?}", result.errors);
    }

    // ========================================================================
    // 循环检测测试
    // ========================================================================

    #[test]
    fn test_cycle_detection_simple_cycle() {
        // A → B → A 简单循环
        let node_a = Node {
            uuid: "node-a".to_string(),
            node_type: "entry".to_string(),
            name: "节点A".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };
        let node_b = Node {
            uuid: "node-b".to_string(),
            node_type: "http".to_string(),
            name: "节点B".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let conn1 = create_connection(&node_a.uuid, &node_b.uuid);
        let conn2 = create_connection(&node_b.uuid, &node_a.uuid);

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "循环测试".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            nodes: vec![node_a, node_b],
            connections: vec![conn1, conn2],
            hooks: None,
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("cycle")));
    }

    #[test]
    fn test_cycle_detection_no_cycle() {
        // A → B → C  无循环
        let node_a = create_entry_node();
        let node_b = Node {
            uuid: Uuid::new_v4().to_string(),
            node_type: "http".to_string(),
            name: "节点B".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };
        let node_c = create_end_node();

        let conn1 = create_connection(&node_a.uuid, &node_b.uuid);
        let conn2 = create_connection(&node_b.uuid, &node_c.uuid);

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "无循环测试".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            nodes: vec![node_a, node_b, node_c],
            connections: vec![conn1, conn2],
            hooks: None,
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        assert!(result.valid, "Linear flow should have no cycles, but errors: {:?}", result.errors);
    }

    #[test]
    fn test_cycle_detection_complex_cycle() {
        // A → B → C → D → B  复杂循环
        let names = ["入口", "节点B", "节点C", "节点D"];
        let nodes: Vec<Node> = names
            .iter()
            .enumerate()
            .map(|(i, name)| Node {
                uuid: format!("node-{}", i),
                node_type: if i == 0 { "entry".to_string() } else { "http".to_string() },
                name: name.to_string(),
                position: NodePosition { x: (i as f64) * 100.0, y: 0.0 },
                data: if i == 0 { NodeData::entry() } else { NodeData::http("https://example.com") },
                ports: None,
                has_breakpoint: None,
            on_error: None,
            })
            .collect();

        let connections = vec![
            create_connection(&nodes[0].uuid, &nodes[1].uuid), // A → B
            create_connection(&nodes[1].uuid, &nodes[2].uuid), // B → C
            create_connection(&nodes[2].uuid, &nodes[3].uuid), // C → D
            create_connection(&nodes[3].uuid, &nodes[1].uuid), // D → B (循环)
        ];

        let flow = Flow {
            uuid: Uuid::new_v4().to_string(),
            name: "复杂循环测试".to_string(),
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            nodes,
            connections,
            hooks: None,
        };

        let validator = FlowValidator::new();
        let result = validator.validate(&flow);

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("cycle")));
    }
}
