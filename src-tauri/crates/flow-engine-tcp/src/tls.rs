//! TLS 连接支持
//!
//! 提供三种 TLS 模式：
//! - **plaintext**: 明文 TCP（`use_tls=false`）
//! - **standard**: 标准 TLS，使用 webpki-roots 根证书验证（`use_tls=true, tls_skip_verify=false`）
//! - **skip-verify**: 跳过证书验证，仅用于开发/测试（`use_tls=true, tls_skip_verify=true`）

use std::sync::Arc;

use rustls::ClientConfig;
use tokio_rustls::TlsConnector;

// ---------------------------------------------------------------------------
// 不验证证书的 ServerCertVerifier
// ---------------------------------------------------------------------------

/// 不验证任何证书的 ServerCertVerifier（仅用于 `tls_skip_verify=true`）
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

// ---------------------------------------------------------------------------
// CryptoProvider 初始化
// ---------------------------------------------------------------------------

/// 确保已安装 CryptoProvider（幂等，多次调用安全）
///
/// rustls 0.23 需要在首次创建 `ClientConfig` 前安装 `CryptoProvider`。
/// 此函数使用 ring 作为加密后端。
fn ensure_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

// ---------------------------------------------------------------------------
// TLS 配置创建
// ---------------------------------------------------------------------------

/// 创建 TLS 连接器
///
/// # 参数
/// - `skip_verify`: 是否跳过证书验证（用于开发/测试环境）
///
/// # 返回
/// Success返回 `TlsConnector`，Failed返回Error描述。
pub fn create_tls_connector(skip_verify: bool) -> Result<TlsConnector, String> {
    ensure_crypto_provider();

    let config = if skip_verify {
        create_skip_verify_config()?
    } else {
        create_standard_config()?
    };

    Ok(TlsConnector::from(Arc::new(config)))
}

/// 创建跳过验证的 TLS 配置
///
/// 使用自定义的 `NoVerifier` 跳过所有证书验证。
/// **警告**: 仅用于开发/测试环境。
fn create_skip_verify_config() -> Result<ClientConfig, String> {
    let config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerifier))
        .with_no_client_auth();
    Ok(config)
}

/// 创建标准 TLS 配置（使用 webpki-roots 根证书）
///
/// 加载 Mozilla 的根证书集合用于服务器证书验证。
fn create_standard_config() -> Result<ClientConfig, String> {
    let root_store = rustls::RootCertStore::from_iter(
        webpki_roots::TLS_SERVER_ROOTS
            .iter()
            .cloned(),
    );

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    Ok(config)
}

/// 创建 TLS 连接器（支持自定义 CA 证书）
///
/// 在 webpki-roots 系统根证书基础上，额外添加自定义 CA 证书。
/// 自定义 CA 证书为 PEM 格式，解析后会追加到根证书存储中。
///
/// # 参数
/// - `skip_verify`: 是否跳过证书验证（用于开发/测试环境）
/// - `ca_cert_pem`: 自定义 CA 证书（PEM 格式），None 时仅使用系统根证书
///
/// # 返回
/// Success返回 `TlsConnector`，Failed返回Error描述。
pub fn create_tls_connector_with_ca(
    skip_verify: bool,
    ca_cert_pem: Option<&[u8]>,
) -> Result<TlsConnector, String> {
    ensure_crypto_provider();

    if skip_verify {
        // 跳过验证模式不需要 CA 证书
        let config = create_skip_verify_config()?;
        return Ok(TlsConnector::from(Arc::new(config)));
    }

    // 构建根证书存储：以 webpki-roots 为基础，追加自定义 CA 证书
    let mut root_store = rustls::RootCertStore::from_iter(
        webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
    );

    if let Some(pem_bytes) = ca_cert_pem {
        let mut cursor = std::io::Cursor::new(pem_bytes);
        let certs: Vec<_> = rustls_pemfile::certs(&mut cursor)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to parse CA certificate PEM: {}", e))?;
        for cert in certs {
            root_store.add(cert).map_err(|e| format!("Failed to add CA certificate: {}", e))?;
        }
    }

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Ok(TlsConnector::from(Arc::new(config)))
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试跳过验证模式创建 TLS 连接器
    #[test]
    fn test_create_tls_connector_skip_verify() {
        let result = create_tls_connector(true);
        assert!(
            result.is_ok(),
            "跳过验证模式应Success创建 TLS 连接器: {:?}",
            result.err()
        );
    }

    /// 测试标准模式创建 TLS 连接器
    #[test]
    fn test_create_tls_connector_standard() {
        let result = create_tls_connector(false);
        assert!(
            result.is_ok(),
            "标准模式应Success创建 TLS 连接器: {:?}",
            result.err()
        );
    }

    /// 测试多次调用 create_tls_connector 不会 panic（CryptoProvider 幂等安装）
    #[test]
    fn test_create_tls_connector_idempotent() {
        let result1 = create_tls_connector(true);
        let result2 = create_tls_connector(false);
        let result3 = create_tls_connector(true);
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(result3.is_ok());
    }

    /// 测试 TLS 连接器可以克隆（TlsConnector 内部使用 Arc）
    #[test]
    fn test_tls_connector_clonable() {
        let connector = create_tls_connector(true).expect("创建应Success");
        let _cloned = connector.clone();
    }

    /// 测试 create_tls_connector_with_ca 跳过验证模式
    #[test]
    fn test_create_tls_connector_with_ca_skip_verify() {
        let result = create_tls_connector_with_ca(true, None);
        assert!(
            result.is_ok(),
            "跳过验证模式应Success: {:?}",
            result.err()
        );
    }

    /// 测试 create_tls_connector_with_ca 标准模式（无自定义 CA）
    #[test]
    fn test_create_tls_connector_with_ca_standard_no_ca() {
        let result = create_tls_connector_with_ca(false, None);
        assert!(
            result.is_ok(),
            "标准模式（无自定义 CA）应Success: {:?}",
            result.err()
        );
    }

    /// 测试 create_tls_connector_with_ca 无效 PEM 数据
    #[test]
    fn test_create_tls_connector_with_ca_invalid_pem() {
        let invalid_pem = b"not a valid pem data";
        let result = create_tls_connector_with_ca(false, Some(invalid_pem));
        // 无效 PEM 不应包含有效证书，但不会报错（只是没有证书被添加）
        // 或者解析为空证书列表
        assert!(
            result.is_ok(),
            "无效 PEM 应被忽略（空证书列表）或报错: {:?}",
            result.err()
        );
    }

    /// 测试 create_tls_connector_with_ca 跳过验证时忽略 CA 证书
    #[test]
    fn test_create_tls_connector_with_ca_skip_verify_ignores_ca() {
        let some_ca = b"-----BEGIN CERTIFICATE-----\ninvalid\n-----END CERTIFICATE-----";
        let result = create_tls_connector_with_ca(true, Some(some_ca));
        assert!(
            result.is_ok(),
            "跳过验证模式应忽略 CA 证书参数: {:?}",
            result.err()
        );
    }
}
