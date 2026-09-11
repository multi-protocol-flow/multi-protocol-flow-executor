//! GraphQL 连接管理器
//!
//! 实现 [`GraphqlProvider`] trait，管理 GraphQL 连接的生命周期。
//! 使用 reqwest 执行 Query/Mutation/Introspection，
//! WebSocket 订阅将在 T14 任务中实现。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use log::{info, warn};
use tokio::sync::RwLock;

use flow_engine_core::node_config::{
    GraphqlConnectConfig, GraphqlDisconnectConfig, GraphqlIntrospectConfig, GraphqlQueryConfig,
    GraphqlSubscribeConfig,
};
use flow_engine_core::traits::{
    GraphqlConnectOutput, GraphqlConnectionInfo, GraphqlIntrospectOutput, GraphqlProvider,
    GraphqlQueryOutput, GraphqlQueryTemplate, GraphqlSubscribeOutput, GraphqlSubscriptionEvent,
    GraphqlTimingInfo,
};

use crate::graphql_client::{GraphqlAuth, GraphqlClient};

// ---------------------------------------------------------------------------
// 连接状态
// ---------------------------------------------------------------------------

/// GraphQL 连接状态
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum GraphqlConnectionStatus {
    /// 已连接
    Connected,
    /// 已断开
    Disconnected,
    /// Error状态
    Error(String),
}

impl std::fmt::Display for GraphqlConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connected => write!(f, "connected"),
            Self::Disconnected => write!(f, "disconnected"),
            Self::Error(e) => write!(f, "error: {}", e),
        }
    }
}

// ---------------------------------------------------------------------------
// GraphqlConnection
// ---------------------------------------------------------------------------

/// GraphQL 连接
///
/// 持有 HTTP 端点信息、认证配置和连接元数据。
/// GraphQL 不像 WebSocket/MQTT 那样保持长连接，而是通过 HTTP 请求执行操作。
/// 此结构存储连接配置，供后续查询/变更/订阅操作使用。
#[derive(Debug, Clone)]
pub struct GraphqlConnection {
    /// HTTP 端点 URL
    pub http_endpoint: String,
    /// WebSocket 端点 URL（用于订阅）
    pub ws_endpoint: Option<String>,
    /// 自定义请求头
    pub headers: HashMap<String, String>,
    /// 认证配置
    pub auth_config: GraphqlAuth,
    /// 创建时间（毫秒时间戳）
    pub created_at: i64,
}

// ---------------------------------------------------------------------------
// GraphqlConnectionManager
// ---------------------------------------------------------------------------

/// GraphQL 连接管理器（per-flow-execution 作用域）
///
/// 管理 GraphQL 连接池，实现 `GraphqlProvider` trait。
/// 线程安全，使用 `Arc<RwLock<HashMap>>` 保护连接池。
pub struct GraphqlConnectionManager {
    /// 活跃连接池: `connection_id` → [`GraphqlConnection`]
    connections: Arc<RwLock<HashMap<String, GraphqlConnection>>>,
    /// GraphQL HTTP 客户端
    client: GraphqlClient,
    /// 连接Status: `connection_id` → [`GraphqlConnectionStatus`]
    statuses: Arc<RwLock<HashMap<String, GraphqlConnectionStatus>>>,
    /// 是否为共享模式（压力测试）
    is_shared: bool,
}

// ---------------------------------------------------------------------------
// 构造器
// ---------------------------------------------------------------------------

impl GraphqlConnectionManager {
    /// 创建新的连接管理器（普通模式）
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            client: GraphqlClient::new(),
            statuses: Arc::new(RwLock::new(HashMap::new())),
            is_shared: false,
        }
    }

    /// 创建共享模式的连接管理器（压测场景）
    ///
    /// 共享模式下 disconnect 为 no-op，避免并发虚拟用户互相关闭连接。
    pub fn new_shared() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            client: GraphqlClient::new(),
            statuses: Arc::new(RwLock::new(HashMap::new())),
            is_shared: true,
        }
    }

    /// 获取当前时间戳（毫秒）
    fn now_ms() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }
}

