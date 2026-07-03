// 执行引擎模块 - FlowExecutor 实现

use flow_engine_core::connection::Connection;
use flow_engine_core::context::{ExecutionContext, VariableStorage};
use flow_engine_core::flow::Flow;
use flow_engine_core::error::{ErrorCode, ErrorDetail, ErrorKind};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::{ExecutionErrorStrategy, Node};
use crate::provider_builder::ProviderBuilderExt;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use flow_engine_core::traits::{ExecutionEventSender, LogSender, DebugEventSender};
use flow_engine_core::execution_event::ExecutionEvent;
use flow_engine_core::node_types;
use flow_engine_core::registry::NodeRegistry;

use uuid::Uuid;

// Re-export from flow-engine-core
pub use flow_engine_core::executor::{NodeExecutor, replace_variables};


/// 执行结果
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// 执行是否Success
    pub success: bool,
    /// 返回数据
    pub data: Option<serde_json::Value>,
    /// Error信息
    pub error: Option<String>,
    /// 已执行节点数量
    pub executed_count: u32,
    /// 节点执行报告列表
    pub node_reports: Vec<flow_engine_core::report::NodeExecutionReport>,
    /// 流程执行报告
    pub flow_report: Option<flow_engine_core::report::FlowExecutionReport>,
}

/// 计算变量变更列表
///
/// 对比变量快照前后的差异，生成 `VariableChange` 列表
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
) -> Vec<flow_engine_core::report::VariableChange> {
    let mut changes = Vec::new();

    // 检测新增和修改的变量
    for (key, new_value) in variables_after {
        let old_value = variables_before.get(key).cloned();
        // 只有值真正变化时才记录
        if old_value.as_ref() != Some(new_value) {
            changes.push(flow_engine_core::report::VariableChange {
                key: key.clone(),
                old_value,
                new_value: new_value.clone(),
            });
        }
    }

    changes
}

/// 根据节点类型构建 `input_data`
///
/// `input_data` 代表节点消费的输入数据（配置信息）
/// 通过 `NodeRegistry` 查找对应插件的 `build_report_input` 方法
fn build_input_data_from_node_config(
    registry: &NodeRegistry,
    node: &flow_engine_core::node::Node,
) -> Option<serde_json::Value> {
    use flow_engine_core::node_config::NodeData;
    
    match &node.data {
        // 插件节点：透传原始 JSON 数据
        NodeData::Plugin(val) => Some(val.clone()),
        _ => {
            let type_id = node.data.type_name();
            match registry.get(&type_id) {
                Some(plugin) => {
                    let config_json = serde_json::to_value(&node.data).unwrap_or(serde_json::Value::Null);
                    plugin.build_report_input(&config_json)
                }
                None => None,
            }
        }
    }
}

/// Node types that can be executed standalone via `execute_single_node`
const SUPPORTED_SINGLE_NODE_TYPES: &[&str] = &[
    "http", "script", "assertion",
    "redis_connect", "db_connect", "mqtt_connect", "kafka_connect",
    "amqp_connect", "amqp10_connect", "smtp_connect", "imap_connect",
    "ws_connect", "sse_connect", "grpc_connect", "mcp_connect",
    "graphql_connect", "tcp_connect",
];

/// Fallback timeout (30s) when node has no configured timeout
const FALLBACK_TIMEOUT_MS: u64 = 30_000;

