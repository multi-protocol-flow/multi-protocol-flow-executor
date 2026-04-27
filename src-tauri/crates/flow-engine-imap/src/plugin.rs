// IMAP 节点插件 — NodePlugin trait 实现
//
// 提供 13 个 IMAP 节点的元数据、端口定义、配置验证和执行能力：
// - ImapConnectPlugin: IMAP 连接
// - ImapSelectPlugin: 选择邮箱
// - ImapSearchPlugin: 搜索邮件
// - ImapFetchPlugin: 获取邮件
// - ImapStoreFlagsPlugin: 设置邮件标志
// - ImapCopyPlugin: 复制邮件
// - ImapMovePlugin: 移动邮件
// - ImapExpungePlugin: 永久删除已标记邮件
// - ImapListPlugin: 列出邮箱
// - ImapFolderCreatePlugin: 创建文件夹
// - ImapFolderDeletePlugin: 删除文件夹
// - ImapIdlePlugin: IDLE 监听
// - ImapClosePlugin: 关闭连接
// 以及 ImapPluginGroup 批量注册辅助。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;
use std::sync::Arc;

/// 获取 IMAP manager 的辅助宏
macro_rules! get_imap_manager {
    ($context:expr) => {
        $context.imap_manager.clone().unwrap_or_else(|| {
            Arc::new(crate::imap_connection_manager::ImapConnectionManager::new())
        })
    };
}

// true/false 端口定义
fn true_false_ports(success_label: &str, fail_label: &str) -> Vec<PortDef> {
    vec![
        PortDef { id: "true".into(), label: success_label.into(), position: "right".into() },
        PortDef { id: "false".into(), label: fail_label.into(), position: "bottom".into() },
    ]
}

/// 检查 $.success 的 resolve_port
fn resolve_success_port(output: &Option<serde_json::Value>) -> String {
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

// ---------------------------------------------------------------------------
// ImapConnectPlugin
// ---------------------------------------------------------------------------

pub struct ImapConnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for ImapConnectPlugin {
    fn type_id(&self) -> &str { "imap_connect" }
    fn display_name(&self) -> &str { "IMAP 连接" }
    fn category(&self) -> &str { "email" }
    fn icon(&self) -> &str { "mail" }
    fn color(&self) -> &str { "#2563eb" }

    fn ports(&self) -> Vec<PortDef> {
        true_false_ports("连接成功", "连接失败")
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({ "host": "imap.gmail.com", "port": 993, "tls_mode": "implicit" })
    }

    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String {
        resolve_success_port(output)
    }

    fn is_protocol_node(&self) -> bool { true }
    fn supports_false_port(&self) -> bool { true }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "host": config.get("host"), "port": config.get("port"), "tls_mode": config.get("tls_mode") }))
    }

    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mgr = get_imap_manager!(context);
        let executor = crate::imap_connect_executor::ImapConnectExecutor::new(mgr);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// ImapSelectPlugin
// ---------------------------------------------------------------------------

pub struct ImapSelectPlugin;

#[async_trait::async_trait]
impl NodePlugin for ImapSelectPlugin {
    fn type_id(&self) -> &str { "imap_select" }
    fn display_name(&self) -> &str { "IMAP 选择邮箱" }
    fn category(&self) -> &str { "email" }
    fn icon(&self) -> &str { "mail" }
    fn color(&self) -> &str { "#2563eb" }

    fn ports(&self) -> Vec<PortDef> { true_false_ports("选择成功", "选择失败") }
    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({ "connection_id": "", "mailbox": "INBOX", "readonly": false })
    }
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String { resolve_success_port(output) }
    fn is_protocol_node(&self) -> bool { true }
    fn supports_false_port(&self) -> bool { true }
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "connection_id": config.get("connection_id"), "mailbox": config.get("mailbox") }))
    }
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mgr = get_imap_manager!(context);
        let executor = crate::imap_select_executor::ImapSelectExecutor::new(mgr);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// ImapSearchPlugin
// ---------------------------------------------------------------------------

pub struct ImapSearchPlugin;

#[async_trait::async_trait]
impl NodePlugin for ImapSearchPlugin {
    fn type_id(&self) -> &str { "imap_search" }
    fn display_name(&self) -> &str { "IMAP 搜索" }
    fn category(&self) -> &str { "email" }
    fn icon(&self) -> &str { "mail" }
    fn color(&self) -> &str { "#2563eb" }
    fn ports(&self) -> Vec<PortDef> { true_false_ports("搜索成功", "搜索失败") }
    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({ "connection_id": "", "criteria": "ALL" })
    }
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String { resolve_success_port(output) }
    fn is_protocol_node(&self) -> bool { true }
    fn supports_false_port(&self) -> bool { true }
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "connection_id": config.get("connection_id"), "criteria": config.get("criteria") }))
    }
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mgr = get_imap_manager!(context);
        let executor = crate::imap_search_executor::ImapSearchExecutor::new(mgr);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// ImapFetchPlugin
