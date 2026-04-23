//! gRPC 提供者抽象 trait

use async_trait::async_trait;

/// gRPC 服务信息（service + method 列表）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrpcServiceInfo {
    /// 服务全名（如 "package.ServiceName"）
    pub service_name: String,
    /// 方法列表
    pub methods: Vec<GrpcMethodInfo>,
}

/// gRPC 方法信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrpcMethodInfo {
    /// 方法名
    pub method_name: String,
    /// 输入类型全名
    pub input_type: String,
    /// 输出类型全名
    pub output_type: String,
    /// 是否为服务端流式
    pub is_server_streaming: bool,
}

/// gRPC 连接信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrpcConnectionInfo {
    /// 连接 ID
    pub connection_id: String,
    /// 连接 URL
    pub url: String,
    /// 连接状态
    pub status: String,
    /// 可用服务列表
    pub services: Vec<GrpcServiceInfo>,
}

/// gRPC 提供者 trait
///
/// 抽象 gRPC 连接管理功能，使得执行器可以与具体的 gRPC 实现解耦。
#[async_trait]
pub trait GrpcProvider: Send + Sync {
    /// 建立 gRPC 连接
    ///
    /// # 参数
    /// - `uuid` - 连接唯一标识（使用节点 UUID）
    /// - `url` - gRPC 服务器地址
    /// - `use_tls` - 是否使用 TLS
    /// - `tls_skip_verify` - 是否跳过 TLS 证书验证
    /// - `enable_reflection` - 是否启用 Server Reflection
    /// - `proto_files` - proto 文件内容列表（当不使用 Reflection 时）
    /// - `metadata` - 连接级默认 metadata（key-value）
    /// - `connect_timeout_ms` - 连接超时（毫秒）
    async fn connect(
        &self,
        uuid: String,
        url: String,
        use_tls: bool,
        tls_skip_verify: bool,
        enable_reflection: bool,
        proto_files: Vec<(String, String)>, // (filename, content)
        metadata: Vec<(String, String)>,     // (key, value)
        connect_timeout_ms: u64,
    ) -> Result<Vec<GrpcServiceInfo>, String>;

    /// gRPC Unary 调用
    async fn call_unary(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_json: &str,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
    ) -> Result<serde_json::Value, String>;

    /// gRPC Server Streaming 调用
    async fn call_server_streaming(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_json: &str,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
    ) -> Result<Vec<serde_json::Value>, String>;

    /// 关闭 gRPC 连接
    async fn close(&self, connection_id: &str) -> Result<(), String>;

    /// 获取连接状态
    fn get_status(&self, connection_id: &str) -> Option<String>;

    /// 获取连接的可用服务列表
    fn list_services(&self, connection_id: &str) -> Option<Vec<GrpcServiceInfo>>;
}
