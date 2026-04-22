//! 脚本节点执行器
//!
//! 执行 JavaScript 脚本，支持变量操作、响应处理和流程控制。
//!
//! # 功能
//!
//! - 使用 `QuickJS` 执行 JavaScript 脚本
//! - 支持变量读写 (`fn.variables.*`)
//! - 支持响应数据访问 (`fn.response.*`)
//! - 支持请求数据访问 (`fn.request.*`)
//! - 支持日志输出 (`fn.console.*`)
//! - 支持流程控制 (`fn.flow.*`)
//! - 支持流程/节点信息 (`fn.info.*`)
//! - 支持工具函数 (`fn.util.*`)
//! - 支持链式断言 (`fn.expect()`)
//! - 内存限制和超时控制
//! - 错误处理策略 (致命/可恢复)

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::{ErrorDetail, ErrorKind};
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{NodeData, ScriptErrorStrategy, ScriptNodeConfig};
use crate::script_api::{LogEntry, RequestData, ResponseData, ScriptExecutionContext};
use rquickjs::{CatchResultExt, Context as JsContext, Runtime as JsRuntime};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// 脚本执行结果
struct ScriptExecutionResult {
    /// 脚本修改后的变量
    variables: HashMap<String, serde_json::Value>,
    /// 脚本通过 `fn.console` 输出的日志
    logs: Vec<LogEntry>,
    /// 是否跳过后续节点 (`fn.flow.skip()`)
    should_skip: bool,
    /// 是否终止流程并报错 (`fn.flow.abort()`)
    should_abort: bool,
    /// 终止消息
    abort_message: Option<String>,
    /// 是否正常终止流程 (`fn.flow.stop()`)
    should_stop: bool,
    /// `fn.expect()` 断言结果
    test_results: Vec<crate::script_api::TestResult>,
}

/// 脚本节点执行器
pub struct ScriptNodeExecutor;

impl ScriptNodeExecutor {
    /// 创建新的脚本节点执行器
    pub fn new() -> Self {
        Self
    }
}

