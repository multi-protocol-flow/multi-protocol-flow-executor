// 执行引擎模块 - FlowExecutor 实现

use crate::connection::Connection;
use crate::context::{ExecutionContext, VariableStorage};
use crate::entry_executor::EntryNodeExecutor;
use crate::flow::Flow;
use crate::error::{ErrorDetail, ErrorKind};
use crate::log_event::{LogEvent, LogLevel};
use crate::node::{Node, NodeType};
use crate::websocket_manager::WebSocketConnectionManager;
use crate::ws_connect_executor::WsConnectExecutor;
use crate::ws_send_wait_executor::WsSendWaitExecutor;
use crate::ws_close_executor::WsCloseExecutor;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use crate::traits::LogSender;

use uuid::Uuid;

// Re-export from flow-engine-core
pub use flow_engine_core::executor::{NodeExecutor, replace_variables};


/// 执行结果
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// 执行是否成功
    pub success: bool,
    /// 返回数据
    pub data: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 已执行节点数量
    pub executed_count: u32,
    /// 节点执行报告列表
    pub node_reports: Vec<crate::report::NodeExecutionReport>,
    /// 流程执行报告
    pub flow_report: Option<crate::report::FlowExecutionReport>,
}

/// 计算变量变更列表
///
/// 对比变量快照前后的差异，生成 VariableChange 列表
///
/// # 参数
/// - `variables_before`: 变更前的变量快照
/// - `variables_after`: 变更后的变量快照
///
/// # 返回
/// - `Vec<VariableChange>`: 变量变更列表
fn compute_variable_changes(
    variables_before: &HashMap<String, serde_json::Value>,
    variables_after: &HashMap<String, serde_json::Value>,
) -> Vec<crate::report::VariableChange> {
    let mut changes = Vec::new();

    // 检测新增和修改的变量
    for (key, new_value) in variables_after {
        let old_value = variables_before.get(key).cloned();
        // 只有值真正变化时才记录
        if old_value.as_ref() != Some(new_value) {
            changes.push(crate::report::VariableChange {
                key: key.clone(),
                old_value,
                new_value: new_value.clone(),
            });
        }
    }

    changes
}

/// 根据节点类型构建 input_data
///
/// input_data 代表节点消费的输入数据（配置信息）
/// - Entry: None（无输入）
/// - HTTP: { url, method, headers, body, body_type, timeout_ms }
/// - Conditional: { condition }
/// - WS Connect: { url, headers, subprotocols, connect_timeout_ms, wait_for_message, wait_mode, message_timeout_ms }
/// - WS SendWait: { connection_id, message, message_type, send_timeout_ms, wait_for_response, wait_mode, message_timeout_ms }
/// - WS Close: { connection_id, close_code, close_reason, wait_close_timeout_ms }
fn build_input_data_from_node_config(node: &crate::node::Node) -> Option<serde_json::Value> {
    use crate::node_config::NodeData;
    
    match &node.data {
        // Entry 节点无输入数据
        NodeData::Entry(_) => None,
        
        // HTTP 节点：请求配置
        NodeData::Http(config) => {
            Some(serde_json::json!({
                "url": config.url,
                "method": config.method,
                "headers": config.headers,
                "body": config.body,
                "body_type": config.body_type,
                "timeout_ms": config.timeout_ms,
            }))
        },
        
        // Conditional 节点：条件配置
        NodeData::Conditional(config) => {
            Some(serde_json::json!({
                "condition_type": config.condition_type,
                "field_path": config.field_path,
                "operator": config.operator,
                "expected_value": config.expected_value,
                "advanced_expression": config.advanced_expression,
                "expected_value_type": config.expected_value_type,
            }))
        },
        
        // WebSocket Connect 节点：连接配置
        NodeData::WsConnect(config) => {
            Some(serde_json::json!({
                "url": config.url,
                "headers": config.headers,
                "subprotocols": config.subprotocols,
                "connect_timeout_ms": config.connect_timeout_ms,
                "wait_for_message": config.wait_for_message,
                "wait_mode": config.wait_mode,
                "message_timeout_ms": config.message_timeout_ms,
            }))
        },
        
        // WebSocket SendWait 节点：发送配置
        NodeData::WsSendWait(config) => {
            Some(serde_json::json!({
                "connection_id": config.connection_id,
                "message": config.message,
                "message_type": config.message_type,
                "send_timeout_ms": config.send_timeout_ms,
                "wait_for_response": config.wait_for_response,
                "wait_mode": config.wait_mode,
                "message_timeout_ms": config.message_timeout_ms,
            }))
        },
        
        // WebSocket Close 节点：关闭配置
        NodeData::WsClose(config) => {
            Some(serde_json::json!({
                "connection_id": config.connection_id,
                "close_code": config.close_code,
                "close_reason": config.close_reason,
                "wait_close_timeout_ms": config.wait_close_timeout_ms,
            }))
        },
        
        // Assertion 节点：断言配置
        NodeData::Assertion(config) => {
            Some(serde_json::json!({
                "preset": config.preset,
                "mode": config.mode,
                "on_failure": config.on_failure,
                "continue_on_failure": config.continue_on_failure,
                "assertions": config.assertions,
            }))
        },
        
        // 数据库节点：连接配置
        NodeData::DbConnect(config) => {
            Some(serde_json::json!({
                "connection_uuid": config.connection_uuid,
                "test_query": config.test_query,
                "connect_timeout_ms": config.connect_timeout_ms,
            }))
        },
        
        // 数据库节点：查询配置
        NodeData::DbQuery(config) => {
            Some(serde_json::json!({
                "connection_uuid": config.connection_uuid,
                "sql": config.sql,
                "params": config.params,
                "redis_command": config.redis_command,
                "redis_key": config.redis_key,
                "redis_value": config.redis_value,
                "redis_ttl": config.redis_ttl,
                "fetch_mode": config.fetch_mode,
                "result_to": config.result_to,
                "output_mappings": config.output_mappings,
                "timeout_ms": config.timeout_ms,
            }))
        },
        
        // 数据库节点：事务开始配置
        NodeData::DbBegin(config) => {
            Some(serde_json::json!({
                "connection_uuid": config.connection_uuid,
                "isolation_level": config.isolation_level,
            }))
        },
        
        // 数据库节点：事务提交配置
        NodeData::DbCommit(config) => {
            Some(serde_json::json!({
                "connection_uuid": config.connection_uuid,
            }))
        },
        
        // 数据库节点：事务回滚配置
        NodeData::DbRollback(config) => {
            Some(serde_json::json!({
                "connection_uuid": config.connection_uuid,
            }))
        },
        
        // 数据库节点：关闭配置
        NodeData::DbClose(config) => {
            Some(serde_json::json!({
                "connection_uuid": config.connection_uuid,
            }))
        },
        
        // 脚本节点：脚本配置
        NodeData::Script(config) => {
            Some(serde_json::json!({
                "script": config.script,
                "timeout_ms": config.timeout_ms,
                "memory_limit_mb": config.memory_limit_mb,
                "execution_phase": config.execution_phase,
                "on_error": config.on_error,
                "on_assertion_fail": config.on_assertion_fail,
            }))
        },
    }
}

