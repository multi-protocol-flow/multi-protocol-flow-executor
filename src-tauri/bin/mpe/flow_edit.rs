//! mpe flow edit 子命令 — 节点/连接/变量的增删改操作

use super::config::AppConfig;
use super::output::CliOutput;
use super::CliError;
use super::FlowEditCommands;

use app_lib::node::{Node, NodePosition, NodeType};
use app_lib::node_config::NodeData;
use app_lib::connection::Connection;
use std::collections::HashMap;

/// 执行 flow edit 子命令
pub async fn execute(
    _format: &str,
    command: FlowEditCommands,
) -> Result<(), CliError> {
    let config = AppConfig::load().map_err(CliError::Config)?;
    let flows_dir = &config.flows_dir;

    match command {
        FlowEditCommands::NodeAdd {
            flow,
            r#type,
            name,
            config,
        } => execute_node_add(flows_dir, &flow, &r#type, &name, config.as_deref()).await,
        FlowEditCommands::NodeRemove { flow, node } => {
            execute_node_remove(flows_dir, &flow, &node).await
        }
        FlowEditCommands::NodeUpdate {
            flow,
            node,
            config,
        } => execute_node_update(flows_dir, &flow, &node, config.as_deref()).await,
        FlowEditCommands::ConnectionAdd {
            flow,
            source,
            target,
            source_port,
            target_port,
        } => {
            execute_connection_add(flows_dir, &flow, &source, &target, &source_port, &target_port)
                .await
        }
        FlowEditCommands::ConnectionRemove { flow, connection } => {
            execute_connection_remove(flows_dir, &flow, &connection).await
        }
        FlowEditCommands::VariableSet { flow, key, value } => {
            execute_variable_set(flows_dir, &flow, &key, &value).await
        }
        FlowEditCommands::VariableRemove { flow, key } => {
            execute_variable_remove(flows_dir, &flow, &key).await
        }
    }
}

/// 解析字符串为 `NodeType`
fn parse_node_type(type_str: &str) -> Result<NodeType, CliError> {
    serde_json::from_value(serde_json::Value::String(type_str.to_string()))
        .map_err(|e| CliError::InvalidArgs(format!("无效的节点类型 '{}': {}", type_str, e)))
}

/// 根据节点类型和可选配置 JSON 创建 `NodeData`
fn create_node_data(
    node_type: &NodeType,
    config_json: Option<&str>,
) -> Result<NodeData, CliError> {
    // 如果提供了配置 JSON，直接解析为 NodeData（带 type tag）
    if let Some(json_str) = config_json {
        // 尝试带 type tag 解析
        let data: NodeData = serde_json::from_str(json_str)
            .map_err(|e| CliError::InvalidArgs(format!("解析节点配置失败: {}", e)))?;
        return Ok(data);
    }

    // 无配置时使用默认值
    Ok(default_node_data(node_type))
}

/// 根据 `NodeType` 返回默认 `NodeData`
fn default_node_data(node_type: &NodeType) -> NodeData {
    match node_type {
        NodeType::Entry => NodeData::entry(),
        NodeType::Http => NodeData::http(""),
        NodeType::Conditional => NodeData::conditional_simple(
            "",
            app_lib::node_config::ConditionOperator::Equal,
            serde_json::Value::Null,
        ),
        NodeType::WsConnect => NodeData::ws_connect(""),
        NodeType::WsSendCollect => NodeData::ws_send_collect("", ""),
        NodeType::WsClose => NodeData::ws_close(""),
        NodeType::Assertion => NodeData::assertion(),
        NodeType::DbConnect => NodeData::db_connect(""),
        NodeType::DbQuery => NodeData::db_query(""),
        NodeType::DbBegin => NodeData::db_begin(""),
        NodeType::DbCommit => NodeData::db_commit(""),
        NodeType::DbRollback => NodeData::db_rollback(""),
        NodeType::DbClose => NodeData::db_close(""),
        NodeType::Script => NodeData::script(""),
    }
}

/// 计算新节点的自动位置（现有节点最大 x + 250）
fn auto_position(nodes: &[Node]) -> NodePosition {
    let max_x = nodes.iter().map(|n| n.position.x).fold(0.0_f64, f64::max);
    NodePosition {
        x: max_x + 250.0,
        y: 0.0,
    }
}

// ── 节点操作 ──────────────────────────────────────────────────────

/// 添加节点
async fn execute_node_add(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    type_str: &str,
    name: &str,
    config_json: Option<&str>,
) -> Result<(), CliError> {
    let mut flow = app_lib::storage::load_flow_file(flow_uuid, flows_dir)
        .map_err(CliError::Execution)?;

    let node_type = parse_node_type(type_str)?;
    let data = create_node_data(&node_type, config_json)?;

    let node = Node {
        uuid: uuid::Uuid::new_v4().to_string(),
        node_type,
        name: name.to_string(),
        position: auto_position(&flow.nodes),
        data,
        ports: None,
        has_breakpoint: None,
    };

    flow.nodes.push(node);

    app_lib::storage::save_flow_file(&flow, flows_dir)
        .map_err(CliError::Execution)?;

    let output = CliOutput::success(&flow);
    super::output::output_json(&output);
    Ok(())
}

/// 移除节点（同时移除关联连接）
async fn execute_node_remove(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    node_uuid: &str,
) -> Result<(), CliError> {
    let mut flow = app_lib::storage::load_flow_file(flow_uuid, flows_dir)
        .map_err(CliError::Execution)?;

    // 检查节点是否存在
    let node_idx = flow
        .nodes
        .iter()
        .position(|n| n.uuid == node_uuid)
        .ok_or_else(|| CliError::InvalidArgs(format!("节点不存在: {}", node_uuid)))?;

    // 移除节点
    flow.nodes.remove(node_idx);

    // 移除所有引用该节点的连接
    flow.connections
        .retain(|c| c.source_node_uuid != node_uuid && c.target_node_uuid != node_uuid);

    app_lib::storage::save_flow_file(&flow, flows_dir)
        .map_err(CliError::Execution)?;

    let output = CliOutput::success(&flow);
    super::output::output_json(&output);
    Ok(())
}

/// 更新节点配置
async fn execute_node_update(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    node_uuid: &str,
    config_json: Option<&str>,
) -> Result<(), CliError> {
    let mut flow = app_lib::storage::load_flow_file(flow_uuid, flows_dir)
        .map_err(CliError::Execution)?;

    let node = flow
        .nodes
        .iter_mut()
        .find(|n| n.uuid == node_uuid)
        .ok_or_else(|| CliError::InvalidArgs(format!("节点不存在: {}", node_uuid)))?;

    if let Some(json_str) = config_json {
        // 解析为 NodeData（带 type tag）
        let new_data: NodeData = serde_json::from_str(json_str)
            .map_err(|e| CliError::InvalidArgs(format!("解析节点配置失败: {}", e)))?;
        node.data = new_data;
    }
    // config 为 None 时不做任何修改

    app_lib::storage::save_flow_file(&flow, flows_dir)
        .map_err(CliError::Execution)?;

    let output = CliOutput::success(&flow);
    super::output::output_json(&output);
    Ok(())
}

// ── 连接操作 ──────────────────────────────────────────────────────

/// 添加连接
async fn execute_connection_add(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    source_uuid: &str,
    target_uuid: &str,
    source_port: &str,
    target_port: &str,
) -> Result<(), CliError> {
    let mut flow = app_lib::storage::load_flow_file(flow_uuid, flows_dir)
        .map_err(CliError::Execution)?;

    // 验证源节点存在
    if !flow.nodes.iter().any(|n| n.uuid == source_uuid) {
        return Err(CliError::InvalidArgs(format!(
            "源节点不存在: {}",
            source_uuid
        )));
    }

    // 验证目标节点存在
    if !flow.nodes.iter().any(|n| n.uuid == target_uuid) {
        return Err(CliError::InvalidArgs(format!(
            "目标节点不存在: {}",
            target_uuid
        )));
    }

    let connection = Connection {
        id: uuid::Uuid::new_v4().to_string(),
        source_node_uuid: source_uuid.to_string(),
        target_node_uuid: target_uuid.to_string(),
        source_port_id: source_port.to_string(),
        target_port_id: target_port.to_string(),
    };

    flow.connections.push(connection);

    app_lib::storage::save_flow_file(&flow, flows_dir)
        .map_err(CliError::Execution)?;

    let output = CliOutput::success(&flow);
    super::output::output_json(&output);
    Ok(())
}

/// 移除连接
async fn execute_connection_remove(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    connection_id: &str,
) -> Result<(), CliError> {
    let mut flow = app_lib::storage::load_flow_file(flow_uuid, flows_dir)
        .map_err(CliError::Execution)?;

    let before = flow.connections.len();
    flow.connections.retain(|c| c.id != connection_id);

    if flow.connections.len() == before {
        return Err(CliError::InvalidArgs(format!(
            "连接不存在: {}",
            connection_id
        )));
    }

    app_lib::storage::save_flow_file(&flow, flows_dir)
        .map_err(CliError::Execution)?;

    let output = CliOutput::success(&flow);
    super::output::output_json(&output);
    Ok(())
}

// ── 变量操作 ──────────────────────────────────────────────────────

/// 设置变量
async fn execute_variable_set(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    key: &str,
    value: &str,
) -> Result<(), CliError> {
    let mut flow = app_lib::storage::load_flow_file(flow_uuid, flows_dir)
        .map_err(CliError::Execution)?;

    // 尝试解析为 JSON 值，失败则作为字符串
    let json_value: serde_json::Value =
        serde_json::from_str(value).unwrap_or(serde_json::Value::String(value.to_string()));

    // 确保 initial_variables 存在
    if flow.initial_variables.is_none() {
        flow.initial_variables = Some(HashMap::new());
    }

    if let Some(ref mut vars) = flow.initial_variables {
        vars.insert(key.to_string(), json_value);
    }

    app_lib::storage::save_flow_file(&flow, flows_dir)
        .map_err(CliError::Execution)?;

    let output = CliOutput::success(&flow);
    super::output::output_json(&output);
    Ok(())
}

/// 移除变量
async fn execute_variable_remove(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    key: &str,
) -> Result<(), CliError> {
    let mut flow = app_lib::storage::load_flow_file(flow_uuid, flows_dir)
        .map_err(CliError::Execution)?;

    match flow.initial_variables {
        Some(ref mut vars) => {
            if vars.remove(key).is_none() {
                return Err(CliError::InvalidArgs(format!(
                    "变量不存在: {}",
                    key
                )));
            }
            // 变量清空后设为 None
            if vars.is_empty() {
                flow.initial_variables = None;
            }
        }
        None => {
            return Err(CliError::InvalidArgs(format!(
                "变量不存在: {}",
                key
            )));
        }
    }

    app_lib::storage::save_flow_file(&flow, flows_dir)
        .map_err(CliError::Execution)?;

    let output = CliOutput::success(&flow);
    super::output::output_json(&output);
    Ok(())
}