impl Default for ScriptNodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl NodeExecutor for ScriptNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        // 发送执行开始日志
        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("脚本节点 {} 开始执行", node.name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        log::info!(
            "[流程 {}] 执行脚本节点: {} ({})",
            context.flow_uuid,
            node.name,
            node.uuid
        );

        // 1. 获取配置
        let config = match &node.data {
            NodeData::Script(config) => config.clone(),
            _ => return Err("节点配置类型不匹配，期望 ScriptNodeConfig".to_string()),
        };

        // 2. 准备脚本上下文数据（在主线程提取，避免跨线程问题）
        let (initial_vars, response_data, request_data) =
            self.prepare_context_data(node, context);

        // 3. 准备执行所需的数据（在主线程提取，避免跨线程问题）
        let script = config.script.clone();
        let timeout_ms = config.timeout_ms;
        let memory_limit_mb = config.memory_limit_mb;

        // 超时中断标志（在外部创建，以便超时/取消时能设置）
        let timeout_flag = Arc::new(AtomicBool::new(false));
        let timeout_flag_for_interrupt = timeout_flag.clone();

        // 取消信号监听器：收到取消信号时设置 timeout_flag，触发 JS 中断
        let cancel_handle = context.cancel_receiver.as_ref().map(|rx| {
            let mut rx = rx.clone();
            let flag = timeout_flag.clone();
            tokio::spawn(async move {
                // 等待取消信号（值变为 true）
                if rx.changed().await.is_ok() {
                    flag.store(true, Ordering::SeqCst);
                }
            })
        });

        // 4. 准备流程/节点信息
        let flow_uuid = context.flow_uuid.clone();
        let node_uuid = node.uuid.clone();
        let node_name = node.name.clone();

        // 5. 执行脚本（在 spawn_blocking 内部创建 Runtime 和 Context，避免 Send 问题）
        // ScriptExecutionContext 使用 Rc<RefCell>（非 Send），必须在闭包内创建
        let result = tokio::task::spawn_blocking(move || {
            // 创建 QuickJS Runtime 并设置内存/超时限制
            let rt = JsRuntime::new().map_err(|e| format!("创建 JS Runtime 失败: {}", e))?;
            rt.set_memory_limit((memory_limit_mb * 1024 * 1024) as usize);

            // 设置超时中断处理器
            rt.set_interrupt_handler(Some(Box::new(move || {
                timeout_flag_for_interrupt.load(Ordering::Relaxed)
            })));

            // 创建 JS Context
            let ctx = JsContext::full(&rt).map_err(|e| format!("创建 JS Context 失败: {}", e))?;

            ctx.with(|ctx| {
                // 构建脚本执行上下文（在闭包内创建，因为使用 Rc<RefCell>）
                let mut script_ctx = ScriptExecutionContext::new();

                // 注入初始变量
                for (key, value) in &initial_vars {
                    script_ctx
                        .variables
                        .borrow_mut()
                        .insert(key.clone(), value.clone());
                }

                // 设置响应数据
                if let Some(ref response) = response_data {
                    script_ctx = script_ctx.with_response(response.clone());
                }

                // 设置请求数据
                if let Some(ref request) = request_data {
                    script_ctx = script_ctx.with_request(request.clone());
                }

                // 设置流程/节点信息
                script_ctx = script_ctx.with_flow_info(flow_uuid.clone(), flow_uuid.clone());
                script_ctx = script_ctx.with_node_info(node_uuid.clone(), node_name.clone());

                // 注册全部 fn.* API
                script_ctx
                    .register_all_apis(&ctx)
                    .map_err(|e| format!("注册脚本 API 失败: {}", e))?;

                // 执行脚本
                let _: () = ctx
                    .eval(script.as_bytes())
                    .catch(&ctx)
                    .map_err(|e| format!("脚本执行错误: {}", e))?;

                // 收集执行结果
                let result_vars = script_ctx.variables.borrow().clone();
                let logs = script_ctx.logs.borrow().clone();
                let should_skip = *script_ctx.should_skip.borrow();
                let should_abort = *script_ctx.should_abort.borrow();
                let abort_message = script_ctx.abort_message.borrow().clone();
                let should_stop = *script_ctx.should_stop.borrow();
                let test_results = script_ctx.test_results.borrow().clone();

                Ok::<_, String>(ScriptExecutionResult {
                    variables: result_vars,
                    logs,
                    should_skip,
                    should_abort,
                    abort_message,
                    should_stop,
                    test_results,
                })
            })
        });

        // 7. 等待结果（带超时）
        let result = tokio::time::timeout(Duration::from_millis(timeout_ms), result).await;

        // 清理取消监听器（脚本已结束，不再需要监听取消信号）
        if let Some(handle) = cancel_handle {
            handle.abort();
        }

        // 8. 处理结果
        match result {
            Ok(Ok(Ok(exec_result))) => {
                // 脚本执行成功，同步变量回 ExecutionContext
                for (key, value) in exec_result.variables {
                    log::debug!(
                        "[流程 {}] 同步变量: {} = {:?}",
                        context.flow_uuid,
                        key,
                        value
                    );
                    context.variables.set_variable(key, value, node.uuid.clone());
                }

                // 发送脚本 console 日志到执行上下文
                for log_entry in &exec_result.logs {
                    let level = match log_entry.level.as_str() {
                        "error" => LogLevel::Error,
                        "warn" => LogLevel::Warn,
                        "debug" => LogLevel::Debug,
                        _ => LogLevel::Info,
                    };
                    context
                        .emit_log(LogEvent {
                            level,
                            message: format!("[console.{}] {}", log_entry.level, log_entry.message),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .expect("系统时间错误")
                                .as_millis() as u64,
                            node_uuid: Some(node.uuid.clone()),
                            node_name: Some(node.name.clone()),
                            flow_uuid: context.flow_uuid.clone(),
                        })
                        .await;
                }

                // 处理 fn.flow.abort() — 终止流程并报错
                if exec_result.should_abort {
                    let msg = exec_result
                        .abort_message
                        .unwrap_or_else(|| "流程被脚本终止 (fn.flow.abort)".to_string());
                    return self
                        .handle_error(
                            ErrorDetail::new(ErrorKind::ScriptRuntime, msg),
                            &config,
                            context,
                            node,
                            connections,
                        )
                        .await;
                }

                // 发送完成日志
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Info,
                        message: format!("脚本节点 {} 执行完成", node.name),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                // 构建输出数据（包含断言结果和流程控制标记）
                let mut output_data = serde_json::json!({
                    "success": true,
                    "logs": exec_result.logs,
                });

                // 包含断言结果
                if !exec_result.test_results.is_empty() {
                    output_data["test_results"] = serde_json::json!(exec_result.test_results);
                    let all_passed = exec_result.test_results.iter().all(|r| r.passed);
                    output_data["all_assertions_passed"] = serde_json::json!(all_passed);
                }

                // 包含流程控制标记
                if exec_result.should_skip {
                    output_data["flow_skip"] = serde_json::json!(true);
                }
                if exec_result.should_stop {
                    output_data["flow_stop"] = serde_json::json!(true);
                }

                // 返回 true 端口的连接
                let next_nodes = self.get_next_nodes_by_port(node, "true", connections);
                Ok((next_nodes, Some(output_data)))
            }
            Ok(Ok(Err(e))) => {
                // 脚本执行错误
                self.handle_error(
                    ErrorDetail::new(ErrorKind::ScriptRuntime, e),
                    &config,
                    context,
                    node,
                    connections,
                )
                .await
            }
            Ok(Err(e)) => {
                // spawn_blocking 错误
                let error_msg = format!("脚本执行任务失败: {}", e);
                self.handle_error(
                    ErrorDetail::new(ErrorKind::ScriptRuntime, error_msg),
                    &config,
                    context,
                    node,
                    connections,
                )
                .await
            }
            Err(_) => {
                // 超时
                timeout_flag.store(true, Ordering::Relaxed);
                let error_msg = format!("脚本执行超时 ({}ms)", timeout_ms);

                // 发送超时日志
                context
                    .emit_log(LogEvent {
                        level: LogLevel::Error,
                        message: error_msg.clone(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("系统时间错误")
                            .as_millis() as u64,
                        node_uuid: Some(node.uuid.clone()),
                        node_name: Some(node.name.clone()),
                        flow_uuid: context.flow_uuid.clone(),
                    })
                    .await;

                self.handle_error(
                    ErrorDetail::new(ErrorKind::ScriptTimeout, error_msg),
                    &config,
                    context,
                    node,
                    connections,
                )
                .await
            }
        }
    }
}

