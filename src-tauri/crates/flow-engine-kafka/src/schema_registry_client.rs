//! Schema Registry HTTP 客户端
//!
//! 基于 reqwest 的 Confluent Schema Registry REST API 客户端。
//! 支持认证（Basic Auth + Bearer Token）和所有常用 API 端点。

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error类型（本地定义，后续与 KafkaError 合并）
// ---------------------------------------------------------------------------

/// Schema Registry 操作Error类型
#[derive(Debug, Error)]
pub enum SchemaRegistryError {
    /// Subject 不存在
    #[error("Subject not found: {0}")]
    SubjectNotFound(String),

    /// 版本不存在
    #[error("Version not found: {subject} version {version}")]
    VersionNotFound {
        /// Subject 名称
        subject: String,
        /// 版本号
        version: i32,
    },

    /// Schema 不兼容
    #[error("Incompatible schema for subject: {0}")]
    IncompatibleSchema(String),

    /// 无效 Schema
    #[error("Invalid schema: {0}")]
    InvalidSchema(String),

    /// HTTP 请求Failed
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    /// 响应解析Failed
    #[error("Failed to parse response: {0}")]
    ParseError(String),

    /// 认证Failed
    #[error("Authentication failed: {0}")]
    AuthError(String),

    /// 服务端Error
    #[error("Server error ({status}): {message}")]
    ServerError {
        /// HTTP 状态码
        status: u16,
        /// Error消息
        message: String,
    },
}

// ---------------------------------------------------------------------------
// 数据结构
// ---------------------------------------------------------------------------

/// Schema Registry 认证配置
#[derive(Debug, Clone)]
pub struct SchemaRegistryAuth {
    /// Basic Auth 用户名
    pub username: Option<String>,
    /// Basic Auth 密码
    pub password: Option<String>,
    /// Bearer Token
    pub bearer_token: Option<String>,
}

/// Schema 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    /// Subject 名称
    pub subject: String,
    /// Schema 版本号
    pub version: i32,
    /// Schema 全局 ID
    pub id: i32,
    /// Schema 类型（AVRO / JSON / PROTOBUF）
    #[serde(rename = "schemaType", skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<String>,
    /// Schema 定义字符串
    #[serde(rename = "schema")]
    pub schema: String,
}

/// 兼容性检查请求体
#[derive(Debug, Serialize)]
struct CompatibilityRequest {
    #[serde(rename = "schema")]
    schema: String,
    #[serde(rename = "schemaType", skip_serializing_if = "Option::is_none")]
    schema_type: Option<String>,
}

/// 兼容性检查响应
#[derive(Debug, Deserialize)]
struct CompatibilityResponse {
    #[serde(rename = "is_compatible")]
    is_compatible: bool,
}

/// 注册 Schema 响应
#[derive(Debug, Deserialize)]
struct RegisterResponse {
    id: i32,
}

/// 兼容性级别配置请求体
#[derive(Debug, Serialize)]
struct ConfigRequest {
    compatibility: String,
}

/// 兼容性级别配置响应
#[derive(Debug, Deserialize)]
struct ConfigResponse {
    compatibility: String,
}

/// Schema Registry 通用Error响应体
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    #[serde(default)]
    #[allow(dead_code)]
    error_code: i32,
    #[serde(default)]
    message: String,
}

// ---------------------------------------------------------------------------
// 客户端实现
// ---------------------------------------------------------------------------

/// Schema Registry HTTP 客户端
pub struct SchemaRegistryClient {
    /// API 基础 URL（如 `<http://localhost:8081>`）
    base_url: String,
    /// reqwest HTTP 客户端
    client: reqwest::Client,
    /// 认证配置
    auth: Option<SchemaRegistryAuth>,
}

