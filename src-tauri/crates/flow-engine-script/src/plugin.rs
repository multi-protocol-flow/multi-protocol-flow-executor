//! 脚本节点插件
//!
//! 实现 `NodePlugin` trait，提供脚本节点的元数据、配置、端口路由和执行能力。

use flow_engine_core::connection::Connection;
use flow_engine_core::error::ErrorDetail;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef, ValidationError};
use serde_json::Value;
use std::collections::HashMap;

/// 脚本节点插件
pub struct ScriptPlugin;

#[async_trait::async_trait]
impl NodePlugin for ScriptPlugin {
    fn type_id(&self) -> &str {
        "script"
    }

    fn display_name(&self) -> &str {
        "Script"
    }

    fn category(&self) -> &str {
        "tools"
    }

    fn icon(&self) -> &str {
        "file-code"
    }

    fn color(&self) -> &str {
        "#8B5CF6"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "Success".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "Failed".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "script": "",
            "timeout_ms": 5000,
            "memory_limit_mb": 10,
            "execution_phase": "post_request",
            "on_error": "recoverable",
        })
    }

    /// 脚本检查 $.success → "true" / "false"
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        match output {
            Some(serde_json::Value::Object(map)) => {
                if let Some(serde_json::Value::Bool(true)) = map.get("success") {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            _ => "false".to_string(),
        }
    }

    fn validate(&self, config: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // 1. script must be non-empty
        match config.get("script").and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => {}
            _ => errors.push(ValidationError {
                field: "script".to_string(),
                message: "Script cannot be empty".to_string(),
            }),
        }

        // 2. timeout_ms must be between 1000 and 60000
        if let Some(Value::Number(n)) = config.get("timeout_ms") {
            if let Some(val) = n.as_u64() {
                if val < 1000 || val > 60000 {
                    errors.push(ValidationError {
                        field: "timeout_ms".to_string(),
                        message: format!(
                            "timeout_ms must be between 1000 and 60000, got {}",
                            val
                        ),
                    });
                }
            }
        }

        // 3. memory_limit_mb must be between 1 and 1024
        if let Some(Value::Number(n)) = config.get("memory_limit_mb") {
            if let Some(val) = n.as_u64() {
                if val < 1 || val > 1024 {
                    errors.push(ValidationError {
                        field: "memory_limit_mb".to_string(),
                        message: format!(
                            "memory_limit_mb must be between 1 and 1024, got {}",
                            val
                        ),
                    });
                }
            }
        }

        errors
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "script": config.get("script"),
            "timeout_ms": config.get("timeout_ms"),
            "memory_limit_mb": config.get("memory_limit_mb"),
            "execution_phase": config.get("execution_phase"),
            "on_error": config.get("on_error"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let executor = crate::script_executor::ScriptNodeExecutor::new();
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}
