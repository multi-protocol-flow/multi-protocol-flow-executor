//! HTTP 代理模块
//!
//! 支持通过 HTTP 代理发送请求（SOCKS5 为后续扩展）

use base64::Engine;
use flow_engine_core::node_config::HttpProxyConfig;

/// 代理连接结果
#[derive(Debug)]
pub struct ProxyResult {
    /// 代理目标地址
    pub target_host: String,
    /// 代理目标端口
    pub target_port: u16,
    /// 是否需要代理认证头
    pub proxy_auth_header: Option<String>,
}

/// 解析代理配置，返回代理连接信息
pub fn resolve_proxy_config(proxy_config: &HttpProxyConfig) -> Option<ProxyResult> {
    if proxy_config.proxy_type == flow_engine_core::node_config::HttpProxyType::None {
        return None;
    }

    let proxy_url = proxy_config.proxy_url.as_deref()?;
    let parsed = url::Url::parse(proxy_url).ok()?;

    let target_host = parsed.host_str()?.to_string();
    let target_port = parsed.port_or_known_default().unwrap_or(8080);

    let proxy_auth_header = match (&proxy_config.proxy_username, &proxy_config.proxy_password) {
        (Some(user), Some(pass)) if !user.is_empty() => {
            let credentials = base64::engine::general_purpose::STANDARD
                .encode(format!("{}:{}", user, pass));
            Some(format!("Basic {}", credentials))
        }
        _ => None,
    };

    Some(ProxyResult {
        target_host,
        target_port,
        proxy_auth_header,
    })
}

/// 构建 HTTP CONNECT 请求（用于 HTTPS 通过代理）
pub fn build_connect_request(host: &str, port: u16, auth_header: Option<&str>) -> Vec<u8> {
    let mut request = format!(
        "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n",
        host, port, host, port
    );

    if let Some(auth) = auth_header {
        request.push_str(&format!("Proxy-Authorization: {}\r\n", auth));
    }

    request.push_str("\r\n");
    request.into_bytes()
}

/// 解析 CONNECT 代理响应
pub fn parse_connect_response(response_bytes: &[u8]) -> Result<(), String> {
    let response_str = std::str::from_utf8(response_bytes)
        .map_err(|e| format!("代理响应非 UTF-8: {}", e))?;

    let first_line = response_str
        .lines()
        .next()
        .ok_or_else(|| "代理响应为空".to_string())?;

    // 检查 "HTTP/1.x 200" 响应
    if first_line.contains("200") {
        Ok(())
    } else {
        Err(format!("代理连接失败: {}", first_line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_proxy_config_none() {
        let config = HttpProxyConfig::default();
        assert!(resolve_proxy_config(&config).is_none());
    }

    #[test]
    fn test_resolve_proxy_config_http() {
        let config = HttpProxyConfig {
            proxy_type: flow_engine_core::node_config::HttpProxyType::Http,
            proxy_url: Some("http://proxy.example.com:8080".to_string()),
            proxy_username: Some("user".to_string()),
            proxy_password: Some("pass".to_string()),
        };
        let result = resolve_proxy_config(&config);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.target_host, "proxy.example.com");
        assert_eq!(r.target_port, 8080);
        assert!(r.proxy_auth_header.is_some());
    }

    #[test]
    fn test_build_connect_request() {
        let req = build_connect_request("target.com", 443, None);
        let req_str = String::from_utf8(req).unwrap();
        assert!(req_str.contains("CONNECT target.com:443"));
        assert!(req_str.contains("Host: target.com:443"));
    }
}