// ---------------------------------------------------------------------------

pub struct ImapFetchPlugin;

#[async_trait::async_trait]
impl NodePlugin for ImapFetchPlugin {
    fn type_id(&self) -> &str { "imap_fetch" }
    fn display_name(&self) -> &str { "IMAP 获取" }
    fn category(&self) -> &str { "email" }
    fn icon(&self) -> &str { "mail" }
    fn color(&self) -> &str { "#2563eb" }
    fn ports(&self) -> Vec<PortDef> { true_false_ports("获取成功", "获取失败") }
    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({ "connection_id": "", "uids": "1:*", "fetch_items": "(ENVELOPE BODY.PEEK[] FLAGS)", "max_fetch_count": 50, "max_size_bytes": 5242880 })
    }
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String { resolve_success_port(output) }
    fn is_protocol_node(&self) -> bool { true }
    fn supports_false_port(&self) -> bool { true }
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "connection_id": config.get("connection_id"), "uids": config.get("uids") }))
    }
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mgr = get_imap_manager!(context);
        let executor = crate::imap_fetch_executor::ImapFetchExecutor::new(mgr);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// ImapStoreFlagsPlugin
// ---------------------------------------------------------------------------

pub struct ImapStoreFlagsPlugin;

#[async_trait::async_trait]
impl NodePlugin for ImapStoreFlagsPlugin {
    fn type_id(&self) -> &str { "imap_store_flags" }
    fn display_name(&self) -> &str { "IMAP 设置标志" }
    fn category(&self) -> &str { "email" }
    fn icon(&self) -> &str { "mail" }
    fn color(&self) -> &str { "#2563eb" }
    fn ports(&self) -> Vec<PortDef> { true_false_ports("设置成功", "设置失败") }
    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({ "connection_id": "", "uids": "", "operation": "add", "flags": [] })
    }
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String { resolve_success_port(output) }
    fn is_protocol_node(&self) -> bool { true }
    fn supports_false_port(&self) -> bool { true }
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "connection_id": config.get("connection_id"), "uids": config.get("uids"), "operation": config.get("operation") }))
    }
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mgr = get_imap_manager!(context);
        let executor = crate::imap_store_flags_executor::ImapStoreFlagsExecutor::new(mgr);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// ImapCopyPlugin
// ---------------------------------------------------------------------------

pub struct ImapCopyPlugin;

#[async_trait::async_trait]
impl NodePlugin for ImapCopyPlugin {
    fn type_id(&self) -> &str { "imap_copy" }
    fn display_name(&self) -> &str { "IMAP 复制" }
    fn category(&self) -> &str { "email" }
    fn icon(&self) -> &str { "mail" }
    fn color(&self) -> &str { "#2563eb" }
    fn ports(&self) -> Vec<PortDef> { true_false_ports("复制成功", "复制失败") }
    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({ "connection_id": "", "uids": "", "target_mailbox": "" })
    }
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String { resolve_success_port(output) }
    fn is_protocol_node(&self) -> bool { true }
    fn supports_false_port(&self) -> bool { true }
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "connection_id": config.get("connection_id"), "target_mailbox": config.get("target_mailbox") }))
    }
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mgr = get_imap_manager!(context);
        let executor = crate::imap_copy_executor::ImapCopyExecutor::new(mgr);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// ImapMovePlugin
// ---------------------------------------------------------------------------

pub struct ImapMovePlugin;

#[async_trait::async_trait]
impl NodePlugin for ImapMovePlugin {
    fn type_id(&self) -> &str { "imap_move" }
    fn display_name(&self) -> &str { "IMAP 移动" }
    fn category(&self) -> &str { "email" }
    fn icon(&self) -> &str { "mail" }
    fn color(&self) -> &str { "#2563eb" }
    fn ports(&self) -> Vec<PortDef> { true_false_ports("移动成功", "移动失败") }
    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({ "connection_id": "", "uids": "", "target_mailbox": "" })
    }
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String { resolve_success_port(output) }
    fn is_protocol_node(&self) -> bool { true }
    fn supports_false_port(&self) -> bool { true }
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "connection_id": config.get("connection_id"), "target_mailbox": config.get("target_mailbox") }))
    }
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mgr = get_imap_manager!(context);
        let executor = crate::imap_move_executor::ImapMoveExecutor::new(mgr);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// ImapExpungePlugin
