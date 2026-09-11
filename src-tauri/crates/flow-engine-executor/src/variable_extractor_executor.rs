// 变量提取节点执行器 — 从上游输出中提取变量写入执行上下文

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use std::collections::HashMap;

/// 变量提取节点执行器
///
/// 从 `last_node_output` 或指定 `source_node_id` 的执行报告中提取数据，
/// 通过 `output_mappings` 映射为上下文变量。
pub struct VariableExtractorExecutor;

#[async_trait::async_trait]
impl NodeExecutor for VariableExtractorExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
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
        let output = if let Some(ref source_node_id) = config.source_node_id {
            // 从指定节点的执行报告中读取输出
            context
                .node_reports
                .iter()
                .find(|r| r.node_uuid == *source_node_id)
                .and_then(|r| r.output_data.clone())
                .ok_or_else(|| {
                    ErrorDetail::execution_error(format!(
                        "execution result not found for node {}",
                        source_node_id
                    ))
                })?
        } else {
            // 从 last_node_output 读取
            context.last_node_output.clone().ok_or_else(|| {
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
            node_type: "variable_extractor".to_string(),
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
            r#type: "variable_extractor".to_string(),
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
            r#type: "variable_extractor".to_string(),
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
            r#type: "variable_extractor".to_string(),
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
            r#type: "variable_extractor".to_string(),
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
            node_type: "http".to_string(),
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
            r#type: "variable_extractor".to_string(),
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
}