impl ScriptNodeExecutor {
    /// 准备脚本上下文数据
    ///
    /// 返回 (初始变量, 响应数据, 请求数据)
    fn prepare_context_data(
        &self,
        _node: &Node,
        context: &ExecutionContext,
    ) -> (
        HashMap<String, serde_json::Value>,
        Option<ResponseData>,
        Option<RequestData>,
    ) {
        // 从 ExecutionContext 获取合并后的变量
        let initial_vars = context.variables.get_merged_variables();

        // 提取响应数据（如果有 last_node_output）
        let response_data = context
            .last_node_output
            .as_ref()
            .and_then(|output| self.extract_response_data(output));

        // 提取请求数据（如果有）
        let request_data = context
            .last_node_output
            .as_ref()
            .and_then(|output| self.extract_request_data(output));

        (initial_vars, response_data, request_data)
    }

    /// 从 `last_node_output` 提取响应数据
    fn extract_response_data(&self, output: &serde_json::Value) -> Option<ResponseData> {
        // 检查是否是 HTTP 响应格式
        let status = output.get("status")?.as_u64()? as u16;
        let body = output
            .get("body")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let headers = output
            .get("headers")
            .and_then(|h| serde_json::from_value(h.clone()).ok())
            .unwrap_or_default();

        let raw_body = match &body {
            serde_json::Value::String(s) => s.clone(),
            _ => body.to_string(),
        };
        let size = raw_body.len();

        Some(ResponseData {
            code: status,
            status: if (200..300).contains(&status) {
                "OK".to_string()
            } else {
                "Error".to_string()
            },
            headers,
            body,
            raw_body,
            size,
            time: 0, // TODO: 从 output 提取实际时间
        })
    }