// ---------------------------------------------------------------------------

pub struct ImapExpungePlugin;

#[async_trait::async_trait]
impl NodePlugin for ImapExpungePlugin {
    fn type_id(&self) -> &str { "imap_expunge" }
    fn display_name(&self) -> &str { "IMAP 清除" }
    fn category(&self) -> &str { "email" }
    fn icon(&self) -> &str { "mail" }
    fn color(&self) -> &str { "#2563eb" }
    fn ports(&self) -> Vec<PortDef> { true_false_ports("清除成功", "清除失败") }
    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({ "connection_id": "" })
    }
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String { resolve_success_port(output) }
    fn is_protocol_node(&self) -> bool { true }
    fn supports_false_port(&self) -> bool { true }
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "connection_id": config.get("connection_id") }))
    }
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mgr = get_imap_manager!(context);
        let executor = crate::imap_expunge_executor::ImapExpungeExecutor::new(mgr);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// ImapListPlugin
// ---------------------------------------------------------------------------

pub struct ImapListPlugin;

#[async_trait::async_trait]
impl NodePlugin for ImapListPlugin {
    fn type_id(&self) -> &str { "imap_list" }
    fn display_name(&self) -> &str { "IMAP 列出邮箱" }
    fn category(&self) -> &str { "email" }
    fn icon(&self) -> &str { "mail" }
    fn color(&self) -> &str { "#2563eb" }
    fn ports(&self) -> Vec<PortDef> { true_false_ports("列出成功", "列出失败") }
    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({ "connection_id": "", "reference_name": "", "pattern": "*" })
    }
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String { resolve_success_port(output) }
    fn is_protocol_node(&self) -> bool { true }
    fn supports_false_port(&self) -> bool { true }
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "connection_id": config.get("connection_id"), "pattern": config.get("pattern") }))
    }
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mgr = get_imap_manager!(context);
        let executor = crate::imap_list_executor::ImapListExecutor::new(mgr);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// ImapFolderCreatePlugin
// ---------------------------------------------------------------------------

pub struct ImapFolderCreatePlugin;

#[async_trait::async_trait]
impl NodePlugin for ImapFolderCreatePlugin {
    fn type_id(&self) -> &str { "imap_folder_create" }
    fn display_name(&self) -> &str { "IMAP 创建文件夹" }
    fn category(&self) -> &str { "email" }
    fn icon(&self) -> &str { "mail" }
    fn color(&self) -> &str { "#2563eb" }
    fn ports(&self) -> Vec<PortDef> { true_false_ports("创建成功", "创建失败") }
    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({ "connection_id": "", "folder_name": "" })
    }
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String { resolve_success_port(output) }
    fn is_protocol_node(&self) -> bool { true }
    fn supports_false_port(&self) -> bool { true }
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "connection_id": config.get("connection_id"), "folder_name": config.get("folder_name") }))
    }
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mgr = get_imap_manager!(context);
        let executor = crate::imap_folder_create_executor::ImapFolderCreateExecutor::new(mgr);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// ImapFolderDeletePlugin
// ---------------------------------------------------------------------------

pub struct ImapFolderDeletePlugin;

#[async_trait::async_trait]
impl NodePlugin for ImapFolderDeletePlugin {
    fn type_id(&self) -> &str { "imap_folder_delete" }
    fn display_name(&self) -> &str { "IMAP 删除文件夹" }
    fn category(&self) -> &str { "email" }
    fn icon(&self) -> &str { "mail" }
    fn color(&self) -> &str { "#2563eb" }
    fn ports(&self) -> Vec<PortDef> { true_false_ports("删除成功", "删除失败") }
    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({ "connection_id": "", "folder_name": "" })
    }
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String { resolve_success_port(output) }
    fn is_protocol_node(&self) -> bool { true }
    fn supports_false_port(&self) -> bool { true }
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "connection_id": config.get("connection_id"), "folder_name": config.get("folder_name") }))
    }
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mgr = get_imap_manager!(context);
        let executor = crate::imap_folder_delete_executor::ImapFolderDeleteExecutor::new(mgr);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// ImapIdlePlugin
// ---------------------------------------------------------------------------

pub struct ImapIdlePlugin;

