// 执行引擎模块 - FlowExecutor 实现

use crate::connection::Connection;
use crate::context::{ExecutionContext, VariableStorage};
use crate::entry_executor::EntryNodeExecutor;
use crate::flow::Flow;
use crate::error::{ErrorDetail, ErrorKind};
use crate::node::{Node, NodeType};
use regex::Regex;
use serde_json::json;
use std::collections::HashMap;

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

/// 替换模板中的变量
///
/// # 参数
/// - `template`: 包含变量占位符的模板字符串，使用 `{{variable_path}}` 语法
/// - `context`: 执行上下文，包含变量数据
///
/// # 返回
/// - `Result<String, String>`: 替换后的字符串，或错误信息
///
/// # 功能
/// - 支持简单的变量替换，如 `{{initial_variables.api_key}}`
/// - 支持嵌套路径访问，如 `{{user.profile.name}}`
/// - 严格模式：变量不存在时返回错误
/// - 不支持索引访问（如 `items[0]`）和函数调用
pub fn replace_variables(template: &str, context: &ExecutionContext) -> Result<String, String> {
    let re = Regex::new(r"\{\{([^}]+)\}\}").map_err(|e| format!("正则表达式错误: {}", e))?;
    let mut result = template.to_string();

    for cap in re.captures_iter(template) {
        let full_match = cap.get(0).unwrap().as_str();
        let variable_path = cap.get(1).unwrap().as_str().trim();

        // 检查是否包含索引访问（不支持）
        if variable_path.contains('[') {
            return Err(format!("不支持索引访问: {}", variable_path));
        }

        // 检查是否包含函数调用（不支持）
        if variable_path.contains('(') {
            return Err(format!("不支持函数调用: {}", variable_path));
        }

        // 按点分割路径
        let path_parts: Vec<&str> = variable_path.split('.').collect();

        // 从 context.variables.global 中获取值
        let mut current_value: Option<&serde_json::Value> =
            context.variables.global.get(path_parts[0]);
        let mut current_path = String::new();

        if current_value.is_none() {
            return Err(format!("未找到变量: {}", variable_path));
        }

        for part in &path_parts[1..] {
            if !current_path.is_empty() {
                current_path.push('.');
            }
            current_path.push_str(part);

            match current_value {
                Some(serde_json::Value::Object(map)) => {
                    if let Some(value) = map.get(*part) {
                        current_value = Some(value);
                    } else {
                        return Err(format!("未找到变量: {}", current_path));
                    }
                }
                _ => {
                    return Err(format!("变量路径 '{}' 不是对象类型", current_path));
                }
            }
        }

        // 将值转换为字符串
        let replacement = match current_value {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::Bool(b)) => b.to_string(),
            Some(serde_json::Value::Null) => "null".to_string(),
            Some(serde_json::Value::Array(_) | serde_json::Value::Object(_)) => {
                return Err(format!(
                    "变量 '{}' 不支持直接替换（复杂类型）",
                    variable_path
                ));
            }
            None => {
                return Err(format!("变量 '{}' 为空", variable_path));
            }
        };

        result = result.replace(full_match, &replacement);
    }

    Ok(result)
}

/// 节点执行器 trait
///
/// 定义了所有节点执行器必须实现的接口
#[async_trait::async_trait]
pub trait NodeExecutor {
    /// 执行节点
    ///
    /// # 参数
    /// - `node`: 要执行的节点
    /// - `context`: 执行上下文
    /// - `connections`: 流程的所有连接
    /// - `node_map`: 节点 ID 到节点的映射
    ///
    /// # 返回
    /// - `Result<(Vec<String>, Option<serde_json::Value>), String>`:
    ///   - Ok((next_node_ids, output_data)): 执行成功，返回下一个节点 ID 列表和输出数据
    ///     - 空列表 `[]`: 流程终止
    ///     - 单元素列表 `[node_id]`: 继续到指定节点
    ///   - Err(error): 执行失败，返回错误信息
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String>;
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

/// 流程执行器
pub struct FlowExecutor;

impl Default for FlowExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl FlowExecutor {
    /// 创建新的流程执行器
    pub fn new() -> Self {
        FlowExecutor
    }

