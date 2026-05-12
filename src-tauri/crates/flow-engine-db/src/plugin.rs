// 数据库节点插件 — NodePlugin trait 实现
//
// 提供 6 个数据库节点的元数据、端口定义、配置验证和执行能力。
// 执行委托给各数据库节点执行器。
// DbPluginGroup 支持批量注册全部数据库节点插件。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;
use std::sync::Arc;

/// 从执行上下文获取数据库连接池管理器
///
/// 如果上下文中没有设置 `db_manager，则创建一个新的空管理器`。
fn get_db_manager(context: &ExecutionContext) -> Arc<dyn flow_engine_core::traits::DbProvider> {
    context
        .db_manager
        .clone()
        .unwrap_or_else(|| Arc::new(crate::db::pool::DatabaseConnectionManager::new()))
}

// ============================================================================
// DbConnectPlugin — 数据库连接节点
// ============================================================================

/// 数据库连接节点插件
pub struct DbConnectPlugin;

#[async_trait::async_trait]
impl NodePlugin for DbConnectPlugin {
    fn type_id(&self) -> &str {
        "db_connect"
    }

    fn display_name(&self) -> &str {
        "数据库连接"
    }

    fn category(&self) -> &str {
        "database"
    }

    fn icon(&self) -> &str {
        "database"
    }

