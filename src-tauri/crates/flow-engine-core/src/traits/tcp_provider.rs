//! TCP 提供者抽象 trait

use async_trait::async_trait;

/// TCP 帧定界模式
///
/// 定义 TCP 数据流的分帧策略，用于将连续的字节流分割为独立的消息。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TcpFramingMode {
    /// 长度前缀模式：消息前 N 字节表示消息体长度
    LengthPrefix {
        /// 长度前缀字节数（1/2/4/8）
        prefix_bytes: u8,
    },
    /// 分隔符模式：以特定字符/字符串作为消息边界
    Delimiter {
        /// 分隔符内容（如 "\n"、"\r\n"）
        delimiter: String,
    },
    /// 固定长度模式：每条消息长度固定
    FixedLength {
        /// 固定消息长度
        length: usize,
    },
    /// 超时模式：在指定时间内收到的数据视为一条消息
    Timeout {
        /// 超时时间（毫秒）
        timeout_ms: u64,
    },
    /// 手动模式：不自动分帧，由用户自行处理
    Manual,
}

impl Default for TcpFramingMode {
    fn default() -> Self {
        Self::LengthPrefix { prefix_bytes: 4 }
    }
}

/// TCP 帧配置
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FramingConfig {
    /// 帧定界模式
    pub mode: TcpFramingMode,
    /// 最大消息长度（字节），超过此长度的消息将被截断或报错
    #[serde(default = "default_max_frame_size")]
    pub max_frame_size: usize,
    /// 是否包含前缀/分隔符在消息内容中
    #[serde(default)]
    pub include_framing_in_data: bool,
}

fn default_max_frame_size() -> usize {
    1024 * 1024 // 1MB
}

impl Default for FramingConfig {
    fn default() -> Self {
        Self {
            mode: TcpFramingMode::LengthPrefix { prefix_bytes: 4 },
            max_frame_size: default_max_frame_size(),
            include_framing_in_data: false,
        }
    }
}

/// TCP 连接信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TcpConnectionInfo {
    /// 连接唯一标识
    pub uuid: String,
    /// 连接名称
    pub name: String,
    /// 连接状态
    pub status: String,
    /// 目标主机地址
    pub host: String,
    /// 目标端口
    pub port: u16,
    /// 创建时间（Unix 时间戳，毫秒）
    pub created_at: u64,
    /// 是否使用 TLS
    pub use_tls: bool,
    /// 最近一次Error信息
    pub last_error: Option<String>,
}

/// TCP 提供者 trait
///
/// 抽象 TCP 连接管理功能，使得执行器可以与具体的 TCP 实现解耦。
#[async_trait]
pub trait TcpProvider: Send + Sync {
    /// 建立 TCP 连接
    ///
    /// # 参数
    /// - `uuid` - 连接唯一标识（使用节点 UUID）
    /// - `name` - 连接名称
    /// - `host` - 目标主机地址
    /// - `port` - 目标端口
    /// - `use_tls` - 是否使用 TLS 加密
    /// - `tls_skip_verify` - 是否跳过 TLS 证书验证
    /// - `connect_timeout_ms` - 连接超时（毫秒）
    /// - `framing_config` - 帧定界配置
    async fn connect(
        &self,
        uuid: String,
        name: String,
        host: String,
        port: u16,
        use_tls: bool,
        tls_skip_verify: bool,
        connect_timeout_ms: u64,
        framing_config: FramingConfig,
    ) -> Result<(), String>;

    /// 发送数据
    ///
    /// # 参数
    /// - `uuid` - 连接唯一标识
    /// - `data` - 要发送的数据（二进制）
    /// - `send_timeout_ms` - 发送超时（毫秒）
    async fn send(&self, uuid: &str, data: Vec<u8>, send_timeout_ms: u64) -> Result<(), String>;

    /// 接收数据
    ///
    /// # 参数
    /// - `uuid` - 连接唯一标识
    /// - `receive_timeout_ms` - 接收超时（毫秒）
    ///
    /// # 返回
    /// Success时返回接收到的数据，超时或无数据时返回 None
    async fn receive(&self, uuid: &str, receive_timeout_ms: u64)
        -> Result<Option<Vec<u8>>, String>;

    /// 关闭连接
    ///
    /// # 参数
    /// - `uuid` - 连接唯一标识
    /// - `close_timeout_ms` - 关闭超时（毫秒）
    async fn close(&self, uuid: &str, close_timeout_ms: u64) -> Result<(), String>;

