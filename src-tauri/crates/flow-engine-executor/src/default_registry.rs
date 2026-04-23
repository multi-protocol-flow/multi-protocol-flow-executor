//! 默认节点注册表 — 注册全部 17 个内置节点插件
//!
//! 此模块位于 `flow-engine-executor` crate 中，因为它需要依赖所有协议 crate
//! （http、ws、db、script、assertion、grpc），而 `flow-engine-core` 作为叶子 crate
//! 不能依赖它们。

use flow_engine_core::registry::NodeRegistry;

/// 创建包含全部 17 个内置节点的默认注册表
///
/// # 注册的插件
///
/// | 分类 | 插件 | type_id |
/// |------|------|---------|
/// | Logic | EntryPlugin | `entry` |
/// | Logic | ConditionalPlugin | `conditional` |
/// | HTTP | HttpPlugin | `http` |
/// | WebSocket | WsConnectPlugin | `ws_connect` |
/// | WebSocket | WsSendCollectPlugin | `ws_send_collect` |
/// | WebSocket | WsClosePlugin | `ws_close` |
/// | Database | DbConnectPlugin | `db_connect` |
/// | Database | DbQueryPlugin | `db_query` |
/// | Database | DbBeginPlugin | `db_begin` |
/// | Database | DbCommitPlugin | `db_commit` |
/// | Database | DbRollbackPlugin | `db_rollback` |
/// | Database | DbClosePlugin | `db_close` |
/// | Script | ScriptPlugin | `script` |
/// | Assertion | AssertionPlugin | `assertion` |
/// | gRPC | GrpcConnectPlugin | `grpc_connect` |
/// | gRPC | GrpcCallPlugin | `grpc_call` |
/// | gRPC | GrpcClosePlugin | `grpc_close` |
pub fn default_registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();

    // Logic (flow-engine-executor 内置)
    registry.register(std::sync::Arc::new(crate::plugin_entry::EntryPlugin));
    registry.register(std::sync::Arc::new(
        crate::plugin_conditional::ConditionalPlugin,
    ));

    // HTTP (flow-engine-http)
    registry.register(std::sync::Arc::new(flow_engine_http::plugin::HttpPlugin));

    // WebSocket (flow-engine-ws) — 批量注册 3 个插件
    flow_engine_ws::plugin::WsPluginGroup::register_all(&mut registry);

    // Database (flow-engine-db) — 批量注册 6 个插件
    flow_engine_db::plugin::DbPluginGroup::register_all(&mut registry);

    // Script (flow-engine-script)
    registry.register(std::sync::Arc::new(
        flow_engine_script::plugin::ScriptPlugin,
    ));

    // Assertion (flow-engine-assertion)
    registry.register(std::sync::Arc::new(
        flow_engine_assertion::plugin::AssertionPlugin,
    ));

    // gRPC (flow-engine-grpc) — 批量注册 3 个插件
    flow_engine_grpc::plugin::GrpcPluginGroup::register_all(&mut registry);

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry_has_17_plugins() {
        let registry = default_registry();
        assert_eq!(registry.len(), 17);

        let metadata = registry.all_metadata();
        assert_eq!(metadata.len(), 17);

        // 验证每个 type_id 都已注册
        let type_ids: Vec<&str> = metadata.iter().map(|m| m.type_id.as_str()).collect();
        let expected = [
            "entry",
            "conditional",
            "http",
            "ws_connect",
            "ws_send_collect",
            "ws_close",
            "assertion",
            "db_connect",
            "db_query",
            "db_begin",
            "db_commit",
            "db_rollback",
            "db_close",
            "script",
            "grpc_connect",
            "grpc_call",
            "grpc_close",
        ];

        for exp in &expected {
            assert!(
                type_ids.contains(exp),
                "Missing plugin: {}",
                exp
            );
        }
    }

    #[test]
    fn test_default_registry_all_plugins_have_metadata() {
        let registry = default_registry();
        let metadata = registry.all_metadata();

        for meta in &metadata {
            // 验证元数据基本完整性
            assert!(!meta.type_id.is_empty(), "type_id 不能为空");
            assert!(!meta.display_name.is_empty(), "display_name 不能为空");
            assert!(!meta.category.is_empty(), "category 不能为空");
            assert!(!meta.icon.is_empty(), "icon 不能为空");
            assert!(!meta.color.is_empty(), "color 不能为空");
            assert!(!meta.ports.is_empty(), "ports 不能为空");
        }
    }
}
