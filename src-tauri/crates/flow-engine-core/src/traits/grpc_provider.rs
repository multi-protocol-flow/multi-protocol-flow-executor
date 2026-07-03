//! gRPC 提供者抽象 trait

use async_trait::async_trait;
use crate::traits::DebugEventSender;
use std::sync::Arc;

/// gRPC 通信模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrpcPattern {
    /// Unary（一问一答）
    Unary,
    /// Server Streaming（一问多答）
    ServerStreaming,
    /// Client Streaming（多问一答）
    ClientStreaming,
    /// Bidi Streaming（多问多答）
    BidiStreaming,
}

impl GrpcPattern {
    /// 根据方法信息推断通信模式
    pub fn from_method_info(is_client_streaming: bool, is_server_streaming: bool) -> Self {
        match (is_client_streaming, is_server_streaming) {
            (false, false) => GrpcPattern::Unary,
            (false, true) => GrpcPattern::ServerStreaming,
            (true, false) => GrpcPattern::ClientStreaming,
            (true, true) => GrpcPattern::BidiStreaming,
        }
    }
}

/// gRPC 计时信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrpcTimingInfo {
    /// 总耗时（毫秒）
    pub total_ms: u64,
    /// 首个响应耗时（毫秒）
    pub first_response_ms: Option<u64>,
}

/// gRPC 流式结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrpcStreamingResult {
    /// gRPC 状态码
    pub status: i32,
    /// 响应列表
    pub responses: Vec<serde_json::Value>,
    /// 已发送消息数
    pub sent_count: usize,
    /// 已接收消息数
    pub received_count: usize,
    /// Trailers metadata
    pub trailers: std::collections::HashMap<String, String>,
    /// 响应 Initial Metadata（从 tonic Response 提取的初始 metadata）
    #[serde(default)]
    pub response_metadata: Vec<(String, String)>,
    /// 计时信息
    pub timing: GrpcTimingInfo,
    /// 是否被用户取消
    #[serde(default)]
    pub cancelled: bool,
}

/// gRPC 服务信息（service + method 列表）
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GrpcServiceInfo {
    /// 服务全名（如 "package.ServiceName"）
    pub service_name: String,
    /// 方法列表
    pub methods: Vec<GrpcMethodInfo>,
}

/// gRPC 方法信息
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GrpcMethodInfo {
    /// 方法名
    pub method_name: String,
    /// 输入类型全名
    pub input_type: String,
    /// 输出类型全名
    pub output_type: String,
    /// 是否为服务端流式
    pub is_server_streaming: bool,
    /// 是否为客户端流式
    #[serde(default)]
    pub is_client_streaming: bool,
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

/// gRPC 流式消息（用于实时流式传递）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrpcStreamMessage {
    /// 消息序号（从 1 开始）
    pub index: usize,
    /// 消息数据
    pub data: serde_json::Value,
    /// 接收时间戳（毫秒）
    pub received_at_ms: u64,
    /// 消息大小（字节）
    pub size_bytes: usize,
}

/// 健康检查响应
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthCheckResponse {
    /// 服务状态："UNKNOWN", "SERVING", "`NOT_SERVING`", "`NOT_IMPLEMENTED`"
    pub status: String,
}

