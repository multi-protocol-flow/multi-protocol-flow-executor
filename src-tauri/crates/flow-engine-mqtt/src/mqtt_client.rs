//! MQTT 客户端封装层
//!
//! 封装 rumqttc-next SDK，提供 MQTT 5.0 客户端创建功能。
//! 支持 TCP、TLS (rustls)、WebSocket、WSS 四种传输方式。
//! 所有 rumqttc 类型保持在模块内部，通过 [`MqttClientResult`] 暴露必要字段。

use std::sync::Arc;
use std::time::Duration;

use flow_engine_core::node_config::MqttConnectConfig;
use log::info;
use rumqttc::{AsyncClient, Broker, EventLoop, LastWill, LastWillProperties, MqttOptions, QoS, TlsConfiguration, Transport};

// ---------------------------------------------------------------------------
// 公共类型
// ---------------------------------------------------------------------------

/// MQTT 客户端创建结果
///
/// 包含异步客户端和事件循环，供连接管理器在后台 poll。
pub struct MqttClientResult {
    /// 异步客户端（用于发布/订阅操作）
    pub client: AsyncClient,
    /// 事件循环（需由 `connection_manager` 在后台 poll）
    pub eventloop: EventLoop,
    /// Broker 版本信息（连接后从 CONNACK 获取，初始为 None）
    pub broker_version: Option<String>,
}

/// 根据连接配置创建 MQTT 5.0 客户端
///
/// 解析 URL 确定传输方式：
/// - `mqtt://host:port` → 明文 TCP
/// - `mqtts://host:port` → TLS (rustls)
/// - `ws://host:port/path` → WebSocket
/// - `wss://host:port/path` → 安全 WebSocket
///
/// 支持认证、Keep Alive、遗嘱消息、MQTT 5.0 属性等配置。
pub fn create_mqtt_client(config: &MqttConnectConfig) -> Result<MqttClientResult, String> {
    // 解析 URL
    let (transport_type, host, port, path) = parse_mqtt_url(&config.url)?;

    let client_id = config.client_id.as_deref().unwrap_or("mpe-mqtt-client");

    // 创建 MqttOptions（根据传输方式选择 Broker 和 Transport）
    let mut options =
        create_mqtt_options(client_id, &host, port, path.as_deref(), &transport_type, config)?;

    // 配置认证
    if let (Some(username), Some(password)) = (&config.username, &config.password) {
        options.set_credentials(username.as_str(), password.clone());
    }

    // 配置 Keep Alive（秒）
    options.set_keep_alive(config.keep_alive_secs.unwrap_or(60));

    // 配置 Clean Start
    options.set_clean_start(config.clean_session.unwrap_or(true));

    // 配置遗嘱消息
    configure_last_will(&mut options, config);

    // 配置 MQTT 5.0 特有选项
    configure_mqtt5_options(&mut options, config);

    // 配置连接超时
    let timeout = config.connect_timeout_ms.unwrap_or(5000);
    options.set_connect_timeout(Duration::from_millis(timeout));

    // 创建异步客户端和事件循环
    let (client, eventloop) = AsyncClient::new(options, 10);

    info!(
        "MQTT 客户端创建成功: {} ({:?})",
        config.url, transport_type
    );

    Ok(MqttClientResult {
        client,
        eventloop,
        broker_version: None,
    })
}

// ---------------------------------------------------------------------------
// URL 解析
// ---------------------------------------------------------------------------

/// 传输类型
#[derive(Debug)]
enum TransportType {
    /// 明文 TCP
    Tcp,
    /// TLS 加密
    Tls,
    /// WebSocket
    Ws,
    /// 安全 WebSocket
    Wss,
}

/// 解析 MQTT URL，提取传输类型、主机、端口和路径
fn parse_mqtt_url(url: &str) -> Result<(TransportType, String, u16, Option<String>), String> {
    let (transport, remainder) = if let Some(rest) = url.strip_prefix("mqtt://") {
        (TransportType::Tcp, rest)
    } else if let Some(rest) = url.strip_prefix("mqtts://") {
        (TransportType::Tls, rest)
    } else if let Some(rest) = url.strip_prefix("ws://") {
        (TransportType::Ws, rest)
    } else if let Some(rest) = url.strip_prefix("wss://") {
        (TransportType::Wss, rest)
    } else {
        // 无协议前缀，默认按 TCP 处理
        (TransportType::Tcp, url)
    };

    // 分离 主机:端口 和 路径
    let (host_port, path) = match remainder.find('/') {
        Some(idx) => (&remainder[..idx], Some(remainder[idx..].to_string())),
        None => (remainder, None),
    };

    // 分离主机和端口
    let (host, port) = parse_host_port(host_port, &transport)?;

    Ok((transport, host, port, path))
}

