// 节点注册表 — 管理所有已注册的节点插件
//
// NodeRegistry 使用 HashMap 按 type_id 存储节点插件，
// 提供注册、查找、元数据查询等操作。

use crate::plugin::{NodeMetadata, NodePlugin};
use std::collections::HashMap;
use std::sync::Arc;

/// 节点插件注册表
///
/// 使用 `type_id` 作为键管理所有已注册的 `NodePlugin` 实现。
/// 支持注册、查找和元数据批量查询。
pub struct NodeRegistry {
    plugins: HashMap<String, Arc<dyn NodePlugin>>,
}

impl NodeRegistry {
    /// 创建空的注册表
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// 注册一个节点插件
    ///
    /// 如果 `type_id` 已存在，则覆盖旧的插件。
    pub fn register(&mut self, plugin: Arc<dyn NodePlugin>) {
        let type_id = plugin.type_id().to_string();
        self.plugins.insert(type_id, plugin);
    }

    /// 根据 `type_id` 查找插件
    pub fn get(&self, type_id: &str) -> Option<&Arc<dyn NodePlugin>> {
        self.plugins.get(type_id)
    }

    /// 返回全部已注册节点的元数据（给前端用）
    pub fn all_metadata(&self) -> Vec<NodeMetadata> {
        self.plugins
            .values()
            .map(|p| NodeMetadata {
                type_id: p.type_id().to_string(),
                display_name: p.display_name().to_string(),
                category: p.category().to_string(),
                icon: p.icon().to_string(),
                color: p.color().to_string(),
                ports: p.ports(),
                default_config: p.default_config(),
                config_schema: p.config_schema(),
                frontend: p.frontend().cloned(),
                viewer: p.viewer().cloned(),
                supports_single_node: p.supports_single_node(),
            })
            .collect()
    }

    /// 返回已注册节点数量
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Connection;
    use crate::context::ExecutionContext;
    use crate::error::ErrorDetail;
    use crate::node::Node;
    use async_trait::async_trait;
    use std::collections::HashMap;

    /// 最小化的 mock 插件，用于测试 `NodeRegistry`
    struct MockPlugin {
        type_id: String,
        display_name: String,
        category: String,
        frontend: Option<crate::plugin::FrontendMetadata>,
    }

    impl MockPlugin {
        fn new(type_id: &str, display_name: &str, category: &str) -> Self {
            Self {
                type_id: type_id.to_string(),
                display_name: display_name.to_string(),
                category: category.to_string(),
                frontend: None,
            }
        }
    }

    #[async_trait]
    impl NodePlugin for MockPlugin {
        fn type_id(&self) -> &str {
            &self.type_id
        }

        fn display_name(&self) -> &str {
            &self.display_name
        }

        fn category(&self) -> &str {
            &self.category
        }

        fn icon(&self) -> &str {
            "test-icon"
        }

        fn color(&self) -> &str {
            "#000000"
        }

        fn ports(&self) -> Vec<crate::plugin::PortDef> {
            vec![crate::plugin::PortDef {
                id: "output".to_string(),
                label: "输出".to_string(),
                position: "right".to_string(),
            }]
        }

        fn default_config(&self) -> serde_json::Value {
            serde_json::json!({})
        }

        fn frontend(&self) -> Option<&crate::plugin::FrontendMetadata> {
            self.frontend.as_ref()
        }

        async fn execute(
            &self,
            _node: &Node,
            _context: &mut ExecutionContext,
            _connections: &[Connection],
            _node_map: &HashMap<String, &Node>,
        ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
            Ok((vec![], None))
        }
    }

    /// 测试：注册后可以通过 `type_id` 查找插件
    #[test]
    fn test_register_and_get() {
        let mut registry = NodeRegistry::new();
        let plugin = Arc::new(MockPlugin::new("http:request", "HTTP 请求", "http"));

        registry.register(plugin);

        let found = registry.get("http:request");
        assert!(found.is_some());
        assert_eq!(
            found.unwrap_or_else(|| panic!("expected Some")).type_id(),
            "http:request"
        );
        assert_eq!(
            found
                .unwrap_or_else(|| panic!("expected Some"))
                .display_name(),
            "HTTP 请求"
        );
    }

