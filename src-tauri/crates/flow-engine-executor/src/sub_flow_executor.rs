// SubFlow 节点执行器实现

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::{ErrorDetail, ErrorKind};
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::flow::Flow;
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{NodeData, SubFlowConfig};
use flow_engine_core::registry::NodeRegistry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 子流程节点执行器
pub struct SubFlowNodeExecutor;

impl Default for SubFlowNodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl SubFlowNodeExecutor {
    /// 创建新的 `SubFlowNodeExecutor`
    pub fn new() -> Self {
        SubFlowNodeExecutor
    }

    /// 子流程搜索目录（按优先级）
    ///
    /// 1. 当前流程文件所在目录 —— 相对路径引用的自然基准（CLI `run --flow <path>` 的
    ///    同级/相对引用，与面板提示一致）
    /// 2. 当前生效的流程目录 —— 宿主登记的 `flowsDir` 或默认数据目录
    ///
    /// GUI 的流程目录可被设置为任意工作区目录，按 ID 解析必须使用该目录，
    /// 否则会出现「列表可见、运行时找不到」。
    fn search_dirs(context: &ExecutionContext) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = Vec::new();
        if let Some(parent) = context.flow_file_path.as_ref().and_then(|p| p.parent()) {
            dirs.push(parent.to_path_buf());
        }
        if let Ok(dir) = crate::storage::get_flows_dir() {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
        dirs
    }

    #[allow(clippy::result_large_err)]
    /// 按显式路径解析：绝对路径直接使用，相对路径依次在搜索目录下尝试
    pub fn resolve_by_path(
        path_str: &str,
        dirs: &[PathBuf],
    ) -> Result<(Flow, Option<PathBuf>), ErrorDetail> {
        let path = Path::new(path_str);
        let mut bases: Vec<PathBuf> = Vec::new();
        if path.is_absolute() {
            bases.push(PathBuf::from(path_str));
        } else {
            for dir in dirs {
                bases.push(dir.join(path));
            }
            bases.push(PathBuf::from(path_str));
        }

        let mut tried: Vec<String> = Vec::new();
        for base in &bases {
            // 允许省略扩展名：按原样、.mpf、.json 依次尝试
            for candidate in [
                base.clone(),
                base.with_extension("mpf"),
                base.with_extension("json"),
            ] {
                if !candidate.is_file() {
                    continue;
                }
                tried.push(candidate.display().to_string());
                match crate::storage::load_flow(&candidate.to_string_lossy()) {
                    Ok(flow) => return Ok((flow, Some(candidate))),
                    Err(e) => {
                        return Err(ErrorDetail::new(
                            ErrorKind::Configuration,
                            format!("Failed to load subflow from {}: {}", candidate.display(), e),
                        ));
                    }
                }
            }
        }

        Err(ErrorDetail::new(
            ErrorKind::Configuration,
            format!(
                "Target subflow not found at path '{}' (searched: {})",
                path_str,
                if tried.is_empty() {
                    bases
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    tried.join(", ")
                }
            ),
        ))
    }

    #[allow(clippy::result_large_err)]
    /// 按流程 UUID 解析：在搜索目录下递归扫描 `.mpf`，命中后取绝对路径加载
    ///
    /// 同时返回文件路径，供子流程内部的相对引用继续解析。
    pub fn resolve_by_id(
        flow_id: &str,
        dirs: &[PathBuf],
    ) -> Result<(Flow, Option<PathBuf>), ErrorDetail> {
        let mut searched: Vec<String> = Vec::new();
        for dir in dirs {
            searched.push(dir.display().to_string());
            if let Ok((flow_file, path)) = crate::storage::load_flow_file_with_path(flow_id, dir) {
                return Ok((flow_file.flow, Some(path)));
            }
        }

        Err(ErrorDetail::new(
            ErrorKind::Configuration,
            format!(
                "Target subflow with ID '{}' not found (searched: {})",
                flow_id,
                if searched.is_empty() {
                    "no candidate directory".to_string()
                } else {
                    searched.join(", ")
                }
            ),
        ))
    }

