//! 断言节点执行器
//!
//! 实现断言节点的执行逻辑，支持：
//! - 预设断言模板
//! - 15 种操作符
//! - all/any 组合模式
//! - 双端口路由（true/false）
//! - 调试事件发送

use crate::assertion::evaluator::AssertionEvaluator;
use crate::assertion::preset::get_preset;
use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::debug_types::DebugEvent;
use flow_engine_core::executor::{NodeExecutor, replace_variables};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{Assertion, AssertionConfig, AssertionOutput, NodeData, OnFailureMode};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::time::Instant;

/// 断言节点执行器
pub struct AssertionNodeExecutor;

impl Default for AssertionNodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl AssertionNodeExecutor {
    /// 创建新的断言节点执行器
    pub fn new() -> Self {
        Self
    }

    /// 发送调试事件（如果调试模式启用）
    fn emit_debug_event(context: &ExecutionContext, event: DebugEvent) {
        if let Some(sender) = &context.debug_event_sender {
            if let Err(e) = sender.send_event(event) {
                log::warn!(
                    "[流程 {}] 发送调试事件失败: {}",
                    context.flow_uuid,
                    e
                );
            }
        }
    }
}

#[async_trait::async_trait]
impl NodeExecutor for AssertionNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let start_time = Instant::now();
        log::info!(
            "[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: assertion",
            context.flow_uuid,
            node.uuid,
            node.name
        );

        // 发送执行开始日志
        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("断言节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 1. 解析 AssertionConfig from node.data
        let assertion_config = match &node.data {
            NodeData::Assertion(config) => config.clone(),
            _ => {
                let error_msg = format!("节点数据类型不是 AssertionConfig: {}", node.uuid);
                log::error!(
                    "[流程 {}] [错误] 节点UUID: {}, 节点名称: {}, 错误: {}",
                    context.flow_uuid,
                    node.uuid,
                    node.name,
                    error_msg
                );
                return Err(error_msg);
            }
        };

        // 1.5. 变量替换：对断言配置中的 target/expected/description 执行变量插值
        let assertion_config = resolve_assertion_config_variables(&assertion_config, context);

        // 2. 解析数据来源：若指定了 source_node_id，从该节点的执行报告中读取输出；
        //    否则使用 last_node_output（最近上游节点输出）
        let input: serde_json::Value = if let Some(ref source_node_id) = assertion_config.source_node_id {
            context
                .node_reports
                .iter()
                .find(|r| r.node_uuid == *source_node_id)
                .and_then(|r| r.output_data.clone())
                .unwrap_or_else(|| {
                    log::warn!(
                        "[流程 {}] 节点UUID: {}, 未找到指定来源节点 {} 的输出数据，使用空对象",
                        context.flow_uuid,
                        node.uuid,
                        source_node_id
                    );
                    json!({})
                })
        } else {
            context.last_node_output.clone().unwrap_or_else(|| json!({}))
        };

        // 3. 合并预设和自定义断言
        let merged_config = self.merge_preset_and_custom(&assertion_config);

        // 检查是否有有效的断言
        let enabled_count = merged_config
            .assertions
            .iter()
            .filter(|a| a.enabled)
            .count();

        // 发送断言开始调试事件
        Self::emit_debug_event(
            context,
            DebugEvent::AssertionStarted {
                node_uuid: node.uuid.clone(),
                node_name: node.name.clone(),
                total_assertions: enabled_count,
                mode: format!("{:?}", assertion_config.mode).to_lowercase(),
            },
        );

        // 4. 空断言列表或全禁用断言：Pass + warning
        if enabled_count == 0 {
            let warning_msg = if merged_config.assertions.is_empty() {
                "断言列表为空，自动通过"
            } else {
                "所有断言已禁用，自动通过"
            };

            log::warn!(
                "[流程 {}] [警告] 节点UUID: {}, 节点名称: {}, {}",
                context.flow_uuid,
                node.uuid,
                node.name,
                warning_msg
            );

            context
                .emit_log(LogEvent {
                    level: LogLevel::Warn,
                    message: format!("断言节点 {}: {}", node.name, warning_msg),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                })
                .await;

            // 构建输出（空断言通过）
            let output = AssertionOutput {
                passed: true,
                total: 0,
                passed_count: 0,
                failed_count: 0,
                results: vec![],
                failed_assertions: vec![],
                input,
            };

            let output_data = serde_json::to_value(&output).unwrap_or(serde_json::Value::Null);

            // 发送断言完成调试事件（空断言）
            Self::emit_debug_event(
                context,
                DebugEvent::AssertionCompleted {
                    node_uuid: node.uuid.clone(),
                    node_name: node.name.clone(),
                    passed: true,
                    passed_count: 0,
                    failed_count: 0,
                    total: 0,
                    failed_assertions: vec![],
                },
            );

            // 路由到 true 端口
            let next_node_ids = self.find_next_nodes(connections, &node.uuid, true);

            log::info!(
                "[流程 {}] [完成] 节点UUID: {}, 耗时: {}ms, 结果: passed, 端口: true",
                context.flow_uuid,
                node.uuid,
                start_time.elapsed().as_millis()
            );

            return Ok((next_node_ids, Some(output_data)));
        }

        // 5. 执行断言（使用 AssertionEvaluator）
        let assertion_output = AssertionEvaluator::evaluate_all(&merged_config, &input);

        // 6. 计算 all/any 模式结果
        let passed = assertion_output.passed;

        // 发送断言结果日志
        let result_msg = format!(
            "断言结果: {} (通过 {}/{})",
            if passed { "通过" } else { "失败" },
            assertion_output.passed_count,
            assertion_output.total
        );

        context
            .emit_log(LogEvent {
                level: if passed {
                    LogLevel::Info
                } else {
                    LogLevel::Warn
                },
                message: format!("断言节点 {}: {}", node.name, result_msg),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        // 发送断言完成调试事件
        Self::emit_debug_event(
            context,
            DebugEvent::AssertionCompleted {
                node_uuid: node.uuid.clone(),
                node_name: node.name.clone(),
                passed: assertion_output.passed,
                passed_count: assertion_output.passed_count,
                failed_count: assertion_output.failed_count,
                total: assertion_output.total,
                failed_assertions: assertion_output.failed_assertions.clone(),
            },
        );

        // 7. 处理失败行为
        if !passed {
            match assertion_config.on_failure {
                OnFailureMode::Throw => {
                    // 抛出错误
                    let error_msg = self.build_failure_message(&assertion_output);
                    log::error!(
                        "[流程 {}] [错误] 节点UUID: {}, 节点名称: {}, 断言失败: {}",
                        context.flow_uuid,
                        node.uuid,
                        node.name,
                        error_msg
                    );

                    context
                        .emit_log(LogEvent {
                            level: LogLevel::Error,
                            message: format!("断言节点 {} 失败: {}", node.name, error_msg),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                            node_uuid: Some(node.uuid.clone()),
                            node_name: Some(node.name.clone()),
                            flow_uuid: context.flow_uuid.clone(),
                        })
                        .await;

                    return Err(error_msg);
                }
                OnFailureMode::RouteFalse => {
                    // 路由到 false 端口
                    log::info!(
                        "[流程 {}] [路由] 节点UUID: {}, 断言失败，路由到 false 端口",
                        context.flow_uuid,
                        node.uuid
                    );
                }
            }
        }

        // 8. 构建输出数据
        let output_data = serde_json::to_value(&assertion_output).unwrap_or(serde_json::Value::Null);

        // 9. 根据结果路由到对应端口
        let next_node_ids = self.find_next_nodes(connections, &node.uuid, passed);

        let duration_ms = start_time.elapsed().as_millis();
        log::info!(
            "[流程 {}] [完成] 节点UUID: {}, 耗时: {}ms, 结果: {}, 端口: {}",
            context.flow_uuid,
            node.uuid,
            duration_ms,
            if passed { "passed" } else { "failed" },
            if passed { "true" } else { "false" }
        );

        Ok((next_node_ids, Some(output_data)))
    }
}

impl AssertionNodeExecutor {
    /// 合并预设和自定义断言
    ///
    /// 合并顺序：preset first, then custom
    fn merge_preset_and_custom(&self, config: &AssertionConfig) -> AssertionConfig {
        let mut merged_assertions = Vec::new();

        // 1. 先添加预设断言（如果存在）
        if let Some(preset_name) = &config.preset {
            if let Some(preset_assertions) = get_preset(preset_name) {
                log::info!("[断言] 加载预设: {}", preset_name);
                merged_assertions.extend(preset_assertions);
            } else {
                log::warn!("[断言] 预设不存在: {}", preset_name);
            }
        }

        // 2. 再添加自定义断言
        merged_assertions.extend(config.assertions.clone());

        AssertionConfig {
            preset: config.preset.clone(),
            mode: config.mode.clone(),
            on_failure: config.on_failure.clone(),
            continue_on_failure: config.continue_on_failure,
            assertions: merged_assertions,
            source_node_id: config.source_node_id.clone(),
        }
    }

    /// 查找下一个节点
    ///
    /// 根据断言结果选择 true 或 false 端口
    fn find_next_nodes(
        &self,
        connections: &[Connection],
        node_uuid: &str,
        passed: bool,
    ) -> Vec<String> {
        let port_id = if passed { "true" } else { "false" };

        connections
            .iter()
            .filter(|conn| conn.source_node_uuid == node_uuid && conn.source_port_id == port_id)
            .map(|conn| conn.target_node_uuid.clone())
            .collect()
    }

    /// 构建失败消息
    fn build_failure_message(&self, output: &AssertionOutput) -> String {
        if output.failed_assertions.is_empty() {
            return format!(
                "断言失败 (模式: all, 通过 {}/{})",
                output.passed_count, output.total
            );
        }

        let failed_details: Vec<String> = output
            .failed_assertions
            .iter()
            .map(|r| {
                let desc = r
                    .description
                    .as_ref()
                    .map(|d| format!("{}: ", d))
                    .unwrap_or_default();
                if let Some(error) = &r.error {
                    format!("{}[{}] {}", desc, r.id, error)
                } else {
                    format!(
                        "{}[{}] {} {} {} (实际: {})",
                        desc,
                        r.id,
                        r.target,
                        r.operator,
                        r.expected,
                        r.actual
                    )
                }
            })
            .collect();

        format!(
            "断言失败 (模式: {}, 通过 {}/{}): {}",
            if output.passed_count > 0 {
                "any"
            } else {
                "all"
            },
            output.passed_count,
            output.total,
            failed_details.join("; ")
        )
    }
}

// ============================================================================
// 变量替换辅助函数
// ============================================================================

/// 替换断言配置中的变量（target / expected / description）
///
/// 对每个启用的断言：
/// - `target`: 字符串变量替换（`{{var}}` → 字符串）
/// - `expected`: JSON 变量替换（精确 `{{var}}` 引用保留原始 JSON 类型，部分替换得字符串）
/// - `description`: 字符串变量替换
fn resolve_assertion_config_variables(
    config: &AssertionConfig,
    context: &ExecutionContext,
) -> AssertionConfig {
    let assertions: Vec<Assertion> = config
        .assertions
        .iter()
        .map(|assertion| {
            if !assertion.enabled {
                return assertion.clone();
            }

            let mut a = assertion.clone();

            // 替换 target (JSONPath 表达式)
            if let Ok(resolved) = replace_variables(&a.target, context) {
                a.target = resolved;
            }

            // 替换 expected（保留 JSON 类型）
            a.expected = resolve_expected_variables(&a.expected, context);

            // 替换 description
            if let Some(ref desc) = a.description {
                if let Ok(resolved) = replace_variables(desc, context) {
                    a.description = Some(resolved);
                }
            }

            a
        })
        .collect();

    AssertionConfig {
        assertions,
        ..config.clone()
    }
}

/// 递归解析期望值中的变量占位符，尽可能保留原始 JSON 类型
///
/// - 精确 `{{var}}` → 从上下文中查找变量并返回原始 JSON 值，保留类型
/// - 含 `{{var}}` 的部分字符串 → 进行字符串替换，结果为字符串
/// - 对象/数组 → 递归处理每个字段/元素
/// - 数值/布尔/null → 保持不变
fn resolve_expected_variables(value: &Value, context: &ExecutionContext) -> Value {
    match value {
        // 精确变量引用 `{{name}}` → 获取原始 JSON 值（保留类型）
        Value::String(s) if is_exact_variable_ref(s) => {
            let var_path = &s[2..s.len() - 2].trim();
            resolve_json_value(var_path, context).unwrap_or_else(|| value.clone())
        }
        // 含 {{var}} 的字符串 → 字符串替换
        Value::String(s) => replace_variables(s, context)
            .map(Value::String)
            .unwrap_or_else(|_| value.clone()),
        // 对象 → 递归处理
        Value::Object(map) => {
            let resolved: Map<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), resolve_expected_variables(v, context)))
                .collect();
            Value::Object(resolved)
        }
        // 数组 → 递归处理
        Value::Array(arr) => {
            let resolved: Vec<Value> =
                arr.iter().map(|v| resolve_expected_variables(v, context)).collect();
            Value::Array(resolved)
        }
        // 数值/布尔/null → 保持不变
        _ => value.clone(),
    }
}