impl Default for GraphqlConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for GraphqlConnectionManager {
    fn drop(&mut self) {
        // Drop 时尝试清理所有连接状态（非阻塞）
        if let Ok(mut statuses) = self.statuses.try_write() {
            for (id, _status) in statuses.drain() {
                info!("GraphQL connection {} cleaned up on manager drop", id);
            }
        }
        if let Ok(mut connections) = self.connections.try_write() {
            connections.clear();
        }
    }
}

// ---------------------------------------------------------------------------
// GraphqlProvider 实现
// ---------------------------------------------------------------------------

#[async_trait]
impl GraphqlProvider for GraphqlConnectionManager {
    /// 建立 GraphQL 连接
    ///
    /// GraphQL 基于 HTTP，不需要真正的长连接。
    /// 此方法验证端点可达性，存储连接配置供后续操作使用。
    async fn connect(
        &self,
        uuid: &str,
        config: &GraphqlConnectConfig,
    ) -> Result<GraphqlConnectOutput, String> {
        info!(
            "GraphQL connecting: endpoint={}, id={}",
            config.endpoint, uuid
        );

        // UUID 去重：如果已存在且已连接，直接返回Success
        {
            let conns = self.connections.read().await;
            if conns.contains_key(uuid) {
                let statuses = self.statuses.read().await;
                if let Some(status) = statuses.get(uuid) {
                    if *status == GraphqlConnectionStatus::Connected {
                        info!("GraphQL reusing existing connection: id={}", uuid);
                        return Ok(GraphqlConnectOutput {
                            success: true,
                            connection_id: uuid.to_string(),
                            server_info: None,
                            error: None,
                        });
                    }
                }
            }
        } // 读锁释放

        // 构建认证配置
        let auth_config = Self::build_auth(config);

        // 验证端点可达性（发送简单的 introspection 查询）
        let timeout_ms = if config.connection_timeout_ms > 0 {
            config.connection_timeout_ms
        } else {
            5000
        };
        let headers_ref = config.headers.as_ref();
        let verification_result = self
            .client
            .execute_query(
                &config.endpoint,
                "{ __typename }",
                None,
                headers_ref,
                &auth_config,
                timeout_ms,
            )
            .await;

        match verification_result {
            Ok(response) => {
                // 检查是否有 GraphQL Error
                if let Some(errors) = &response.errors {
                    if !errors.is_empty() {
                        let error_msg = errors
                            .iter()
                            .map(|e| e.message.as_str())
                            .collect::<Vec<_>>()
                            .join("; ");
                        warn!(
                            "GraphQL connection verification returned errors: {}",
                            error_msg
                        );
                        // 仍然算连接Success（可能是服务端不支持 __typename 查询）
                    }
                }

                let connection = GraphqlConnection {
                    http_endpoint: config.endpoint.clone(),
                    ws_endpoint: config.ws_endpoint.clone(),
                    headers: config.headers.clone().unwrap_or_default(),
                    auth_config,
                    created_at: Self::now_ms(),
                };

                // 存入连接池
                {
                    let mut conns = self.connections.write().await;
                    conns.insert(uuid.to_string(), connection);
                }
                {
                    let mut statuses = self.statuses.write().await;
                    statuses.insert(uuid.to_string(), GraphqlConnectionStatus::Connected);
                }

                info!("GraphQL connection succeeded: id={}", uuid);

                Ok(GraphqlConnectOutput {
                    success: true,
                    connection_id: uuid.to_string(),
                    server_info: None,
                    error: None,
                })
            }
            Err(e) => {
                warn!("GraphQL connection verification failed: {}", e);

                // 仍然存储连接（可能验证查询格式不兼容）
                let connection = GraphqlConnection {
                    http_endpoint: config.endpoint.clone(),
                    ws_endpoint: config.ws_endpoint.clone(),
                    headers: config.headers.clone().unwrap_or_default(),
                    auth_config,
                    created_at: Self::now_ms(),
                };

                {
                    let mut conns = self.connections.write().await;
                    conns.insert(uuid.to_string(), connection);
                }
                {
                    let mut statuses = self.statuses.write().await;
                    statuses.insert(uuid.to_string(), GraphqlConnectionStatus::Error(e.clone()));
                }

                Ok(GraphqlConnectOutput {
                    success: false,
                    connection_id: uuid.to_string(),
                    server_info: None,
                    error: Some(e),
                })
            }
        }
    }