    #[allow(clippy::result_large_err)]
    /// 解析并加载目标子流程及文件路径（指定候选搜索目录）
    pub fn resolve_flow_with_dirs(
        config: &SubFlowConfig,
        dirs: &[PathBuf],
    ) -> Result<(Flow, Option<PathBuf>), ErrorDetail> {
        if let Some(path_str) = config
            .target_flow_path
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            return Self::resolve_by_path(path_str.trim(), dirs);
        }

        if let Some(flow_id) = config
            .target_flow_id
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            return Self::resolve_by_id(flow_id.trim(), dirs);
        }

        Err(ErrorDetail::new(
            ErrorKind::Configuration,
            "Target subflow not specified (neither target_flow_id nor target_flow_path is set)"
                .to_string(),
        ))
    }

    #[allow(clippy::result_large_err)]
    /// 解析并加载目标子流程及文件路径
    ///
    /// 文件路径优先（与后端解析顺序一致），其次按流程 UUID 解析。
    pub fn resolve_flow(
        config: &SubFlowConfig,
        context: &ExecutionContext,
    ) -> Result<(Flow, Option<PathBuf>), ErrorDetail> {
        let dirs = Self::search_dirs(context);
        Self::resolve_flow_with_dirs(config, &dirs)
    }
}

#[async_trait::async_trait]
impl NodeExecutor for SubFlowNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let start_time = tokio::time::Instant::now();

        context
            .emit_log(LogEvent {
                level: LogLevel::Info,
                message: format!("SubFlow node '{}' started", node.name),
                timestamp: flow_engine_core::utils::unix_timestamp_ms(),
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            })
            .await;

        let config = match &node.data {
            NodeData::SubFlow(cfg) => cfg.clone(),
            NodeData::Plugin(val) => serde_json::from_value(val.clone()).map_err(|e| {
                ErrorDetail::new(
                    ErrorKind::Configuration,
                    format!("Invalid sub_flow configuration: {}", e),
                )
            })?,
            _ => {
                return Err(ErrorDetail::new(
                    ErrorKind::Configuration,
                    "Node data is not SubFlowConfig".to_string(),
                ));
            }
        };

        // 1. 解析目标子流程
        let (child_flow, child_flow_path) = Self::resolve_flow(&config, context)?;

        // 2. 循环与递归深度检测
        if !child_flow.uuid.is_empty() && context.call_stack.contains(&child_flow.uuid) {
            return Err(ErrorDetail::new(
                ErrorKind::Validation,
                format!("Cycle detected in subflow call chain: {}", child_flow.uuid),
            ));
        }

        if context.call_stack.len() >= 16 {
            return Err(ErrorDetail::new(
                ErrorKind::Validation,
                "Subflow call stack exceeded maximum depth (16)".to_string(),
            ));
        }

        // 3. 准备子流程输入变量
        let mut child_vars: HashMap<String, serde_json::Value> = if config.inherit_all_variables {
            context.variables.get_merged_variables()
        } else {
            HashMap::new()
        };

        for mapping in &config.variable_mappings {
            if mapping.source.is_empty() || mapping.target.is_empty() {
                continue;
            }

            let val = if let Some(var_val) = context.variables.get_variable(&mapping.source) {
                var_val.clone()
            } else if mapping.source.contains("{{") {
                match flow_engine_core::executor::replace_variables(&mapping.source, context) {
                    Ok(s) => serde_json::Value::String(s),
                    Err(_) => serde_json::Value::Null,
                }
            } else if mapping.source.starts_with('$') {
                flow_engine_core::utils::extract_value_by_path(
                    &context
                        .last_node_output
                        .clone()
                        .unwrap_or(serde_json::Value::Null),
                    &mapping.source,
                )
                .unwrap_or(serde_json::Value::Null)
            } else if let Some(v) = context
                .variables
                .get_merged_variables()
                .get(&mapping.source)
            {
                v.clone()
            } else {
                serde_json::Value::String(mapping.source.clone())
            };

            child_vars.insert(mapping.target.clone(), val);
        }

        // 4. 构建子流程 FlowExecutor
        let registry = context
            .get_provider::<NodeRegistry>("node_registry")
            .unwrap_or_else(|| Arc::new(crate::default_registry::default_registry()));

        let mut child_executor = crate::executor::FlowExecutor::new()
            .with_registry(registry)
            .with_call_stack(context.call_stack.clone())
            // 子流程内部节点的事件会带上本 SubFlow 节点 UUID：前端据此把子流程
            // 视为**单个节点**（顶层计数不含子节点），同时仍可在该节点上显示嵌套进度。
            .with_parent_node_uuid(node.uuid.clone());

        if let Some(cancel) = &context.cancel_receiver {
            child_executor = child_executor.with_cancel_receiver(cancel.clone());
        }
        if let Some(sender) = &context.execution_event_sender {
            child_executor = child_executor.with_execution_event_sender(sender.clone());
        }
        if let Some(flow_path) = &child_flow_path {
            child_executor = child_executor.with_flow_file_path(flow_path.clone());
        } else if let Some(parent_flow_path) = &context.flow_file_path {
            child_executor = child_executor.with_flow_file_path(parent_flow_path.clone());
        }

        // 5. 执行子流程
        let child_result = child_executor
            .execute_flow(
                &child_flow,
                Some(serde_json::Value::Object(child_vars.into_iter().collect())),
                None,
                context.log_sender.clone(),
            )
            .await;

        // 6. 暂存子流程报告树至父上下文
        context.current_sub_flow_report = child_result.flow_report.clone().map(Box::new);

        // 7. 处理执行结果与变量导出
        let duration_ms = start_time.elapsed().as_millis() as u64;

        if child_result.success {
            if let Some(ref report) = child_result.flow_report {
                if let Some(serde_json::Value::Object(final_vars)) = &report.final_variables {
                    if config.export_all_variables {
                        for (key, val) in final_vars {
                            context
                                .variables
                                .set_variable(key.clone(), val.clone(), String::new());
                        }
                    } else {
                        for mapping in &config.output_mappings {
                            let child_key = if !mapping.source.is_empty() {
                                &mapping.source
                            } else {
                                &mapping.target
                            };
                            let parent_key = if !mapping.target.is_empty() {
                                &mapping.target
                            } else {
                                &mapping.source
                            };
                            if let Some(val) = final_vars.get(child_key) {
                                context.variables.set_variable(
                                    parent_key.clone(),
                                    val.clone(),
                                    String::new(),
                                );
                            }
                        }
                    }
                }
            }

            let next_node_ids = connections
                .iter()
                .filter(|c| {
                    c.source_node_uuid == node.uuid
                        && (c.source_port_id == "out" || c.source_port_id.is_empty())
                })
                .map(|c| c.target_node_uuid.clone())
                .collect();

            let mut output_map = serde_json::Map::new();
            output_map.insert("success".to_string(), serde_json::Value::Bool(true));
            output_map.insert("duration_ms".to_string(), serde_json::json!(duration_ms));
            if let Some(ref rep) = child_result.flow_report {
                output_map.insert(
                    "sub_flow_uuid".to_string(),
                    serde_json::Value::String(rep.flow_uuid.clone()),
                );
                output_map.insert(
                    "sub_flow_name".to_string(),
                    serde_json::Value::String(rep.flow_name.clone()),
                );
                output_map.insert(
                    "executed_count".to_string(),
                    serde_json::json!(rep.executed_count),
                );
                if let Some(ref fv) = rep.final_variables {
                    output_map.insert("final_variables".to_string(), fv.clone());
                }
            }
            if let Some(ref data) = child_result.data {
                output_map.insert("data".to_string(), data.clone());
            }

            Ok((next_node_ids, Some(serde_json::Value::Object(output_map))))
        } else {
            let err_msg = child_result
                .node_reports
                .iter()
                .rev()
                .find_map(|r| r.error.as_ref().map(|e| e.message.clone()))
                .or_else(|| {
                    child_result
                        .flow_report
                        .as_ref()
                        .and_then(|r| r.flow_error.as_ref().map(|e| e.message.clone()))
                })
                .or(child_result.error)
                .unwrap_or_else(|| "Subflow execution failed".to_string());
            Err(ErrorDetail::new(ErrorKind::Execution, err_msg))
        }
    }
}