/// 判断字符串是否是精确的变量引用 `{{name}}`（不含嵌套 `{{}}`）
fn is_exact_variable_ref(s: &str) -> bool {
    s.len() > 4 && s.starts_with("{{") && s.ends_with("}}") && !s[2..s.len() - 2].contains("{{")
}

/// 从上下文中按点号分隔路径查找变量的原始 JSON 值
fn resolve_json_value(path: &str, context: &ExecutionContext) -> Option<Value> {
    let merged_vars = context.variables.get_merged_variables();
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = merged_vars.get(parts[0])?;
    for part in &parts[1..] {
        current = current.as_object()?.get(*part)?;
    }
    Some(current.clone())
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::debug_types::DebugEvent;
    use flow_engine_core::node::{NodePosition};
    use flow_engine_core::node_config::{
        Assertion, AssertionMode, AssertionOperator, OnFailureMode,
    };
    use flow_engine_core::traits::DebugEventSender;
    use std::sync::Arc;

    /// mpsc UnboundedSender adapter for DebugEventSender (测试用)
    struct MpscDebugEventSender {
        sender: tokio::sync::mpsc::UnboundedSender<DebugEvent>,
    }

    impl MpscDebugEventSender {
        fn new(sender: tokio::sync::mpsc::UnboundedSender<DebugEvent>) -> Self {
            Self { sender }
        }
    }

    impl DebugEventSender for MpscDebugEventSender {
        fn send_event(&self, event: DebugEvent) -> Result<(), String> {
            self.sender
                .send(event)
                .map_err(|e| format!("Failed to send debug event: {}", e))
        }
    }

    /// 创建测试上下文
    fn create_context(flow_id: &str) -> ExecutionContext {
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
            current_node_uuid: None,
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            grpc_manager: None,
            mcp_manager: None,
            tcp_manager: None,
            udp_manager: None,
            mqtt_manager: None,
            kafka_manager: None,
            sse_manager: None,
            graphql_manager: None,
            amqp_manager: None,
            smtp_manager: None,
            imap_manager: None, redis_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    /// 创建断言节点
    fn create_assertion_node(uuid: &str, config: AssertionConfig) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "assertion".to_string(),
            name: "断言节点".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Assertion(config),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建单条断言
    fn create_assertion(
        id: &str,
        target: &str,
        operator: AssertionOperator,
        expected: serde_json::Value,
    ) -> Assertion {
        Assertion {
            id: id.to_string(),
            description: None,
            target: target.to_string(),
            operator,
            expected,
            enabled: true,
        }
    }

    // ========================================================================
    // 基础测试
    // ========================================================================

    #[tokio::test]
    async fn test_eq_operator_pass() {
        let mut context = create_context("test-flow");
        context.last_node_output = Some(json!({"status": 200}));

        let assertions = vec![create_assertion(
            "a1",
            "$.status",
            AssertionOperator::Eq,
            json!(200),
        )];
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output) = result.unwrap();
        assert!(next_nodes.is_empty()); // 没有连接
        assert!(output.is_some());

        let output: AssertionOutput =
            serde_json::from_value(output.unwrap()).expect("输出反序列化失败");
        assert!(output.passed);
        assert_eq!(output.passed_count, 1);
        assert_eq!(output.failed_count, 0);
    }

    #[tokio::test]
    async fn test_eq_operator_fail() {
        let mut context = create_context("test-flow");
        context.last_node_output = Some(json!({"status": 404}));

        let assertions = vec![create_assertion(
            "a1",
            "$.status",
            AssertionOperator::Eq,
            json!(200),
        )];
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, output) = result.unwrap();
        assert!(next_nodes.is_empty()); // 没有连接，走 false 端口

        let output: AssertionOutput =
            serde_json::from_value(output.unwrap()).expect("输出反序列化失败");
        assert!(!output.passed);
        assert_eq!(output.passed_count, 0);
        assert_eq!(output.failed_count, 1);
    }

    #[tokio::test]
    async fn test_empty_assertions_pass() {
        let mut context = create_context("test-flow");
        context.last_node_output = Some(json!({"status": 200}));

        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions: vec![],
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let output: AssertionOutput =
            serde_json::from_value(result.unwrap().1.unwrap()).expect("输出反序列化失败");
        assert!(output.passed);
        assert_eq!(output.total, 0);
    }

    #[tokio::test]
    async fn test_all_disabled_assertions_pass() {
        let mut context = create_context("test-flow");
        context.last_node_output = Some(json!({"status": 200}));

        let mut assertion = create_assertion("a1", "$.status", AssertionOperator::Eq, json!(999));
        assertion.enabled = false;

        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions: vec![assertion],
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let output: AssertionOutput =
            serde_json::from_value(result.unwrap().1.unwrap()).expect("输出反序列化失败");
        assert!(output.passed);
        assert_eq!(output.total, 0);
    }

    // ========================================================================
    // 双端口路由测试
    // ========================================================================

    #[tokio::test]
    async fn test_dual_routing_true_port() {
        let mut context = create_context("test-flow");
        context.last_node_output = Some(json!({"status": 200}));

        let assertions = vec![create_assertion(
            "a1",
            "$.status",
            AssertionOperator::Eq,
            json!(200),
        )];
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "assertion-1".to_string(),
            target_node_uuid: "next-true".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "in".to_string(),
        }];

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["next-true"]);
    }

    #[tokio::test]
    async fn test_dual_routing_false_port() {
        let mut context = create_context("test-flow");
        context.last_node_output = Some(json!({"status": 404}));

        let assertions = vec![create_assertion(
            "a1",
            "$.status",
            AssertionOperator::Eq,
            json!(200),
        )];
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let connections = vec![
            Connection {
                id: "conn-1".to_string(),
                source_node_uuid: "assertion-1".to_string(),
                target_node_uuid: "next-true".to_string(),
                source_port_id: "true".to_string(),
                target_port_id: "in".to_string(),
            },
            Connection {
                id: "conn-2".to_string(),
                source_node_uuid: "assertion-1".to_string(),
                target_node_uuid: "next-false".to_string(),
                source_port_id: "false".to_string(),
                target_port_id: "in".to_string(),
            },
        ];

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["next-false"]);
    }

    // ========================================================================
    // all/any 模式测试
    // ========================================================================

    #[tokio::test]
    async fn test_mode_all_partial_fail() {
        let mut context = create_context("test-flow");
        context.last_node_output = Some(json!({"status": 200, "count": 3}));

        let assertions = vec![
            create_assertion("a1", "$.status", AssertionOperator::Eq, json!(200)),
            create_assertion("a2", "$.count", AssertionOperator::Gt, json!(5)),
        ];
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let output: AssertionOutput =
            serde_json::from_value(result.unwrap().1.unwrap()).expect("输出反序列化失败");
        assert!(!output.passed); // all 模式，有一个失败则失败
        assert_eq!(output.passed_count, 1);
        assert_eq!(output.failed_count, 1);
    }

    #[tokio::test]
    async fn test_mode_any_pass() {
        let mut context = create_context("test-flow");
        context.last_node_output = Some(json!({"status": 200, "count": 3}));

        let assertions = vec![
            create_assertion("a1", "$.status", AssertionOperator::Eq, json!(999)), // 失败
            create_assertion("a2", "$.count", AssertionOperator::Gt, json!(5)),    // 失败
            create_assertion("a3", "$.status", AssertionOperator::Eq, json!(200)), // 通过
        ];
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::Any,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let output: AssertionOutput =
            serde_json::from_value(result.unwrap().1.unwrap()).expect("输出反序列化失败");
        assert!(output.passed); // any 模式，有一个通过则通过
        assert!(output.passed_count > 0);
    }

    // ========================================================================
    // on_failure 行为测试
    // ========================================================================

    #[tokio::test]
    async fn test_on_failure_throw() {
        let mut context = create_context("test-flow");
        context.last_node_output = Some(json!({"status": 404}));

        let assertions = vec![create_assertion(
            "a1",
            "$.status",
            AssertionOperator::Eq,
            json!(200),
        )];
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: OnFailureMode::Throw,
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.find("断言失败").is_some());
    }

    // ========================================================================
    // 预设测试
    // ========================================================================

    #[tokio::test]
    async fn test_preset_http_success() {
        let mut context = create_context("test-flow");
        context.last_node_output = Some(json!({"status": 200}));

        let config = AssertionConfig {
            preset: Some("http_success".to_string()),
            mode: AssertionMode::All,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions: vec![],
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let output: AssertionOutput =
            serde_json::from_value(result.unwrap().1.unwrap()).expect("输出反序列化失败");
        assert!(output.passed);
    }

    // ========================================================================
    // continue_on_failure 测试
    // ========================================================================

    #[tokio::test]
    async fn test_continue_on_failure() {
        let mut context = create_context("test-flow");
        context.last_node_output = Some(json!({"status": 404, "count": 10}));

        let assertions = vec![
            create_assertion("a1", "$.status", AssertionOperator::Eq, json!(200)), // 失败
            create_assertion("a2", "$.count", AssertionOperator::Gt, json!(5)),    // 通过
        ];
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: true, // 继续执行
            assertions,
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let output: AssertionOutput =
            serde_json::from_value(result.unwrap().1.unwrap()).expect("输出反序列化失败");
        // 两个断言都执行了
        assert_eq!(output.total, 2);
        assert_eq!(output.passed_count, 1);
        assert_eq!(output.failed_count, 1);
    }

    // ========================================================================
    // 调试事件测试
    // ========================================================================

    /// 创建带调试事件发送器的测试上下文
    fn create_context_with_debug_sender(
        flow_id: &str,
    ) -> (
        ExecutionContext,
        tokio::sync::mpsc::UnboundedReceiver<DebugEvent>,
    ) {
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let context = ExecutionContext {
            flow_uuid: flow_id.to_string(),
            current_node_uuid: None,
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            grpc_manager: None,
            mcp_manager: None,
            tcp_manager: None,
            udp_manager: None,
            mqtt_manager: None,
            kafka_manager: None,
            sse_manager: None,
            graphql_manager: None,
            amqp_manager: None,
            smtp_manager: None,
            imap_manager: None, redis_manager: None,
            last_node_output: None,
            debug_event_sender: Some(Arc::new(MpscDebugEventSender::new(event_sender))),
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        };
        (context, event_receiver)
    }

    #[tokio::test]
    async fn test_debug_events_on_pass() {
        let (mut context, mut event_receiver) = create_context_with_debug_sender("test-flow");
        context.last_node_output = Some(json!({"status": 200}));

        let assertions = vec![create_assertion(
            "a1",
            "$.status",
            AssertionOperator::Eq,
            json!(200),
        )];
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());

        // 收集调试事件
        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        // 验证收到两个事件：AssertionStarted 和 AssertionCompleted
        assert_eq!(events.len(), 2);

        // 验证 AssertionStarted 事件
        if let DebugEvent::AssertionStarted {
            node_uuid,
            node_name,
            total_assertions,
            mode,
        } = &events[0]
        {
            assert_eq!(node_uuid, "assertion-1");
            assert_eq!(node_name, "断言节点");
            assert_eq!(*total_assertions, 1);
            assert_eq!(mode, "all");
        } else {
            panic!("Expected AssertionStarted event, got: {:?}", events[0]);
        }

        // 验证 AssertionCompleted 事件
        if let DebugEvent::AssertionCompleted {
            node_uuid,
            passed,
            passed_count,
            failed_count,
            total,
            ..
        } = &events[1]
        {
            assert_eq!(node_uuid, "assertion-1");
            assert!(*passed);
            assert_eq!(*passed_count, 1);
            assert_eq!(*failed_count, 0);
            assert_eq!(*total, 1);
        } else {
            panic!("Expected AssertionCompleted event, got: {:?}", events[1]);
        }
    }

    #[tokio::test]
    async fn test_debug_events_on_fail() {
        let (mut context, mut event_receiver) = create_context_with_debug_sender("test-flow");
        context.last_node_output = Some(json!({"status": 404}));

        let assertions = vec![create_assertion(
            "a1",
            "$.status",
            AssertionOperator::Eq,
            json!(200),
        )];
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());

        // 收集调试事件
        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        // 验证收到两个事件：AssertionStarted 和 AssertionCompleted
        assert_eq!(events.len(), 2);

        // 验证 AssertionCompleted 事件（失败）
        if let DebugEvent::AssertionCompleted {
            node_uuid,
            passed,
            passed_count,
            failed_count,
            total,
            failed_assertions,
            ..
        } = &events[1]
        {
            assert_eq!(node_uuid, "assertion-1");
            assert!(!(*passed));
            assert_eq!(*passed_count, 0);
            assert_eq!(*failed_count, 1);
            assert_eq!(*total, 1);
            assert_eq!(failed_assertions.len(), 1);
            assert_eq!(failed_assertions[0].id, "a1");
        } else {
            panic!("Expected AssertionCompleted event, got: {:?}", events[1]);
        }
    }

    #[tokio::test]
    async fn test_debug_events_empty_assertions() {
        let (mut context, mut event_receiver) = create_context_with_debug_sender("test-flow");
        context.last_node_output = Some(json!({"status": 200}));

        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions: vec![],
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());

        // 收集调试事件
        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        // 验证收到两个事件：AssertionStarted 和 AssertionCompleted
        assert_eq!(events.len(), 2);

        // 验证 AssertionStarted 事件（total_assertions = 0）
        if let DebugEvent::AssertionStarted { total_assertions, .. } = &events[0] {
            assert_eq!(*total_assertions, 0);
        } else {
            panic!("Expected AssertionStarted event, got: {:?}", events[0]);
        }

        // 验证 AssertionCompleted 事件（空断言通过）
        if let DebugEvent::AssertionCompleted {
            passed,
            total,
            passed_count,
            failed_count,
            ..
        } = &events[1]
        {
            assert!(*passed);
            assert_eq!(*total, 0);
            assert_eq!(*passed_count, 0);
            assert_eq!(*failed_count, 0);
        } else {
            panic!("Expected AssertionCompleted event, got: {:?}", events[1]);
        }
    }

    #[tokio::test]
    async fn test_debug_events_mode_any() {
        let (mut context, mut event_receiver) = create_context_with_debug_sender("test-flow");
        context.last_node_output = Some(json!({"status": 200}));

        let assertions = vec![
            create_assertion("a1", "$.status", AssertionOperator::Eq, json!(404)), // 失败
            create_assertion("a2", "$.status", AssertionOperator::Eq, json!(200)), // 通过
        ];
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::Any, // any 模式
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        assert!(result.is_ok());

        // 收集调试事件
        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        // 验证 AssertionStarted 事件（mode = any）
        if let DebugEvent::AssertionStarted { mode, .. } = &events[0] {
            assert_eq!(mode, "any");
        } else {
            panic!("Expected AssertionStarted event, got: {:?}", events[0]);
        }

        // 验证 AssertionCompleted 事件（any 模式下有一个通过则整体通过）
        if let DebugEvent::AssertionCompleted { passed, .. } = &events[1] {
            assert!(*passed); // any 模式，有一个通过则整体通过
        } else {
            panic!("Expected AssertionCompleted event, got: {:?}", events[1]);
        }
    }

    #[tokio::test]
    async fn test_no_debug_events_without_sender() {
        // 没有调试事件发送器的上下文
        let mut context = create_context("test-flow");
        context.last_node_output = Some(json!({"status": 200}));

        let assertions = vec![create_assertion(
            "a1",
            "$.status",
            AssertionOperator::Eq,
            json!(200),
        )];
        let config = AssertionConfig {
            preset: None,
            mode: AssertionMode::All,
            on_failure: OnFailureMode::RouteFalse,
            continue_on_failure: false,
            assertions,
            source_node_id: None,
        };
        let node = create_assertion_node("assertion-1", config);

        let executor = AssertionNodeExecutor::new();
        let result = executor
            .execute(&node, &mut context, &[], &HashMap::new())
            .await;

        // 执行应该成功，即使没有调试事件发送器
        assert!(result.is_ok());
    }
}