/// 解析 "host:port" 字符串，未指定端口时使用协议默认端口
fn parse_host_port(
    host_port: &str,
    transport: &TransportType,
) -> Result<(String, u16), String> {
    match host_port.rfind(':') {
        Some(idx) => {
            let host = &host_port[..idx];
            let port_str = &host_port[idx + 1..];
            let port: u16 = port_str
                .parse()
                .map_err(|_| format!("无效的端口号: '{}'", port_str))?;
            if host.is_empty() {
                return Err("Hostname is required".to_string());
            }
            Ok((host.to_string(), port))
        }
        None => {
            if host_port.is_empty() {
                return Err("Hostname is required".to_string());
            }
            let default_port = match transport {
                TransportType::Tcp | TransportType::Ws => 1883,
                TransportType::Tls | TransportType::Wss => 8883,
            };
            Ok((host_port.to_string(), default_port))
        }
    }
}

// ---------------------------------------------------------------------------
// MqttOptions 创建
// ---------------------------------------------------------------------------

/// 根据传输类型创建 `MqttOptions` 并配置对应的 Transport
fn create_mqtt_options(
    client_id: &str,
    host: &str,
    port: u16,
    path: Option<&str>,
    transport_type: &TransportType,
    config: &MqttConnectConfig,
) -> Result<MqttOptions, String> {
    let skip_verify = config.tls_skip_verify.unwrap_or(false);

    match transport_type {
        TransportType::Tcp => {
            // 明文 TCP：使用默认 Transport（无需额外配置）
            let broker = format!("{}:{}", host, port);
            Ok(MqttOptions::new(client_id, broker))
        }
        TransportType::Tls => {
            // TLS：配置 rustls Transport
            let broker = format!("{}:{}", host, port);
            let mut opts = MqttOptions::new(client_id, broker);
            let tls_config = create_tls_config(skip_verify)?;
            opts.set_transport(Transport::tls_with_config(tls_config));
            Ok(opts)
        }
        TransportType::Ws => {
            // WebSocket：通过 Broker::websocket 指定，Transport 默认为 Ws
            let ws_url = build_ws_url(host, port, path);
            let broker = Broker::websocket(&ws_url)
                .map_err(|e| format!("无效的 WebSocket URL '{}': {}", ws_url, e))?;
            Ok(MqttOptions::new(client_id, broker))
        }
        TransportType::Wss => {
            // 安全 WebSocket：Broker::websocket + WSS Transport
            // 注意：传给 Broker::websocket 的 URL 始终使用 ws:// 协议，
            // TLS 通过 Transport::wss_with_config 单独配置
            let ws_url = build_ws_url(host, port, path);
            let broker = Broker::websocket(&ws_url)
                .map_err(|e| format!("无效的 WebSocket URL '{}': {}", ws_url, e))?;
            let mut opts = MqttOptions::new(client_id, broker);
            let tls_config = create_tls_config(skip_verify)?;
            opts.set_transport(Transport::wss_with_config(tls_config));
            Ok(opts)
        }
    }
}

// ---------------------------------------------------------------------------
// TLS 配置
// ---------------------------------------------------------------------------

/// 不验证证书的 ServerCertVerifier（仅用于 `tls_skip_verify=true`）
///
/// **警告**: 仅用于开发/测试环境，切勿在生产环境使用。
#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}

/// 确保 rustls `CryptoProvider` 已安装（幂等，多次调用安全）
fn ensure_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// 创建 TLS 配置
///
/// - `skip_verify=false`: 使用 webpki-roots 根证书验证
/// - `skip_verify=true`:  跳过证书验证（仅开发/测试）
fn create_tls_config(skip_verify: bool) -> Result<TlsConfiguration, String> {
    ensure_crypto_provider();

    let client_config = if skip_verify {
        // 跳过验证模式：使用自定义 NoVerifier
        rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth()
    } else {
        // 标准模式：使用 webpki-roots Mozilla 根证书集合
        let root_store = rustls::RootCertStore::from_iter(
            webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
        );
        rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth()
    };

    Ok(client_config.into())
}

// ---------------------------------------------------------------------------
// 遗嘱消息配置
// ---------------------------------------------------------------------------

/// 配置 MQTT Last Will 消息
fn configure_last_will(options: &mut MqttOptions, config: &MqttConnectConfig) {
    let Some(will_topic) = &config.will_topic else {
        return;
    };

    let payload = config.will_payload.as_deref().unwrap_or("");
    let qos = match config.will_qos.unwrap_or(0) {
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => QoS::AtMostOnce,
    };
    let retain = config.will_retain.unwrap_or(false);

    // 构建 MQTT 5.0 遗嘱属性
    let will_properties = if config.will_delay_interval.is_some() {
        Some(LastWillProperties {
            delay_interval: config.will_delay_interval,
            payload_format_indicator: None,
            message_expiry_interval: None,
            content_type: None,
            response_topic: None,
            correlation_data: None,
            user_properties: Vec::new(),
        })
    } else {
        None
    };

    let will = LastWill::new(will_topic, payload, qos, retain, will_properties);
    options.set_last_will(will);
}

// ---------------------------------------------------------------------------
// MQTT 5.0 选项
// ---------------------------------------------------------------------------