#[async_trait::async_trait]
impl NodePlugin for ImapIdlePlugin {
    fn type_id(&self) -> &str { "imap_idle" }
    fn display_name(&self) -> &str { "IMAP IDLE" }
    fn category(&self) -> &str { "email" }
    fn icon(&self) -> &str { "mail" }
    fn color(&self) -> &str { "#2563eb" }
    fn ports(&self) -> Vec<PortDef> { true_false_ports("监听成功", "监听失败") }
    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({ "connection_id": "", "mailbox": null, "timeout_seconds": 300 })
    }
    fn resolve_port(&self, output: &Option<serde_json::Value>) -> String { resolve_success_port(output) }
    fn is_protocol_node(&self) -> bool { true }
    fn supports_false_port(&self) -> bool { true }
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "connection_id": config.get("connection_id"), "timeout_seconds": config.get("timeout_seconds") }))
    }
    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mgr = get_imap_manager!(context);
        let executor = crate::imap_idle_executor::ImapIdleExecutor::new(mgr);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// ImapClosePlugin
// ---------------------------------------------------------------------------

pub struct ImapClosePlugin;

#[async_trait::async_trait]
impl NodePlugin for ImapClosePlugin {
    fn type_id(&self) -> &str { "imap_close" }
    fn display_name(&self) -> &str { "IMAP 关闭" }
    fn category(&self) -> &str { "email" }
    fn icon(&self) -> &str { "mail" }
    fn color(&self) -> &str { "#2563eb" }

    fn ports(&self) -> Vec<PortDef> {
        vec![PortDef { id: "out".into(), label: "输出".into(), position: "right".into() }]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({ "connection_id": "" })
    }

    /// Close 始终返回 "out" 端口
    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String { "out".to_string() }

    fn is_protocol_node(&self) -> bool { true }
    fn supports_false_port(&self) -> bool { false }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "connection_id": config.get("connection_id") }))
    }

    async fn execute(&self, node: &Node, context: &mut ExecutionContext, connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let mgr = get_imap_manager!(context);
        let executor = crate::imap_close_executor::ImapCloseExecutor::new(mgr);
        executor.execute(node, context, connections, node_map).await
    }
}

// ---------------------------------------------------------------------------
// ImapPluginGroup
// ---------------------------------------------------------------------------

/// IMAP 插件组 — 批量注册辅助
pub struct ImapPluginGroup;