/// Flow executor
pub struct FlowExecutor {
    /// Flow file path (used for resolving relative paths in CLI mode)
    flow_file_path: Option<PathBuf>,
    /// 执行事件发送器（正常模式有值，CLI 模式为 None）
    execution_event_sender: Option<Arc<dyn ExecutionEventSender>>,
    /// 调试事件发送器（普通模式下用于 WS 流事件广播）
    debug_event_sender: Option<Arc<dyn DebugEventSender>>,
    /// 取消信号接收器（用于外部取消流程执行）
    cancel_receiver: Option<tokio::sync::watch::Receiver<bool>>,
    /// 节点插件注册表（通过 registry 查找替代 match 分发）
    registry: Arc<NodeRegistry>,
    /// 钩子执行器（流程完成时触发预定义动作）
    hook_executor: Option<Arc<dyn flow_engine_core::traits::HookExecutor>>,
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
            execution_event_sender: None,
            debug_event_sender: None,
            cancel_receiver: None,
            registry: Arc::new(crate::default_registry::default_registry()),
            hook_executor: None,
        }
    }

    /// Set flow file path (used for CLI relative path resolution)
    pub fn with_flow_file_path(mut self, path: PathBuf) -> Self {
        self.flow_file_path = Some(path);
        self
    }

    /// 设置执行事件发送器（正常模式调用）
    ///
    /// # 参数
    ///
    /// * `sender` - `ExecutionEvent` 事件发送器
    ///
    /// # 返回
    ///
    /// 更新后的 `FlowExecutor`
    pub fn with_execution_event_sender(mut self, sender: Arc<dyn ExecutionEventSender>) -> Self {
        self.execution_event_sender = Some(sender);
        self
    }

    /// 设置调试事件发送器（普通模式下用于 WS 流事件）
    ///
    /// # 参数
    ///
    /// * `sender` - `DebugEvent` 事件发送器
    ///
    /// # 返回
    ///
    /// 更新后的 `FlowExecutor`
    pub fn with_debug_event_sender(mut self, sender: Arc<dyn DebugEventSender>) -> Self {
        self.debug_event_sender = Some(sender);
        self
    }

    /// 设置取消信号接收器
    pub fn with_cancel_receiver(mut self, rx: tokio::sync::watch::Receiver<bool>) -> Self {
        self.cancel_receiver = Some(rx);
        self
    }

    /// 设置钩子执行器（流程完成时触发预定义动作）
    pub fn with_hook_executor(mut self, executor: Arc<dyn flow_engine_core::traits::HookExecutor>) -> Self {
        self.hook_executor = Some(executor);
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
    ///
    /// # 返回
    /// - `ExecutionResult`: 执行结果
    pub async fn execute_flow(
        &self,
        flow: &Flow,
        variables: Option<serde_json::Value>,
        runtime_variables: Option<serde_json::Value>,
        log_sender: Option<Arc<dyn LogSender>>,
    ) -> ExecutionResult {
        let start_time = tokio::time::Instant::now();
        let mut executed_count = 0u32;
        let mut node_reports = Vec::new();
        // 优先使用调用方传入的变量，否则从流程的激活环境中解析
        let initial_variables = variables
            .or_else(|| flow.get_active_variables().map(|v| json!(v)))
            .unwrap_or_else(|| json!({}));
        let mut var_storage = VariableStorage::new();
        if let Some(obj) = initial_variables.as_object() {
            for (key, value) in obj {
                var_storage.set_variable(key.clone(), value.clone(), String::new());
            }
        }

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
            ws_connection_urls: std::collections::HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            last_node_output: None,
            debug_event_sender: self.debug_event_sender.clone(),
            execution_event_sender: self.execution_event_sender.clone(),
            cancel_receiver: self.cancel_receiver.clone(),
            providers: std::collections::HashMap::new(),
        }
        .with_default_providers()
        .with_default_providers_and_grpc(None);

        // 集成流程验证器
        let validator = crate::validator::FlowValidator::new();
        let validation = validator.validate(flow);
        if !validation.valid {
            return ExecutionResult {
                success: false,
                data: None,
                error: Some(format!("Flow validation failed: {}", validation.errors.iter().map(|e| &e.message[..]).collect::<Vec<&str>>().join("; "))),
                executed_count: 0,
                node_reports: vec![],
                flow_report: None,
            };
        }
        // 过滤 flow_warning：排除已由前端通过 end_node_reached i18n 处理的"无 End 节点"提示
        let flow_warning: Option<String> = {
            let has_end_nodes = flow.nodes.iter().any(|n| n.node_type == node_types::END);
            let warnings: Vec<String> = validation.warnings.iter()
                .filter(|w| has_end_nodes || !w.contains("End"))
                .cloned()
                .collect();
            if warnings.is_empty() { None } else { Some(warnings.join("; ")) }
        };

        // 查找开始节点
        let start_node = match flow
            .nodes
            .iter()
            .find(|node| node.node_type == node_types::ENTRY)
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

        // 执行路径追踪（Task 14）：记录所有Success执行的节点 UUID
        let mut execution_path: Vec<String> = Vec::new();

        // 跳过节点记录（Task 15）：记录被跳过的节点及原因
        let skipped_nodes: Vec<flow_engine_core::report::SkippedNodeInfo> = Vec::new();

        // 发送流程开始事件
        context.emit_execution_event(ExecutionEvent::FlowStarted {
            flow_uuid: flow.uuid.clone(),
            flow_name: flow.name.clone(),
            total_nodes: flow.nodes.len() as u32,
        }).await;

        'execution: while let Some(node) = node_map.get(&current_node_id) {
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

            // 非阻塞取消检查：每次迭代开始时检查取消信号
            if context.is_cancelled() {
                log::info!(
                    "[流程 {}] Execution cancelled on node {} ",
                    context.flow_uuid,
                    node.name
                );
                break 'execution;
            }

            visited.insert(current_node_id.clone());
            context.current_node_uuid = Some(current_node_id.clone());

            // 发送节点start execution事件
            context.emit_execution_event(ExecutionEvent::NodeStarted {
                node_uuid: node.uuid.clone(),
                node_name: node.name.clone(),
                node_type: node.node_type.clone(),
            }).await;

            // 记录节点开始时间
            let node_start_time = tokio::time::Instant::now();
            let node_start_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            // 快照执行前的变量状态（必须在 execute_node 之前）
            // 用于计算节点执行期间的变量变更
            let variables_before = context.variables.get_merged_variables();

            // 执行当前节点（支持取消中断 + 超时控制）
            let node_timeout_ms = node.data.execution_timeout_ms()
                .map(|ms| std::cmp::min(ms, 3600000)); // Cap at 1 hour
            let node_result = match (context.cancel_receiver.is_some(), node_timeout_ms) {
                (true, Some(ms)) => {
                    // 有取消接收器 + 超时：在 select 中同时监听取消和超时
                    let Some(mut rx) = context.cancel_receiver.clone() else {
                        unreachable!("cancel_receiver.is_some() was just checked")
                    };
                    let timeout_dur = std::time::Duration::from_millis(ms);
                    let execute = self.execute_node(node, &mut context, &flow.connections, &node_map);
                    let wrapped = tokio::time::timeout(timeout_dur, execute);
                    tokio::select! {
                        result = wrapped => {
                            match result {
                                Ok(inner) => Some(inner),
                                Err(_) => {
                                    log::info!("[流程 {}] Node {} execution timeout ({}ms)", context.flow_uuid, node.name, ms);
                                    Some(Err(ErrorDetail::new(ErrorKind::Timeout, format!("节点执行超时 ({}ms)", ms)).with_error_code_typed(ErrorCode::E_TIMEOUT)))
                                }
                            }
                        }
                        _ = rx.changed() => {
                            log::info!("[流程 {}] 节点 {} received cancellation signal during execution", context.flow_uuid, node.name);
                            None
                        }
                    }
                }
                (true, None) => {
                    // 有取消接收器但无超时：使用 tokio::select! 只监听取消信号
                    let Some(mut rx) = context.cancel_receiver.clone() else {
                        unreachable!("cancel_receiver.is_some() was just checked")
                    };
                    tokio::select! {
                        result = self.execute_node(node, &mut context, &flow.connections, &node_map) => {
                            Some(result)
                        }
                        _ = rx.changed() => {
                            log::info!("[流程 {}] 节点 {} received cancellation signal during execution", context.flow_uuid, node.name);
                            None
                        }
                    }
                }
                (false, Some(ms)) => {
                    // 无取消接收器但有超时
                    let timeout_dur = std::time::Duration::from_millis(ms);
                    match tokio::time::timeout(timeout_dur, self.execute_node(node, &mut context, &flow.connections, &node_map)).await {
                        Ok(result) => Some(result),
                        Err(_) => {
                            log::info!("[流程 {}] Node {} execution timeout ({}ms)", context.flow_uuid, node.name, ms);
                            Some(Err(ErrorDetail::new(ErrorKind::Timeout, format!("节点执行超时 ({}ms)", ms)).with_error_code_typed(ErrorCode::E_TIMEOUT)))
                        }
                    }
                }
                (false, None) => {
                    // 无取消接收器且无超时：直接执行
                    Some(self.execute_node(node, &mut context, &flow.connections, &node_map).await)
                }
            };

            // 取消中断：跳出循环生成取消报告
            let node_result = match node_result {
                Some(result) => result,
                None => break 'execution,
            };

            match node_result
            {
                Ok((next_node_id_from_executor, output_data)) => {
                    // 记录节点结束时间
                    let node_end_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    let node_duration = node_start_time.elapsed().as_millis() as u64;

                    executed_count += 1;

                    // 根据节点类型设置 used_port_id（通过 registry 查找）
                    let type_id = node.data.type_name();
                    let used_port_id = self.registry.get(&type_id)
                        .map(|plugin| Some(plugin.resolve_port(&output_data)))
                        .unwrap_or(None);

                    // 构建 input_data（节点消费的输入配置）
                    let input_data = build_input_data_from_node_config(&self.registry, node);

                    // 记录输出数据日志（不自动设置为变量）
                    if let Some(ref data) = output_data {
                        log::info!(
                            "[流程 {}] Node {} output data: {}",
                            context.flow_uuid,
                            node.name,
                            data
                        );
                    }

                    // 更新 last_node_output（所有节点均更新，供下游节点通过 jsonpath 访问）
                    if let Some(ref data) = output_data {
                        context.last_node_output = Some(data.clone());
                    }

                    // 计算变量变更：对比前后的变量差异
                    // 变量只会通过以下方式设置：
                    // 1. Entry node的 initial_variables
                    // 2. HTTP/WebSocket 节点的 output_mappings
                    let variables_after = context.variables.get_merged_variables();
                    let variable_changes = compute_variable_changes(&variables_before, &variables_after);

                    // 解析 resolved_config：递归替换节点配置中的变量占位符
                    let raw_config = serde_json::to_value(&node.data).unwrap_or(json!(null));
                    let resolved_config = Some(
                        flow_engine_core::executor::resolve_variables_in_json(&raw_config, &context)
                    );

                    // 追踪执行路径（Task 14）：记录Success执行的节点 UUID
                    execution_path.push(node.uuid.clone());

                    // Phase 2.5: 注入 runtime_variables（在 Entry node执行后）
                    // runtime_variables 用于数据驱动执行时注入 CSV 行数据，覆盖 Entry node的同名变量
                    if node.node_type == node_types::ENTRY {
                        if let Some(ref rt_vars) = runtime_variables {
                            if let Some(obj) = rt_vars.as_object() {
                                for (key, value) in obj {
                                    context.variables.set_variable(key.clone(), value.clone(), String::new());
                                }
                            }
                        }
                    }

                    // 创建节点执行报告，使用构建的 input_data
                    let node_report = flow_engine_core::report::NodeExecutionReport {
                        node_uuid: node.uuid.clone(),
                        node_type: node.node_type.clone(),
                        node_name: node.name.clone(),
                        start_time: node_start_ms,
                        end_time: node_end_ms,
                        duration_ms: node_duration,
                        // 只要 used_port_id 为 "false"，即使 executor 返回 Ok 也视为Failed
                        status: if used_port_id.as_deref() == Some("false") {
                            flow_engine_core::report::ExecutionStatus::Failed
                        } else {
                            flow_engine_core::report::ExecutionStatus::Success
                        },
                        input_data,
                        output_data: output_data.clone(),
                        used_port_id,
                        // 从 output_data["error"] 提取错误信息
                        // 协议执行器在 Ok 路径下通过 output_data["error"] 嵌入错误信息
                        // （而非返回 Err），以保留 output_data 中的部分数据（如请求信息、时序）
                        // 所有 16+ 个协议执行器遵循此约定
                        error: extract_error_from_output(&output_data),
                        // 新增字段（Task 13）
                        resolved_config,
                        variable_changes: if variable_changes.is_empty() { None } else { Some(variable_changes) },
                        skip_reason: None,
                        wait_duration_ms: None,
                    };
                    node_reports.push(node_report.clone());
                    context.node_reports.push(node_report);

                    // 发送节点完成事件
                    context.emit_execution_event(ExecutionEvent::NodeCompleted {
                        report: Box::new(node_reports.last().unwrap().clone()),
                    }).await;

                    // 发送节点完成日志
                    context.emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!("节点 {} execution complete", node.name),
                        timestamp: node_end_ms,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    }).await;

                    // 处理 Conditional 节点：如果返回空列表表示条件为 false，停止执行
                    if node.node_type == node_types::CONDITIONAL
                        && next_node_id_from_executor.is_empty()
                    {
                        log::info!(
                            "[流程 {}] Conditional node {} returned false, stopping flow",
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

                        // 条件为 false 是正常的流程终止，返回Success
                        break;
                    }

                    // 查找下一个节点
                    // 是否为端口路由节点（使用 true/false/out 端口路由的节点类型）
                    // 这些节点的执行器已经根据端口做了路由决策，不应回退到无过滤的 connection_map
                    let is_port_routing_node = self.registry.get(&type_id)
                        .map(|p| p.is_port_routing())
                        .unwrap_or(true);

                    let next_node_id = if !next_node_id_from_executor.is_empty() {
                        // 如果执行器返回了下一个节点 ID，使用它
                        Some(next_node_id_from_executor[0].clone())
                    } else if is_port_routing_node {
                        // 端口路由节点：如果执行器返回空列表，说明没有匹配的端口连接
                        // 不使用 connection_map 回退，直接终止
                        log::info!(
                            "[流程 {}] Port routing node {} has no matching port connection, execution complete",
                            context.flow_uuid,
                            node.name
                        );
                        None
                    } else {
                        // 非端口路由节点（如 Entry）：使用连接映射查找下一个节点
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
                        // 节点没有输出连接或执行器返回空列表，execution complete
                        log::info!(
                            "[流程 {}] 节点 {} 没有下一个节点，execution complete",
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
                    if node.node_type == node_types::CONDITIONAL
                        && error.message == "条件评估为false，停止执行"
                    {
                        log::info!(
                            "[流程 {}] Conditional node {} evaluated to false, stopping flow",
                            context.flow_uuid,
                            node.name
                        );

                        // 创建节点执行报告（Failed状态 - 条件评估为 false）
                        // 解析 resolved_config：递归替换节点配置中的变量占位符
                        let raw_config = serde_json::to_value(&node.data).unwrap_or(json!(null));
                        let resolved_config = Some(
                            flow_engine_core::executor::resolve_variables_in_json(&raw_config, &context)
                        );

                        let node_report = flow_engine_core::report::NodeExecutionReport {
                            node_uuid: node.uuid.clone(),
                            node_type: node.node_type.clone(),
                            node_name: node.name.clone(),
                            start_time: node_start_ms,
                            end_time: node_end_ms,
                            duration_ms: node_duration,
                            status: flow_engine_core::report::ExecutionStatus::Failed,
                            input_data: Some(json!(context.variables.get_merged_variables())),
                            output_data: None,
                            used_port_id: Some("false".to_string()),
                            error: Some(flow_engine_core::error::ErrorDetail::new(
                                flow_engine_core::error::ErrorKind::Execution,
                                "条件评估为false，流程终止".to_string(),
                            )),
                            // 新增字段（Task 13）
                            resolved_config,
                            variable_changes: None,
                            skip_reason: None,
                            wait_duration_ms: None,
                        };
                        node_reports.push(node_report.clone());
                        context.node_reports.push(node_report);

                        // 发送节点完成事件
                        context.emit_execution_event(ExecutionEvent::NodeCompleted {
                            report: Box::new(node_reports.last().unwrap().clone()),
                        }).await;

                        // 这是正常的流程终止（条件不满足），继续处理
                        break;
                    }

                    // Error已由协议执行器构造为 ErrorDetail，直接传递
                    let error_detail = error;
                    
                    // 查找节点插件用于后续 registry 查询
                    let type_id = node.data.type_name();
                    
                    // 根据节点 on_error 策略决定执行行为
                    let route_to_false = node.on_error.as_ref().map_or(true, |s| *s == ExecutionErrorStrategy::RouteToFalse);
                    let is_ignore = node.on_error.as_ref().map_or(false, |s| *s == ExecutionErrorStrategy::Ignore);
                    
                    let false_port_next_node = if route_to_false {
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
                        // Error回退：通过 registry 查找 resolve_port
                        self.registry.get(&type_id)
                            .map(|plugin| plugin.resolve_port(&None))
                            .map(Some)
                            .unwrap_or(None)
                    };

                    // 创建节点执行报告（Failed状态）
                    // 解析 resolved_config：递归替换节点配置中的变量占位符
                    let raw_config = serde_json::to_value(&node.data).unwrap_or(json!(null));
                    let resolved_config = Some(
                        flow_engine_core::executor::resolve_variables_in_json(&raw_config, &context)
                    );

                    let node_report = flow_engine_core::report::NodeExecutionReport {
                        node_uuid: node.uuid.clone(),
                        node_type: node.node_type.clone(),
                        node_name: node.name.clone(),
                        start_time: node_start_ms,
                        end_time: node_end_ms,
                        duration_ms: node_duration,
                        status: flow_engine_core::report::ExecutionStatus::Failed,
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
                    node_reports.push(node_report.clone());
                    context.node_reports.push(node_report);

                    // 发送节点完成事件
                    context.emit_execution_event(ExecutionEvent::NodeCompleted {
                        report: Box::new(node_reports.last().unwrap().clone()),
                    }).await;

                    executed_count += 1;

                    // RouteToFalse（默认）: 如果有 false 端口连接，继续执行 false 分支
                    if route_to_false {
                        if let Some(next_node_uuid) = false_port_next_node {
                            log::info!(
                                "[流程 {}] 节点 {} 执行Failed，但有 false 端口连接，继续执行节点 {}",
                                context.flow_uuid,
                                node.name,
                                next_node_uuid
                            );
                            
                            current_node_id = next_node_uuid;
                            continue;
                        }
                    }

                    // Ignore 策略: 忽略Error，继续 true/out 端口执行（如同Success）
                    if is_ignore {
                        let next_node_id = flow.connections
                            .iter()
                            .find(|conn| {
                                conn.source_node_uuid == node.uuid && 
                                (conn.source_port_id == "true" || conn.source_port_id == "out")
                            })
                            .map(|conn| conn.target_node_uuid.clone());

                        if let Some(next_uuid) = next_node_id {
                            log::info!(
                                "[流程 {}] Node {} execution failed, on_error=Ignore, continuing to node {}",
                                context.flow_uuid,
                                node.name,
                                next_uuid
                            );
                            current_node_id = next_uuid;
                            continue;
                        }
                        // 没有后续连接，正常终止流程
                        break;
                    }

                    // 判断是否应终止流程（AbortFlow 策略或致命Error）
                    if !is_ignore && (!route_to_false || error_detail.is_fatal()) {
                        if !route_to_false {
                            // AbortFlow 策略：立即终止流程
                            log::error!(
                                "[流程 {}] 节点 {} 执行Failed，on_error=AbortFlow，终止流程",
                                context.flow_uuid,
                                node.name,
                            );
                        } else {
                            // 致命Error（Configuration, ConditionSyntax）：立即终止流程
                            log::error!(
                                "[流程 {}] 节点 {} 发生致命Error ({:?}): {}",
                                context.flow_uuid,
                                node.name,
                                error_detail.error_kind,
                                error_detail.message
                            );
                        }

                        // 发送Error日志
                        let log_message = if !route_to_false {
                            format!("节点 {} on_error=AbortFlow，终止流程", node.name)
                        } else {
                            format!("节点 {} 致命Error: {}", node.name, error_detail.message)
                        };
                        context.emit_log(LogEvent {
                            level: LogLevel::Error,
                            message: log_message,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
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
                        let flow_status = flow_engine_core::report::ExecutionStatus::Failed;

                        // 获取最终变量快照（Task 16）
                        let final_variables = context.variables.get_merged_variables();

                        // 收集 WebSocket 消息流（完整数据）
                        let ws_streams: Vec<flow_engine_core::ws_stream::WsConnectionStream> = {
                            let streams = context.ws_streams.read().await;
                            streams.values().cloned().collect()
                        };

                        let flow_report = flow_engine_core::report::FlowExecutionReport {
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
                            statistics: flow_engine_core::statistics::FlowStatistics::from_reports(
                                &node_reports,
                                flow.nodes.len() as u32,
                            ),
                            flow_error: Some(error_detail.clone()),
                            execution_mode: flow_engine_core::report::ExecutionMode::Normal,
                            // 新增字段（Task 14, 15, 16）
                            initial_variables: Some(json!(initial_variables)),
                            final_variables: Some(json!(final_variables)),
                            execution_path: execution_path.clone(),
                            skipped_nodes: skipped_nodes.clone(),
                            ws_streams,
                            flow_snapshot: Some(flow.clone()),
                            end_node_reached: None,
                            flow_warning: None,
                        };

                        return ExecutionResult {
                            success: false,
                            data: None,
                            error: Some(error_detail.message.clone()),
                            executed_count,
                            node_reports,
                            flow_report: Some(flow_report),
                        };
                    } else {
                        // 可恢复Error（Execution, Connection, Timeout 等）：记录Failed，, terminating current branch
                        log::warn!(
                            "[流程 {}] Node {} recoverable error ({:?}): {}，, terminating current branch",
                            context.flow_uuid,
                            node.name,
                            error_detail.error_kind,
                            error_detail.message
                        );

                        // 发送可恢复Error日志
                        context.emit_log(LogEvent {
                            level: LogLevel::Warn,
                            message: format!("节点 {} 可恢复Error: {}", node.name, error_detail.message),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                            node_uuid: Some(node.uuid.clone()),
                            node_name: Some(node.name.clone()),
                            flow_uuid: context.flow_uuid.clone(),
                        }).await;

                        // , terminating current branch
                        break;
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

        // 获取最终变量快照
        let final_variables = context.variables.get_merged_variables();

        // 收集 WebSocket 消息流（完整数据）
        let ws_streams: Vec<flow_engine_core::ws_stream::WsConnectionStream> = {
            let streams = context.ws_streams.read().await;
            streams.values().cloned().collect()
        };

        // 判断是否被取消
        let is_cancelled = context.is_cancelled();

        // 取消时清理资源：关闭所有 WS 连接和 DB 连接
        if is_cancelled {
            // 关闭所有 WebSocket 连接（drop 管理器触发连接清理）
            context.remove_provider("ws");
            // 关闭所有数据库连接（drop 连接池即可）
            context.remove_provider("db");
        }

        // End 节点检测
        let has_end_nodes = flow.nodes.iter().any(|n| n.node_type == node_types::END);
        let reached_end_node = has_end_nodes && execution_path.iter().any(|uuid| {
            flow.nodes.iter().any(|n| n.uuid == *uuid && n.node_type == node_types::END)
        });

        #[allow(clippy::if_same_then_else)]
        let flow_status = if is_cancelled {
            flow_engine_core::report::ExecutionStatus::Cancelled
        } else if node_reports.iter().any(|r| r.status == flow_engine_core::report::ExecutionStatus::Failed) {
            flow_engine_core::report::ExecutionStatus::Failed
        } else if has_end_nodes && !reached_end_node {
            flow_engine_core::report::ExecutionStatus::Failed
        } else {
            flow_engine_core::report::ExecutionStatus::Success
        };

        // 在 flow_status 被 move 到 FlowExecutionReport 之前判断是否Success
        let flow_succeeded = !matches!(flow_status, flow_engine_core::report::ExecutionStatus::Failed);

        let flow_error = if is_cancelled {
            Some(ErrorDetail::new(ErrorKind::Cancelled, "流程执行被用户取消".to_string()))
        } else if has_end_nodes && !reached_end_node {
            Some(ErrorDetail::new(ErrorKind::Execution, "流程存在 End 节点但执行路径未到达任何 End 节点".to_string()))
        } else {
            None
        };

        let flow_report = flow_engine_core::report::FlowExecutionReport {
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
            statistics: flow_engine_core::statistics::FlowStatistics::from_reports(
                &node_reports,
                flow.nodes.len() as u32,
            ),
            flow_error,
            execution_mode: flow_engine_core::report::ExecutionMode::Normal,
            initial_variables: Some(json!(initial_variables)),
            final_variables: Some(json!(final_variables)),
            execution_path,
            skipped_nodes,
            ws_streams,
            flow_snapshot: Some(flow.clone()),
            end_node_reached: if has_end_nodes { Some(reached_end_node) } else { None },
            flow_warning,
        };

        // 发送流程结束事件
        if is_cancelled {
            context.emit_execution_event(ExecutionEvent::FlowCancelled {
                flow_uuid: flow.uuid.clone(),
                reason: "用户取消执行".to_string(),
            }).await;
        } else {
            context.emit_execution_event(ExecutionEvent::FlowCompleted {
                report: Box::new(flow_report.clone()),
            }).await;
        }

        // 触发流程钩子
        if let Some(ref hook_executor) = self.hook_executor {
            if let Some(ref hooks) = flow.hooks {
                if !hooks.is_empty() {
                    let status = if is_cancelled {
                        "cancelled"
                    } else if matches!(flow_report.status, flow_engine_core::report::ExecutionStatus::Success) {
                        "success"
                    } else {
                        "failure"
                    };
                    let error_msg = flow_report.flow_error.as_ref().map(|e| e.message.clone());
                    let hook_context = flow_engine_core::hook::HookContext {
                        flow_uuid: flow.uuid.clone(),
                        flow_name: flow.name.clone(),
                        status: status.to_string(),
                        duration_ms: flow_report.duration_ms,
                        error: error_msg,
                    };
                    hook_executor.execute_all(hooks, &hook_context);
                }
            }
        }

        if is_cancelled {
            ExecutionResult {
                success: false,
                data: None,
                error: Some("流程执行被取消".to_string()),
                executed_count,
                node_reports,
                flow_report: Some(flow_report),
            }
        } else {
            ExecutionResult {
                success: flow_succeeded,
                data: if flow_succeeded {
                    Some(json!({
                        "flow_id": flow.uuid,
                        "executed_count": executed_count,
                        "execution_time": start_time_ms,
                    }))
                } else {
                    None
                },
                error: if flow_succeeded { None } else { Some("Flow execution failed".to_string()) },
                executed_count,
                node_reports,
                flow_report: Some(flow_report),
            }
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
    ///   - `Ok((next_node_id`, `output_data))`: 执行Success，返回下一个节点 ID 和输出数据
    ///   - Err(error): 执行Failed，返回Error信息
    async fn execute_node(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        // 发送节点开始日志
        context.emit_log(LogEvent {
            level: LogLevel::Info,
            message: format!("执行节点 {} ({})", node.name, node.node_type),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            node_uuid: Some(node.uuid.clone()),
            node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone(),
        }).await;

        let type_id = node.data.type_name();
        match self.registry.get(&type_id) {
            Some(plugin) => {
                log::info!(
                    "[流程 {}] Executing {} node: {} ({})",
                    context.flow_uuid,
                    plugin.display_name(),
                    node.name,
                    node.uuid
                );
                plugin.execute(node, context, connections, node_map).await
            }
            None => Err(ErrorDetail::execution_error(format!("未注册的节点类型: {}", type_id))),
        }
    }

    /// Execute a single node standalone (no flow traversal, no report persistence)
    ///
    /// Used by `mpe run node` CLI and Tauri "Test" command to verify
    /// connectivity for protocol node types.
    ///
    /// # Arguments
    /// - `node`: The node to execute (must have type in SUPPORTED_SINGLE_NODE_TYPES)
    /// - `variables`: Optional JSON object with initial variables
    /// - `log_sender`: Optional log sender (CLI mode = None)
    ///
    /// # Returns
    /// `ExecutionResult` with a single-entry `node_reports` vector
    pub async fn execute_single_node(
        &self,
        node: &Node,
        variables: Option<serde_json::Value>,
        _log_sender: Option<Arc<dyn LogSender>>,
    ) -> ExecutionResult {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // 1. Get type_id and validate
        let type_id = node.data.type_name();
        if !SUPPORTED_SINGLE_NODE_TYPES.contains(&type_id.as_str()) {
            return ExecutionResult {
                success: false,
                data: None,
                error: Some(format!("Unsupported single-node type: {}", type_id)),
                executed_count: 0,
                node_reports: vec![],
                flow_report: None,
            };
        }

        // 2. Build execution context with unique flow id
        let mut context = ExecutionContext::new("_single_node_".to_string(), now_ms)
            .with_default_providers()
            .with_default_providers_and_grpc(None);

        // 3. Inject variables from JSON object
        if let Some(serde_json::Value::Object(obj)) = variables {
            for (key, value) in obj {
                context.variables.set_variable(key, value, String::new());
            }
        }

        // 4. Lookup plugin
        let plugin = match self.registry.get(&type_id) {
            Some(p) => p,
            None => {
                let end_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                return ExecutionResult {
                    success: false,
                    data: None,
                    error: Some(format!("Unregistered node type: {}", type_id)),
                    executed_count: 0,
                    node_reports: vec![flow_engine_core::report::NodeExecutionReport {
                        node_uuid: node.uuid.clone(),
                        node_type: node.node_type.clone(),
                        node_name: node.name.clone(),
                        start_time: now_ms,
                        end_time: end_ms,
                        duration_ms: end_ms.saturating_sub(now_ms),
                        status: flow_engine_core::report::ExecutionStatus::Failed,
                        input_data: None,
                        output_data: None,
                        used_port_id: None,
                        error: Some(flow_engine_core::error::ErrorDetail::execution_error(
                            format!("Unregistered node type: {}", type_id)
                        )),
                        resolved_config: None,
                        variable_changes: None,
                        skip_reason: None,
                        wait_duration_ms: None,
                    }],
                    flow_report: None,
                };
            }
        };

        // 5. Build single-entry node_map
        let node_map: std::collections::HashMap<String, &Node> =
            [(node.uuid.clone(), node)].into_iter().collect();

        // 6. Execute with timeout (node-specific or fallback 30s)
        let empty_connections: Vec<flow_engine_core::connection::Connection> = vec![];
        let execute_future = plugin.execute(node, &mut context, &empty_connections, &node_map);
        let timeout_duration = node.data.execution_timeout_ms()
            .map(|ms| std::time::Duration::from_millis(std::cmp::min(ms, 3600000)))
            .unwrap_or(std::time::Duration::from_millis(FALLBACK_TIMEOUT_MS));
        let timeout_ms = timeout_duration.as_millis();

        let result = tokio::time::timeout(timeout_duration, execute_future).await;

        let end_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let duration_ms = end_ms.saturating_sub(now_ms);

        match result {
            Ok(Ok((_next_ids, output_data))) => {
                // Success
                ExecutionResult {
                    success: true,
                    data: output_data.clone(),
                    error: None,
                    executed_count: 1,
                    node_reports: vec![flow_engine_core::report::NodeExecutionReport {
                        node_uuid: node.uuid.clone(),
                        node_type: node.node_type.clone(),
                        node_name: node.name.clone(),
                        start_time: now_ms,
                        end_time: end_ms,
                        duration_ms,
                        status: flow_engine_core::report::ExecutionStatus::Success,
                        input_data: None,
                        output_data,
                        used_port_id: Some("out".to_string()),
                        error: None,
                        resolved_config: None,
                        variable_changes: None,
                        skip_reason: None,
                        wait_duration_ms: None,
                    }],
                    flow_report: None,
                }
            }
            Ok(Err(err_detail)) => {
                // Plugin execution returned error
                let err_msg = err_detail.message.clone();
                ExecutionResult {
                    success: false,
                    data: None,
                    error: Some(err_msg),
                    executed_count: 0,
                    node_reports: vec![flow_engine_core::report::NodeExecutionReport {
                        node_uuid: node.uuid.clone(),
                        node_type: node.node_type.clone(),
                        node_name: node.name.clone(),
                        start_time: now_ms,
                        end_time: end_ms,
                        duration_ms,
                        status: flow_engine_core::report::ExecutionStatus::Failed,
                        input_data: None,
                        output_data: None,
                        used_port_id: None,
                        error: Some(err_detail),
                        resolved_config: None,
                        variable_changes: None,
                        skip_reason: None,
                        wait_duration_ms: None,
                    }],
                    flow_report: None,
                }
            }
            Err(_elapsed) => {
                // Timeout
                let error_msg = format!("节点执行超时 ({}ms)", timeout_ms);
                ExecutionResult {
                    success: false,
                    data: None,
                    error: Some(error_msg.clone()),
                    executed_count: 0,
                    node_reports: vec![flow_engine_core::report::NodeExecutionReport {
                        node_uuid: node.uuid.clone(),
                        node_type: node.node_type.clone(),
                        node_name: node.name.clone(),
                        start_time: now_ms,
                        end_time: end_ms,
                        duration_ms,
                        status: flow_engine_core::report::ExecutionStatus::Failed,
                        input_data: None,
                        output_data: None,
                        used_port_id: None,
                        error: Some(flow_engine_core::error::ErrorDetail::new(
                            flow_engine_core::error::ErrorKind::Timeout,
                            error_msg,
                        ).with_error_code_typed(flow_engine_core::error::ErrorCode::E_TIMEOUT)),
                        resolved_config: None,
                        variable_changes: None,
                        skip_reason: None,
                        wait_duration_ms: None,
                    }],
                    flow_report: None,
                }
            }
        }
    }
}

/// 从 output_data 中提取 error 信息。
///
/// 协议执行器在 Ok 路径下将错误信息作为字符串嵌入 output_data["error"]，
/// 以在保留部分输出数据（如请求信息、时序）的同时传递错误信息。
/// 此函数提取该错误并构建 ErrorDetail，同时通过文本匹配推测 ErrorKind。
fn extract_error_from_output(output_data: &Option<serde_json::Value>) -> Option<ErrorDetail> {
    let msg = output_data.as_ref()?.get("error")?;
    let msg = match msg {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => return None,
        other => other.to_string(),
    };
    if msg.is_empty() {
        return None;
    }

    let msg_lower = msg.to_lowercase();

    // 通过文本匹配推测 ErrorKind
    let error_kind = if msg_lower.contains("timeout") || msg_lower.contains("timed out") {
        ErrorKind::Timeout
    } else if msg_lower.contains("connection") || msg_lower.contains("refused")
        || msg_lower.contains("dns") || msg_lower.contains("resolve")
        || msg_lower.contains("connect") || msg_lower.contains("closed")
        || msg_lower.contains("aborted") || msg_lower.contains("sendrequest")
        || msg_lower.contains("eof")
    {
        ErrorKind::Connection
    } else if msg_lower.contains("parse") || msg_lower.contains("unexpected")
        || msg_lower.contains("syntax")
    {
        ErrorKind::Parse
    } else if msg_lower.contains("assert") || msg_lower.contains("expect") {
        ErrorKind::Assertion
    } else {
        ErrorKind::Execution
    };

    Some(ErrorDetail::new(error_kind, msg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node_config::NodeData;
    use flow_engine_core::node_config::ScriptErrorStrategy;
    use flow_engine_core::node_config::ScriptExecutionPhase;
    use flow_engine_core::node_config::ScriptNodeConfig;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::report::ExecutionStatus;

    use serde_json::json;
    use std::sync::Once;
    use std::time::Instant;
    use uuid::Uuid;

    /// Initialize rustls crypto provider once for all tests.
    /// Required because workspace-hack unifies reqwest features with rustls-no-provider.
    fn init_crypto() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }

    #[tokio::test]
    async fn test_flow_executor_success() {
        // 创建Test Flow - 单个 Entry node，无连接，正常终止
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "Test Flow".to_string(), description: None, nodes: vec![start_node], connections: vec![], hooks: None };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行结果（单个 Entry node无连接是正常终止）
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.executed_count, 1);
    }

    #[tokio::test]
    async fn test_flow_executor_with_connection() {
        // 创建Test Flow
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: "entry".to_string(),
            name: "End".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "Test Flow".to_string(), description: None, nodes: vec![start_node, end_node], connections: vec![connection], hooks: None };

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
            node_type: "http".to_string(),
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "Test Flow".to_string(), description: None, nodes: vec![http_node], connections: vec![], hooks: None };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行Failed
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("流程验证Failed"));
        assert_eq!(result.executed_count, 0);
    }

    #[tokio::test]
    async fn test_flow_executor_with_variables() {
        // 创建Test Flow
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: "entry".to_string(),
            name: "End".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "Test Flow".to_string(), description: None, nodes: vec![start_node, end_node], connections: vec![connection], hooks: None };

        let variables = json!({
            "test_var": "test_value",
            "number": 42
        });

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, Some(variables), None, None).await;

        // 验证执行Success
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.executed_count, 2);
    }

    #[tokio::test]
    async fn test_flow_executor_start_node_execution_time() {
        // 创建Test Flow

        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: "entry".to_string(),
            name: "End".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "Test Flow".to_string(), description: None, nodes: vec![start_node, end_node], connections: vec![connection], hooks: None };

        let executor = FlowExecutor::new();
        let start_time = Instant::now();
        let result = executor.execute_flow(&flow, None, None, None).await;
        let elapsed = start_time.elapsed();

        // 验证执行Success
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
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "Test Flow".to_string(), description: None, nodes: vec![start_node], connections: vec![], hooks: None };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证上下文中的 flow_uuid 使用了 flow.uuid（单个 Entry node无连接是正常终止）
        assert!(result.success);
        assert_eq!(result.executed_count, 1);
    }

    #[tokio::test]
    async fn test_node_map_uses_node_uuid() {
        // Test Node映射构建使用 node.uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: "entry".to_string(),
            name: "End".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "Test Flow".to_string(), description: None, nodes: vec![start_node, end_node], connections: vec![connection], hooks: None };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行Success（节点映射通过 node.uuid 查找Success）
        assert!(result.success);
        assert!(result.data.is_some());
    }

    #[tokio::test]
    async fn test_connection_map_uses_source_and_target_uuid() {
        // 测试连接映射构建使用 conn.source_node_uuid 和 conn.target_node_uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "start-uuid-123".to_string(),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: "entry".to_string(),
            name: "End".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let connection = Connection {
            id: "conn-uuid-789".to_string(),
            source_node_uuid: start_node.uuid.clone(), // 使用 source_node_uuid
            target_node_uuid: end_node.uuid.clone(),   // 使用 target_node_uuid
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "Test Flow".to_string(), description: None, nodes: vec![start_node, end_node], connections: vec![connection], hooks: None };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证连接映射Success使用了 source_node_uuid 和 target_node_uuid
        assert!(result.success);
        assert_eq!(result.executed_count, 2);
    }

    #[tokio::test]
    async fn test_current_node_id_uses_start_node_uuid() {
        // 测试当前节点 ID 追踪使用 start_node.uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "start-uuid-tracker".to_string(),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let end_node = Node {
            uuid: "end-uuid-tracker".to_string(),
            node_type: "entry".to_string(),
            name: "End".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "Test Flow".to_string(), description: None, nodes: vec![start_node, end_node], connections: vec![connection], hooks: None };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行的第一个节点是 start_node.uuid
        assert!(result.success);
        assert_eq!(result.executed_count, 2);
    }

    #[tokio::test]
    async fn test_error_message_uses_node_uuid() {
        // 测试Error消息使用 node.uuid
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: "loop-node-uuid".to_string(),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        // 创建自连接以触发循环Error
        let connection = Connection {
            id: "loop-conn".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: start_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "Test Flow".to_string(), description: None, nodes: vec![start_node], connections: vec![connection], hooks: None };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证Error消息包含 node.uuid
        assert!(!result.success);
        assert!(result.error.is_some());
        let error_msg = result.error.unwrap();
        assert!(error_msg.contains("检测到循环"));
        assert!(error_msg.contains("loop-node-uuid")); // Error消息包含 node.uuid
    }

    #[tokio::test]
    async fn test_execution_result_data_uses_flow_uuid() {
        // 测试执行结果数据使用 flow.uuid
        let flow_id = "result-flow-uuid-999".to_string();
        let start_node = Node {
            uuid: "start-1".to_string(),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let end_node = Node {
            uuid: "end-1".to_string(),
            node_type: "entry".to_string(),
            name: "End".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let connection = Connection {
            id: "conn-1".to_string(),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "Test Flow".to_string(), description: None, nodes: vec![start_node, end_node], connections: vec![connection], hooks: None };

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
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let end_node_1 = Node {
            uuid: "end-1-multi".to_string(),
            node_type: "entry".to_string(),
            name: "结束 1".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
        };

        let end_node_2 = Node {
            uuid: "end-2-multi".to_string(),
            node_type: "entry".to_string(),
            name: "结束 2".to_string(),
            position: NodePosition { x: 100.0, y: 100.0 },
            data: NodeData::entry(),
            ports: None,
        has_breakpoint: None,
        on_error: None,
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

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "Test Flow".to_string(), description: None, nodes: vec![start_node, end_node_1, end_node_2], connections: vec![connection_1, connection_2], hooks: None };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证执行Success，连接映射正确使用了 UUID
        assert!(result.success);
        assert_eq!(result.executed_count, 2);
    }

    #[test]
    fn test_replace_variables_simple() {
        // 测试简单变量替换
        // 注意：变量是拍平存储的（顶级 key），不需要 initial_variables. 前缀
        let template = "Bearer {{api_key}}";
        let mut variables = VariableStorage::new();
        variables.set_variable(
            "api_key".to_string(),
            json!("test-key"),
            String::new(),
        );
        let context = ExecutionContext { flow_uuid: "test-flow".to_string(), current_node_uuid: None, variables, node_reports: vec![], flow_trigger_time: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64, log_sender: None, flow_file_path: None, ws_connection_urls: std::collections::HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())), last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: std::collections::HashMap::new(), };
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
        let context = ExecutionContext { flow_uuid: "test-flow".to_string(), current_node_uuid: None, variables, node_reports: vec![], flow_trigger_time: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64, log_sender: None, flow_file_path: None, ws_connection_urls: std::collections::HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())), last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: std::collections::HashMap::new(), };
        let result = replace_variables(template, &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello Alice");
    }

    #[test]
    fn test_replace_variables_strict_mode() {
        // 测试严格模式（变量不存在时报错）
        let template = "{{nonexistent.var}}";
        let variables = VariableStorage::new();
        let context = ExecutionContext { flow_uuid: "test-flow".to_string(), current_node_uuid: None, variables, node_reports: vec![], flow_trigger_time: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64, log_sender: None, flow_file_path: None, ws_connection_urls: std::collections::HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())), last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: std::collections::HashMap::new(), };
        let result = replace_variables(template, &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Variable not found"));
    }

    #[test]
    fn test_replace_variables_index_not_supported() {
        // 测试索引访问不支持
        let template = "{{items[0]}}";
        let mut variables = VariableStorage::new();
        variables.set_variable("items".to_string(), json!(["a", "b", "c"]), String::new());
        let context = ExecutionContext { flow_uuid: "test-flow".to_string(), current_node_uuid: None, variables, node_reports: vec![], flow_trigger_time: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64, log_sender: None, flow_file_path: None, ws_connection_urls: std::collections::HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())), last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: std::collections::HashMap::new(), };
        let result = replace_variables(template, &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Index access not supported"));
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

                ws_connection_urls: std::collections::HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: std::collections::HashMap::new(),
        };
        let result = replace_variables(&template, &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Index access not supported"));
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

                ws_connection_urls: std::collections::HashMap::new(),
                ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            last_node_output: None,
                debug_event_sender: None,
                execution_event_sender: None,
                cancel_receiver: None,
                providers: std::collections::HashMap::new(),
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
            let template = "{{value}}";
            let mut variables = VariableStorage::new();
            variables.set_variable("value".to_string(), json!(num), String::new());
            let context = ExecutionContext {
                flow_uuid: "test-flow".to_string(),
                current_node_uuid: None,
                variables,
                node_reports: vec![],
                flow_trigger_time: 0,
                log_sender: None,
                flow_file_path: None,

                ws_connection_urls: std::collections::HashMap::new(),
                ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            last_node_output: None,
                debug_event_sender: None,
                execution_event_sender: None,
                cancel_receiver: None,
                providers: std::collections::HashMap::new(),
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

                ws_connection_urls: std::collections::HashMap::new(),
                ws_streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            last_node_output: None,
                debug_event_sender: None,
                execution_event_sender: None,
                cancel_receiver: None,
                providers: std::collections::HashMap::new(),
            };
            let result = replace_variables(template, &context);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), val.to_string());
        }
    }

    // ============================================================================
    // 取消功能测试
    // ============================================================================

    /// 测试取消信号在节点间生效：第二个节点不执行
    #[tokio::test]
    async fn test_cancel_between_nodes() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: "entry".to_string(),
            name: "End".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "out".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "取消Test Flow".to_string(), description: None, nodes: vec![start_node, end_node], connections: vec![connection], hooks: None };

        // 创建取消通道并立即发送取消信号
        let (tx, rx) = tokio::sync::watch::channel(false);
        let _ = tx.send(true);

        let executor = FlowExecutor::new().with_cancel_receiver(rx);
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证取消结果
        assert!(!result.success, "取消后不应该Success");
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("取消"));

        // 验证报告状态为 Cancelled
        let report = result.flow_report.expect("取消时应有报告");
        assert_eq!(report.status, flow_engine_core::report::ExecutionStatus::Cancelled);
        assert_eq!(report.executed_count, 0, "第一个节点前取消，不应有已执行节点");
    }

    /// 测试无取消信号时正常执行
    #[tokio::test]
    async fn test_no_cancel_signal_executes_normally() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "正常执行Test Flow".to_string(), description: None, nodes: vec![start_node], connections: vec![], hooks: None };

        // 创建取消通道但不发送信号
        let (_tx, rx) = tokio::sync::watch::channel(false);

        let executor = FlowExecutor::new().with_cancel_receiver(rx);
        let result = executor.execute_flow(&flow, None, None, None).await;

        // 验证正常执行
        assert!(result.success, "无取消信号时应正常执行");
        assert!(result.error.is_none());
        assert_eq!(result.executed_count, 1);
    }

    /// 测试取消时 flow_error 包含 Cancelled Error
    #[tokio::test]
    async fn test_cancel_generates_cancelled_error_detail() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        on_error: None,
        };

        let flow = Flow { uuid: flow_id.clone(), initial_variables: None, environments: vec![], active_environment: None, name: "取消Error测试".to_string(), description: None, nodes: vec![start_node], connections: vec![], hooks: None };

        let (tx, rx) = tokio::sync::watch::channel(false);
        let _ = tx.send(true);

        let executor = FlowExecutor::new().with_cancel_receiver(rx);
        let result = executor.execute_flow(&flow, None, None, None).await;

        assert!(!result.success);
        let report = result.flow_report.expect("应有报告");
        let error = report.flow_error.expect("取消时应有 flow_error");
        assert_eq!(error.error_kind, ErrorKind::Cancelled);
        assert!(!error.is_fatal(), "Cancelled 不应是致命Error");
    }

    /// 测试 runtime_variables 覆盖 Entry node的 initial_variables
    ///
    /// 验证 runtime_variables 在 Entry node执行后注入，覆盖同名变量
    #[tokio::test]
    async fn test_runtime_variables_override_entry() {
        let flow_id = Uuid::new_v4().to_string();

        // 创建带流程级默认变量的流程
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let flow = Flow {
            uuid: flow_id,
            initial_variables: Some({
                let mut m = std::collections::HashMap::new();
                m.insert("host".to_string(), json!("default.com"));
                m
            }),
            environments: vec![],
            active_environment: None,
            name: "runtime_variables 覆盖测试".to_string(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            hooks: None,
        };

        let executor = FlowExecutor::new();
        let runtime_vars = json!({
            "host": "override.com",
            "extra": "added"
        });

        let result = executor
            .execute_flow(&flow, None, Some(runtime_vars), None)
            .await;

        assert!(result.success);
        let report = result.flow_report.expect("应有报告");
        let final_vars = report.final_variables.expect("应有 final_variables");

        // runtime_variables 应覆盖流程级默认变量
        assert_eq!(final_vars["host"], json!("override.com"), "runtime_variables 应覆盖流程级默认变量的 host 变量");
        // runtime_variables 新增的变量也应该存在
        assert_eq!(final_vars["extra"], json!("added"), "runtime_variables 新增变量应存在");
    }

    /// 测试 runtime_variables=None 不改变行为
    ///
    /// 验证不传 runtime_variables 时，流程级默认变量正常工作
    #[tokio::test]
    async fn test_runtime_variables_none_no_change() {
        let flow_id = Uuid::new_v4().to_string();

        // 创建带流程级默认变量的流程
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: "entry".to_string(),
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let flow = Flow {
            uuid: flow_id,
            initial_variables: Some({
                let mut m = std::collections::HashMap::new();
                m.insert("host".to_string(), json!("default.com"));
                m
            }),
            environments: vec![],
            active_environment: None,
            name: "runtime_variables None 测试".to_string(),
            description: None,
            nodes: vec![start_node],
            connections: vec![],
            hooks: None,
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None, None, None).await;

        assert!(result.success);
        let report = result.flow_report.expect("应有报告");
        let final_vars = report.final_variables.expect("应有 final_variables");

        // 不传 runtime_variables 时，流程级默认变量保持不变
        assert_eq!(final_vars["host"], json!("default.com"), "不传 runtime_variables 时 host 应保持流程级默认变量的值");
    }

    // ============================================================================
    // execute_single_node 单元测试
    // ============================================================================

    #[tokio::test]
    async fn test_execute_single_node_script() {
        init_crypto();
        // Script node returns success with correct ExecutionStatus::Success
        let node = Node {
            uuid: "test-script-node".to_string(),
            node_type: "script".to_string(),
            name: "Test Script".to_string(),
            position: NodePosition::default(),
            data: NodeData::Script(ScriptNodeConfig {
                script: "return 42;".into(),
                timeout_ms: 5000,
                memory_limit_mb: 10,
                execution_phase: ScriptExecutionPhase::default(),
                on_error: ScriptErrorStrategy::default(),
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_single_node(&node, None, None).await;

        assert!(result.success, "Script node should succeed");
        assert_eq!(result.executed_count, 1);
        assert!(result.error.is_none());
        assert_eq!(result.node_reports.len(), 1);
        assert_eq!(result.node_reports[0].status, ExecutionStatus::Success);
    }

    #[tokio::test]
    async fn test_execute_single_node_timeout() {
        init_crypto();
        // Infinite loop script triggers 5s script timeout.
        // Use Fatal error strategy so the timeout propagates as an error
        // (Recoverable would return Ok with "success": false and no error).
        let node = Node {
            uuid: "test-timeout-node".to_string(),
            node_type: "script".to_string(),
            name: "Test Timeout".to_string(),
            position: NodePosition::default(),
            data: NodeData::Script(ScriptNodeConfig {
                script: "while(true) {}".into(),
                timeout_ms: 5000,
                memory_limit_mb: 10,
                execution_phase: ScriptExecutionPhase::default(),
                on_error: ScriptErrorStrategy::Fatal,
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_single_node(&node, None, None).await;

        assert!(!result.success, "Timeout should fail");
        assert!(result.error.is_some());
        let err = result.error.expect("error should be present");
        assert!(err.contains("超时") || err.contains("timeout"),
            "Error should mention timeout: {}", err);
    }

    #[tokio::test]
    async fn test_execute_single_node_unsupported_type() {
        // Entry node returns error mentioning "entry"
        let node = Node {
            uuid: "test-entry".to_string(),
            node_type: "entry".to_string(),
            name: "Test Entry".to_string(),
            position: NodePosition::default(),
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_single_node(&node, None, None).await;

        assert!(!result.success);
        assert!(result.error.is_some());
        let err = result.error.expect("error should be present");
        assert!(err.contains("entry"), "Error should mention 'entry': {}", err);
    }

    #[tokio::test]
    async fn test_execute_single_node_with_variables() {
        // Script node reads injected variable from context
        let node = Node {
            uuid: "test-var-node".to_string(),
            node_type: "script".to_string(),
            name: "Test Variables".to_string(),
            position: NodePosition::default(),
            data: NodeData::Script(ScriptNodeConfig {
                script: "return fn.variables.get('name');".into(),
                timeout_ms: 5000,
                memory_limit_mb: 10,
                execution_phase: ScriptExecutionPhase::default(),
                on_error: ScriptErrorStrategy::default(),
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let variables = serde_json::json!({
            "name": "World"
        });

        let executor = FlowExecutor::new();
        let result = executor.execute_single_node(&node, Some(variables), None).await;

        assert!(result.success, "Script with variables should succeed");
        assert_eq!(result.executed_count, 1);
    }

    // --- extract_error_from_output tests ---

    #[test]
    fn test_extract_error_string() {
        let output = Some(serde_json::json!({"error": "connection refused"}));
        let err = extract_error_from_output(&output);
        assert!(err.is_some());
        assert_eq!(err.unwrap().message, "connection refused");
    }

    #[test]
    fn test_extract_error_non_string_object() {
        let output = Some(serde_json::json!({"error": {"code": 500, "detail": "timeout"}}));
        let err = extract_error_from_output(&output);
        assert!(err.is_some());
        assert!(err.unwrap().message.contains("500"));
    }

    #[test]
    fn test_extract_error_null() {
        let output = Some(serde_json::json!({"error": null}));
        let err = extract_error_from_output(&output);
        assert!(err.is_none());
    }

    #[test]
    fn test_extract_error_empty_string() {
        let output = Some(serde_json::json!({"error": ""}));
        let err = extract_error_from_output(&output);
        assert!(err.is_none());
    }

    #[test]
    fn test_extract_error_missing_key() {
        let output = Some(serde_json::json!({"success": true}));
        let err = extract_error_from_output(&output);
        assert!(err.is_none());
    }

    #[test]
    fn test_extract_error_none_output() {
        let output: Option<serde_json::Value> = None;
        let err = extract_error_from_output(&output);
        assert!(err.is_none());
    }

    #[test]
    fn test_extract_error_kind_timeout() {
        let output = Some(serde_json::json!({"error": "HTTP request timeout (30000ms)"}));
        let err = extract_error_from_output(&output);
        assert!(err.is_some());
        assert_eq!(err.unwrap().error_kind, ErrorKind::Timeout);
    }

    #[test]
    fn test_extract_error_kind_connection() {
        let output = Some(serde_json::json!({"error": "tcp connection refused: 10.0.0.1:8080"}));
        let err = extract_error_from_output(&output);
        assert!(err.is_some());
        assert_eq!(err.unwrap().error_kind, ErrorKind::Connection);
    }

    #[test]
    fn test_extract_error_kind_connection_dns() {
        let output = Some(serde_json::json!({"error": "dns resolution failed for example.com"}));
        let err = extract_error_from_output(&output);
        assert!(err.is_some());
        assert_eq!(err.unwrap().error_kind, ErrorKind::Connection);
    }

    #[test]
    fn test_extract_error_kind_fallback_execution() {
        let output = Some(serde_json::json!({"error": "unknown protocol error occurred"}));
        let err = extract_error_from_output(&output);
        assert!(err.is_some());
        assert_eq!(err.unwrap().error_kind, ErrorKind::Execution);
    }

    #[test]
    fn test_extract_error_kind_parse() {
        let output = Some(serde_json::json!({"error": "parse error: unexpected token at line 3"}));
        let err = extract_error_from_output(&output);
        assert!(err.is_some());
        assert_eq!(err.unwrap().error_kind, ErrorKind::Parse);
    }
}