/// gRPC Error详情
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrpcErrorDetail {
    /// Error详情类型 URL
    pub type_url: String,
    /// 解码后的Error详情
    pub decoded: Option<serde_json::Value>,
    /// 原始十六进制数据
    pub raw_hex: Option<String>,
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
    /// - `tls_ca_cert` - 自定义 CA 证书文件路径（可选）
    /// - `tls_client_cert` - 客户端证书文件路径（可选，mTLS）
    /// - `tls_client_key` - 客户端私钥文件路径（可选，mTLS）
    /// - `tls_server_name_override` - TLS SNI 覆盖（可选）
    /// - `reflection_metadata` - 反射请求附加 metadata（用于认证的反射服务）
    /// - `compression_encoding` - 压缩编码："gzip" 或 None
    /// - `keepalive_time_ms` - Keepalive PING 间隔（毫秒，可选）
    /// - `keepalive_timeout_ms` - Keepalive PING 超时（毫秒，可选）
    /// - `keepalive_permit_without_streams` - 无活跃流时是否允许 PING（可选）
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
        tls_ca_cert: Option<String>,
        tls_client_cert: Option<String>,
        tls_client_key: Option<String>,
        tls_server_name_override: Option<String>,
        reflection_metadata: Vec<(String, String)>, // 反射请求附加 metadata
        compression_encoding: Option<String>,       // 压缩编码："gzip" 或 None
        keepalive_time_ms: Option<u64>,             // Keepalive PING 间隔
        keepalive_timeout_ms: Option<u64>,          // Keepalive PING 超时
        keepalive_permit_without_streams: Option<bool>, // 无流时允许 PING
        endpoints: Option<Vec<String>>,             // 多端点列表（负载均衡）
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
        compression_encoding: Option<String>, // 调用级压缩覆盖（None 表示使用连接级默认）
        debug_sender: Option<Arc<dyn DebugEventSender>>,
        node_uuid: &str,
        call_id: &str,
    ) -> Result<(serde_json::Value, Vec<(String, String)>), String>;

    /// gRPC Server Streaming 调用
    async fn call_server_streaming(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_json: &str,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
        compression_encoding: Option<String>, // 调用级压缩覆盖（None 表示使用连接级默认）
        debug_sender: Option<Arc<dyn DebugEventSender>>,
        node_uuid: &str,
        call_id: &str,
    ) -> Result<(Vec<serde_json::Value>, Vec<(String, String)>), String>;

    /// gRPC Client Streaming 调用
    async fn call_client_streaming(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_messages: Vec<String>,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
        compression_encoding: Option<String>, // 调用级压缩覆盖（None 表示使用连接级默认）
        debug_sender: Option<Arc<dyn DebugEventSender>>,
        node_uuid: &str,
        call_id: &str,
    ) -> Result<(serde_json::Value, Vec<(String, String)>), String>;

    /// gRPC Bidi Streaming 调用
    async fn call_bidi_streaming(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_messages: Vec<String>,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
        compression_encoding: Option<String>, // 调用级压缩覆盖（None 表示使用连接级默认）
        debug_sender: Option<Arc<dyn DebugEventSender>>,
        node_uuid: &str,
        call_id: &str,
    ) -> Result<GrpcStreamingResult, String>;

    /// 关闭 gRPC 连接
    async fn close(&self, connection_id: &str) -> Result<(), String>;

    /// 获取连接状态
    fn get_status(&self, connection_id: &str) -> Option<String>;

    /// 获取连接的可用服务列表
    fn list_services(&self, connection_id: &str) -> Option<Vec<GrpcServiceInfo>>;

    /// 健康检查（gRPC Health Checking Protocol unary Check）
    async fn health_check(
        &self,
        connection_id: &str,
        service: &str,
    ) -> Result<HealthCheckResponse, String>;

    /// 取消进行中的流式调用
    fn cancel_stream(&self, connection_id: &str, call_id: &str) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_pattern_from_method_info_unary() {
        let pattern = GrpcPattern::from_method_info(false, false);
        assert_eq!(pattern, GrpcPattern::Unary);
    }

    #[test]
    fn test_grpc_pattern_from_method_info_server_streaming() {
        let pattern = GrpcPattern::from_method_info(false, true);
        assert_eq!(pattern, GrpcPattern::ServerStreaming);
    }

    #[test]
    fn test_grpc_pattern_from_method_info_client_streaming() {
        let pattern = GrpcPattern::from_method_info(true, false);
        assert_eq!(pattern, GrpcPattern::ClientStreaming);
    }

    #[test]
    fn test_grpc_pattern_from_method_info_bidi_streaming() {
        let pattern = GrpcPattern::from_method_info(true, true);
        assert_eq!(pattern, GrpcPattern::BidiStreaming);
    }

    #[test]
    fn test_grpc_pattern_serde_roundtrip() {
        for pattern in [
            GrpcPattern::Unary,
            GrpcPattern::ServerStreaming,
            GrpcPattern::ClientStreaming,
            GrpcPattern::BidiStreaming,
        ] {
            let json = serde_json::to_string(&pattern).expect("序列化应Success");
            let deserialized: GrpcPattern =
                serde_json::from_str(&json).expect("反序列化应Success");
            assert_eq!(pattern, deserialized, "GrpcPattern serde 往返Failed: {:?}", pattern);
        }
    }

    #[test]
    fn test_grpc_method_info_default_client_streaming() {
        // is_client_streaming 有 #[serde(default)]，缺少时应默认为 false
        let json = r#"{"method_name":"M","input_type":"I","output_type":"O","is_server_streaming":true}"#;
        let info: GrpcMethodInfo = serde_json::from_str(json).expect("反序列化应Success");
        assert!(!info.is_client_streaming);
        assert!(info.is_server_streaming);
    }

    #[test]
    fn test_grpc_timing_info_serialization() {
        let timing = GrpcTimingInfo {
            total_ms: 150,
            first_response_ms: Some(50),
        };
        let json = serde_json::to_string(&timing).expect("序列化应Success");
        let deserialized: GrpcTimingInfo =
            serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(deserialized.total_ms, 150);
        assert_eq!(deserialized.first_response_ms, Some(50));
    }

    #[test]
    fn test_grpc_streaming_result_serialization() {
        let result = GrpcStreamingResult {
            status: 0,
            responses: vec![serde_json::json!({"key": "value"})],
            sent_count: 3,
            received_count: 1,
            trailers: std::collections::HashMap::new(),
            response_metadata: vec![],
            timing: GrpcTimingInfo {
                total_ms: 200,
                first_response_ms: Some(100),
            },
            cancelled: false,
        };
        let json = serde_json::to_string(&result).expect("序列化应Success");
        let deserialized: GrpcStreamingResult =
            serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(deserialized.status, 0);
        assert_eq!(deserialized.sent_count, 3);
        assert_eq!(deserialized.received_count, 1);
    }
}