impl ImapPluginGroup {
    /// 将所有 IMAP 插件注册到 NodeRegistry
    pub fn register_all(registry: &mut flow_engine_core::registry::NodeRegistry) {
        registry.register(Arc::new(ImapConnectPlugin));
        registry.register(Arc::new(ImapSelectPlugin));
        registry.register(Arc::new(ImapSearchPlugin));
        registry.register(Arc::new(ImapFetchPlugin));
        registry.register(Arc::new(ImapStoreFlagsPlugin));
        registry.register(Arc::new(ImapCopyPlugin));
        registry.register(Arc::new(ImapMovePlugin));
        registry.register(Arc::new(ImapExpungePlugin));
        registry.register(Arc::new(ImapListPlugin));
        registry.register(Arc::new(ImapFolderCreatePlugin));
        registry.register(Arc::new(ImapFolderDeletePlugin));
        registry.register(Arc::new(ImapIdlePlugin));
        registry.register(Arc::new(ImapClosePlugin));
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- 元数据测试 ----

    #[test]
    fn test_imap_connect_plugin_metadata() {
        let p = ImapConnectPlugin;
        assert_eq!(p.type_id(), "imap_connect");
        assert_eq!(p.display_name(), "IMAP 连接");
        assert_eq!(p.category(), "email");
        assert_eq!(p.icon(), "mail");
        assert_eq!(p.color(), "#2563eb");
        assert!(p.is_protocol_node());
        assert!(p.supports_false_port());
        let ports = p.ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].id, "true");
        assert_eq!(ports[1].id, "false");
    }

    #[test]
    fn test_imap_close_plugin_metadata() {
        let p = ImapClosePlugin;
        assert_eq!(p.type_id(), "imap_close");
        assert_eq!(p.display_name(), "IMAP 关闭");
        assert!(p.is_protocol_node());
        assert!(!p.supports_false_port());
        let ports = p.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].id, "out");
    }

    #[test]
    fn test_all_plugins_have_correct_color() {
        let plugins: Vec<Box<dyn NodePlugin>> = vec![
            Box::new(ImapConnectPlugin), Box::new(ImapSelectPlugin),
            Box::new(ImapSearchPlugin), Box::new(ImapFetchPlugin),
            Box::new(ImapStoreFlagsPlugin), Box::new(ImapCopyPlugin),
            Box::new(ImapMovePlugin), Box::new(ImapExpungePlugin),
            Box::new(ImapListPlugin), Box::new(ImapFolderCreatePlugin),
            Box::new(ImapFolderDeletePlugin), Box::new(ImapIdlePlugin),
            Box::new(ImapClosePlugin),
        ];
        for p in &plugins {
            assert_eq!(p.color(), "#2563eb", "{} color mismatch", p.type_id());
            assert_eq!(p.category(), "email", "{} category mismatch", p.type_id());
            assert_eq!(p.icon(), "mail", "{} icon mismatch", p.type_id());
        }
    }

    // ---- resolve_port 测试 ----

    #[test]
    fn test_connect_resolve_port() {
        let p = ImapConnectPlugin;
        assert_eq!(p.resolve_port(&Some(serde_json::json!({"success": true}))), "true");
        assert_eq!(p.resolve_port(&Some(serde_json::json!({"success": false}))), "false");
        assert_eq!(p.resolve_port(&None), "false");
    }

    #[test]
    fn test_close_resolve_port_always_out() {
        let p = ImapClosePlugin;
        assert_eq!(p.resolve_port(&Some(serde_json::json!({"success": true}))), "out");
        assert_eq!(p.resolve_port(&Some(serde_json::json!({"success": false}))), "out");
        assert_eq!(p.resolve_port(&None), "out");
    }

    // ---- default_config 测试 ----

    #[test]
    fn test_default_configs_not_empty() {
        let configs: Vec<(&str, serde_json::Value)> = vec![
            ("imap_connect", ImapConnectPlugin.default_config()),
            ("imap_select", ImapSelectPlugin.default_config()),
            ("imap_search", ImapSearchPlugin.default_config()),
            ("imap_fetch", ImapFetchPlugin.default_config()),
            ("imap_store_flags", ImapStoreFlagsPlugin.default_config()),
            ("imap_copy", ImapCopyPlugin.default_config()),
            ("imap_move", ImapMovePlugin.default_config()),
            ("imap_expunge", ImapExpungePlugin.default_config()),
            ("imap_list", ImapListPlugin.default_config()),
            ("imap_folder_create", ImapFolderCreatePlugin.default_config()),
            ("imap_folder_delete", ImapFolderDeletePlugin.default_config()),
            ("imap_idle", ImapIdlePlugin.default_config()),
            ("imap_close", ImapClosePlugin.default_config()),
        ];
        for (name, config) in &configs {
            assert!(config.is_object(), "{} default_config should be object", name);
        }
    }

    // ---- ImapPluginGroup.register_all 测试 ----

    #[test]
    fn test_imap_plugin_group_register_all() {
        let mut registry = flow_engine_core::registry::NodeRegistry::new();
        assert!(registry.is_empty());
        ImapPluginGroup::register_all(&mut registry);
        assert_eq!(registry.len(), 13);

        let expected = [
            "imap_connect", "imap_select", "imap_search", "imap_fetch",
            "imap_store_flags", "imap_copy", "imap_move", "imap_expunge",
            "imap_list", "imap_folder_create", "imap_folder_delete",
            "imap_idle", "imap_close",
        ];
        for id in &expected {
            assert!(registry.get(id).is_some(), "Missing plugin: {}", id);
        }
    }

    #[test]
    fn test_build_report_input_returns_some() {
        let config = serde_json::json!({ "connection_id": "c1" });
        assert!(ImapConnectPlugin.build_report_input(&config).is_some());
        assert!(ImapSelectPlugin.build_report_input(&config).is_some());
        assert!(ImapSearchPlugin.build_report_input(&config).is_some());
        assert!(ImapFetchPlugin.build_report_input(&config).is_some());
        assert!(ImapStoreFlagsPlugin.build_report_input(&config).is_some());
        assert!(ImapCopyPlugin.build_report_input(&config).is_some());
        assert!(ImapMovePlugin.build_report_input(&config).is_some());
        assert!(ImapExpungePlugin.build_report_input(&config).is_some());
        assert!(ImapListPlugin.build_report_input(&config).is_some());
        assert!(ImapFolderCreatePlugin.build_report_input(&config).is_some());
        assert!(ImapFolderDeletePlugin.build_report_input(&config).is_some());
        assert!(ImapIdlePlugin.build_report_input(&config).is_some());
        assert!(ImapClosePlugin.build_report_input(&config).is_some());
    }
}
