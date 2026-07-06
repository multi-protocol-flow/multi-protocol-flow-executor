//! mpe flow edit subcommand — node/connection/variable CRUD operations

use super::config::AppConfig;
use super::output::CliOutput;
use super::CliError;
use super::FlowEditCommands;

use flow_engine::connection::Connection;
use flow_engine::node::{Node, NodePosition};
use flow_engine::node_config::NodeData;
use flow_engine::node_types;
use std::collections::HashMap;

/// Execute flow edit subcommand
pub async fn execute(_format: &str, command: FlowEditCommands) -> Result<(), CliError> {
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
        FlowEditCommands::NodeUpdate { flow, node, config } => {
            execute_node_update(flows_dir, &flow, &node, config.as_deref()).await
        }
        FlowEditCommands::ConnectionAdd {
            flow,
            source,
            target,
            source_port,
            target_port,
        } => {
            execute_connection_add(
                flows_dir,
                &flow,
                &source,
                &target,
                &source_port,
                &target_port,
            )
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

/// Parse string as `NodeType`
fn parse_node_type(type_str: &str) -> Result<String, CliError> {
    // Validate node type string (via serde deserialization into NodeType)
    let _node_type: flow_engine::node::NodeType =
        serde_json::from_value(serde_json::Value::String(type_str.to_string())).map_err(|e| {
            CliError::InvalidArgs(format!("invalid node type '{}': {}", type_str, e))
        })?;
    Ok(type_str.to_string())
}

/// Create `NodeData` from node type and optional config JSON
fn create_node_data(node_type: &str, config_json: Option<&str>) -> Result<NodeData, CliError> {
    // If config JSON provided, parse as NodeData (with type tag)
    if let Some(json_str) = config_json {
        // Try parsing with type tag
        let data: NodeData = serde_json::from_str(json_str)
            .map_err(|e| CliError::InvalidArgs(format!("failed to parse node config: {}", e)))?;
        return Ok(data);
    }

    // Use default when no config
    Ok(default_node_data(node_type))
}

/// Return default `NodeData` for given node type string
fn default_node_data(node_type: &str) -> NodeData {
    match node_type {
        node_types::ENTRY => NodeData::entry(),
        node_types::HTTP => NodeData::http(""),
        node_types::CONDITIONAL => NodeData::conditional_simple(
            "",
            flow_engine::node_config::ConditionOperator::Equal,
            serde_json::Value::Null,
        ),
        node_types::WS_CONNECT => NodeData::ws_connect(""),
        node_types::WS_SEND_COLLECT => NodeData::ws_send_collect("", ""),
        node_types::WS_CLOSE => NodeData::ws_close(""),
        node_types::ASSERTION => NodeData::assertion(),
        node_types::DB_CONNECT => NodeData::db_connect(""),
        node_types::DB_QUERY => NodeData::db_query(""),
        node_types::DB_BEGIN => NodeData::db_begin(""),
        node_types::DB_COMMIT => NodeData::db_commit(""),
        node_types::DB_ROLLBACK => NodeData::db_rollback(""),
        node_types::DB_CLOSE => NodeData::db_close(""),
        node_types::SCRIPT => NodeData::script(""),
        _ => NodeData::entry(),
    }
}

/// Calculate auto position for new node (max x + 250)
fn auto_position(nodes: &[Node]) -> NodePosition {
    let max_x = nodes.iter().map(|n| n.position.x).fold(0.0_f64, f64::max);
    NodePosition {
        x: max_x + 250.0,
        y: 0.0,
    }
}

// ── Node operations ─────────────────────────────────────────────────────

/// Add node
async fn execute_node_add(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    type_str: &str,
    name: &str,
    config_json: Option<&str>,
) -> Result<(), CliError> {
    let mut flow =
        flow_engine::storage::load_flow_file(flow_uuid, flows_dir).map_err(CliError::Execution)?;

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
        on_error: None,
    };

    flow.nodes.push(node);

    flow_engine::storage::save_flow_file(&flow, flows_dir).map_err(CliError::Execution)?;

    let flow_file = flow_engine::storage::FlowFile {
        _version: 1,
        flow,
        _meta: None,
    };
    let output = CliOutput::success(&flow_file);
    super::output::output_json(&output);
    Ok(())
}

/// Remove node (also removes associated connections)
async fn execute_node_remove(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    node_uuid: &str,
) -> Result<(), CliError> {
    let mut flow =
        flow_engine::storage::load_flow_file(flow_uuid, flows_dir).map_err(CliError::Execution)?;

    // Check if node exists
    let node_idx = flow
        .nodes
        .iter()
        .position(|n| n.uuid == node_uuid)
        .ok_or_else(|| CliError::InvalidArgs(format!("node not found: {}", node_uuid)))?;

    // Remove node
    flow.nodes.remove(node_idx);

    // Remove all connections referencing this node
    flow.connections
        .retain(|c| c.source_node_uuid != node_uuid && c.target_node_uuid != node_uuid);

    flow_engine::storage::save_flow_file(&flow, flows_dir).map_err(CliError::Execution)?;

    let flow_file = flow_engine::storage::FlowFile {
        _version: 1,
        flow,
        _meta: None,
    };
    let output = CliOutput::success(&flow_file);
    super::output::output_json(&output);
    Ok(())
}

/// Update node config
async fn execute_node_update(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    node_uuid: &str,
    config_json: Option<&str>,
) -> Result<(), CliError> {
    let mut flow =
        flow_engine::storage::load_flow_file(flow_uuid, flows_dir).map_err(CliError::Execution)?;

    let node = flow
        .nodes
        .iter_mut()
        .find(|n| n.uuid == node_uuid)
        .ok_or_else(|| CliError::InvalidArgs(format!("node not found: {}", node_uuid)))?;

    if let Some(json_str) = config_json {
        // Parse as NodeData (with type tag)
        let new_data: NodeData = serde_json::from_str(json_str)
            .map_err(|e| CliError::InvalidArgs(format!("failed to parse node config: {}", e)))?;
        node.data = new_data;
    }
    // No modification when config is None

    flow_engine::storage::save_flow_file(&flow, flows_dir).map_err(CliError::Execution)?;

    let flow_file = flow_engine::storage::FlowFile {
        _version: 1,
        flow,
        _meta: None,
    };
    let output = CliOutput::success(&flow_file);
    super::output::output_json(&output);
    Ok(())
}

// ── Connection operations ──────────────────────────────────────────────

/// Add connection
async fn execute_connection_add(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    source_uuid: &str,
    target_uuid: &str,
    source_port: &str,
    target_port: &str,
) -> Result<(), CliError> {
    let mut flow =
        flow_engine::storage::load_flow_file(flow_uuid, flows_dir).map_err(CliError::Execution)?;

    // Validate source node exists
    if !flow.nodes.iter().any(|n| n.uuid == source_uuid) {
        return Err(CliError::InvalidArgs(format!(
            "source node not found: {}",
            source_uuid
        )));
    }

    // Validate target node exists
    if !flow.nodes.iter().any(|n| n.uuid == target_uuid) {
        return Err(CliError::InvalidArgs(format!(
            "target node not found: {}",
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

    flow_engine::storage::save_flow_file(&flow, flows_dir).map_err(CliError::Execution)?;

    let flow_file = flow_engine::storage::FlowFile {
        _version: 1,
        flow,
        _meta: None,
    };
    let output = CliOutput::success(&flow_file);
    super::output::output_json(&output);
    Ok(())
}

/// Remove connection
async fn execute_connection_remove(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    connection_id: &str,
) -> Result<(), CliError> {
    let mut flow =
        flow_engine::storage::load_flow_file(flow_uuid, flows_dir).map_err(CliError::Execution)?;

    let before = flow.connections.len();
    flow.connections.retain(|c| c.id != connection_id);

    if flow.connections.len() == before {
        return Err(CliError::InvalidArgs(format!(
            "connection not found: {}",
            connection_id
        )));
    }

    flow_engine::storage::save_flow_file(&flow, flows_dir).map_err(CliError::Execution)?;

    let flow_file = flow_engine::storage::FlowFile {
        _version: 1,
        flow,
        _meta: None,
    };
    let output = CliOutput::success(&flow_file);
    super::output::output_json(&output);
    Ok(())
}

// ── Variable operations ───────────────────────────────────────────────

/// Set variable
async fn execute_variable_set(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    key: &str,
    value: &str,
) -> Result<(), CliError> {
    let mut flow =
        flow_engine::storage::load_flow_file(flow_uuid, flows_dir).map_err(CliError::Execution)?;

    // Try parsing as JSON value, fallback to string
    let json_value: serde_json::Value =
        serde_json::from_str(value).unwrap_or(serde_json::Value::String(value.to_string()));

    // Ensure initial_variables exists
    if flow.initial_variables.is_none() {
        flow.initial_variables = Some(HashMap::new());
    }

    if let Some(ref mut vars) = flow.initial_variables {
        vars.insert(key.to_string(), json_value);
    }

    flow_engine::storage::save_flow_file(&flow, flows_dir).map_err(CliError::Execution)?;

    let flow_file = flow_engine::storage::FlowFile {
        _version: 1,
        flow,
        _meta: None,
    };
    let output = CliOutput::success(&flow_file);
    super::output::output_json(&output);
    Ok(())
}

/// Remove variable
async fn execute_variable_remove(
    flows_dir: &std::path::Path,
    flow_uuid: &str,
    key: &str,
) -> Result<(), CliError> {
    let mut flow =
        flow_engine::storage::load_flow_file(flow_uuid, flows_dir).map_err(CliError::Execution)?;

    match flow.initial_variables {
        Some(ref mut vars) => {
            if vars.remove(key).is_none() {
                return Err(CliError::InvalidArgs(format!(
                    "variable not found: {}",
                    key
                )));
            }
            // Set to None after clearing all variables
            if vars.is_empty() {
                flow.initial_variables = None;
            }
        }
        None => {
            return Err(CliError::InvalidArgs(format!(
                "variable not found: {}",
                key
            )));
        }
    }

    flow_engine::storage::save_flow_file(&flow, flows_dir).map_err(CliError::Execution)?;

    let flow_file = flow_engine::storage::FlowFile {
        _version: 1,
        flow,
        _meta: None,
    };
    let output = CliOutput::success(&flow_file);
    super::output::output_json(&output);
    Ok(())
}
