// 节点插件 trait 定义 — 所有节点类型的统一接口
//
// NodePlugin trait 提供节点元数据、配置验证、端口路由和执行能力。
// 各协议 crate（http/ws/db/script/assertion）在 flow-engine-core 中实现此 trait，
// 由 NodeRegistry 统一注册管理。

use crate::error::ErrorDetail;

/// 端口定义（给前端用的端口描述）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortDef {
    /// 端口标识，如 "true"、"false"、"output"
    pub id: String,
    /// 端口显示名称，如 "Success"、"Failed"、"输出"
    pub label: String,
    /// 端口位置，如 "right"、"bottom"
    pub position: String,
}

/// 配置验证Error
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationError {
    /// 出错的字段名
    pub field: String,
    /// Error描述
    pub message: String,
}

/// 前端元数据（插件节点的自定义配置面板等，给前端 iframe 渲染用）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrontendMetadata {
    /// 前端类型，如 `"inline"`、`"iframe"`、`"config-panel-url"`
    #[serde(rename = "type")]
    pub kind: String,
    /// 内联 HTML 内容（`kind == "inline"` 时使用）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// iframe URL（宿主生成：macOS/Linux 为
    /// `mpe-plugin://<plugin-id>.localhost/<page>`，Windows/Android 为
    /// `http://<plugin-id>.localhost/<page>`，见 D5/F3）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// 节点元数据（给前端展示用）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeMetadata {
    /// 节点类型标识，如 `"http"`、`"db_connect"`
    pub type_id: String,
    /// 显示名称，如 "HTTP 请求"
    pub display_name: String,
    /// 分组分类，如 "http"、"database"、"logic"、"websocket"、"tools"
    pub category: String,
    /// 图标标识
    pub icon: String,
    /// 节点颜色（十六进制）
    pub color: String,
    /// 端口定义列表
    pub ports: Vec<PortDef>,
    /// 拖拽创建节点时的默认配置
    pub default_config: serde_json::Value,
    /// 配置 JSON Schema（插件节点动态表单；内置节点通常为 None）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<serde_json::Value>,
    /// 前端元数据（插件节点的 iframe 配置面板；内置节点为 None）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontend: Option<FrontendMetadata>,
    /// 是否支持单节点独立执行（`execute_single_node` 连接性验证）。
    /// 插件节点由 describe 的 `capabilities.single_node` 透传而来；
    /// 内置节点默认 false（由宿主白名单常量另行放行）。
    #[serde(default)]
    pub supports_single_node: bool,
}

/// 节点插件 trait — 所有节点类型的统一接口
///
/// 每种节点类型（HTTP、WebSocket、数据库等）在各自的 crate 中实现此 trait，
/// 通过 `NodeRegistry` 注册后可供执行器调用。
#[async_trait::async_trait]
pub trait NodePlugin: Send + Sync {
    /// 节点类型标识（与 `NodeType` 的 serde rename 一致）
    fn type_id(&self) -> &str;

    /// 显示名称
    fn display_name(&self) -> &str;

    /// 分组分类
    fn category(&self) -> &str;

    /// 图标标识
    fn icon(&self) -> &str;

    /// 节点颜色（十六进制）
    fn color(&self) -> &str;

    /// 端口定义
    fn ports(&self) -> Vec<PortDef>;

    /// 拖拽创建节点时的默认配置
    fn default_config(&self) -> serde_json::Value;

    /// 根据执行输出决定走哪个端口
    ///
    /// 默认实现：检查 output 中的 `success` 字段，
    /// 为 true 返回 "true"，否则返回 "false"。
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

    /// 是否是端口路由节点（默认 true）
    fn is_port_routing(&self) -> bool {
        true
    }

    /// 是否是协议节点（默认 false）
    fn is_protocol_node(&self) -> bool {
        false
    }

    /// 报告输入数据
    fn build_report_input(&self, _config: &serde_json::Value) -> Option<serde_json::Value> {
        None
    }

    /// 配置验证（默认无Error）
    fn validate(&self, _config: &serde_json::Value) -> Vec<ValidationError> {
        Vec::new()
    }

    /// 返回节点配置的 JSON Schema（用于前端动态表单渲染）
    /// 默认返回 None，表示该节点类型使用前端硬编码的配置面板
    fn config_schema(&self) -> Option<serde_json::Value> {
        None
    }

    /// 节点的前端元数据（插件 iframe 配置面板等）
    ///
    /// 默认返回 None，内置节点无需任何改动；插件节点（
    /// `SidecarPlugin` / `PendingPlugin`）返回其 describe 产出的前端描述。
    fn frontend(&self) -> Option<&FrontendMetadata> {
        None
    }

    /// 是否支持单节点独立执行（`execute_single_node` 连接性验证）。
    ///
    /// 默认 false。内置节点由执行器的 `SUPPORTED_SINGLE_NODE_TYPES`
    /// 常量另行放行（不逐个覆写）；插件节点（`SidecarPlugin`）透传
    /// describe 的 `capabilities.single_node`，从而把单节点路径从
    /// 宿主硬编码白名单收敛为 capability 驱动。
    fn supports_single_node(&self) -> bool {
        false
    }

