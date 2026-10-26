// 变量提取节点执行器 — 从上游输出中提取变量写入执行上下文

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use std::collections::{HashMap, HashSet, VecDeque};

/// 变量提取节点执行器
///
/// 从 `last_node_output` 或指定 `source_node_id` 的执行报告中提取数据，
/// 通过 `output_mappings` 映射为上下文变量。
pub struct VariableExtractorExecutor;

impl VariableExtractorExecutor {
    /// 向上回溯连线查找真实数据生产者节点的输出数据（自动跳过 assertion/conditional 等控制节点）
    fn find_upstream_output<'a>(
        node: &'a Node,
        context: &ExecutionContext,
        connections: &'a [Connection],
        node_map: &HashMap<String, &'a Node>,
    ) -> Option<serde_json::Value> {
        let mut queue: VecDeque<&'a str> = VecDeque::new();
        let mut visited: HashSet<&'a str> = HashSet::new();
        visited.insert(node.uuid.as_str());

        for conn in connections {
            if conn.target_node_uuid == node.uuid {
                queue.push_back(conn.source_node_uuid.as_str());
            }
        }

        while let Some(current_uuid) = queue.pop_front() {
            if !visited.insert(current_uuid) {
                continue;
            }

            let current_node = node_map.get(current_uuid).copied();
            let is_intermediate = match current_node {
                Some(n) => {
                    matches!(
                        n.node_type.as_str(),
                        "assertion" | "conditional" | "proxy" | "entry" | "variable:extractor"
                    ) && context
                        .node_reports
                        .iter()
                        .any(|r| r.node_uuid == current_uuid)
                }
                None => context
                    .node_reports
                    .iter()
                    .rev()
                    .find(|r| r.node_uuid == current_uuid)
                    .map(|r| {
                        matches!(
                            r.node_type.as_str(),
                            "assertion" | "conditional" | "proxy" | "entry" | "variable:extractor"
                        )
                    })
                    .unwrap_or(false),
            };

            if !is_intermediate {
                if let Some(report) = context
                    .node_reports
                    .iter()
                    .rev()
                    .find(|r| r.node_uuid == current_uuid)
                {
                    if let Some(out) = &report.output_data {
                        return Some(out.clone());
                    }
                }
            }

            for conn in connections {
                if conn.target_node_uuid == current_uuid
                    && !visited.contains(conn.source_node_uuid.as_str())
                {
                    queue.push_back(conn.source_node_uuid.as_str());
                }
            }
        }

        None
    }
}

