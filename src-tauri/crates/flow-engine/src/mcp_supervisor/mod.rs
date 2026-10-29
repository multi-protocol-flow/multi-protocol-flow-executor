//! MCP Server Process Supervisor and Governance facade.
//!
//! Re-exports the MCP domain service adapter from the unified `crate::supervisor` subsystem.
#![allow(missing_docs)]

pub use crate::mcp_server::tools::sanitize_tool_name as sanitize_name;
pub use crate::supervisor::services::mcp::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_server::DEFAULT_MCP_PORT;

    #[test]
    fn test_check_plugin_status_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let status = check_plugin_status(temp_dir.path());
        assert!(!status.installed);
        assert!(status.binary_path.is_none());
        assert!(status.error.is_some());
    }

    #[test]
    fn test_sanitize_tool_name() {
        assert_eq!(sanitize_name("User Search Flow!"), "user_search_flow");
        assert_eq!(sanitize_name("___Hello_World___"), "hello_world");
        assert_eq!(sanitize_name(""), "mpe_flow");
    }

    #[test]
    fn test_inspect_flow_tools_and_variables() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let flow_content = r#"{
            "flow": {
                "uuid": "test-uuid",
                "name": "Customer Search",
                "description": "Find customer by query string",
                "initial_variables": {
                    "query": "Alice",
                    "limit": 10
                },
                "nodes": [
                    {
                        "uuid": "n1",
                        "type": "http:request",
                        "name": "HTTP Call",
                        "data": {
                            "type": "http:request",
                            "url": "https://api.example.com/search?q={{query}}&page={{page}}"
                        }
                    }
                ],
                "connections": []
            }
        }"#;
        std::fs::write(temp_file.path(), flow_content).unwrap();

        let insp = inspect_flow_tools(&temp_file.path().to_string_lossy()).unwrap();
        assert_eq!(insp.tool_name, "customer_search");
        assert_eq!(insp.description, "Find customer by query string");
        assert!(insp.variables.contains(&"query".to_string()));
        assert!(insp.variables.contains(&"limit".to_string()));
        assert!(insp.variables.contains(&"page".to_string()));

        let props = insp.input_schema["properties"].as_object().unwrap();
        assert_eq!(props["query"]["type"], "string");
        assert_eq!(props["limit"]["type"], "number");
        assert_eq!(props["page"]["type"], "string");
    }

    #[test]
    fn test_inspect_flow_tools_yaml() {
        let temp_file = tempfile::Builder::new().suffix(".mpf").tempfile().unwrap();
        let flow_yaml = r#"_version: 1
flow:
  uuid: test-uuid-yaml
  name: OpenAPI Models
  description: Find models by schema
  initial_variables:
    schema_name: User
  nodes:
    - uuid: n1
      type: "http:request"
      name: HTTP Call
      data:
        type: "http:request"
        url: "https://api.example.com/models?name={{schema_name}}&version={{version}}"
  connections: []
"#;
        std::fs::write(temp_file.path(), flow_yaml).unwrap();

        let insp = inspect_flow_tools(&temp_file.path().to_string_lossy()).unwrap();
        assert_eq!(insp.tool_name, "openapi_models");
        assert_eq!(insp.description, "Find models by schema");
        assert!(insp.variables.contains(&"schema_name".to_string()));
        assert!(insp.variables.contains(&"version".to_string()));
    }

    #[test]
    fn test_get_client_config_generation() {
        let stdio_server = McpServerInstance {
            id: "my-flow-1".to_string(),
            name: "my_flow".to_string(),
            pid: 100,
            transport: "stdio".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8000,
            flow_target: Some("flows/demo.mpf".to_string()),
            dir_target: None,
            tools_count: 1,
            started_at: 1000,
            managed_by: "cli".to_string(),
            status: "running".to_string(),
            tool_name: Some("my_flow".to_string()),
            slug: Some("my-flow".to_string()),
            shared: false,
        };

        let snippets = get_client_config(&stdio_server, Some("/opt/mpe/mpe-plugin-mcp"));
        let claude = &snippets.claude_desktop["mcpServers"]["my_flow"];
        assert_eq!(claude["command"], "/opt/mpe/mpe-plugin-mcp");
        let args = claude["args"].as_array().unwrap();
        assert_eq!(args[0], "--server");
        assert_eq!(args[1], "--flow");
        assert_eq!(args[2], "flows/demo.mpf");
        assert_eq!(args[3], "--transport");
        assert_eq!(args[4], "stdio");

        let streamable_server = McpServerInstance {
            id: "my-http-1".to_string(),
            name: "my_http".to_string(),
            pid: 101,
            transport: "streamable".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8900,
            flow_target: Some("flows/demo.mpf".to_string()),
            dir_target: None,
            tools_count: 1,
            started_at: 1000,
            managed_by: "cli".to_string(),
            status: "running".to_string(),
            tool_name: Some("my_http".to_string()),
            slug: Some("my-http".to_string()),
            shared: true,
        };
        let streamable_snippets = get_client_config(&streamable_server, None);
        let http_claude = &streamable_snippets.claude_desktop["mcpServers"]["my_http"];
        // Shared instances use the aggregate gateway endpoint: one client
        // registration discovers every published flow's tools.
        assert_eq!(http_claude["url"], "http://127.0.0.1:8900/mcp");
    }

    #[test]
    fn test_endpoint_dedicated_vs_shared() {
        let shared = McpServerInstance {
            id: "shared-1".to_string(),
            name: "shared".to_string(),
            pid: 1,
            transport: "streamable".to_string(),
            host: "127.0.0.1".to_string(),
            port: DEFAULT_MCP_PORT,
            flow_target: Some("flows/demo.mpf".to_string()),
            dir_target: None,
            tools_count: 1,
            started_at: 0,
            managed_by: "gui".to_string(),
            status: "running".to_string(),
            tool_name: Some("demo_flow".to_string()),
            slug: Some("demo-flow".to_string()),
            shared: true,
        };
        assert_eq!(
            shared.endpoint(),
            format!("http://127.0.0.1:{DEFAULT_MCP_PORT}/mcp")
        );
        assert_eq!(
            shared.scoped_endpoint().as_deref(),
            Some(format!("http://127.0.0.1:{DEFAULT_MCP_PORT}/demo-flow/mcp").as_str())
        );

        let dedicated = McpServerInstance { slug: None, shared: false, ..shared.clone() };
        assert_eq!(dedicated.endpoint(), "http://127.0.0.1:9528/mcp");
        assert_eq!(dedicated.scoped_endpoint(), None);
    }

    #[test]
    fn test_is_process_alive_zero() {
        assert!(!is_process_alive(0));
    }
}