impl SchemaRegistryClient {
    /// 确保 rustls 加密提供者已注册
    ///
    /// 当 `reqwest` 使用 `rustls-no-provider` feature 时，必须在构造 Client 之前初始化
    /// 一个加密提供者（如 `ring）。多次调用安全（install_default` 幂等）。
    fn ensure_crypto_provider() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    /// 创建新的 Schema Registry 客户端
    ///
    /// # 参数
    /// - `base_url`: Schema Registry 地址（如 `http://localhost:8081`）
    /// - `auth`: 可选认证配置
    pub fn new(
        base_url: &str,
        auth: Option<SchemaRegistryAuth>,
    ) -> Result<Self, SchemaRegistryError> {
        Self::ensure_crypto_provider();
        let client = reqwest::Client::builder().build().map_err(|e| {
            SchemaRegistryError::HttpError(format!("Failed to create HTTP client: {e}"))
        })?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
            auth,
        })
    }

    // -----------------------------------------------------------------------
    // API 方法
    // -----------------------------------------------------------------------

    /// 列出所有 Subject
    /// GET /subjects
    pub async fn list_subjects(&self) -> Result<Vec<String>, SchemaRegistryError> {
        let url = format!("{}/subjects", self.base_url);
        let response = self.get(&url).await?;
        self.handle_list_response(response).await
    }

    /// 获取指定 Subject 的所有版本号
    /// GET /subjects/{subject}/versions
    pub async fn get_subject_versions(
        &self,
        subject: &str,
    ) -> Result<Vec<i32>, SchemaRegistryError> {
        let url = format!("{}/subjects/{}/versions", self.base_url, subject);
        let response = self.get(&url).await?;
        self.handle_version_list_response(response, subject).await
    }

    /// 获取指定版本的 Schema
    /// GET /subjects/{subject}/versions/{version}
    pub async fn get_schema_by_version(
        &self,
        subject: &str,
        version: i32,
    ) -> Result<SchemaInfo, SchemaRegistryError> {
        let url = format!(
            "{}/subjects/{}/versions/{}",
            self.base_url, subject, version
        );
        let response = self.get(&url).await?;
        self.handle_schema_response(response, subject, Some(version))
            .await
    }

    /// 获取最新版本的 Schema
    /// GET /subjects/{subject}/versions/latest
    pub async fn get_latest_schema(
        &self,
        subject: &str,
    ) -> Result<SchemaInfo, SchemaRegistryError> {
        let url = format!("{}/subjects/{}/versions/latest", self.base_url, subject);
        let response = self.get(&url).await?;
        self.handle_schema_response(response, subject, None).await
    }

    /// 注册新 Schema
    /// POST /subjects/{subject}/versions
    ///
    /// 返回新注册 Schema 的全局 ID
    pub async fn register_schema(
        &self,
        subject: &str,
        schema: &str,
        schema_type: &str,
    ) -> Result<i32, SchemaRegistryError> {
        let url = format!("{}/subjects/{}/versions", self.base_url, subject);
        let body = CompatibilityRequest {
            schema: schema.to_string(),
            schema_type: Some(schema_type.to_string()),
        };
        let response = self.post_json(&url, &body).await?;
        self.handle_register_response(response, subject).await
    }

    /// 检查 Schema 兼容性
    /// POST /compatibility/subjects/{subject}/versions/{version}
    ///
    /// `version` 为 None 时使用 "latest"
    pub async fn check_compatibility(
        &self,
        subject: &str,
        schema: &str,
        schema_type: &str,
        version: Option<i32>,
    ) -> Result<bool, SchemaRegistryError> {
        let ver = version
            .map(|v| v.to_string())
            .unwrap_or_else(|| "latest".to_string());
        let url = format!(
            "{}/compatibility/subjects/{}/versions/{}",
            self.base_url, subject, ver
        );
        let body = CompatibilityRequest {
            schema: schema.to_string(),
            schema_type: Some(schema_type.to_string()),
        };
        let response = self.post_json(&url, &body).await?;
        self.handle_compatibility_response(response, subject).await
    }

    /// 获取 Subject 的兼容性级别配置
    /// GET /config/{subject}
    pub async fn get_compatibility(&self, subject: &str) -> Result<String, SchemaRegistryError> {
        let url = format!("{}/config/{}", self.base_url, subject);
        let response = self.get(&url).await?;
        self.handle_config_response(response).await
    }

    /// 设置 Subject 的兼容性级别
    /// PUT /config/{subject}
    pub async fn set_compatibility(
        &self,
        subject: &str,
        level: &str,
    ) -> Result<(), SchemaRegistryError> {
        let url = format!("{}/config/{}", self.base_url, subject);
        let body = ConfigRequest {
            compatibility: level.to_string(),
        };
        let response = self.put_json(&url, &body).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            Err(SchemaRegistryError::ServerError {
                status,
                message: text,
            })
        }
    }

    // -----------------------------------------------------------------------
    // 内部 HTTP 辅助方法
    // -----------------------------------------------------------------------

    /// 构建 GET 请求（含认证头）
    async fn get(&self, url: &str) -> Result<reqwest::Response, SchemaRegistryError> {
        let mut builder = self.client.get(url);
        builder = self.apply_auth(builder);
        builder
            .send()
            .await
            .map_err(|e| SchemaRegistryError::HttpError(format!("GET {} failed: {e}", url)))
    }

    /// 构建 POST JSON 请求（含认证头）
    async fn post_json<T: Serialize>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<reqwest::Response, SchemaRegistryError> {
        let mut builder = self.client.post(url).json(body);
        builder = self.apply_auth(builder);
        builder
            .send()
            .await
            .map_err(|e| SchemaRegistryError::HttpError(format!("POST {} failed: {e}", url)))
    }

    /// 构建 PUT JSON 请求（含认证头）
    async fn put_json<T: Serialize>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<reqwest::Response, SchemaRegistryError> {
        let mut builder = self.client.put(url).json(body);
        builder = self.apply_auth(builder);
        builder
            .send()
            .await
            .map_err(|e| SchemaRegistryError::HttpError(format!("PUT {} failed: {e}", url)))
    }

    /// 应用认证头到请求构建器
    fn apply_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let Some(auth) = &self.auth else {
            return builder;
        };

        // Bearer Token 优先
        if let Some(token) = &auth.bearer_token {
            return builder.bearer_auth(token);
        }

        // Basic Auth
        if let (Some(user), Some(pass)) = (&auth.username, &auth.password) {
            return builder.basic_auth(user, Some(pass));
        }

        builder
    }

    // -----------------------------------------------------------------------
    // 响应处理辅助
    // -----------------------------------------------------------------------

    /// 处理 Subject 列表响应
    async fn handle_list_response(
        &self,
        response: reqwest::Response,
    ) -> Result<Vec<String>, SchemaRegistryError> {
        let status = response.status();
        if status.is_success() {
            response.json::<Vec<String>>().await.map_err(|e| {
                SchemaRegistryError::ParseError(format!("Failed to parse subject list: {e}"))
            })
        } else if status.as_u16() == 404 {
            Err(SchemaRegistryError::SubjectNotFound(
                "(list all)".to_string(),
            ))
        } else {
            self.handle_error_response(status, response).await
        }
    }

    /// 处理版本列表响应
    async fn handle_version_list_response(
        &self,
        response: reqwest::Response,
        subject: &str,
    ) -> Result<Vec<i32>, SchemaRegistryError> {
        let status = response.status();
        if status.is_success() {
            response.json::<Vec<i32>>().await.map_err(|e| {
                SchemaRegistryError::ParseError(format!("Failed to parse version list: {e}"))
            })
        } else if status.as_u16() == 404 {
            Err(SchemaRegistryError::SubjectNotFound(subject.to_string()))
        } else {
            self.handle_error_response(status, response).await
        }
    }

    /// 处理 Schema 信息响应
    async fn handle_schema_response(
        &self,
        response: reqwest::Response,
        subject: &str,
        version: Option<i32>,
    ) -> Result<SchemaInfo, SchemaRegistryError> {
        let status = response.status();
        if status.is_success() {
            response.json::<SchemaInfo>().await.map_err(|e| {
                SchemaRegistryError::ParseError(format!("Failed to parse schema info: {e}"))
            })
        } else if status.as_u16() == 404 {
            match version {
                Some(v) => Err(SchemaRegistryError::VersionNotFound {
                    subject: subject.to_string(),
                    version: v,
                }),
                None => Err(SchemaRegistryError::SubjectNotFound(subject.to_string())),
            }
        } else {
            self.handle_error_response(status, response).await
        }
    }

    /// 处理注册 Schema 响应
    async fn handle_register_response(
        &self,
        response: reqwest::Response,
        subject: &str,
    ) -> Result<i32, SchemaRegistryError> {
        let status = response.status();
        if status.is_success() {
            let reg: RegisterResponse = response.json().await.map_err(|e| {
                SchemaRegistryError::ParseError(format!("Failed to parse register response: {e}"))
            })?;
            Ok(reg.id)
        } else if status.as_u16() == 409 {
            Err(SchemaRegistryError::IncompatibleSchema(subject.to_string()))
        } else if status.as_u16() == 422 {
            let text = response.text().await.unwrap_or_default();
            Err(SchemaRegistryError::InvalidSchema(text))
        } else {
            self.handle_error_response(status, response).await
        }
    }

    /// 处理兼容性检查响应
    async fn handle_compatibility_response(
        &self,
        response: reqwest::Response,
        subject: &str,
    ) -> Result<bool, SchemaRegistryError> {
        let status = response.status();
        if status.is_success() {
            let compat: CompatibilityResponse = response.json().await.map_err(|e| {
                SchemaRegistryError::ParseError(format!(
                    "Failed to parse compatibility response: {e}"
                ))
            })?;
            Ok(compat.is_compatible)
        } else if status.as_u16() == 404 {
            Err(SchemaRegistryError::SubjectNotFound(subject.to_string()))
        } else if status.as_u16() == 409 {
            Err(SchemaRegistryError::IncompatibleSchema(subject.to_string()))
        } else {
            self.handle_error_response(status, response).await
        }
    }

    /// 处理配置响应
    async fn handle_config_response(
        &self,
        response: reqwest::Response,
    ) -> Result<String, SchemaRegistryError> {
        let status = response.status();
        if status.is_success() {
            let config: ConfigResponse = response.json().await.map_err(|e| {
                SchemaRegistryError::ParseError(format!("Failed to parse config response: {e}"))
            })?;
            Ok(config.compatibility)
        } else {
            self.handle_error_response(status, response).await
        }
    }

    /// 通用Error响应处理 — 根据 HTTP 状态码映射到具体Error类型
    async fn handle_error_response<T>(
        &self,
        status: reqwest::StatusCode,
        response: reqwest::Response,
    ) -> Result<T, SchemaRegistryError> {
        let status_code = status.as_u16();
        let text = response.text().await.unwrap_or_default();

        // 尝试解析Error响应体以获取更详细信息
        let error_message = serde_json::from_str::<ErrorResponse>(&text)
            .map(|e| e.message)
            .unwrap_or(text.clone());

        match status_code {
            401 => Err(SchemaRegistryError::AuthError(error_message)),
            403 => Err(SchemaRegistryError::AuthError(error_message)),
            404 => Err(SchemaRegistryError::SubjectNotFound(error_message)),
            409 => Err(SchemaRegistryError::IncompatibleSchema(error_message)),
            422 => Err(SchemaRegistryError::InvalidSchema(error_message)),
            _ => Err(SchemaRegistryError::ServerError {
                status: status_code,
                message: error_message,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助：创建指向 mockito mock 服务器的客户端（无认证）
    fn create_client(server: &mockito::ServerGuard) -> SchemaRegistryClient {
        SchemaRegistryClient::new(&server.url(), None).expect("Failed to create client")
    }

    /// 辅助：创建带 Basic Auth 的客户端
    fn create_client_with_basic_auth(server: &mockito::ServerGuard) -> SchemaRegistryClient {
        SchemaRegistryClient::new(
            &server.url(),
            Some(SchemaRegistryAuth {
                username: Some("user".to_string()),
                password: Some("pass".to_string()),
                bearer_token: None,
            }),
        )
        .expect("Failed to create client")
    }

    // ===== 列出所有 Subject =====

    #[tokio::test]
    async fn test_list_subjects() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/subjects")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"["subject1", "subject2"]"#)
            .create_async()
            .await;

        let client = create_client(&server);
        let subjects = client.list_subjects().await.expect("list_subjects failed");

        assert_eq!(subjects, vec!["subject1", "subject2"]);
        mock.assert_async().await;
    }

    // ===== 获取 Subject 版本列表 =====

    #[tokio::test]
    async fn test_get_subject_versions() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/subjects/test/versions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("[1, 2, 3]")
            .create_async()
            .await;

        let client = create_client(&server);
        let versions = client
            .get_subject_versions("test")
            .await
            .expect("get_subject_versions failed");

        assert_eq!(versions, vec![1, 2, 3]);
        mock.assert_async().await;
    }

    // ===== 获取指定版本 Schema =====

    #[tokio::test]
    async fn test_get_schema_by_version() {
        let mut server = mockito::Server::new_async().await;
        let body = serde_json::json!({
            "subject": "test",
            "version": 1,
            "id": 42,
            "schemaType": "AVRO",
            "schema": "{\"type\":\"record\",\"name\":\"Test\"}"
        });
        let mock = server
            .mock("GET", "/subjects/test/versions/1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let client = create_client(&server);
        let schema = client
            .get_schema_by_version("test", 1)
            .await
            .expect("get_schema_by_version failed");

        assert_eq!(schema.subject, "test");
        assert_eq!(schema.version, 1);
        assert_eq!(schema.id, 42);
        assert_eq!(schema.schema_type.as_deref(), Some("AVRO"));
        mock.assert_async().await;
    }

    // ===== 获取最新版本 Schema =====

    #[tokio::test]
    async fn test_get_latest_schema() {
        let mut server = mockito::Server::new_async().await;
        let body = serde_json::json!({
            "subject": "test",
            "version": 3,
            "id": 100,
            "schema": "{\"type\":\"string\"}"
        });
        let mock = server
            .mock("GET", "/subjects/test/versions/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let client = create_client(&server);
        let schema = client
            .get_latest_schema("test")
            .await
            .expect("get_latest_schema failed");

        assert_eq!(schema.version, 3);
        assert_eq!(schema.id, 100);
        mock.assert_async().await;
    }

    // ===== 注册 Schema =====

    #[tokio::test]
    async fn test_register_schema() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/subjects/test/versions")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id": 42}"#)
            .create_async()
            .await;

        let client = create_client(&server);
        let id = client
            .register_schema("test", r#"{"type":"string"}"#, "AVRO")
            .await
            .expect("register_schema failed");

        assert_eq!(id, 42);
        mock.assert_async().await;
    }

    // ===== 兼容性检查 =====

    #[tokio::test]
    async fn test_check_compatibility() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/compatibility/subjects/test/versions/latest")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"is_compatible": true}"#)
            .create_async()
            .await;

        let client = create_client(&server);
        let compatible = client
            .check_compatibility("test", r#"{"type":"string"}"#, "AVRO", None)
            .await
            .expect("check_compatibility failed");

        assert!(compatible);
        mock.assert_async().await;
    }

    // ===== 兼容性检查 — 指定版本号 =====

    #[tokio::test]
    async fn test_check_compatibility_with_version() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/compatibility/subjects/test/versions/2")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"is_compatible": false}"#)
            .create_async()
            .await;

        let client = create_client(&server);
        let compatible = client
            .check_compatibility("test", r#"{"type":"int"}"#, "AVRO", Some(2))
            .await
            .expect("check_compatibility failed");

        assert!(!compatible);
        mock.assert_async().await;
    }

    // ===== 获取兼容性配置 =====

    #[tokio::test]
    async fn test_get_compatibility() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/config/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"compatibility": "BACKWARD"}"#)
            .create_async()
            .await;

        let client = create_client(&server);
        let level = client
            .get_compatibility("test")
            .await
            .expect("get_compatibility failed");

        assert_eq!(level, "BACKWARD");
        mock.assert_async().await;
    }

    // ===== 设置兼容性配置 =====

    #[tokio::test]
    async fn test_set_compatibility() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("PUT", "/config/test")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"compatibility": "FULL"}"#)
            .create_async()
            .await;

        let client = create_client(&server);
        client
            .set_compatibility("test", "FULL")
            .await
            .expect("set_compatibility failed");

        mock.assert_async().await;
    }

    // ===== Basic Auth =====

    #[tokio::test]
    async fn test_auth_basic() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/subjects")
            .match_header("authorization", "Basic dXNlcjpwYXNz") // base64("user:pass")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("[]")
            .create_async()
            .await;

        let client = create_client_with_basic_auth(&server);
        let subjects = client.list_subjects().await.expect("list_subjects failed");
        assert!(subjects.is_empty());
        mock.assert_async().await;
    }

    // ===== Bearer Token Auth =====

    #[tokio::test]
    async fn test_auth_bearer() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/subjects")
            .match_header("authorization", "Bearer my-token-123")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("[]")
            .create_async()
            .await;

        let client = SchemaRegistryClient::new(
            &server.url(),
            Some(SchemaRegistryAuth {
                username: Some("user".to_string()),
                password: Some("pass".to_string()),
                bearer_token: Some("my-token-123".to_string()),
            }),
        )
        .expect("Failed to create client");

        let subjects = client.list_subjects().await.expect("list_subjects failed");
        assert!(subjects.is_empty());
        mock.assert_async().await;
    }

    // ===== 404 Error → SubjectNotFound =====

    #[tokio::test]
    async fn test_error_404() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/subjects/nonexistent/versions")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error_code":40401,"message":"Subject not found"}"#)
            .create_async()
            .await;

        let client = create_client(&server);
        let result = client.get_subject_versions("nonexistent").await;

        match result {
            Err(SchemaRegistryError::SubjectNotFound(msg)) => {
                assert!(
                    msg.contains("nonexistent"),
                    "Expected SubjectNotFound for subject, got: {msg}"
                );
            }
            other => panic!("Expected SubjectNotFound, got: {other:?}"),
        }
        mock.assert_async().await;
    }

    // ===== 409 Error → IncompatibleSchema =====

    #[tokio::test]
    async fn test_error_409() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/subjects/test/versions")
            .with_status(409)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error_code":409,"message":"Schema incompatible"}"#)
            .create_async()
            .await;

        let client = create_client(&server);
        let result = client
            .register_schema("test", r#"{"type":"int"}"#, "AVRO")
            .await;

        match result {
            Err(SchemaRegistryError::IncompatibleSchema(msg)) => {
                assert!(
                    msg.contains("incompatible") || msg.contains("test"),
                    "Expected IncompatibleSchema with message, got: {msg}"
                );
            }
            other => panic!("Expected IncompatibleSchema, got: {other:?}"),
        }
        mock.assert_async().await;
    }

    // ===== 422 Error → InvalidSchema =====

    #[tokio::test]
    async fn test_error_422() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/subjects/test/versions")
            .with_status(422)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error_code":42201,"message":"Invalid schema"}"#)
            .create_async()
            .await;

        let client = create_client(&server);
        let result = client
            .register_schema("test", "not-valid-json", "AVRO")
            .await;

        match result {
            Err(SchemaRegistryError::InvalidSchema(msg)) => {
                assert!(
                    msg.contains("Invalid schema"),
                    "Expected InvalidSchema with message, got: {msg}"
                );
            }
            other => panic!("Expected InvalidSchema, got: {other:?}"),
        }
        mock.assert_async().await;
    }

    // ===== 401 Error → AuthError =====

    #[tokio::test]
    async fn test_error_401() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/subjects")
            .with_status(401)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error_code":401,"message":"Unauthorized"}"#)
            .create_async()
            .await;

        let client = create_client(&server);
        let result = client.list_subjects().await;

        match result {
            Err(SchemaRegistryError::AuthError(msg)) => {
                assert!(
                    msg.contains("Unauthorized"),
                    "Expected AuthError with message, got: {msg}"
                );
            }
            other => panic!("Expected AuthError, got: {other:?}"),
        }
        mock.assert_async().await;
    }

    // ===== URL 尾部斜杠处理 =====

    #[test]
    fn test_base_url_trailing_slash_stripped() {
        let client = SchemaRegistryClient::new("http://localhost:8081/", None)
            .expect("Failed to create client");
        assert_eq!(client.base_url, "http://localhost:8081");
    }

    // ===== 无认证时不发送 Authorization 头 =====

    #[tokio::test]
    async fn test_no_auth_header_when_none() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/subjects")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("[]")
            .expect(1)
            .create_async()
            .await;

        let client = create_client(&server);
        client.list_subjects().await.expect("should succeed");
        mock.assert_async().await;
    }
}