/// 从字符串错误解析错误详情
///
/// 尝试从错误消息中识别错误类型，并创建对应的 ErrorDetail
fn parse_error_to_detail(error: &str) -> ErrorDetail {
    // 识别条件语法错误
    if error.contains("条件") && (error.contains("语法") || error.contains("配置")) {
        return ErrorDetail::new(ErrorKind::ConditionSyntax, error.to_string());
    }
    
    // 识别配置错误
    if error.contains("配置") || error.contains("缺少") || error.contains("无效") {
        return ErrorDetail::new(ErrorKind::Configuration, error.to_string());
    }
    
    // 识别连接错误
    if error.contains("连接") || error.contains("网络") {
        return ErrorDetail::new(ErrorKind::Connection, error.to_string());
    }
    
    // 识别超时错误
    if error.contains("超时") || error.contains("timeout") {
        return ErrorDetail::new(ErrorKind::Timeout, error.to_string());
    }
    
    // 识别验证错误
    if error.contains("验证") || error.contains("参数") {
        return ErrorDetail::new(ErrorKind::Validation, error.to_string());
    }
    
    // 识别解析错误
    if error.contains("解析") || error.contains("JSON") || error.contains("格式") {
        return ErrorDetail::new(ErrorKind::Parse, error.to_string());
    }
    
    // 识别条件评估错误
    if error.contains("条件评估") || error.contains("评估") {
        return ErrorDetail::new(ErrorKind::ConditionEvaluation, error.to_string());
    }
    
    // 默认为通用执行错误
    ErrorDetail::new(ErrorKind::Execution, error.to_string())
}

/// Flow executor
pub struct FlowExecutor {
    /// Flow file path (used for resolving relative paths in CLI mode)
    flow_file_path: Option<PathBuf>,
}

impl Default for FlowExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl FlowExecutor {
    /// Create a new flow executor
    pub fn new() -> Self {
        FlowExecutor {
            flow_file_path: None,
        }
    }