    fn color(&self) -> &str {
        "#EF4444"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_uuid": "",
            "driver": null,
            "connection_string": null,
            "host": null,
            "port": null,
            "database": null,
            "username": null,
            "password": null,
            "min_connections": 1,
            "max_connections": 10,
            "test_query": null,
            "connect_timeout_ms": null,
        })
    }

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

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_uuid": config.get("connection_uuid"),
            "test_query": config.get("test_query"),
            "connect_timeout_ms": config.get("connect_timeout_ms"),
        }))
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "required": ["connection_uuid"],
            "properties": {
                "connection_uuid": {
                    "type": "string",
                    "title": "连接 UUID",
                    "description": "数据库连接标识符"
                },
                "driver": {
                    "type": "string",
                    "title": "数据库驱动",
                    "enum": ["postgresql", "mysql", "sqlite", "redis"]
                },
                "connection_string": {
                    "type": "string",
                    "title": "连接字符串"
                },
                "host": {
                    "type": "string",
                    "title": "主机地址"
                },
                "port": {
                    "type": "integer",
                    "title": "端口号"
                },
                "database": {
                    "type": "string",
                    "title": "数据库名"
                },
                "username": {
                    "type": "string",
                    "title": "用户名"
                },
                "password": {
                    "type": "string",
                    "title": "密码",
                    "format": "password"
                },
                "min_connections": {
                    "type": "integer",
                    "title": "最小连接数",
                    "minimum": 1,
                    "default": 1
                },
                "max_connections": {
                    "type": "integer",
                    "title": "最大连接数",
                    "minimum": 1,
                    "default": 10
                },
                "test_query": {
                    "type": "string",
                    "title": "测试查询语句"
                },
                "connect_timeout_ms": {
                    "type": "integer",
                    "title": "连接超时（毫秒）"
                }
            }
        }))
    }

    async fn execute
        (&self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let db_manager = get_db_manager(context);
        let executor = crate::db_connect_executor::DbConnectExecutor::new(db_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ============================================================================
// DbQueryPlugin — 数据库查询节点
// ============================================================================

/// 数据库查询节点插件
pub struct DbQueryPlugin;

#[async_trait::async_trait]
impl NodePlugin for DbQueryPlugin {
    fn type_id(&self) -> &str {
        "db_query"
    }

    fn display_name(&self) -> &str {
        "数据库查询"
    }

    fn category(&self) -> &str {
        "database"
    }

    fn icon(&self) -> &str {
        "search"
    }

    fn color(&self) -> &str {
        "#EF4444"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_uuid": "",
            "sql": null,
            "params": null,
            "redis_command": null,
            "redis_key": null,
            "redis_value": null,
            "redis_ttl": null,
            "fetch_mode": "all",
            "result_to": null,
            "timeout_ms": null,
        })
    }

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

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_uuid": config.get("connection_uuid"),
            "sql": config.get("sql"),
            "params": config.get("params"),
            "redis_command": config.get("redis_command"),
            "redis_key": config.get("redis_key"),
            "redis_value": config.get("redis_value"),
            "redis_ttl": config.get("redis_ttl"),
            "fetch_mode": config.get("fetch_mode"),
            "result_to": config.get("result_to"),
            "timeout_ms": config.get("timeout_ms"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let db_manager = get_db_manager(context);
        // 注意：DbQueryNodeExecutor 使用 with_db_manager() 构造器
        let executor = crate::db_query_executor::DbQueryNodeExecutor::with_db_manager(db_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ============================================================================
// DbBeginPlugin — 事务开始节点
// ============================================================================

/// 事务开始节点插件
pub struct DbBeginPlugin;

#[async_trait::async_trait]
impl NodePlugin for DbBeginPlugin {
    fn type_id(&self) -> &str {
        "db_begin"
    }

    fn display_name(&self) -> &str {
        "事务开始"
    }

    fn category(&self) -> &str {
        "database"
    }

    fn icon(&self) -> &str {
        "play"
    }

    fn color(&self) -> &str {
        "#EF4444"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_uuid": "",
            "isolation_level": null,
        })
    }

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

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_uuid": config.get("connection_uuid"),
            "isolation_level": config.get("isolation_level"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let db_manager = get_db_manager(context);
        let executor = crate::db_begin_executor::DbBeginExecutor::new(db_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ============================================================================
// DbCommitPlugin — 事务提交节点
// ============================================================================

/// 事务提交节点插件
pub struct DbCommitPlugin;

#[async_trait::async_trait]
impl NodePlugin for DbCommitPlugin {
    fn type_id(&self) -> &str {
        "db_commit"
    }

    fn display_name(&self) -> &str {
        "事务提交"
    }

    fn category(&self) -> &str {
        "database"
    }

    fn icon(&self) -> &str {
        "check"
    }

    fn color(&self) -> &str {
        "#EF4444"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_uuid": "",
        })
    }

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

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_uuid": config.get("connection_uuid"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let db_manager = get_db_manager(context);
        let executor = crate::db_commit_executor::DbCommitExecutor::new(db_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ============================================================================
// DbRollbackPlugin — 事务回滚节点
// ============================================================================

/// 事务回滚节点插件
pub struct DbRollbackPlugin;

#[async_trait::async_trait]
impl NodePlugin for DbRollbackPlugin {
    fn type_id(&self) -> &str {
        "db_rollback"
    }

    fn display_name(&self) -> &str {
        "事务回滚"
    }

    fn category(&self) -> &str {
        "database"
    }

    fn icon(&self) -> &str {
        "undo"
    }

    fn color(&self) -> &str {
        "#EF4444"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "true".into(),
                label: "成功".into(),
                position: "right".into(),
            },
            PortDef {
                id: "false".into(),
                label: "失败".into(),
                position: "bottom".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_uuid": "",
        })
    }

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

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_uuid": config.get("connection_uuid"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let db_manager = get_db_manager(context);
        let executor = crate::db_rollback_executor::DbRollbackExecutor::new(db_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ============================================================================
// DbClosePlugin — 关闭连接节点
// ============================================================================

/// 关闭连接节点插件
///
/// 特殊：只有一个 "out" `端口，resolve_port` 始终返回 "out"
pub struct DbClosePlugin;

#[async_trait::async_trait]
impl NodePlugin for DbClosePlugin {
    fn type_id(&self) -> &str {
        "db_close"
    }

    fn display_name(&self) -> &str {
        "关闭连接"
    }

    fn category(&self) -> &str {
        "database"
    }

    fn icon(&self) -> &str {
        "x"
    }

    fn color(&self) -> &str {
        "#EF4444"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![PortDef {
            id: "out".into(),
            label: "输出".into(),
            position: "right".into(),
        }]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "connection_uuid": "",
        })
    }

    /// `DbClose` 始终返回 "out"（关闭操作是幂等的）
    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String {
        "out".to_string()
    }

    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "connection_uuid": config.get("connection_uuid"),
        }))
    }

    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let db_manager = get_db_manager(context);
        let executor = crate::db_close_executor::DbCloseExecutor::new(db_manager);
        executor
            .execute(node, context, connections, node_map)
            .await
    }
}

// ============================================================================
// DbPluginGroup — 批量注册
// ============================================================================

/// 数据库节点插件组，支持批量注册全部 6 个数据库节点插件
pub struct DbPluginGroup;

impl DbPluginGroup {
    /// 将全部 6 个数据库节点插件注册到注册表
    pub fn register_all(registry: &mut flow_engine_core::registry::NodeRegistry) {
        registry.register(Arc::new(DbConnectPlugin));
        registry.register(Arc::new(DbQueryPlugin));
        registry.register(Arc::new(DbBeginPlugin));
        registry.register(Arc::new(DbCommitPlugin));
        registry.register(Arc::new(DbRollbackPlugin));
        registry.register(Arc::new(DbClosePlugin));
    }
}
