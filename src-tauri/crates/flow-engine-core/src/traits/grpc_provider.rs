//! gRPC 提供者抽象 trait

use async_trait::async_trait;

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
    /// 计时信息
    pub timing: GrpcTimingInfo,
}

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

    /// gRPC Client Streaming 调用
    async fn call_client_streaming(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_messages: Vec<String>,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
    ) -> Result<serde_json::Value, String>;

    /// gRPC Bidi Streaming 调用
    async fn call_bidi_streaming(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_messages: Vec<String>,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
        collect_timeout_ms: u64,
    ) -> Result<GrpcStreamingResult, String>;

    /// 关闭 gRPC 连接
    async fn close(&self, connection_id: &str) -> Result<(), String>;

    /// 获取连接状态
    fn get_status(&self, connection_id: &str) -> Option<String>;

    /// 获取连接的可用服务列表
    fn list_services(&self, connection_id: &str) -> Option<Vec<GrpcServiceInfo>>;
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
            let json = serde_json::to_string(&pattern).expect("序列化应成功");
            let deserialized: GrpcPattern =
                serde_json::from_str(&json).expect("反序列化应成功");
            assert_eq!(pattern, deserialized, "GrpcPattern serde 往返失败: {:?}", pattern);
        }
    }

    #[test]
    fn test_grpc_method_info_default_client_streaming() {
        // is_client_streaming 有 #[serde(default)]，缺少时应默认为 false
        let json = r#"{"method_name":"M","input_type":"I","output_type":"O","is_server_streaming":true}"#;
        let info: GrpcMethodInfo = serde_json::from_str(json).expect("反序列化应成功");
        assert!(!info.is_client_streaming);
        assert!(info.is_server_streaming);
    }

    #[test]
    fn test_grpc_timing_info_serialization() {
        let timing = GrpcTimingInfo {
            total_ms: 150,
            first_response_ms: Some(50),
        };
        let json = serde_json::to_string(&timing).expect("序列化应成功");
        let deserialized: GrpcTimingInfo =
            serde_json::from_str(&json).expect("反序列化应成功");
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
            timing: GrpcTimingInfo {
                total_ms: 200,
                first_response_ms: Some(100),
            },
        };
        let json = serde_json::to_string(&result).expect("序列化应成功");
        let deserialized: GrpcStreamingResult =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(deserialized.status, 0);
        assert_eq!(deserialized.sent_count, 3);
        assert_eq!(deserialized.received_count, 1);
    }
}