    /// 执行流程
    ///
    /// 从开始节点开始，按连接顺序遍历节点
    ///
    /// # 参数
    /// - `flow`: 要执行的流程
    /// - `variables`: 执行上下文变量（可选）
    ///
    /// # 返回
    /// - `ExecutionResult`: 执行结果
    pub async fn execute_flow(
        &self,
        flow: &Flow,
        variables: Option<serde_json::Value>,
    ) -> ExecutionResult {
        let start_time = tokio::time::Instant::now();
        let mut executed_count = 0u32;
        let mut node_reports = Vec::new();
        let initial_variables = variables.unwrap_or_else(|| json!({}));
        let mut var_storage = VariableStorage::new();
        if let Some(obj) = initial_variables.as_object() {
            for (key, value) in obj {
                var_storage.set_global(key.clone(), value.clone());
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

            // 执行当前节点
            match self
                .execute_node(node, &context, &flow.connections, &node_map)
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
                        NodeType::Http => Some("out".to_string()),
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
                    };

                    // 创建节点执行报告
                    let node_report = crate::report::NodeExecutionReport {
                        node_uuid: node.uuid.clone(),
                        node_type: format!("{:?}", node.node_type),
                        node_name: node.name.clone(),
                        start_time: node_start_ms,
                        end_time: node_end_ms,
                        duration_ms: node_duration,
                        status: crate::report::ExecutionStatus::Success,
                        input_data: Some(json!(context.variables.global)),
                        output_data: output_data.clone(),
                        used_port_id,
                        error: None,
                    };
                    node_reports.push(node_report);

                    // 如果有输出数据，更新上下文变量
                    if let Some(data) = output_data {
                        log::info!(
                            "[流程 {}] 节点 {} 输出数据: {}",
                            context.flow_uuid,
                            node.name,
                            data
                        );
                        // 将输出数据合并到上下文中
                        if let Some(obj) = data.as_object() {
                            for (key, value) in obj {
                                context.variables.set_global(key.to_string(), value.clone());
                            }
                        }
                    }

                    // 处理 Conditional 节点：如果返回空列表表示条件为 false，停止执行
                    if matches!(node.node_type, NodeType::Conditional)
                        && next_node_id_from_executor.is_empty()
                    {
                        log::info!(
                            "[流程 {}] 条件节点 {} 返回 false，停止流程",
                            context.flow_uuid,
                            node.name
                        );
                        // 条件为 false 是正常的流程终止，返回成功
                        break;
                    }

                    // 查找下一个节点
                    let next_node_id = if !next_node_id_from_executor.is_empty() {
                        // 如果执行器返回了下一个节点 ID，使用它
                        Some(next_node_id_from_executor[0].clone())
                    } else {
                        // 否则，使用连接映射查找下一个节点
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
                        let node_report = crate::report::NodeExecutionReport {
                            node_uuid: node.uuid.clone(),
                            node_type: format!("{:?}", node.node_type),
                            node_name: node.name.clone(),
                            start_time: node_start_ms,
                            end_time: node_end_ms,
                            duration_ms: node_duration,
                            status: crate::report::ExecutionStatus::Success,
                            input_data: Some(json!(context.variables.global)),
                            output_data: None,
                            used_port_id: Some("false".to_string()),
                            error: None,
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
                        let node_report = crate::report::NodeExecutionReport {
                            node_uuid: node.uuid.clone(),
                            node_type: format!("{:?}", node.node_type),
                            node_name: node.name.clone(),
                            start_time: node_start_ms,
                            end_time: node_end_ms,
                            duration_ms: node_duration,
                            status: crate::report::ExecutionStatus::Success,
                            input_data: Some(json!(context.variables.global)),
                            output_data: None,
                            used_port_id: None,
                            error: None,
                        };
                        node_reports.push(node_report);

                        // 返回成功，已执行的节点都成功完成
                        break;
                    } else {
                    // === 容错模式错误处理 ===
                    // 解析错误类型
                    let error_detail = parse_error_to_detail(&error);
                    
                    // 根据节点类型设置 used_port_id
                    let used_port_id = match node.node_type {
                        NodeType::Entry => Some("out".to_string()),
                        NodeType::Http => Some("out".to_string()),
                        _ => None,
                    };

                    // 创建节点执行报告（Failed状态）
                    let node_report = crate::report::NodeExecutionReport {
                        node_uuid: node.uuid.clone(),
                        node_type: format!("{:?}", node.node_type),
                        node_name: node.name.clone(),
                        start_time: node_start_ms,
                        end_time: node_end_ms,
                        duration_ms: node_duration,
                        status: crate::report::ExecutionStatus::Failed,
                        input_data: Some(json!(context.variables.global)),
                        output_data: None,
                        used_port_id,
                        error: Some(error_detail.clone()),
                    };
                    node_reports.push(node_report);
                    executed_count += 1;

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
                        
                        // 生成部分报告（包含已执行节点）
                        let end_time_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64;
                        let flow_status = crate::report::ExecutionStatus::Failed;
                        
                        let flow_report = crate::report::FlowExecutionReport {
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

        let flow_report = crate::report::FlowExecutionReport {
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
        context: &ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_config::NodeData;
    use crate::NodePosition;
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
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

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
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
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
        let result = executor.execute_flow(&flow, None).await;

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
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![http_node],
            connections: vec![],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

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
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
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
        let result = executor.execute_flow(&flow, Some(variables)).await;

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
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
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
        let result = executor.execute_flow(&flow, None).await;
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
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            initial_variables: None,
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
        };

        let executor = FlowExecutor::new();
        let result = executor.execute_flow(&flow, None).await;

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
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
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
        let result = executor.execute_flow(&flow, None).await;

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
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
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
        let result = executor.execute_flow(&flow, None).await;

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
        };

        let end_node = Node {
            uuid: "end-uuid-tracker".to_string(),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
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
        let result = executor.execute_flow(&flow, None).await;

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
        let result = executor.execute_flow(&flow, None).await;

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
        };

        let end_node = Node {
            uuid: "end-1".to_string(),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
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
        let result = executor.execute_flow(&flow, None).await;

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
        };

        let end_node_1 = Node {
            uuid: "end-1-multi".to_string(),
            node_type: NodeType::Entry,
            name: "结束 1".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let end_node_2 = Node {
            uuid: "end-2-multi".to_string(),
            node_type: NodeType::Entry,
            name: "结束 2".to_string(),
            position: NodePosition { x: 100.0, y: 100.0 },
            data: NodeData::entry(),
            ports: None,
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
        let result = executor.execute_flow(&flow, None).await;

        // 验证执行成功，连接映射正确使用了 UUID
        assert!(result.success);
        assert_eq!(result.executed_count, 2);
    }

    #[test]
    fn test_replace_variables_simple() {
        // 测试简单变量替换
        let template = "Bearer {{initial_variables.api_key}}";
        let mut variables = VariableStorage::new();
        variables.set_global(
            "initial_variables".to_string(),
            json!({
                "api_key": "test-key"
            }),
        );
        let context = ExecutionContext {
            flow_uuid: "test-flow".to_string(),
            current_node_uuid: None,
            variables,
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
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
        variables.set_global(
            "user".to_string(),
            json!({
                "profile": {
                    "name": "Alice"
                }
            }),
        );
        let context = ExecutionContext {
            flow_uuid: "test-flow".to_string(),
            current_node_uuid: None,
            variables,
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
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
                .unwrap()
                .as_millis() as u64,
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
        variables.set_global("items".to_string(), json!(["a", "b", "c"]));
        let context = ExecutionContext {
            flow_uuid: "test-flow".to_string(),
            current_node_uuid: None,
            variables,
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
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
            variables.set_global("items".to_string(), json!(["a", "b", "c"]));
            let context = ExecutionContext {
                flow_uuid: "test-flow".to_string(),
                current_node_uuid: None,
                variables,
                node_reports: vec![],
                flow_trigger_time: 0,
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
            variables.set_global("test_var".to_string(), json!(value.clone()));
            let context = ExecutionContext {
                flow_uuid: "test-flow".to_string(),
                current_node_uuid: None,
                variables,
                node_reports: vec![],
                flow_trigger_time: 0,
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
            variables.set_global("num".to_string(), json!(num));
            let context = ExecutionContext {
                flow_uuid: "test-flow".to_string(),
                current_node_uuid: None,
                variables,
                node_reports: vec![],
                flow_trigger_time: 0,
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
            variables.set_global("flag".to_string(), json!(val));
            let context = ExecutionContext {
                flow_uuid: "test-flow".to_string(),
                current_node_uuid: None,
                variables,
                node_reports: vec![],
                flow_trigger_time: 0,
            };
            let result = replace_variables(template, &context);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), val.to_string());
        }
    }
}