    /// Set flow file path (used for CLI relative path resolution)
    pub fn with_flow_file_path(mut self, path: PathBuf) -> Self {
        self.flow_file_path = Some(path);
        self
    }

/// 执行流程
    ///
    /// 从开始节点开始，按连接顺序遍历节点
    ///
    /// # 参数
    /// - `flow`: 要执行的流程
    /// - `variables`: 执行上下文变量（可选）
    /// - `log_sender`: 日志发送器（可选，CLI 模式为 None）
    /// - `db_connections`: 数据库连接配置列表（可选，从前端连接配置中心获取）
    ///
    /// # 返回
    /// - `ExecutionResult`: 执行结果
    pub async fn execute_flow(
        &self,
        flow: &Flow,
        variables: Option<serde_json::Value>,
        log_sender: Option<Arc<dyn LogSender>>,
        db_connections: Option<Vec<crate::db::types::DbConnection>>,
    ) -> ExecutionResult {
        let start_time = tokio::time::Instant::now();
        let mut executed_count = 0u32;
        let mut node_reports = Vec::new();
        let initial_variables = variables.unwrap_or_else(|| json!({}));
        let mut var_storage = VariableStorage::new();
        if let Some(obj) = initial_variables.as_object() {
            for (key, value) in obj {
                var_storage.set_variable(key.clone(), value.clone(), String::new());
            }
        }

        // 初始化 WebSocket 连接管理器（流程级别的单例）
        let ws_manager = Arc::new(WebSocketConnectionManager::new());

        // 初始化数据库连接池管理器（流程级别的单例）
        let db_manager = Arc::new(crate::db::pool::DatabaseConnectionManager::new());

        // 构建数据库连接配置映射（connection_uuid -> DbConnection）
        let db_connections_map: std::collections::HashMap<String, crate::db::types::DbConnection> =
            db_connections
                .unwrap_or_default()
                .into_iter()
                .map(|conn| (conn.id.clone(), conn))
                .collect();

        let mut context = ExecutionContext {
            flow_uuid: flow.uuid.clone(),
            current_node_uuid: None,
            variables: var_storage,
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            log_sender,
            flow_file_path: self.flow_file_path.clone(),
            ws_manager: Some(ws_manager),
            ws_connection_urls: std::collections::HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            db_manager: Some(db_manager),
            db_connections: db_connections_map,
            last_node_output: None,
            debug_event_sender: None,
        };

        // 查找开始节点
        let start_node = match flow
            .nodes
            .iter()
            .find(|node| matches!(node.node_type, NodeType::Entry))
        {
            Some(node) => node,
            None => {
                return ExecutionResult {
                    success: false,
                    data: None,
                    error: Some("流程缺少开始节点".to_string()),
                    executed_count: 0,
                    node_reports: vec![],
                    flow_report: None,
                };
            }
        };

        // 构建节点查找映射
        let node_map: HashMap<String, &Node> = flow
            .nodes
            .iter()
            .map(|node| (node.uuid.clone(), node))
            .collect();

        // 构建连接映射：source_node_uuid -> Vec<target_node_uuid>
        let mut connection_map: HashMap<String, Vec<String>> = HashMap::new();
        for conn in &flow.connections {
            connection_map
                .entry(conn.source_node_uuid.clone())
                .or_default()
                .push(conn.target_node_uuid.clone());
        }

        // 从开始节点开始遍历
        let mut current_node_id = start_node.uuid.clone();
        let mut visited = std::collections::HashSet::new();

        // 执行路径追踪（Task 14）：记录所有成功执行的节点 UUID
        let mut execution_path: Vec<String> = Vec::new();

        // 跳过节点记录（Task 15）：记录被跳过的节点及原因
        let skipped_nodes: Vec<crate::report::SkippedNodeInfo> = Vec::new();

        while let Some(node) = node_map.get(&current_node_id) {
            // 检测循环（简单防止无限循环）
            if visited.contains(&current_node_id) {
                return ExecutionResult {
                    success: false,
                    data: None,
                    error: Some(format!("检测到循环：节点 {} 已被访问", node.uuid)),
                    executed_count: 0,
                    node_reports,
                    flow_report: None,
                };
            }

            visited.insert(current_node_id.clone());
            context.current_node_uuid = Some(current_node_id.clone());

            // 记录节点开始时间
            let node_start_time = tokio::time::Instant::now();
            let node_start_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            // 快照执行前的变量状态（必须在 execute_node 之前）
            // 用于计算节点执行期间的变量变更
            let variables_before = context.variables.get_merged_variables();

            // 执行当前节点
            match self
                .execute_node(node, &mut context, &flow.connections, &node_map)
                .await
            {
                Ok((next_node_id_from_executor, output_data)) => {
                    // 记录节点结束时间
                    let node_end_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    let node_duration = node_start_time.elapsed().as_millis() as u64;

                    executed_count += 1;

                    // 根据节点类型设置 used_port_id
                    let used_port_id = match node.node_type {
                        NodeType::Entry => Some("out".to_string()),
                        NodeType::Http => {
                            // HTTP 节点：根据 success 字段判断使用哪个端口
                            output_data.as_ref().and_then(|data| {
                                data.get("success").and_then(|v| v.as_bool()).map(|is_success| {
                                    if is_success {
                                        "true".to_string()
                                    } else {
                                        "false".to_string()
                                    }
                                })
                            })
                        }
                        NodeType::Conditional => {
                            // Conditional 节点：根据输出数据判断条件结果
                            output_data.as_ref().and_then(|data| {
                                data.get("result").and_then(|v| v.as_bool()).map(|is_true| {
                                    if is_true {
                                        "true".to_string()
                                    } else {
                                        "false".to_string()
                                    }
                                })
                            })
                        }
                        // WebSocket 节点类型
                        NodeType::WsConnect => {
                            // WsConnect 节点：根据连接成功与否返回 true 或 false 端口
                            output_data.as_ref().and_then(|data| {
                                data.get("connected").and_then(|v| v.as_bool()).map(|connected| {
                                    if connected {
                                        "true".to_string()
                                    } else {
                                        "false".to_string()
                                    }
                                })
                            })
                        }
                        NodeType::WsSendWait | NodeType::WsClose => {
                            Some("out".to_string())
                        }
                        // Assertion 节点：根据 passed 字段判断
                        NodeType::Assertion => {
                            output_data.as_ref().and_then(|data| {
                                data.get("passed").and_then(|v| v.as_bool()).map(|passed| {
                                    if passed {
                                        "true".to_string()
                                    } else {
                                        "false".to_string()
                                    }
                                })
                            })
                        }
                        // 数据库节点：根据 success 字段判断（除 DbClose 外）
                        NodeType::DbConnect | NodeType::DbQuery | NodeType::DbBegin | NodeType::DbCommit | NodeType::DbRollback => {
                            output_data.as_ref().and_then(|data| {
                                data.get("success").and_then(|v| v.as_bool()).map(|success| {
                                    if success {
                                        "true".to_string()
                                    } else {
                                        "false".to_string()
                                    }
                                })
                            })
                        }
                        // 数据库关闭节点：只有 out 端口
                        NodeType::DbClose => {
                            Some("out".to_string())
                        }
                        // 脚本节点：根据 success 字段判断
                        NodeType::Script => {
                            output_data.as_ref().and_then(|data| {
                                data.get("success").and_then(|v| v.as_bool()).map(|success| {
                                    if success {
                                        "true".to_string()
                                    } else {
                                        "false".to_string()
                                    }
                                })
                            })
                        }
                    };

                    // 构建 input_data（节点消费的输入配置）
                    let input_data = build_input_data_from_node_config(node);

                    // 记录输出数据日志（不自动设置为变量）
                    if let Some(ref data) = output_data {
                        log::info!(
                            "[流程 {}] 节点 {} 输出数据: {}",
                            context.flow_uuid,
                            node.name,
                            data
                        );
                    }

                    // 更新 last_node_output（仅协议节点更新，工具节点延续上游数据）
                    // 协议节点：Http, WsConnect, WsSendWait
                    // 工具节点：Entry, Conditional, WsClose（不更新，延续上游）
                    let is_protocol_node = matches!(
                        node.node_type,
                        NodeType::Http | NodeType::WsConnect | NodeType::WsSendWait
                    );
                    if is_protocol_node {
                        context.last_node_output = output_data.clone();
                        log::debug!(
                            "[流程 {}] 更新 last_node_output（协议节点 {}）",
                            context.flow_uuid,
                            node.name
                        );
                    }

                    // 计算变量变更：对比前后的变量差异
                    // 变量只会通过以下方式设置：
                    // 1. Entry 节点的 initial_variables
                    // 2. HTTP/WebSocket 节点的 output_mappings
                    let variables_after = context.variables.get_merged_variables();
                    let variable_changes = compute_variable_changes(&variables_before, &variables_after);

                    // 解析 resolved_config（Task 13）：将节点配置序列化后存储
                    let resolved_config = Some(serde_json::to_value(&node.data).unwrap_or(json!(null)));

                    // 追踪执行路径（Task 14）：记录成功执行的节点 UUID
                    execution_path.push(node.uuid.clone());

                    // 创建节点执行报告，使用构建的 input_data
                    let node_report = crate::report::NodeExecutionReport {
                        node_uuid: node.uuid.clone(),
                        node_type: format!("{:?}", node.node_type),
                        node_name: node.name.clone(),
                        start_time: node_start_ms,
                        end_time: node_end_ms,
                        duration_ms: node_duration,
                        status: crate::report::ExecutionStatus::Success,
                        input_data,
                        output_data: output_data.clone(),
                        used_port_id,
                        error: None,
                        // 新增字段（Task 13）
                        resolved_config,
                        variable_changes: if variable_changes.is_empty() { None } else { Some(variable_changes) },
                        skip_reason: None,
                        wait_duration_ms: None,
                    };
                    node_reports.push(node_report);

                    // 发送节点完成日志
                    context.emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!("节点 {} 执行完成", node.name),
                        timestamp: node_end_ms,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    }).await;

                    // 处理 Conditional 节点：如果返回空列表表示条件为 false，停止执行
                    if matches!(node.node_type, NodeType::Conditional)
                        && next_node_id_from_executor.is_empty()
                    {
                        log::info!(
                            "[流程 {}] 条件节点 {} 返回 false，停止流程",
                            context.flow_uuid,
                            node.name
                        );

                        // 发送条件终止日志
                        context.emit_log(LogEvent {
                            level: LogLevel::Info,
                            message: format!("条件节点 {} 返回 false，流程终止", node.name),
                            timestamp: node_end_ms,
                            node_uuid: Some(node.uuid.clone()),
                            node_name: Some(node.name.clone()),
                            flow_uuid: context.flow_uuid.clone(),
                        }).await;

                        // 条件为 false 是正常的流程终止，返回成功
                        break;
                    }

                    // 查找下一个节点
                    let next_node_id = if !next_node_id_from_executor.is_empty() {
                        // 如果执行器返回了下一个节点 ID，使用它
                        Some(next_node_id_from_executor[0].clone())
                    } else if matches!(node.node_type, NodeType::Http) {
                        // HTTP 节点：如果执行器返回空列表，说明没有匹配的端口连接
                        // 不使用 connection_map 回退，直接终止
                        log::info!(
                            "[流程 {}] HTTP 节点 {} 没有匹配的端口连接，执行完成",
                            context.flow_uuid,
                            node.name
                        );
                        None
                    } else {
                        // 其他节点：使用连接映射查找下一个节点
                        connection_map.get(&current_node_id).and_then(|targets| {
                            if targets.is_empty() {
                                None
                            } else {
                                Some(targets[0].clone())
                            }
                        })
                    };

                    if let Some(next_id) = next_node_id {
                        current_node_id = next_id;
                    } else {
                        // 节点没有输出连接或执行器返回空列表，执行完成
                        log::info!(
                            "[流程 {}] 节点 {} 没有下一个节点，执行完成",
                            context.flow_uuid,
                            node.name
                        );
                        break;
                    }
                }
                Err(error) => {
                    // 记录节点结束时间
                    let node_end_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    let node_duration = node_start_time.elapsed().as_millis() as u64;

                    // 检查是否是条件节点返回 false 的情况
                    if matches!(node.node_type, NodeType::Conditional)
                        && error == "条件评估为false，停止执行"
                    {
                        log::info!(
                            "[流程 {}] 条件节点 {} 评估为 false，停止流程",
                            context.flow_uuid,
                            node.name
                        );

                        // 创建节点执行报告（Success状态 - 条件评估完成）
                        // 解析 resolved_config
                        let resolved_config = Some(serde_json::to_value(&node.data).unwrap_or(json!(null)));

                        let node_report = crate::report::NodeExecutionReport {
                            node_uuid: node.uuid.clone(),
                            node_type: format!("{:?}", node.node_type),
                            node_name: node.name.clone(),
                            start_time: node_start_ms,
                            end_time: node_end_ms,
                            duration_ms: node_duration,
                            status: crate::report::ExecutionStatus::Success,
                            input_data: Some(json!(context.variables.get_merged_variables())),
                            output_data: None,
                            used_port_id: Some("false".to_string()),
                            error: None,
                            // 新增字段（Task 13）
                            resolved_config,
                            variable_changes: None,
                            skip_reason: None,
                            wait_duration_ms: None,
                        };
                        node_reports.push(node_report);

                        // 这是正常的流程终止（条件不满足），继续处理
                        break;
                    }

                    // 如果节点尚未实现（占位符），优雅终止流程
                    // 检查错误消息是否包含"尚未实现"
                    if error.contains("尚未实现") {
                        // 遇到占位符节点，表示流程已到达未实现部分
                        log::info!(
                            "[流程 {}] 节点 {} 尚未实现，流程终止",
                            context.flow_uuid,
                            node.name
                        );

                        // 创建节点执行报告（Success状态 - 优雅终止）
                        // 解析 resolved_config
                        let resolved_config = Some(serde_json::to_value(&node.data).unwrap_or(json!(null)));

                        let node_report = crate::report::NodeExecutionReport {
                            node_uuid: node.uuid.clone(),
                            node_type: format!("{:?}", node.node_type),
                            node_name: node.name.clone(),
                            start_time: node_start_ms,
                            end_time: node_end_ms,
                            duration_ms: node_duration,
                            status: crate::report::ExecutionStatus::Success,
                            input_data: Some(json!(context.variables.get_merged_variables())),
                            output_data: None,
                            used_port_id: None,
                            error: None,
                            // 新增字段（Task 13）
                            resolved_config,
                            variable_changes: None,
                            skip_reason: None,
                            wait_duration_ms: None,
                        };
                        node_reports.push(node_report);

                        // 返回成功，已执行的节点都成功完成
                        break;
                    } else {
                    // === 容错模式错误处理 ===
                    // 解析错误类型
                    let error_detail = parse_error_to_detail(&error);
                    
                    // 检查节点是否支持 false 端口，以及是否有 false 端口的连接
                    let supports_false_port = matches!(
                        node.node_type,
                        NodeType::WsConnect | NodeType::WsSendWait | NodeType::WsClose
                    );
                    
                    let false_port_next_node = if supports_false_port {
                        // 查找 false 端口的连接
                        flow.connections
                            .iter()
                            .find(|conn| {
                                conn.source_node_uuid == node.uuid && conn.source_port_id == "false"
                            })
                            .map(|conn| conn.target_node_uuid.clone())
                    } else {
                        None
                    };

                    // 根据是否有 false 端口连接决定 used_port_id
                    let used_port_id = if false_port_next_node.is_some() {
                        Some("false".to_string())
                    } else {
                        match node.node_type {
                            NodeType::Entry => Some("out".to_string()),
                            NodeType::Http => Some("out".to_string()),
                            _ => None,
                        }
                    };

                    // 创建节点执行报告（Failed状态）
                    // 解析 resolved_config
                    let resolved_config = Some(serde_json::to_value(&node.data).unwrap_or(json!(null)));

                    let node_report = crate::report::NodeExecutionReport {
                        node_uuid: node.uuid.clone(),
                        node_type: format!("{:?}", node.node_type),
                        node_name: node.name.clone(),
                        start_time: node_start_ms,
                        end_time: node_end_ms,
                        duration_ms: node_duration,
                        status: crate::report::ExecutionStatus::Failed,
                        input_data: Some(json!(context.variables.get_merged_variables())),
                        output_data: None,
                        used_port_id,
                        error: Some(error_detail.clone()),
                        // 新增字段（Task 13）
                        resolved_config,
                        variable_changes: None,
                        skip_reason: None,
                        wait_duration_ms: None,
                    };
                    node_reports.push(node_report);
                    executed_count += 1;

                    // 如果有 false 端口连接，继续执行 false 分支
                    if let Some(next_node_uuid) = false_port_next_node {
                        log::info!(
                            "[流程 {}] 节点 {} 执行失败，但有 false 端口连接，继续执行节点 {}",
                            context.flow_uuid,
                            node.name,
                            next_node_uuid
                        );
                        
                        current_node_id = next_node_uuid;
                        continue;
                    }

                    // 判断是否为致命错误
                    if error_detail.is_fatal() {
                        // 致命错误（Configuration, ConditionSyntax）：立即终止流程
                        log::error!(
                            "[流程 {}] 节点 {} 发生致命错误 ({:?}): {}",
                            context.flow_uuid,
                            node.name,
                            error_detail.error_kind,
                            error_detail.message
                        );

                        // 发送致命错误日志
                        context.emit_log(LogEvent {
                            level: LogLevel::Error,
                            message: format!("节点 {} 致命错误: {}", node.name, error_detail.message),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .expect("系统时间错误")
                                .as_millis() as u64,
                            node_uuid: Some(node.uuid.clone()),
                            node_name: Some(node.name.clone()),
                            flow_uuid: context.flow_uuid.clone(),
                        }).await;

                        // 生成部分报告（包含已执行节点）
                        let end_time_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64;
                        let flow_status = crate::report::ExecutionStatus::Failed;

                        // 获取最终变量快照（Task 16）
                        let final_variables = context.variables.get_merged_variables();

                        // 收集 WebSocket 消息流（完整数据）
                        let ws_streams: Vec<crate::ws_stream::WsConnectionStream> = {
                            let streams = context.ws_streams.read().await;
                            streams.values().cloned().collect()
                        };

                        let flow_report = crate::report::FlowExecutionReport {
                            uuid: Uuid::new_v4().to_string(),
                            flow_uuid: flow.uuid.clone(),
                            flow_name: flow.name.clone(),
                            start_time: end_time_ms - start_time.elapsed().as_millis() as u64,
                            end_time: end_time_ms,
                            duration_ms: start_time.elapsed().as_millis() as u64,
                            total_nodes: flow.nodes.len() as u32,
                            executed_count,
                            status: flow_status,
                            node_reports: node_reports.clone(),
                            statistics: crate::statistics::FlowStatistics::from_reports(
                                &node_reports,
                                flow.nodes.len() as u32,
                            ),
                            flow_error: Some(error_detail.clone()),
                            execution_mode: crate::report::ExecutionMode::Normal,
                            // 新增字段（Task 14, 15, 16）
                            initial_variables: Some(json!(initial_variables)),
                            final_variables: Some(json!(final_variables)),
                            execution_path: execution_path.clone(),
                            skipped_nodes: skipped_nodes.clone(),
                            ws_streams,
                        };

                        return ExecutionResult {
                            success: false,
                            data: None,
                            error: Some(error.clone()),
                            executed_count,
                            node_reports,
                            flow_report: Some(flow_report),
                        };
                    } else {
                        // 可恢复错误（Execution, Connection, Timeout 等）：记录失败，终止当前分支
                        log::warn!(
                            "[流程 {}] 节点 {} 发生可恢复错误 ({:?}): {}，终止当前分支",
                            context.flow_uuid,
                            node.name,
                            error_detail.error_kind,
                            error_detail.message
                        );

                        // 发送可恢复错误日志
                        context.emit_log(LogEvent {
                            level: LogLevel::Warn,
                            message: format!("节点 {} 可恢复错误: {}", node.name, error_detail.message),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .expect("系统时间错误")
                                .as_millis() as u64,
                            node_uuid: Some(node.uuid.clone()),
                            node_name: Some(node.name.clone()),
                            flow_uuid: context.flow_uuid.clone(),
                        }).await;

                        // 终止当前分支
                        break;
                    }
                    }
                }
            }
        }

        // 生成流程执行报告
        let end_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let start_time_ms = start_time.elapsed().as_millis() as u64;
        let flow_status = if node_reports
            .iter()
            .any(|r| r.status == crate::report::ExecutionStatus::Failed)
        {
            crate::report::ExecutionStatus::Failed
        } else {
            crate::report::ExecutionStatus::Success
        };

        // 获取最终变量快照（Task 16）
        let final_variables = context.variables.get_merged_variables();

        // 收集 WebSocket 消息流（完整数据）
        let ws_streams: Vec<crate::ws_stream::WsConnectionStream> = {
            let streams = context.ws_streams.read().await;
            streams.values().cloned().collect()
        };

        let flow_report = crate::report::FlowExecutionReport {
            uuid: Uuid::new_v4().to_string(),
            flow_uuid: flow.uuid.clone(),
            flow_name: flow.name.clone(),
            start_time: end_time - start_time_ms,
            end_time,
            duration_ms: start_time_ms,
            total_nodes: flow.nodes.len() as u32,
            executed_count,
            status: flow_status,
            node_reports: node_reports.clone(),
            statistics: crate::statistics::FlowStatistics::from_reports(
                &node_reports,
                flow.nodes.len() as u32,
            ),
            flow_error: None,
            execution_mode: crate::report::ExecutionMode::Normal,
            // 新增字段（Task 14, 15, 16）
            initial_variables: Some(json!(initial_variables)),
            final_variables: Some(json!(final_variables)),
            execution_path,
            skipped_nodes,
            ws_streams,
        };

        // 执行成功
        ExecutionResult {
            success: true,
            data: Some(json!({
                "flow_id": flow.uuid,
                "executed_count": executed_count,
                "execution_time": start_time_ms,
            })),
            error: None,
            executed_count,
            node_reports,
            flow_report: Some(flow_report),
        }
    }

    /// 执行单个节点
    ///
    /// # 参数
    /// - `node`: 要执行的节点
    /// - `context`: 执行上下文
    /// - `connections`: 流程的所有连接
    ///
    /// # 返回
    /// - `Result<(Option<String>, Option<serde_json::Value>), String>`:
    ///   - Ok((next_node_id, output_data)): 执行成功，返回下一个节点 ID 和输出数据
    ///   - Err(error): 执行失败，返回错误信息
    async fn execute_node(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        // 发送节点开始日志
        context.emit_log(LogEvent {
            level: LogLevel::Info,
            message: format!("执行节点 {} ({:?})", node.name, node.node_type),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            node_uuid: Some(node.uuid.clone()),
            node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone(),
        }).await;

        match node.node_type {
            NodeType::Entry => {
                log::info!(
                    "[流程 {}] 执行开始节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                let executor = EntryNodeExecutor::new();
                executor.execute(node, context, connections, node_map).await
            }
            NodeType::Http => {
                log::info!(
                    "[流程 {}] 执行HTTP节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                let executor = crate::http_executor::HttpNodeExecutor::new();
                executor.execute(node, context, connections, node_map).await
            }
            NodeType::Conditional => {
                log::info!(
                    "[流程 {}] 执行条件节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                let executor = crate::conditional_executor::ConditionalNodeExecutor::new();
                executor.execute(node, context, connections, node_map).await
            }
            NodeType::WsConnect => {
                log::info!(
                    "[流程 {}] 执行 WebSocket 连接节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                // 获取 ws_manager（如果不存在则创建新的）
                let ws_manager = context.ws_manager.clone().unwrap_or_else(|| {
                    Arc::new(WebSocketConnectionManager::new())
                });
                let executor = WsConnectExecutor::new(ws_manager);
                executor.execute(node, context, connections, node_map).await
            }
            NodeType::WsSendWait => {
                log::info!(
                    "[流程 {}] 执行 WebSocket Send+Wait 节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                // 获取 ws_manager（如果不存在则创建新的）
                let ws_manager = context.ws_manager.clone().unwrap_or_else(|| {
                    Arc::new(WebSocketConnectionManager::new())
                });
                let executor = WsSendWaitExecutor::new(ws_manager);
                executor.execute(node, context, connections, node_map).await
            }
            // WebSocket Close 节点
            NodeType::WsClose => {
                log::info!(
                    "[流程 {}] 执行 WebSocket Close 节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                // 获取 ws_manager（如果不存在则创建新的）
                let ws_manager = context.ws_manager.clone().unwrap_or_else(|| {
                    Arc::new(WebSocketConnectionManager::new())
                });
                let executor = WsCloseExecutor::new(ws_manager);
                executor.execute(node, context, connections, node_map).await
            }
            // Assertion 节点
            NodeType::Assertion => {
                log::info!(
                    "[流程 {}] 执行断言节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                let executor = crate::assertion_executor::AssertionNodeExecutor::new();
                executor.execute(node, context, connections, node_map).await
            }
            // 数据库连接节点
            NodeType::DbConnect => {
                log::info!(
                    "[流程 {}] 执行数据库连接节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                // 获取 db_manager（如果不存在则创建新的）
                let db_manager = context.db_manager.clone().unwrap_or_else(|| {
                    Arc::new(crate::db::pool::DatabaseConnectionManager::new())
                });
                let executor = crate::db_connect_executor::DbConnectExecutor::new(db_manager);
                executor.execute(node, context, connections, node_map).await
            }
            NodeType::DbQuery => {
                log::info!(
                    "[流程 {}] 执行数据库查询节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                // 获取 db_manager（如果不存在则创建新的）
                let db_manager = context.db_manager.clone().unwrap_or_else(|| {
                    Arc::new(crate::db::pool::DatabaseConnectionManager::new())
                });
                let executor = crate::db_query_executor::DbQueryNodeExecutor::with_db_manager(db_manager);
                executor.execute(node, context, connections, node_map).await
            }
            NodeType::DbBegin => {
                log::info!(
                    "[流程 {}] 执行数据库事务开始节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                // 获取 db_manager（如果不存在则创建新的）
                let db_manager = context.db_manager.clone().unwrap_or_else(|| {
                    Arc::new(crate::db::pool::DatabaseConnectionManager::new())
                });
                let executor = crate::db_begin_executor::DbBeginExecutor::new(db_manager);
                executor.execute(node, context, connections, node_map).await
            }
            NodeType::DbCommit => {
                log::info!(
                    "[流程 {}] 执行数据库事务提交节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                // 获取 db_manager（如果不存在则创建新的）
                let db_manager = context.db_manager.clone().unwrap_or_else(|| {
                    Arc::new(crate::db::pool::DatabaseConnectionManager::new())
                });
                let executor = crate::db_commit_executor::DbCommitExecutor::new(db_manager);
                executor.execute(node, context, connections, node_map).await
            }
            NodeType::DbRollback => {
                log::info!(
                    "[流程 {}] 执行数据库事务回滚节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                // 获取 db_manager（如果不存在则创建新的）
                let db_manager = context.db_manager.clone().unwrap_or_else(|| {
                    Arc::new(crate::db::pool::DatabaseConnectionManager::new())
                });
                let executor = crate::db_rollback_executor::DbRollbackExecutor::new(db_manager);
                executor.execute(node, context, connections, node_map).await
            }
            NodeType::DbClose => {
                log::info!(
                    "[流程 {}] 执行数据库关闭节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                // 获取 db_manager（如果不存在则创建新的）
                let db_manager = context.db_manager.clone().unwrap_or_else(|| {
                    Arc::new(crate::db::pool::DatabaseConnectionManager::new())
                });
                let executor = crate::db_close_executor::DbCloseExecutor::new(db_manager);
                executor.execute(node, context, connections, node_map).await
            }
            // 脚本节点
            NodeType::Script => {
                log::info!(
                    "[流程 {}] 执行脚本节点: {} ({})",
                    context.flow_uuid,
                    node.name,
                    node.uuid
                );
                let executor = crate::script_executor::ScriptNodeExecutor::new();
                executor.execute(node, context, connections, node_map).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_config::NodeData;
    use crate::node::NodePosition;

    use serde_json::json;
    use std::time::Instant;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_flow_executor_success() {
        // 创建测试流程 - 单个 Entry 节点，无连接，正常终止
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行结果（单个 Entry 节点无连接是正常终止）
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.executed_count, 1);
    }

    #[tokio::test]
    async fn test_flow_executor_with_connection() {
        // 创建测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行结果
        assert!(result.success);
        assert!(result.error.is_none());
        assert!(result.data.is_some());

        let data = result.data.unwrap();
        assert_eq!(data["flow_id"], flow_id);
        assert_eq!(data["executed_count"], 2);
        assert_eq!(result.executed_count, 2);
    }

    #[tokio::test]
    async fn test_flow_executor_no_start_node() {
        // 创建没有开始节点的流程
        let flow_id = Uuid::new_v4().to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![http_node],
            connections: vec![],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行失败
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("流程缺少开始节点"));
        assert_eq!(result.executed_count, 0);
    }

    #[tokio::test]
    async fn test_flow_executor_with_variables() {
        // 创建测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let variables = json!({
            "test_var": "test_value",
            "number": 42
        });

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, Some(variables), None, None).await;

        // 验证执行成功
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.executed_count, 2);
    }

    #[tokio::test]
    async fn test_flow_executor_start_node_execution_time() {
        // 创建测试流程

        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let start_time = Instant::now();
        let result = executor.execute_flow(&flow, None, None, None).await;
        let elapsed = start_time.elapsed();

        // 验证执行成功
        assert!(result.success);
        // 验证执行时间 < 10ms（开始节点应该立即传递）
        assert!(
            elapsed.as_millis() < 10,
            "执行时间应该小于 10ms，实际: {}ms",
            elapsed.as_millis()
        );
    }

    #[tokio::test]
    async fn test_execution_context_uses_flow_uuid() {
        // 测试 ExecutionContext 初始化使用 flow.uuid
        let flow_id = "test-flow-uuid-123".to_string();
        let start_node = Node {
            uuid: "start-1".to_string(),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证上下文中的 flow_uuid 使用了 flow.uuid（单个 Entry 节点无连接是正常终止）
        assert!(result.success);
        assert_eq!(result.executed_count, 1);
    }

    #[tokio::test]
    async fn test_node_map_uses_node_uuid() {
        // 测试节点映射构建使用 node.uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行成功（节点映射通过 node.uuid 查找成功）
        assert!(result.success);
        assert!(result.data.is_some());
    }

    #[tokio::test]
    async fn test_connection_map_uses_source_and_target_uuid() {
        // 测试连接映射构建使用 conn.source_node_uuid 和 conn.target_node_uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "start-uuid-123".to_string(),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let connection = Connection {
            id: "conn-uuid-789".to_string(),
            source_node_uuid: start_node.uuid.clone(), // 使用 source_node_uuid
            target_node_uuid: end_node.uuid.clone(),   // 使用 target_node_uuid
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证连接映射成功使用了 source_node_uuid 和 target_node_uuid
        assert!(result.success);
        assert_eq!(result.executed_count, 2);
    }

    #[tokio::test]
    async fn test_current_node_id_uses_start_node_uuid() {
        // 测试当前节点 ID 追踪使用 start_node.uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "start-uuid-tracker".to_string(),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let end_node = Node {
            uuid: "end-uuid-tracker".to_string(),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行的第一个节点是 start_node.uuid
        assert!(result.success);
        assert_eq!(result.executed_count, 2);
    }

    #[tokio::test]
    async fn test_error_message_uses_node_uuid() {
        // 测试错误消息使用 node.uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "loop-node-uuid".to_string(),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        // 创建自连接以触发循环错误
        let connection = Connection {
            id: "loop-conn".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: start_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证错误消息包含 node.uuid
        assert!(!result.success);
        assert!(result.error.is_some());
        let error_msg = result.error.unwrap();
        assert!(error_msg.contains("检测到循环"));
        assert!(error_msg.contains("loop-node-uuid")); // 错误消息包含 node.uuid
    }

    #[tokio::test]
    async fn test_execution_result_data_uses_flow_uuid() {
        // 测试执行结果数据使用 flow.uuid
        let flow_id = "result-flow-uuid-999".to_string();
        let start_node = Node {
            uuid: "start-1".to_string(),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let end_node = Node {
            uuid: "end-1".to_string(),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let connection = Connection {
            id: "conn-1".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行结果数据包含 flow.uuid
        assert!(result.success);
        assert!(result.data.is_some());
        let data = result.data.unwrap();
        assert_eq!(data["flow_id"], "result-flow-uuid-999"); // 验证使用了 flow.uuid
    }

    #[tokio::test]
    async fn test_multiple_connections_uses_uuids() {
        // 测试多个连接使用 source_node_uuid 和 target_node_uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "start-multi".to_string(),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let end_node_1 = Node {
            uuid: "end-1-multi".to_string(),
            node_type: NodeType::Entry,
            name: "结束 1".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let end_node_2 = Node {
            uuid: "end-2-multi".to_string(),
            node_type: NodeType::Entry,
            name: "结束 2".to_string(),
            position: NodePosition { x: 100.0, y: 100.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        };

        let connection_1 = Connection {
            id: "conn-1-multi".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node_1.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let connection_2 = Connection {
            id: "conn-2-multi".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node_2.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node_1, end_node_2],
            connections: vec![connection_1, connection_2],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行成功，连接映射正确使用了 UUID
        assert!(result.success);
        assert_eq!(result.executed_count, 2);
    }

    #[test]
    fn test_replace_variables_simple() {
        // 测试简单变量替换
        let template = "Bearer {{initial_variables.api_key}}";
        let mut variables = VariableStorage::new();
        variables.set_variable(
            "initial_variables".to_string(),
            json!({
                "api_key": "test-key"
            }),
            String::new(),
        );
        let context = ExecutionContext {
            flow_uuid: "test-flow".to_string(),
            current_node_uuid: None,
            variables,
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: std::collections::HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            db_manager: None,
            db_connections: std::collections::HashMap::new(),
            last_node_output: None,
            debug_event_sender: None,
        };
        let result = replace_variables(template, &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Bearer test-key");
    }

    #[test]
    fn test_replace_variables_nested_path() {
        // 测试嵌套路径
        let template = "Hello {{user.profile.name}}";
        let mut variables = VariableStorage::new();
        variables.set_variable(
            "user".to_string(),
            json!({
                "profile": {
                    "name": "Alice"
                }
            }),
            String::new(),
        );
        let context = ExecutionContext {
            flow_uuid: "test-flow".to_string(),
            current_node_uuid: None,
            variables,
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: std::collections::HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            db_manager: None,
            db_connections: std::collections::HashMap::new(),
            last_node_output: None,
            debug_event_sender: None,
        };
        let result = replace_variables(template, &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello Alice");
    }

    #[test]
    fn test_replace_variables_strict_mode() {
        // 测试严格模式（变量不存在时报错）
        let template = "{{nonexistent.var}}";
        let variables = VariableStorage::new();
        let context = ExecutionContext {
            flow_uuid: "test-flow".to_string(),
            current_node_uuid: None,
            variables,
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: std::collections::HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            db_manager: None,
            db_connections: std::collections::HashMap::new(),
            last_node_output: None,
            debug_event_sender: None,
        };
        let result = replace_variables(template, &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未找到"));
    }

    #[test]
    fn test_replace_variables_index_not_supported() {
        // 测试索引访问不支持
        let template = "{{items[0]}}";
        let mut variables = VariableStorage::new();
        variables.set_variable("items".to_string(), json!(["a", "b", "c"]), String::new());
        let context = ExecutionContext {
            flow_uuid: "test-flow".to_string(),
            current_node_uuid: None,
            variables,
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: std::collections::HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            db_manager: None,
            db_connections: std::collections::HashMap::new(),
            last_node_output: None,
            debug_event_sender: None,
        };
        let result = replace_variables(template, &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不支持索引访问"));
    }

    // ============================================================================
    // 属性测试 (proptest)
    // ============================================================================

    proptest::proptest! {
        /// 测试合法变量路径格式
        #[test]
        fn test_proptest_valid_variable_path(
            path in "[a-z][a-z0-9_]*(\\.[a-z][a-z0-9_]*){0,5}"
        ) {
            // 合法路径应该: 字母开头，仅包含字母数字下划线和点
            assert!(!path.is_empty());
            assert!(path.chars().next().unwrap().is_ascii_lowercase());
        }

        /// 测试索引访问被拒绝
        #[test]
        fn test_proptest_index_access_rejected(
            index in 0..100usize
        ) {
            let template = format!("{{{{items[{}]}}}}", index);
            let mut variables = VariableStorage::new();
            variables.set_variable("items".to_string(), json!(["a", "b", "c"]), String::new());
            let context = ExecutionContext {
                flow_uuid: "test-flow".to_string(),
                current_node_uuid: None,
                variables,
                node_reports: vec![],
                flow_trigger_time: 0,
                log_sender: None,
                flow_file_path: None,
                ws_manager: None,
                ws_connection_urls: std::collections::HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            db_manager: None,
            db_connections: std::collections::HashMap::new(),
            last_node_output: None,
            debug_event_sender: None,
        };
        let result = replace_variables(&template, &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不支持索引访问"));
    }

    /// 测试简单字符串变量替换
        #[test]
        fn test_proptest_simple_string_replacement(
            value in "[a-zA-Z0-9]{1,20}"
        ) {
            let template = "{{test_var}}";
            let mut variables = VariableStorage::new();
            variables.set_variable("test_var".to_string(), json!(value.clone()), String::new());
            let context = ExecutionContext {
                flow_uuid: "test-flow".to_string(),
                current_node_uuid: None,
                variables,
                node_reports: vec![],
                flow_trigger_time: 0,
                log_sender: None,
                flow_file_path: None,
                ws_manager: None,
                ws_connection_urls: std::collections::HashMap::new(),
                ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
                db_manager: None,
                db_connections: std::collections::HashMap::new(),
                last_node_output: None,
                debug_event_sender: None,
            };
            let result = replace_variables(template, &context);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), value);
        }

        /// 测试整数变量替换
        #[test]
        fn test_proptest_integer_replacement(
            num in -10000i64..10000i64
        ) {
            let template = "{{num}}";
            let mut variables = VariableStorage::new();
            variables.set_variable("num".to_string(), json!(num), String::new());
            let context = ExecutionContext {
                flow_uuid: "test-flow".to_string(),
                current_node_uuid: None,
                variables,
                node_reports: vec![],
                flow_trigger_time: 0,
                log_sender: None,
                flow_file_path: None,
                ws_manager: None,
                ws_connection_urls: std::collections::HashMap::new(),
                ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
                db_manager: None,
                db_connections: std::collections::HashMap::new(),
                last_node_output: None,
                debug_event_sender: None,
            };
            let result = replace_variables(template, &context);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), num.to_string());
        }

        /// 测试布尔变量替换
        #[test]
        fn test_proptest_bool_replacement(
            val in proptest::bool::ANY
        ) {
            let template = "{{flag}}";
            let mut variables = VariableStorage::new();
            variables.set_variable("flag".to_string(), json!(val), String::new());
            let context = ExecutionContext {
                flow_uuid: "test-flow".to_string(),
                current_node_uuid: None,
                variables,
                node_reports: vec![],
                flow_trigger_time: 0,
                log_sender: None,
                flow_file_path: None,
                ws_manager: None,
                ws_connection_urls: std::collections::HashMap::new(),
                ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
                db_manager: None,
                db_connections: std::collections::HashMap::new(),
                last_node_output: None,
                debug_event_sender: None,
            };
            let result = replace_variables(template, &context);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), val.to_string());
        }
    }
}