/// 配置 MQTT 5.0 特有选项（`session_expiry`、`receive_max`、`topic_alias_max` 等）
fn configure_mqtt5_options(options: &mut MqttOptions, config: &MqttConnectConfig) {
    if let Some(expiry) = config.session_expiry_interval {
        options.set_session_expiry_interval(Some(expiry));
    }
    if let Some(recv_max) = config.receive_maximum {
        options.set_receive_maximum(Some(recv_max));
    }
    if let Some(alias_max) = config.topic_alias_maximum {
        options.set_topic_alias_max(Some(alias_max));
    }
    if let Some(max_size) = config.max_packet_size {
        options.set_max_packet_size(Some(max_size));
    }
    if let Some(props) = &config.user_properties {
        options.set_user_properties(props.clone());
    }
}

// ---------------------------------------------------------------------------
// 工具函数
// ---------------------------------------------------------------------------

/// 构建 WebSocket URL（始终使用 ws:// 协议，TLS 通过 Transport 单独配置）
fn build_ws_url(host: &str, port: u16, path: Option<&str>) -> String {
    let path_str = path.unwrap_or("/mqtt");
    format!("ws://{}:{}{}", host, port, path_str)
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- URL 解析测试 ----

    #[test]
    fn test_parse_mqtt_url_tcp() {
        let (transport, host, port, path) =
            parse_mqtt_url("mqtt://broker.example.com:1883").unwrap();
        assert!(matches!(transport, TransportType::Tcp));
        assert_eq!(host, "broker.example.com");
        assert_eq!(port, 1883);
        assert!(path.is_none());
    }

    #[test]
    fn test_parse_mqtt_url_tcp_default_port() {
        let (transport, host, port, path) =
            parse_mqtt_url("mqtt://broker.example.com").unwrap();
        assert!(matches!(transport, TransportType::Tcp));
        assert_eq!(host, "broker.example.com");
        assert_eq!(port, 1883);
        assert!(path.is_none());
    }

    #[test]
    fn test_parse_mqtt_url_tls() {
        let (transport, host, port, _) =
            parse_mqtt_url("mqtts://broker.example.com:8883").unwrap();
        assert!(matches!(transport, TransportType::Tls));
        assert_eq!(host, "broker.example.com");
        assert_eq!(port, 8883);
    }

    #[test]
    fn test_parse_mqtt_url_tls_default_port() {
        let (transport, _, port, _) =
            parse_mqtt_url("mqtts://broker.example.com").unwrap();
        assert!(matches!(transport, TransportType::Tls));
        assert_eq!(port, 8883);
    }

    #[test]
    fn test_parse_mqtt_url_ws() {
        let (transport, host, port, path) =
            parse_mqtt_url("ws://broker.example.com:8000/mqtt").unwrap();
        assert!(matches!(transport, TransportType::Ws));
        assert_eq!(host, "broker.example.com");
        assert_eq!(port, 8000);
        assert_eq!(path, Some("/mqtt".to_string()));
    }

    #[test]
    fn test_parse_mqtt_url_wss() {
        let (transport, host, port, path) =
            parse_mqtt_url("wss://broker.example.com:8443/ws").unwrap();
        assert!(matches!(transport, TransportType::Wss));
        assert_eq!(host, "broker.example.com");
        assert_eq!(port, 8443);
        assert_eq!(path, Some("/ws".to_string()));
    }

    #[test]
    fn test_parse_mqtt_url_no_scheme() {
        let (transport, host, port, _) =
            parse_mqtt_url("broker.example.com:1883").unwrap();
        assert!(matches!(transport, TransportType::Tcp));
        assert_eq!(host, "broker.example.com");
        assert_eq!(port, 1883);
    }

    #[test]
    fn test_parse_mqtt_url_invalid_port() {
        let result = parse_mqtt_url("mqtt://broker.example.com:abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mqtt_url_empty_host() {
        let result = parse_mqtt_url("mqtt://:1883");
        assert!(result.is_err());
    }

    // ---- WebSocket URL 构建测试 ----

    #[test]
    fn test_build_ws_url_with_path() {
        let url = build_ws_url("broker.example.com", 8000, Some("/mqtt"));
        assert_eq!(url, "ws://broker.example.com:8000/mqtt");
    }

    #[test]
    fn test_build_ws_url_default_path() {
        let url = build_ws_url("broker.example.com", 8000, None);
        assert_eq!(url, "ws://broker.example.com:8000/mqtt");
    }

    // ---- TLS 配置测试 ----

    #[test]
    fn test_create_tls_config_standard() {
        let result = create_tls_config(false);
        assert!(result.is_ok(), "标准 TLS 配置应创建成功: {:?}", result.err());
    }

    #[test]
    fn test_create_tls_config_skip_verify() {
        let result = create_tls_config(true);
        assert!(result.is_ok(), "跳过验证 TLS 配置应创建成功: {:?}", result.err());
    }

    #[test]
    fn test_ensure_crypto_provider_idempotent() {
        ensure_crypto_provider();
        ensure_crypto_provider();
        ensure_crypto_provider();
    }
}