    /// 从 `last_node_output` 提取请求数据
    fn extract_request_data(&self, output: &serde_json::Value) -> Option<RequestData> {
        let request = output.get("request")?;

        Some(RequestData {
            url: request.get("url")?.as_str()?.to_string(),
            method: request.get("method")?.as_str()?.to_string(),
            headers: request
                .get("headers")
                .and_then(|h| serde_json::from_value(h.clone()).ok())
                .unwrap_or_default(),
            body: request
                .get("body")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        })
    }

    /// 处理执行错误
    async fn handle_error(
        &self,
        error: ErrorDetail,
        config: &ScriptNodeConfig,
        context: &mut ExecutionContext,
        node: &Node,
        connections: &[Connection],
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        log::error!(
            "[流程 {}] 脚本节点 {} 执行错误: {}",
            context.flow_uuid,
            node.name,
            error.message
        );

        // 发送错误日志
        context
            .emit_log(LogEvent {
                level: LogLevel::Error,
                message: format!("脚本节点 {} 错误: {}", node.name, error.message),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        match config.on_error {
            ScriptErrorStrategy::Fatal => {
                // 致命错误，终止流程
                Err(error.message)
            }
            ScriptErrorStrategy::Recoverable => {
                // 可恢复错误，路由到 false 端口
                let next_nodes = self.get_next_nodes_by_port(node, "false", connections);
                let output_data = serde_json::json!({
                    "success": false,
                    "error": error.message,
                });
                Ok((next_nodes, Some(output_data)))
            }
        }
    }

    /// 根据端口 ID 获取下一批节点
    fn get_next_nodes_by_port(
        &self,
        node: &Node,
        port_id: &str,
        connections: &[Connection],
    ) -> Vec<String> {
        connections
            .iter()
            .filter(|c| c.source_node_uuid == node.uuid && c.source_port_id == port_id)
            .map(|c| c.target_node_uuid.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node::{NodePosition};
    use flow_engine_core::node_config::{AssertionFailStrategy, ScriptExecutionPhase};

    /// 创建测试用的脚本节点
    fn create_test_node(uuid: &str, script: &str, timeout_ms: u64) -> Node {
        Node {
            uuid: uuid.to_string(),
            node_type: "script".to_string(),
            name: format!("Test Script {}", uuid),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Script(ScriptNodeConfig {
                script: script.to_string(),
                timeout_ms,
                memory_limit_mb: 10,
                execution_phase: ScriptExecutionPhase::Standalone,
                on_error: ScriptErrorStrategy::Recoverable,
                on_assertion_fail: AssertionFailStrategy::RouteFalse,
            }),
            ports: None,
            has_breakpoint: None,
        }
    }

    /// 创建测试用的执行上下文
    fn create_test_context(flow_uuid: &str) -> ExecutionContext {
        ExecutionContext::new(flow_uuid.to_string(), 0)
    }

    #[tokio::test]
    async fn test_script_executor_basic_execution() {
        let executor = ScriptNodeExecutor::new();
        let node = create_test_node(
            "script-001",
            "fn.variables.set('test_var', 'hello');",
            5000,
        );
        let mut context = create_test_context("test-flow");
        let connections = vec![];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok(), "脚本执行应该成功");
        let (next_nodes, output_data) = result.unwrap();
        assert!(next_nodes.is_empty()); // 没有连接时返回空列表
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["success"], true);
    }

    #[tokio::test]
    async fn test_script_executor_variable_set_get() {
        let executor = ScriptNodeExecutor::new();
        let node = create_test_node(
            "script-002",
            r#"
                fn.variables.set('my_var', 'test_value');
                var val = fn.variables.get('my_var');
                if (val !== 'test_value') {
                    throw new Error('Variable not set correctly');
                }
            "#,
            5000,
        );
        let mut context = create_test_context("test-flow");
        let connections = vec![];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok(), "变量设置和获取应该成功");

        // 验证变量已同步到 ExecutionContext
        assert!(context.variables.get_variable("my_var").is_some());
        assert_eq!(
            context.variables.get_variable("my_var"),
            Some(&serde_json::json!("test_value"))
        );
    }

    #[tokio::test]
    async fn test_script_executor_variable_sync_to_context() {
        let executor = ScriptNodeExecutor::new();
        let node = create_test_node(
            "script-003",
            "fn.variables.set('synced_var', 'synced_value');",
            5000,
        );
        let mut context = create_test_context("test-flow");
        let connections = vec![];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());

        // 验证变量已同步回 ExecutionContext
        assert_eq!(
            context.variables.get_variable("synced_var"),
            Some(&serde_json::json!("synced_value"))
        );
    }