    /// 执行节点
    ///
    /// # 参数
    /// - `node`: 要执行的节点
    /// - `context`: 执行上下文（可变借用，允许修改变量等）
    /// - `connections`: 流程的所有连接
    /// - `node_map`: 节点 UUID 到节点的映射
    ///
    /// # 返回
    /// - `Ok((next_node_ids, output_data))`: 执行Success
    /// - `Err(error_detail)`: 执行Failed，包含结构化Error信息
    async fn execute(
        &self,
        node: &crate::node::Node,
        context: &mut crate::context::ExecutionContext,
        connections: &[crate::connection::Connection],
        node_map: &std::collections::HashMap<String, &crate::node::Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试默认 `resolve_port：output` 含 success=true 时返回 "true"
    #[test]
    fn test_resolve_port_success_true() {
        // 使用一个最小化的 mock 来测试 trait 的默认方法
        struct MockPlugin;
        impl MockPlugin {
            fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
                // 复制 trait 默认实现的逻辑
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
        }

        let plugin = MockPlugin;
        let output = Some(serde_json::json!({"success": true}));
        assert_eq!(plugin.resolve_port(&output), "true");
    }

    /// 测试默认 `resolve_port：output` 为 None 时返回 "false"
    #[test]
    fn test_resolve_port_none() {
        struct MockPlugin;
        impl MockPlugin {
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
        }

        let plugin = MockPlugin;
        assert_eq!(plugin.resolve_port(&None), "false");
    }

    /// 测试默认 `resolve_port：output` 含 success=false 时返回 "false"
    #[test]
    fn test_resolve_port_success_false() {
        struct MockPlugin;
        impl MockPlugin {
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
        }

        let plugin = MockPlugin;
        let output = Some(serde_json::json!({"success": false}));
        assert_eq!(plugin.resolve_port(&output), "false");
    }

    /// 测试 `PortDef` 序列化/反序列化
    #[test]
    fn test_port_def_serialization() {
        let port = PortDef {
            id: "true".to_string(),
            label: "Success".to_string(),
            position: "right".to_string(),
        };

        let json = serde_json::to_string(&port).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"true\""));
        assert!(json.contains("\"label\""));
        assert!(json.contains("\"Success\""));

        let deserialized: PortDef = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(deserialized.id, "true");
        assert_eq!(deserialized.label, "Success");
        assert_eq!(deserialized.position, "right");
    }

    /// 测试 `ValidationError` 序列化
    #[test]
    fn test_validation_error_serialization() {
        let error = ValidationError {
            field: "url".to_string(),
            message: "URL is required".to_string(),
        };

        let json = serde_json::to_string(&error).unwrap_or_else(|e| panic!("{e}"));
        let deserialized: ValidationError =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(deserialized.field, "url");
        assert_eq!(deserialized.message, "URL is required");
    }

    /// 测试 `NodeMetadata` 序列化
    #[test]
    fn test_node_metadata_serialization() {
        let metadata = NodeMetadata {
            type_id: "http".to_string(),
            display_name: "HTTP 请求".to_string(),
            category: "http".to_string(),
            icon: "globe".to_string(),
            color: "#3B82F6".to_string(),
            ports: vec![
                PortDef {
                    id: "true".to_string(),
                    label: "Success".to_string(),
                    position: "right".to_string(),
                },
                PortDef {
                    id: "false".to_string(),
                    label: "Failed".to_string(),
                    position: "bottom".to_string(),
                },
            ],
            default_config: serde_json::json!({"method": "get", "url": ""}),
            config_schema: None,
            frontend: Some(FrontendMetadata {
                kind: "inline".to_string(),
                content: Some("<p>hi</p>".to_string()),
                url: None,
            }),
            supports_single_node: false,
        };

        let json = serde_json::to_string(&metadata).unwrap_or_else(|e| panic!("{e}"));
        let deserialized: NodeMetadata =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(deserialized.type_id, "http");
        assert_eq!(deserialized.display_name, "HTTP 请求");
        assert_eq!(deserialized.ports.len(), 2);
        assert_eq!(deserialized.ports[0].id, "true");
        assert_eq!(deserialized.ports[1].id, "false");
        // `frontend.kind` 在 wire 上序列化为 `"type"`（与 mpe-protocol 对齐）
        assert!(json.contains("\"frontend\""));
        let frontend = deserialized
            .frontend
            .unwrap_or_else(|| panic!("expected frontend"));
        assert_eq!(frontend.kind, "inline");
        assert_eq!(frontend.content.as_deref(), Some("<p>hi</p>"));
        assert_eq!(frontend.url, None);
    }

    /// 测试 `NodePlugin::frontend()` 默认实现返回 None（内置节点零改动）
    #[test]
    fn test_frontend_default_is_none() {
        struct MockPlugin;

        #[async_trait::async_trait]
        impl NodePlugin for MockPlugin {
            fn type_id(&self) -> &str {
                "mock"
            }
            fn display_name(&self) -> &str {
                "Mock"
            }
            fn category(&self) -> &str {
                "logic"
            }
            fn icon(&self) -> &str {
                "icon"
            }
            fn color(&self) -> &str {
                "#000000"
            }
            fn ports(&self) -> Vec<PortDef> {
                Vec::new()
            }
            fn default_config(&self) -> serde_json::Value {
                serde_json::json!({})
            }
            async fn execute(
                &self,
                _node: &crate::node::Node,
                _context: &mut crate::context::ExecutionContext,
                _connections: &[crate::connection::Connection],
                _node_map: &std::collections::HashMap<String, &crate::node::Node>,
            ) -> Result<(Vec<String>, Option<serde_json::Value>), crate::error::ErrorDetail>
            {
                Ok((Vec::new(), None))
            }
        }

        let plugin = MockPlugin;
        assert!(plugin.frontend().is_none());
    }
}