    /// 获取连接状态
    ///
    /// 返回连接状态的字符串表示（如 "Connected"、"Closed" 等），如果连接不存在则返回 None
    fn get_status(&self, uuid: &str) -> Option<String>;

    /// 列出所有连接信息
    fn list_connections(&self) -> Vec<TcpConnectionInfo>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framing_mode_length_prefix_serde() {
        let mode = TcpFramingMode::LengthPrefix { prefix_bytes: 4 };
        let json = serde_json::to_string(&mode).expect("序列化应Success");
        let deserialized: TcpFramingMode = serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(mode, deserialized);
    }

    #[test]
    fn test_framing_mode_delimiter_serde() {
        let mode = TcpFramingMode::Delimiter {
            delimiter: "\r\n".to_string(),
        };
        let json = serde_json::to_string(&mode).expect("序列化应Success");
        let deserialized: TcpFramingMode = serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(mode, deserialized);
    }

    #[test]
    fn test_framing_mode_fixed_length_serde() {
        let mode = TcpFramingMode::FixedLength { length: 256 };
        let json = serde_json::to_string(&mode).expect("序列化应Success");
        let deserialized: TcpFramingMode = serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(mode, deserialized);
    }

    #[test]
    fn test_framing_mode_timeout_serde() {
        let mode = TcpFramingMode::Timeout { timeout_ms: 5000 };
        let json = serde_json::to_string(&mode).expect("序列化应Success");
        let deserialized: TcpFramingMode = serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(mode, deserialized);
    }

    #[test]
    fn test_framing_mode_manual_serde() {
        let mode = TcpFramingMode::Manual;
        let json = serde_json::to_string(&mode).expect("序列化应Success");
        let deserialized: TcpFramingMode = serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(mode, deserialized);
    }

    #[test]
    fn test_framing_config_default() {
        let config = FramingConfig::default();
        assert!(matches!(
            config.mode,
            TcpFramingMode::LengthPrefix { prefix_bytes: 4 }
        ));
        assert_eq!(config.max_frame_size, 1024 * 1024);
        assert!(!config.include_framing_in_data);
    }

    #[test]
    fn test_framing_config_serde_roundtrip() {
        let config = FramingConfig {
            mode: TcpFramingMode::Delimiter {
                delimiter: "\n".to_string(),
            },
            max_frame_size: 4096,
            include_framing_in_data: true,
        };
        let json = serde_json::to_string(&config).expect("序列化应Success");
        let deserialized: FramingConfig = serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(config.max_frame_size, deserialized.max_frame_size);
        assert_eq!(
            config.include_framing_in_data,
            deserialized.include_framing_in_data
        );
        assert_eq!(config.mode, deserialized.mode);
    }

    #[test]
    fn test_tcp_connection_info_serialization() {
        let info = TcpConnectionInfo {
            uuid: "conn-001".to_string(),
            name: "测试连接".to_string(),
            status: "Connected".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8080,
            created_at: 1700000000000,
            use_tls: false,
            last_error: None,
        };
        let json = serde_json::to_string(&info).expect("序列化应Success");
        let deserialized: TcpConnectionInfo =
            serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(deserialized.uuid, "conn-001");
        assert_eq!(deserialized.host, "127.0.0.1");
        assert_eq!(deserialized.port, 8080);
        assert!(!deserialized.use_tls);
        assert!(deserialized.last_error.is_none());
    }

    #[test]
    fn test_tcp_connection_info_with_error() {
        let info = TcpConnectionInfo {
            uuid: "conn-002".to_string(),
            name: "Error连接".to_string(),
            status: "Error".to_string(),
            host: "192.168.1.1".to_string(),
            port: 9090,
            created_at: 1700000000000,
            use_tls: true,
            last_error: Some("连接被拒绝".to_string()),
        };
        let json = serde_json::to_string(&info).expect("序列化应Success");
        let deserialized: TcpConnectionInfo =
            serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(deserialized.last_error, Some("连接被拒绝".to_string()));
        assert!(deserialized.use_tls);
    }

    #[test]
    fn test_framing_config_missing_optional_fields() {
        // 仅提供 mode，其余使用默认值
        let json = r#"{"mode":{"length_prefix":{"prefix_bytes":2}}}"#;
        let config: FramingConfig = serde_json::from_str(json).expect("反序列化应Success");
        assert!(matches!(
            config.mode,
            TcpFramingMode::LengthPrefix { prefix_bytes: 2 }
        ));
        assert_eq!(config.max_frame_size, 1024 * 1024);
        assert!(!config.include_framing_in_data);
    }
}