#[async_trait::async_trait]
impl NodeExecutor for VariableExtractorExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        // 1. 提取配置
        let config = match &node.data {
            NodeData::VariableExtractor(c) => c,
            _ => {
                return Err(ErrorDetail::execution_error(
                    "node configuration is not VariableExtractorConfig".to_string(),
                ))
            }
        };

        // 2. 获取上游输出数据
        let output = if let Some(source_node_id) = &config.source_node_id {
            // 从指定节点的执行报告中读取输出 (支持 UUID 匹配、节点名称匹配及 node_map 别名)
            context
                .node_reports
                .iter()
                .rev()
                .find(|r| {
                    r.node_uuid == *source_node_id
                        || r.node_name == *source_node_id
                        || node_map
                            .values()
                            .find(|n| n.name == *source_node_id)
                            .map(|n| &n.uuid)
                            == Some(&r.node_uuid)
                })
                .and_then(|r| r.output_data.clone())
                .ok_or_else(|| {
                    ErrorDetail::execution_error(format!(
                        "execution result not found for node {}",
                        source_node_id
                    ))
                })?
        } else {
            // 未指定 source_node_id 时：
            // 优先向上回溯查找真实数据生产者节点的输出，跳过中间断言/分支等控制节点；
            // 若未找到则回退到 last_node_output
            Self::find_upstream_output(node, context, connections, node_map)
                .or_else(|| context.last_node_output.clone())
                .ok_or_else(|| {
                    ErrorDetail::execution_error("no upstream node output to extract".to_string())
                })?
        };

        // 3. 应用输出映射，将提取的值写入上下文变量
        flow_engine_core::utils::apply_output_mappings(
            &output,
            &config.output_mappings,
            context,
            &node.uuid,
        );

        // 4. 构建输出数据
        let output_data = serde_json::json!({
            "mapped_count": config.output_mappings.len(),
            "source_node_id": config.source_node_id,
            "mappings_applied": true
        });

        // 5. 通过 "out" 端口路由到下游节点
        let next_node_ids = connections
            .iter()
            .filter(|c| c.source_node_uuid == node.uuid && c.source_port_id == "out")
            .map(|c| c.target_node_uuid.clone())
            .collect();

        Ok((next_node_ids, Some(output_data)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node::{Node, NodePosition};
    use flow_engine_core::node_config::VariableExtractorConfig;
    use flow_engine_core::variable::OutputMapping;

    /// 辅助函数：创建Test Node
    fn make_node(config: VariableExtractorConfig) -> Node {
        Node {
            uuid: "ve-node-1".to_string(),
            node_type: "variable:extractor".to_string(),
            name: "Variable Extraction".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::VariableExtractor(config),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }
    }

    /// 测试：基本变量提取 — 从 `last_node_output` 提取两个变量
    #[tokio::test]
    async fn test_variable_extractor_basic() {
        let config = VariableExtractorConfig {
            r#type: "variable:extractor".to_string(),
            output_mappings: vec![
                OutputMapping {
                    source: "$.status".to_string(),
                    target: "status_code".to_string(),
                    description: None,
                },
                OutputMapping {
                    source: "$.body.token".to_string(),
                    target: "auth_token".to_string(),
                    description: None,
                },
            ],
            source_node_id: None,
        };
        let node = make_node(config);

        let mut context = ExecutionContext::new("test-flow".to_string(), 0);
        context.last_node_output = Some(serde_json::json!({
            "status": 200,
            "body": {"token": "abc"}
        }));

        let executor = VariableExtractorExecutor;
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok(), "执行不应Failed");
        let (next_ids, output) = result.unwrap_or_else(|e| panic!("{e}"));
        assert!(next_ids.is_empty(), "无连接时 next_ids 为空");
        let output = output.unwrap_or_else(|| panic!("应有输出数据"));
        assert_eq!(output["mapped_count"], 2);
        assert_eq!(output["mappings_applied"], true);

        // 验证变量已写入上下文
        let status_var = context
            .variables
            .variables
            .iter()
            .find(|v| v.name == "status_code");
        assert!(status_var.is_some(), "status_code 变量应存在");
        assert_eq!(
            status_var.unwrap_or_else(|| panic!("expected Some")).value,
            serde_json::json!(200)
        );

        let token_var = context
            .variables
            .variables
            .iter()
            .find(|v| v.name == "auth_token");
        assert!(token_var.is_some(), "auth_token 变量应存在");
        assert_eq!(
            token_var.unwrap_or_else(|| panic!("expected Some")).value,
            serde_json::json!("abc")
        );
    }

    /// 测试：无上游输出且未指定 `source_node_id` → 返回Error
    #[tokio::test]
    async fn test_variable_extractor_no_upstream_output() {
        let config = VariableExtractorConfig {
            r#type: "variable:extractor".to_string(),
            output_mappings: vec![],
            source_node_id: None,
        };
        let node = make_node(config);

        let mut context = ExecutionContext::new("test-flow".to_string(), 0);
        // last_node_output 为 None
        context.last_node_output = None;

        let executor = VariableExtractorExecutor;
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        let err = result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"));
        assert!(
            err.message.contains("no upstream node output to extract"),
            "Error信息应提示无上游输出: {}",
            err
        );
    }

    /// 测试：指定不存在的 `source_node_id` → 返回Error
    #[tokio::test]
    async fn test_variable_extractor_source_node_not_found() {
        let config = VariableExtractorConfig {
            r#type: "variable:extractor".to_string(),
            output_mappings: vec![],
            source_node_id: Some("nonexistent".to_string()),
        };
        let node = make_node(config);

        let mut context = ExecutionContext::new("test-flow".to_string(), 0);
        context.last_node_output = Some(serde_json::json!({"status": 200}));
        // node_reports 中不包含 "nonexistent"

        let executor = VariableExtractorExecutor;
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        let err = result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"));
        assert!(
            err.message
                .contains("execution result not found for node nonexistent"),
            "Error信息应提示节点未找到: {}",
            err
        );
    }

    /// 测试：从指定 `source_node_id` 的执行报告中提取变量
    #[tokio::test]
    async fn test_variable_extractor_from_specific_node() {
        use flow_engine_core::report::{ExecutionStatus, NodeExecutionReport};

        let config = VariableExtractorConfig {
            r#type: "variable:extractor".to_string(),
            output_mappings: vec![OutputMapping {
                source: "$.token".to_string(),
                target: "extracted_token".to_string(),
                description: None,
            }],
            source_node_id: Some("http-node-1".to_string()),
        };
        let node = make_node(config);

        let mut context = ExecutionContext::new("test-flow".to_string(), 0);
        context.last_node_output = Some(serde_json::json!({"wrong": "data"}));
        // 在 node_reports 中插入 http-node-1 的执行结果
        context.node_reports.push(NodeExecutionReport {
            node_uuid: "http-node-1".to_string(),
            node_type: "http:request".to_string(),
            node_name: "HTTP 请求".to_string(),
            start_time: 0,
            end_time: 0,
            duration_ms: 0,
            status: ExecutionStatus::Success,
            input_data: None,
            output_data: Some(serde_json::json!({"token": "from-http-node"})),
            used_port_id: None,
            error: None,
            resolved_config: None,
            variable_changes: None,
            skip_reason: None,
            wait_duration_ms: None,
            plugin_data: None,
            sub_flow_report: None,
            is_detail_stripped: None,
        });

        let executor = VariableExtractorExecutor;
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok(), "执行不应Failed");

        // 验证变量从指定节点提取
        let token_var = context
            .variables
            .variables
            .iter()
            .find(|v| v.name == "extracted_token");
        assert!(token_var.is_some(), "extracted_token 变量应存在");
        assert_eq!(
            token_var.unwrap_or_else(|| panic!("expected Some")).value,
            serde_json::json!("from-http-node")
        );
    }

    /// 测试：通过 "out" 端口路由到下游节点
    #[tokio::test]
    async fn test_variable_extractor_routing() {
        let config = VariableExtractorConfig {
            r#type: "variable:extractor".to_string(),
            output_mappings: vec![],
            source_node_id: None,
        };
        let node = make_node(config);

        let mut context = ExecutionContext::new("test-flow".to_string(), 0);
        context.last_node_output = Some(serde_json::json!({"status": 200}));

        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "ve-node-1".to_string(),
            target_node_uuid: "next-node".to_string(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        }];

        let executor = VariableExtractorExecutor;
        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_ids, _) = result.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(next_ids, vec!["next-node"]);
    }

    /// 测试：通过节点名称匹配 source_node_id
    #[tokio::test]
    async fn test_variable_extractor_match_by_name() {
        use flow_engine_core::report::{ExecutionStatus, NodeExecutionReport};

        let config = VariableExtractorConfig {
            r#type: "variable:extractor".to_string(),
            output_mappings: vec![OutputMapping {
                source: "$.token".to_string(),
                target: "named_token".to_string(),
                description: None,
            }],
            source_node_id: Some("用户登录请求".to_string()),
        };
        let node = make_node(config);

        let mut context = ExecutionContext::new("test-flow".to_string(), 0);
        context.node_reports.push(NodeExecutionReport {
            node_uuid: "real-uuid-123".to_string(),
            node_type: "http:request".to_string(),
            node_name: "用户登录请求".to_string(),
            start_time: 0,
            end_time: 0,
            duration_ms: 0,
            status: ExecutionStatus::Success,
            input_data: None,
            output_data: Some(serde_json::json!({"token": "token-from-name-match"})),
            used_port_id: None,
            error: None,
            resolved_config: None,
            variable_changes: None,
            skip_reason: None,
            wait_duration_ms: None,
            plugin_data: None,
            sub_flow_report: None,
            is_detail_stripped: None,
        });

        let executor = VariableExtractorExecutor;
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let token_var = context
            .variables
            .variables
            .iter()
            .find(|v| v.name == "named_token");
        assert!(token_var.is_some());
        assert_eq!(
            token_var.unwrap_or_else(|| panic!("expected Some")).value,
            serde_json::json!("token-from-name-match")
        );
    }

    /// 测试：未指定 source_node_id 时，自动跨越中间的 assertion 节点回溯获取 upstream HTTP 节点输出
    #[tokio::test]
    async fn test_variable_extractor_auto_trace_upstream_across_assertion() {
        use flow_engine_core::report::{ExecutionStatus, NodeExecutionReport};

        let config = VariableExtractorConfig {
            r#type: "variable:extractor".to_string(),
            output_mappings: vec![OutputMapping {
                source: "$.body.token".to_string(),
                target: "auto_token".to_string(),
                description: None,
            }],
            source_node_id: None, // 模拟 AI 或用户漏选数据源
        };
        let mut extractor_node = make_node(config);
        extractor_node.uuid = "extractor-uuid".to_string();

        let http_uuid = "http-uuid".to_string();
        let assert_uuid = "assert-uuid".to_string();

        let mut context = ExecutionContext::new("test-flow".to_string(), 0);
        // last_node_output 是断言节点的输出，缺少 body 数据
        context.last_node_output = Some(serde_json::json!({
            "passed": true,
            "total": 1,
            "passed_count": 1
        }));

        // node_reports 中包含 http 节点与 assert 节点
        context.node_reports.push(NodeExecutionReport {
            node_uuid: http_uuid.clone(),
            node_type: "http:request".to_string(),
            node_name: "HTTP 接口".to_string(),
            start_time: 0,
            end_time: 0,
            duration_ms: 0,
            status: ExecutionStatus::Success,
            input_data: None,
            output_data: Some(serde_json::json!({
                "status": 200,
                "body": {"token": "traced-token-abc"}
            })),
            used_port_id: None,
            error: None,
            resolved_config: None,
            variable_changes: None,
            skip_reason: None,
            wait_duration_ms: None,
            plugin_data: None,
            sub_flow_report: None,
            is_detail_stripped: None,
        });
        context.node_reports.push(NodeExecutionReport {
            node_uuid: assert_uuid.clone(),
            node_type: "assertion".to_string(),
            node_name: "断言校验".to_string(),
            start_time: 0,
            end_time: 0,
            duration_ms: 0,
            status: ExecutionStatus::Success,
            input_data: None,
            output_data: Some(serde_json::json!({
                "passed": true
            })),
            used_port_id: None,
            error: None,
            resolved_config: None,
            variable_changes: None,
            skip_reason: None,
            wait_duration_ms: None,
            plugin_data: None,
            sub_flow_report: None,
            is_detail_stripped: None,
        });

        // 连接关系: http -> assert -> extractor
        let connections = vec![
            Connection {
                id: "c1".to_string(),
                source_node_uuid: http_uuid.clone(),
                source_port_id: "true".to_string(),
                target_node_uuid: assert_uuid.clone(),
                target_port_id: "in".to_string(),
            },
            Connection {
                id: "c2".to_string(),
                source_node_uuid: assert_uuid.clone(),
                source_port_id: "true".to_string(),
                target_node_uuid: extractor_node.uuid.clone(),
                target_port_id: "in".to_string(),
            },
        ];

        let mut node_map = HashMap::new();
        let http_node = Node {
            uuid: http_uuid.clone(),
            node_type: "http:request".to_string(),
            name: "HTTP 接口".to_string(),
            position: flow_engine_core::node::NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let assert_node = Node {
            uuid: assert_uuid.clone(),
            node_type: "assertion".to_string(),
            name: "断言校验".to_string(),
            position: flow_engine_core::node::NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::assertion(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        node_map.insert(http_uuid, &http_node);
        node_map.insert(assert_uuid, &assert_node);

        let executor = VariableExtractorExecutor;
        let result = executor
            .execute(&extractor_node, &mut context, &connections, &node_map)
            .await;

        assert!(result.is_ok(), "自动回溯应成功提取: {:?}", result.err());
        let token_var = context
            .variables
            .variables
            .iter()
            .find(|v| v.name == "auto_token");
        assert!(token_var.is_some());
        assert_eq!(
            token_var.unwrap_or_else(|| panic!("expected Some")).value,
            serde_json::json!("traced-token-abc")
        );
    }
}