    /// 执行 GraphQL 查询/变更
    async fn query(
        &self,
        uuid: &str,
        config: &GraphqlQueryConfig,
    ) -> Result<GraphqlQueryOutput, String> {
        // 从连接池获取连接配置
        let (endpoint, headers, auth) = {
            let conns = self.connections.read().await;
            let conn = conns
                .get(uuid)
                .ok_or_else(|| format!("GraphQL connection '{}' does not exist", uuid))?;
            (
                conn.http_endpoint.clone(),
                conn.headers.clone(),
                conn.auth_config.clone(),
            )
        }; // 读锁释放

        let start = std::time::Instant::now();

        // 检查是否有文件上传
        let has_files = config.files.as_ref().is_some_and(|f| !f.is_empty());

        let result = if has_files {
            // 文件上传模式
            let files: Vec<crate::graphql_client::FileUpload> = config
                .files
                .as_ref()
                .map(|f| {
                    f.iter()
                        .map(|file| crate::graphql_client::FileUpload {
                            name: file.name.clone(),
                            // 文件上传使用空数据（实际场景中由执行器读取文件内容）
                            data: file.path.as_ref().map(|_| vec![]).unwrap_or_default(),
                            content_type: file.content_type.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            self.client
                .upload_files(
                    &endpoint,
                    &config.query,
                    config.variables.clone(),
                    &files,
                    Some(&headers),
                    &auth,
                    config.timeout_ms,
                )
                .await
        } else {
            self.client
                .execute_query(
                    &endpoint,
                    &config.query,
                    config.variables.clone(),
                    Some(&headers),
                    &auth,
                    config.timeout_ms,
                )
                .await
        };

        let elapsed = start.elapsed();

        match result {
            Ok(response) => {
                // 转换为 GraphqlQueryOutput
                let has_errors = response.errors.as_ref().is_some_and(|e| !e.is_empty());

                let errors_json: Vec<serde_json::Value> = response
                    .errors
                    .unwrap_or_default()
                    .iter()
                    .map(|e| serde_json::to_value(e).unwrap_or_default())
                    .collect();

                let timing = response.timing.map(|t| GraphqlTimingInfo {
                    total_ms: t.total_ms,
                    dns_ms: t.dns_ms,
                    connect_ms: t.connect_ms,
                    tls_ms: t.tls_ms,
                    server_ms: t.server_ms,
                });

                Ok(GraphqlQueryOutput {
                    success: !has_errors,
                    data: response.data,
                    errors: errors_json,
                    extensions: response.extensions,
                    status_code: Some(response.status_code),
                    timing,
                    error: None,
                })
            }
            Err(e) => {
                let total_ms = elapsed.as_millis() as u64;
                Ok(GraphqlQueryOutput {
                    success: false,
                    data: None,
                    errors: vec![],
                    extensions: None,
                    status_code: None,
                    timing: Some(GraphqlTimingInfo {
                        total_ms,
                        dns_ms: None,
                        connect_ms: None,
                        tls_ms: None,
                        server_ms: None,
                    }),
                    error: Some(e),
                })
            }
        }
    }

    /// 建立 GraphQL 订阅（WebSocket）
    ///
    /// **注意**: 完整的 WebSocket 订阅逻辑将在 T14 任务中实现。
    /// 此处返回空响应。
    async fn subscribe(
        &self,
        uuid: &str,
        config: &GraphqlSubscribeConfig,
    ) -> Result<GraphqlSubscribeOutput, String> {
        // 从连接池获取连接配置
        let (ws_endpoint, headers, auth) = {
            let conns = self.connections.read().await;
            let conn = conns
                .get(uuid)
                .ok_or_else(|| format!("GraphQL connection '{}' does not exist", uuid))?;
            (
                conn.ws_endpoint.clone(),
                conn.headers.clone(),
                conn.auth_config.clone(),
            )
        }; // 读锁释放

        // 检查是否配置了 WebSocket 端点
        let ws_url = ws_endpoint
            .ok_or_else(|| "GraphQL connection has no WebSocket endpoint configured".to_string())?;

        let start = std::time::Instant::now();

        let result = self
            .client
            .execute_subscription_via_ws(
                &ws_url,
                &config.subscription,
                config.variables.clone(),
                config.operation_name.clone(),
                Some(&headers),
                &auth,
                config.max_events,
                config.timeout_ms,
            )
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(sub_response) => {
                // 转换为 GraphqlSubscribeOutput
                let events: Vec<GraphqlSubscriptionEvent> = sub_response
                    .events
                    .iter()
                    .map(|e| GraphqlSubscriptionEvent {
                        data: e.data.clone(),
                        errors: e
                            .errors
                            .iter()
                            .map(|err| serde_json::to_value(err).unwrap_or(serde_json::json!(null)))
                            .collect(),
                        extensions: e.extensions.clone(),
                        received_at: e.received_at,
                    })
                    .collect();

                let event_count = events.len() as u32;

                Ok(GraphqlSubscribeOutput {
                    success: true,
                    events,
                    event_count,
                    duration_ms: sub_response.duration_ms,
                    error: None,
                })
            }
            Err(e) => Ok(GraphqlSubscribeOutput {
                success: false,
                events: vec![],
                event_count: 0,
                duration_ms: elapsed.as_millis() as u64,
                error: Some(e),
            }),
        }
    }

    /// 执行 GraphQL Schema 探索
    async fn introspect(
        &self,
        uuid: &str,
        config: &GraphqlIntrospectConfig,
    ) -> Result<GraphqlIntrospectOutput, String> {
        // 从连接池获取连接配置
        let (endpoint, headers, auth) = {
            let conns = self.connections.read().await;
            let conn = conns
                .get(uuid)
                .ok_or_else(|| format!("GraphQL connection '{}' does not exist", uuid))?;
            (
                conn.http_endpoint.clone(),
                conn.headers.clone(),
                conn.auth_config.clone(),
            )
        }; // 读锁释放

        // 标准 GraphQL Introspection 查询
        let introspection_query = r#"
            query IntrospectionQuery {
                __schema {
                    types {
                        kind
                        name
                        description
                        fields(includeDeprecated: true) {
                            name
                            description
                            args {
                                name
                                description
                                type { ...TypeRef }
                                defaultValue
                            }
                            type { ...TypeRef }
                            isDeprecated
                            deprecationReason
                        }
                        inputFields {
                            name
                            description
                            type { ...TypeRef }
                            defaultValue
                        }
                        interfaces { ...TypeRef }
                        enumValues(includeDeprecated: true) {
                            name
                            description
                            isDeprecated
                            deprecationReason
                        }
                        possibleTypes { ...TypeRef }
                    }
                    queryType { name }
                    mutationType { name }
                    subscriptionType { name }
                }
            }
            fragment TypeRef on __Type {
                kind
                name
                ofType {
                    kind
                    name
                    ofType {
                        kind
                        name
                        ofType {
                            kind
                            name
                            ofType {
                                kind
                                name
                                ofType {
                                    kind
                                    name
                                }
                            }
                        }
                    }
                }
            }
        "#;

        let timeout_ms = if config.timeout_ms > 0 {
            config.timeout_ms
        } else {
            30000
        };

        let result = self
            .client
            .execute_query(
                &endpoint,
                introspection_query,
                None,
                Some(&headers),
                &auth,
                timeout_ms,
            )
            .await;

        match result {
            Ok(response) => {
                let has_errors = response.errors.as_ref().is_some_and(|e| !e.is_empty());

                if has_errors {
                    let error_msg = response
                        .errors
                        .as_ref()
                        .map(|e| {
                            e.iter()
                                .map(|err| err.message.as_str())
                                .collect::<Vec<_>>()
                                .join("; ")
                        })
                        .unwrap_or_default();

                    return Ok(GraphqlIntrospectOutput {
                        success: false,
                        schema: response.data,
                        query_templates: None,
                        error: Some(error_msg),
                    });
                }

                // 从 schema 生成查询模板
                let query_templates = response.data.as_ref().and_then(Self::generate_templates);

                Ok(GraphqlIntrospectOutput {
                    success: true,
                    schema: response.data,
                    query_templates,
                    error: None,
                })
            }
            Err(e) => Ok(GraphqlIntrospectOutput {
                success: false,
                schema: None,
                query_templates: None,
                error: Some(e),
            }),
        }
    }

    /// 断开 GraphQL 连接
    ///
    /// 从连接池移除连接配置。
    /// 共享模式下为 no-op，避免并发虚拟用户互相关闭连接。
    async fn disconnect(
        &self,
        uuid: &str,
        _config: &GraphqlDisconnectConfig,
    ) -> Result<(), String> {
        if self.is_shared {
            info!("ignoring disconnect request in shared mode: {}", uuid);
            return Ok(());
        }

        let mut conns = self.connections.write().await;
        let removed = conns.remove(uuid);
        match removed {
            Some(_) => {
                let mut statuses = self.statuses.write().await;
                statuses.remove(uuid);
                info!("GraphQL disconnected: id={}", uuid);
                Ok(())
            }
            None => {
                warn!(
                    "GraphQL disconnect failed: connection '{}' does not exist",
                    uuid
                );
                Err(format!("GraphQL connection '{}' does not exist", uuid))
            }
        }
    }

    /// 获取连接状态
    fn get_status(&self, uuid: &str) -> Option<String> {
        let statuses = self.statuses.try_read().ok()?;
        let status = statuses.get(uuid)?;
        Some(status.to_string())
    }

    /// 列出所有连接信息
    fn list_connections(&self) -> Vec<GraphqlConnectionInfo> {
        let Ok(conns) = self.connections.try_read() else {
            return vec![];
        };
        let Ok(statuses) = self.statuses.try_read() else {
            return vec![];
        };
        conns
            .iter()
            .map(|(uuid, conn)| {
                let status_str = statuses
                    .get(uuid)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                GraphqlConnectionInfo {
                    connection_id: uuid.clone(),
                    endpoint: conn.http_endpoint.clone(),
                    status: status_str,
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// 内部辅助方法
// ---------------------------------------------------------------------------

impl GraphqlConnectionManager {
    /// 从连接配置构建认证信息
    fn build_auth(config: &GraphqlConnectConfig) -> GraphqlAuth {
        if let Some(ref headers) = config.headers {
            // 检查 Authorization 头
            if let Some(auth_value) = headers.get("Authorization") {
                if auth_value.starts_with("Bearer ") {
                    return GraphqlAuth::Bearer(
                        auth_value.trim_start_matches("Bearer ").to_string(),
                    );
                }
            }
            // 检查 Cookie 头
            if let Some(cookie) = headers.get("Cookie") {
                return GraphqlAuth::Cookie(cookie.clone());
            }
            // 检查常见 API Key 头
            if let Some(api_key) = headers.get("X-API-Key") {
                return GraphqlAuth::ApiKey {
                    name: "X-API-Key".to_string(),
                    value: api_key.clone(),
                };
            }
        }
        GraphqlAuth::None
    }

    /// 从 Schema 数据生成查询模板
    fn generate_templates(schema_data: &serde_json::Value) -> Option<Vec<GraphqlQueryTemplate>> {
        let schema = schema_data.get("__schema")?;
        let mut templates = Vec::new();

        // 生成 Query 模板
        if let Some(query_type) = schema.get("queryType").and_then(|qt| qt.get("name")) {
            let type_name = query_type.as_str().unwrap_or("Query");
            if let Some(types) = schema.get("types").and_then(|t| t.as_array()) {
                if let Some(query_type_def) = types
                    .iter()
                    .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(type_name))
                {
                    if let Some(fields) = query_type_def.get("fields").and_then(|f| f.as_array()) {
                        for field in fields {
                            let field_name =
                                field.get("name").and_then(|n| n.as_str()).unwrap_or("");
                            if field_name.starts_with("__") {
                                continue; // 跳过内部字段
                            }
                            templates.push(GraphqlQueryTemplate {
                                type_name: type_name.to_string(),
                                operation: "query".to_string(),
                                template: format!("{{ {} }}", field_name),
                            });
                        }
                    }
                }
            }
        }

        // 生成 Mutation 模板
        if let Some(mutation_type) = schema.get("mutationType").and_then(|mt| mt.get("name")) {
            let type_name = mutation_type.as_str().unwrap_or("Mutation");
            if let Some(types) = schema.get("types").and_then(|t| t.as_array()) {
                if let Some(mutation_type_def) = types
                    .iter()
                    .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(type_name))
                {
                    if let Some(fields) = mutation_type_def.get("fields").and_then(|f| f.as_array())
                    {
                        for field in fields {
                            let field_name =
                                field.get("name").and_then(|n| n.as_str()).unwrap_or("");
                            templates.push(GraphqlQueryTemplate {
                                type_name: type_name.to_string(),
                                operation: "mutation".to_string(),
                                template: format!("mutation {{ {} }}", field_name),
                            });
                        }
                    }
                }
            }
        }

        // 生成 Subscription 模板
        if let Some(subscription_type) =
            schema.get("subscriptionType").and_then(|st| st.get("name"))
        {
            let type_name = subscription_type.as_str().unwrap_or("Subscription");
            if let Some(types) = schema.get("types").and_then(|t| t.as_array()) {
                if let Some(subscription_type_def) = types
                    .iter()
                    .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(type_name))
                {
                    if let Some(fields) = subscription_type_def
                        .get("fields")
                        .and_then(|f| f.as_array())
                    {
                        for field in fields {
                            let field_name =
                                field.get("name").and_then(|n| n.as_str()).unwrap_or("");
                            if field_name.starts_with("__") {
                                continue;
                            }
                            templates.push(GraphqlQueryTemplate {
                                type_name: type_name.to_string(),
                                operation: "subscription".to_string(),
                                template: format!("subscription {{ {} }}", field_name),
                            });
                        }
                    }
                }
            }
        }

        Some(templates)
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_new() {
        let manager = GraphqlConnectionManager::new();
        assert!(manager.list_connections().is_empty());
    }

    #[test]
    fn test_connection_manager_new_shared() {
        let manager = GraphqlConnectionManager::new_shared();
        assert!(manager.list_connections().is_empty());
    }

    #[test]
    fn test_connection_manager_default() {
        let manager = GraphqlConnectionManager::default();
        assert!(manager.list_connections().is_empty());
    }

    #[tokio::test]
    async fn test_disconnect_nonexistent_connection() {
        let manager = GraphqlConnectionManager::new();
        let config = GraphqlDisconnectConfig::default();
        let result = manager.disconnect("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("does not exist"));
    }

    #[tokio::test]
    async fn test_shared_mode_disconnect_is_noop() {
        let manager = GraphqlConnectionManager::new_shared();
        let config = GraphqlDisconnectConfig::default();
        // 共享模式下 disconnect 不存在的连接也返回 Ok
        let result = manager.disconnect("non-existent", &config).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_status_nonexistent_connection() {
        let manager = GraphqlConnectionManager::new();
        assert!(manager.get_status("non-existent").is_none());
    }

    #[test]
    fn test_list_connections_empty() {
        let manager = GraphqlConnectionManager::new();
        let connections = manager.list_connections();
        assert!(connections.is_empty());
    }

    #[tokio::test]
    async fn test_query_nonexistent_connection() {
        let manager = GraphqlConnectionManager::new();
        let config = GraphqlQueryConfig::default();
        let result = manager.query("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("does not exist"));
    }

    #[tokio::test]
    async fn test_subscribe_nonexistent_connection() {
        let manager = GraphqlConnectionManager::new();
        let config = GraphqlSubscribeConfig::default();
        let result = manager.subscribe("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("does not exist"));
    }

    #[tokio::test]
    async fn test_introspect_nonexistent_connection() {
        let manager = GraphqlConnectionManager::new();
        let config = GraphqlIntrospectConfig::default();
        let result = manager.introspect("non-existent", &config).await;
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap_or_else(|| panic!("expected Err, got Ok"))
            .contains("does not exist"));
    }

    // ---- 状态显示测试 ----

    #[test]
    fn test_connection_status_display() {
        assert_eq!(GraphqlConnectionStatus::Connected.to_string(), "connected");
        assert_eq!(
            GraphqlConnectionStatus::Disconnected.to_string(),
            "disconnected"
        );
        let err_status = GraphqlConnectionStatus::Error("timeout".to_string());
        assert!(err_status.to_string().contains("timeout"));
    }

    // ---- 认证构建测试 ----

    #[test]
    fn test_build_auth_none() {
        let config = GraphqlConnectConfig {
            endpoint: "http://localhost/graphql".to_string(),
            ws_endpoint: None,
            headers: None,
            connection_timeout_ms: 0,
            connection_id: None,
            ..GraphqlConnectConfig::default()
        };
        let auth = GraphqlConnectionManager::build_auth(&config);
        assert_eq!(auth, GraphqlAuth::None);
    }

    #[test]
    fn test_build_auth_bearer() {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            "Bearer my-secret-token".to_string(),
        );
        let config = GraphqlConnectConfig {
            endpoint: "http://localhost/graphql".to_string(),
            ws_endpoint: None,
            headers: Some(headers),
            connection_timeout_ms: 0,
            connection_id: None,
            ..GraphqlConnectConfig::default()
        };
        let auth = GraphqlConnectionManager::build_auth(&config);
        assert_eq!(auth, GraphqlAuth::Bearer("my-secret-token".to_string()));
    }

    #[test]
    fn test_build_auth_api_key() {
        let mut headers = HashMap::new();
        headers.insert("X-API-Key".to_string(), "api-key-123".to_string());
        let config = GraphqlConnectConfig {
            endpoint: "http://localhost/graphql".to_string(),
            ws_endpoint: None,
            headers: Some(headers),
            connection_timeout_ms: 0,
            connection_id: None,
            ..GraphqlConnectConfig::default()
        };
        let auth = GraphqlConnectionManager::build_auth(&config);
        assert_eq!(
            auth,
            GraphqlAuth::ApiKey {
                name: "X-API-Key".to_string(),
                value: "api-key-123".to_string()
            }
        );
    }

    #[test]
    fn test_build_auth_cookie() {
        let mut headers = HashMap::new();
        headers.insert("Cookie".to_string(), "session=abc123".to_string());
        let config = GraphqlConnectConfig {
            endpoint: "http://localhost/graphql".to_string(),
            ws_endpoint: None,
            headers: Some(headers),
            connection_timeout_ms: 0,
            connection_id: None,
            ..GraphqlConnectConfig::default()
        };
        let auth = GraphqlConnectionManager::build_auth(&config);
        assert_eq!(auth, GraphqlAuth::Cookie("session=abc123".to_string()));
    }

    // ---- 模板生成测试 ----

    #[test]
    fn test_generate_templates_with_schema() {
        let schema = serde_json::json!({
            "__schema": {
                "queryType": {"name": "Query"},
                "mutationType": {"name": "Mutation"},
                "subscriptionType": {"name": "Subscription"},
                "types": [
                    {
                        "name": "Query",
                        "fields": [
                            {"name": "users"},
                            {"name": "user"},
                            {"name": "__internal"}
                        ]
                    },
                    {
                        "name": "Mutation",
                        "fields": [
                            {"name": "createUser"},
                            {"name": "deleteUser"}
                        ]
                    },
                    {
                        "name": "Subscription",
                        "fields": [
                            {"name": "messageAdded"},
                            {"name": "userOnline"}
                        ]
                    }
                ]
            }
        });

        let templates = GraphqlConnectionManager::generate_templates(&schema)
            .unwrap_or_else(|| panic!("expected templates to be generated"));

        // Query 模板（排除 __internal）
        assert_eq!(
            templates.iter().filter(|t| t.operation == "query").count(),
            2
        );
        // Mutation 模板
        assert_eq!(
            templates
                .iter()
                .filter(|t| t.operation == "mutation")
                .count(),
            2
        );
        // Subscription 模板
        assert_eq!(
            templates
                .iter()
                .filter(|t| t.operation == "subscription")
                .count(),
            2
        );

        // 验证跳过了 __internal
        assert!(templates.iter().all(|t| !t.template.contains("__internal")));
    }

    #[test]
    fn test_generate_templates_empty_schema() {
        let schema = serde_json::json!({});
        let result = GraphqlConnectionManager::generate_templates(&schema);
        assert!(result.is_none());
    }

    // ---- 连接管理器手动操作测试 ----

    #[tokio::test]
    async fn test_add_and_list_connection() {
        let manager = GraphqlConnectionManager::new();
        let conn = GraphqlConnection {
            http_endpoint: "http://localhost/graphql".to_string(),
            ws_endpoint: Some("ws://localhost/graphql".to_string()),
            headers: HashMap::new(),
            auth_config: GraphqlAuth::None,
            created_at: 1234567890,
        };

        // 手动添加连接
        {
            let mut conns = manager.connections.write().await;
            conns.insert("test-conn".to_string(), conn);
        }
        {
            let mut statuses = manager.statuses.write().await;
            statuses.insert("test-conn".to_string(), GraphqlConnectionStatus::Connected);
        }

        let connections = manager.list_connections();
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].connection_id, "test-conn");
        assert_eq!(connections[0].endpoint, "http://localhost/graphql");
        assert_eq!(connections[0].status, "connected");
    }

    #[tokio::test]
    async fn test_remove_connection() {
        let manager = GraphqlConnectionManager::new();
        let conn = GraphqlConnection {
            http_endpoint: "http://localhost/graphql".to_string(),
            ws_endpoint: None,
            headers: HashMap::new(),
            auth_config: GraphqlAuth::None,
            created_at: 0,
        };

        {
            let mut conns = manager.connections.write().await;
            conns.insert("test-conn".to_string(), conn);
        }
        {
            let mut statuses = manager.statuses.write().await;
            statuses.insert("test-conn".to_string(), GraphqlConnectionStatus::Connected);
        }

        assert_eq!(manager.list_connections().len(), 1);

        let config = GraphqlDisconnectConfig {
            connection_id: "test-conn".to_string(),
        };
        let result = manager.disconnect("test-conn", &config).await;
        assert!(result.is_ok());
        assert!(manager.list_connections().is_empty());
        assert!(manager.get_status("test-conn").is_none());
    }

    // ---- 连接复用测试 ----

    #[tokio::test]
    async fn test_connect_reuse_existing() {
        let manager = GraphqlConnectionManager::new();

        // 手动添加一个已连接状态
        let conn = GraphqlConnection {
            http_endpoint: "http://localhost/graphql".to_string(),
            ws_endpoint: None,
            headers: HashMap::new(),
            auth_config: GraphqlAuth::None,
            created_at: 0,
        };
        {
            let mut conns = manager.connections.write().await;
            conns.insert("reuse-conn".to_string(), conn);
        }
        {
            let mut statuses = manager.statuses.write().await;
            statuses.insert("reuse-conn".to_string(), GraphqlConnectionStatus::Connected);
        }

        // 尝试连接相同 UUID（不会真正发请求，因为检测到已存在）
        let config = GraphqlConnectConfig {
            endpoint: "http://localhost/graphql".to_string(),
            ws_endpoint: None,
            headers: None,
            connection_timeout_ms: 1000,
            connection_id: None,
            ..GraphqlConnectConfig::default()
        };
        let result = manager.connect("reuse-conn", &config).await;
        assert!(result.is_ok());
        let output = result.unwrap_or_else(|e| panic!("connection should succeed: {e}"));
        assert!(output.success);
    }
}