    #[tokio::test]
    async fn test_script_executor_existing_variables() {
        let executor = ScriptNodeExecutor::new();
        let node = create_test_node(
            "script-004",
            r#"
                var existing = fn.variables.get('existing_var');
                if (existing !== 'initial_value') {
                    throw new Error('Existing variable not found: ' + existing);
                }
                fn.variables.set('result', 'ok');
            "#,
            5000,
        );
        let mut context = create_test_context("test-flow");

        // 预设变量
        context.variables.set_variable(
            "existing_var".to_string(),
            serde_json::json!("initial_value"),
            "entry-node".to_string(),
        );

        let connections = vec![];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok(), "应该能访问预设变量");
        assert_eq!(
            context.variables.get_variable("result"),
            Some(&serde_json::json!("ok"))
        );
    }

    #[tokio::test]
    async fn test_script_executor_syntax_error() {
        let executor = ScriptNodeExecutor::new();
        let node = create_test_node("script-005", "invalid javascript syntax {{{", 5000);
        let mut context = create_test_context("test-flow");
        let connections = vec![];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        // 由于 on_error 是 Recoverable，应该返回 Ok 并走 false 端口
        assert!(result.is_ok());
        let (next_nodes, output_data) = result.unwrap();
        assert!(next_nodes.is_empty()); // 没有连接
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["success"], false);
        assert!(output["error"].is_string());
    }

    #[tokio::test]
    async fn test_script_executor_runtime_error() {
        let executor = ScriptNodeExecutor::new();
        let node = create_test_node("script-006", "throw new Error('Runtime error');", 5000);
        let mut context = create_test_context("test-flow");
        let connections = vec![];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        // 由于 on_error 是 Recoverable，应该返回 Ok 并走 false 端口
        assert!(result.is_ok());
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["success"], false);
    }

    #[tokio::test]
    async fn test_script_executor_fatal_error() {
        let executor = ScriptNodeExecutor::new();

        // 创建致命错误配置的节点
        let node = Node {
            uuid: "script-007".to_string(),
            node_type: "script".to_string(),
            name: "Fatal Script".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Script(ScriptNodeConfig {
                script: "throw new Error('Fatal error');".to_string(),
                timeout_ms: 5000,
                memory_limit_mb: 10,
                execution_phase: ScriptExecutionPhase::Standalone,
                on_error: ScriptErrorStrategy::Fatal, // 致命错误
                on_assertion_fail: AssertionFailStrategy::RouteFalse,
            }),
            ports: None,
            has_breakpoint: None,
        };

        let mut context = create_test_context("test-flow");
        let connections = vec![];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        // 致命错误应该返回 Err
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Fatal error"));
    }

    #[tokio::test]
    async fn test_script_executor_timeout() {
        let executor = ScriptNodeExecutor::new();

        // 创建一个会超时的脚本（短超时 + 长循环）
        let node = create_test_node(
            "script-008",
            "while(true) { /* infinite loop */ }",
            100, // 100ms 超时
        );
        let mut context = create_test_context("test-flow");
        let connections = vec![];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        // 超时应该走 false 端口（Recoverable 策略）
        assert!(result.is_ok());
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["success"], false);
        assert!(output["error"].as_str().unwrap().contains("超时"));
    }

    #[tokio::test]
    async fn test_script_executor_port_routing() {
        let executor = ScriptNodeExecutor::new();
        let node = create_test_node("script-009", "fn.variables.set('test', 'value');", 5000);
        let mut context = create_test_context("test-flow");

        // 创建连接：true 端口 -> next-node-1
        let connections = vec![Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "script-009".to_string(),
            target_node_uuid: "next-node-1".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "in".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["next-node-1"]);
    }

    #[tokio::test]
    async fn test_script_executor_false_port_routing() {
        let executor = ScriptNodeExecutor::new();
        let node = create_test_node("script-010", "throw new Error('Test error');", 5000);
        let mut context = create_test_context("test-flow");

        // 创建连接：false 端口 -> error-handler-node
        let connections = vec![Connection {
            id: "conn-2".to_string(),
            source_node_uuid: "script-010".to_string(),
            target_node_uuid: "error-handler-node".to_string(),
            source_port_id: "false".to_string(),
            target_port_id: "in".to_string(),
        }];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        // Recoverable 错误应该走 false 端口
        assert!(result.is_ok());
        let (next_nodes, _) = result.unwrap();
        assert_eq!(next_nodes, vec!["error-handler-node"]);
    }

    #[tokio::test]
    async fn test_script_executor_with_response_data() {
        let executor = ScriptNodeExecutor::new();
        let node = create_test_node(
            "script-011",
            r#"
                var code = fn.response.code;
                fn.variables.set('response_code', code.toString());
            "#,
            5000,
        );
        let mut context = create_test_context("test-flow");

        // 设置 last_node_output 模拟 HTTP 响应
        context.last_node_output = Some(serde_json::json!({
            "status": 200,
            "body": {"token": "abc123"},
            "headers": {"Content-Type": "application/json"}
        }));

        let connections = vec![];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        assert_eq!(
            context.variables.get_variable("response_code"),
            Some(&serde_json::json!("200"))
        );
    }

    #[tokio::test]
    async fn test_script_executor_variable_has() {
        let executor = ScriptNodeExecutor::new();
        let node = create_test_node(
            "script-012",
            r#"
                if (!fn.variables.has('existing_var')) {
                    throw new Error('has() should return true');
                }
                if (fn.variables.has('nonexistent_var')) {
                    throw new Error('has() should return false');
                }
                fn.variables.set('result', 'ok');
            "#,
            5000,
        );
        let mut context = create_test_context("test-flow");

        // 预设变量
        context.variables.set_variable(
            "existing_var".to_string(),
            serde_json::json!("value"),
            "entry-node".to_string(),
        );

        let connections = vec![];

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        assert!(result.is_ok());
        assert_eq!(
            context.variables.get_variable("result"),
            Some(&serde_json::json!("ok"))
        );
    }

    #[tokio::test]
    async fn test_script_executor_cancel_infinite_loop() {
        let executor = ScriptNodeExecutor::new();
        // 创建一个会死循环的脚本，超时设得很长（5s），依赖取消信号来中断
        let node = create_test_node(
            "script-cancel-001",
            "while(true) { /* infinite loop */ }",
            5000, // 5s 超时，不应该靠超时来结束
        );
        let mut context = create_test_context("test-flow");

        // 设置取消信号
        let (tx, rx) = tokio::sync::watch::channel(false);
        context.cancel_receiver = Some(rx);
        let connections = vec![];

        // 在另一个任务中延迟发送取消信号
        let cancel_tx = tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let _ = cancel_tx.send(true);
        });

        let result = executor
            .execute(&node, &mut context, &connections, &HashMap::new())
            .await;

        // 取消应该走 false 端口（Recoverable 策略）
        assert!(result.is_ok());
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["success"], false);
    }
}
