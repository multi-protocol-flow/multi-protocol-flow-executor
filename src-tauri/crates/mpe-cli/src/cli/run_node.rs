//! mpe run node subcommand — execute a single protocol node
//!
//! Calls `FlowExecutor::execute_single_node()` with the given JSON config.
//! No flow traversal, no report persistence.

use std::sync::Arc;

use super::output::{self, CliOutput};
use super::CliError;
use flow_engine::executor::FlowExecutor;
use flow_engine::hook_runner::DefaultHookExecutor;
use flow_engine::node::Node;
use flow_engine::node::NodePosition;
use flow_engine::node_config::NodeData;
use serde_json::Value;
// ── Main entrypoint ──────────────────────────────────────────────────

/// Execute a single protocol node
pub async fn execute(
    _format: &str,
    node_json: &str,
    vars: &[(String, String)],
    registry: std::sync::Arc<flow_engine::registry::NodeRegistry>,
) -> Result<(), CliError> {
    // 1. Parse JSON as NodeData
    let node_data: NodeData = serde_json::from_str(node_json)
        .map_err(|e| CliError::InvalidArgs(format!("Invalid node JSON: {}", e)))?;

    // 2. Wrap into Node with auto-generated UUID
    let node = Node {
        uuid: uuid::Uuid::new_v4().to_string(),
        node_type: node_data.type_name(),
        name: node_data.type_name(), // default name = type
        position: NodePosition::default(),
        data: node_data,
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    // 3. Parse --var pairs into JSON object
    let variables = if vars.is_empty() {
        None
    } else {
        let mut map = serde_json::Map::new();
        for (key, val) in vars {
            let json_val = serde_json::from_str(val).unwrap_or(Value::String(val.clone()));
            map.insert(key.clone(), json_val);
        }
        Some(Value::Object(map))
    };

    // 4. Build executor with the shared registry (built-ins + plugins, so
    // `mpe run node` can drive plugin connect nodes like `redis:connect`).
    let executor = FlowExecutor::new()
        .with_registry(registry)
        .with_hook_executor(Arc::new(DefaultHookExecutor::new()));

    // 5. Execute single node
    let result = executor.execute_single_node(&node, variables, None).await;

    // 6. Output discipline
    let output_data = serde_json::json!({
        "success": result.success,
        "data": result.node_reports.first().map(|r| r.output_data.clone()),
        "error": result.error,
    });

    if result.success {
        let cli_output = CliOutput::success(output_data);
        output::output_json(&cli_output);
        Ok(())
    } else {
        Err(CliError::Execution {
            message: result
                .error
                .unwrap_or_else(|| "Execution failed".to_string()),
            data: Some(output_data),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use flow_engine::node_config::NodeData;

    #[test]
    fn test_parse_node_data_from_json() {
        // Valid JSON → NodeData::type_name() == "http"
        let json = r#"{"type":"http","method":"get","url":"https://example.com"}"#;
        let node_data: NodeData = serde_json::from_str(json).unwrap();
        assert_eq!(node_data.type_name(), "http");

        // Valid JSON → script type
        let script_json = r#"{"type":"script","script":"return 1+1;"}"#;
        let script_data: NodeData = serde_json::from_str(script_json).unwrap();
        assert_eq!(script_data.type_name(), "script");

        // Invalid JSON should fail
        let invalid = r#"not-json"#;
        assert!(serde_json::from_str::<NodeData>(invalid).is_err());
    }

    #[test]
    fn test_parse_variables() {
        // Key-value pairs → correct JSON map
        let vars = vec![
            ("host".to_string(), "localhost".to_string()),
            ("port".to_string(), "8080".to_string()),
        ];

        let result = if vars.is_empty() {
            None
        } else {
            let mut map = serde_json::Map::new();
            for (key, val) in &vars {
                let json_val =
                    serde_json::from_str(val).unwrap_or(serde_json::Value::String(val.clone()));
                map.insert(key.clone(), json_val);
            }
            Some(serde_json::Value::Object(map))
        };

        assert!(result.is_some());
        let obj = result.unwrap();
        assert_eq!(obj["host"], "localhost");
        // "8080" is valid JSON (number), so serde parses it as Number(8080)
        assert_eq!(obj["port"], serde_json::json!(8080));
    }
}