    /// `测试：all_metadata` 返回正确的元数据
    #[test]
    fn test_all_metadata() {
        let mut registry = NodeRegistry::new();

        registry.register(Arc::new(MockPlugin::new(
            "http:request",
            "HTTP 请求",
            "http",
        )));

        registry.register(Arc::new(MockPlugin::new("tcp:connect", "TCP 连接", "tcp")));

        let metadata = registry.all_metadata();
        assert_eq!(metadata.len(), 2);

        // 验证元数据内容（不依赖顺序）
        let http_meta = metadata
            .iter()
            .find(|m| m.type_id == "http:request")
            .unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(http_meta.display_name, "HTTP 请求");
        assert_eq!(http_meta.category, "http");
        assert_eq!(http_meta.icon, "test-icon");
        assert_eq!(http_meta.color, "#000000");
        assert_eq!(http_meta.ports.len(), 1);
        assert_eq!(http_meta.ports[0].id, "output");

        let tcp_meta = metadata
            .iter()
            .find(|m| m.type_id == "tcp:connect")
            .unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(tcp_meta.display_name, "TCP 连接");
        assert_eq!(tcp_meta.category, "tcp");
    }

    /// 测试：`all_metadata` 携带插件节点的 `frontend` 元数据
    #[test]
    fn test_all_metadata_carries_frontend() {
        let mut registry = NodeRegistry::new();

        // 带前端的插件节点
        let mut plugin = MockPlugin::new("acme:panel", "Panel", "plugin");
        plugin.frontend = Some(crate::plugin::FrontendMetadata {
            kind: "iframe".to_string(),
            content: None,
            url: Some("mpe-plugin://acme.localhost/config-panel".to_string()),
        });
        registry.register(Arc::new(plugin));

        // 无前端的内置节点
        registry.register(Arc::new(MockPlugin::new(
            "http:request",
            "HTTP 请求",
            "http",
        )));

        let metadata = registry.all_metadata();
        let plugin_meta = metadata
            .iter()
            .find(|m| m.type_id == "acme:panel")
            .unwrap_or_else(|| panic!("expected Some"));
        let frontend = plugin_meta
            .frontend
            .as_ref()
            .unwrap_or_else(|| panic!("plugin frontend must be carried"));
        assert_eq!(frontend.kind, "iframe");
        assert_eq!(
            frontend.url.as_deref(),
            Some("mpe-plugin://acme.localhost/config-panel")
        );

        // 内置节点无 frontend
        let http_meta = metadata
            .iter()
            .find(|m| m.type_id == "http:request")
            .unwrap_or_else(|| panic!("expected Some"));
        assert!(http_meta.frontend.is_none());
    }

    /// 测试：查找不存在的 `type_id` 返回 None
    #[test]
    fn test_get_unknown_returns_none() {
        let mut registry = NodeRegistry::new();
        registry.register(Arc::new(MockPlugin::new(
            "http:request",
            "HTTP 请求",
            "http",
        )));

        assert!(registry.get("nonexistent").is_none());
    }

    /// 测试：注册相同 `type_id` 的插件会覆盖旧插件
    #[test]
    fn test_register_overwrites() {
        let mut registry = NodeRegistry::new();

        registry.register(Arc::new(MockPlugin::new("http:request", "旧 HTTP", "http")));

        assert_eq!(registry.len(), 1);
        assert_eq!(
            registry
                .get("http:request")
                .unwrap_or_else(|| panic!("expected Some"))
                .display_name(),
            "旧 HTTP"
        );

        // 注册同 type_id 的新插件
        registry.register(Arc::new(MockPlugin::new("http:request", "新 HTTP", "http")));

        assert_eq!(registry.len(), 1);
        assert_eq!(
            registry
                .get("http:request")
                .unwrap_or_else(|| panic!("expected Some"))
                .display_name(),
            "新 HTTP"
        );
    }

    /// 测试：len 和 `is_empty`
    #[test]
    fn test_len_and_is_empty() {
        let mut registry = NodeRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        registry.register(Arc::new(MockPlugin::new(
            "http:request",
            "HTTP 请求",
            "http",
        )));

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
    }

    /// 测试：Default trait
    #[test]
    fn test_default() {
        let registry = NodeRegistry::default();
        assert!(registry.is_empty());
    }
}
