//! HTTP 代理模块
//!
//! 支持通过 HTTP 代理（CONNECT 隧道）和 SOCKS5 代理转发请求。
//! 两种代理模式均由 `context.providers["proxy_state"]` 中的 `ProxyState` 驱动。

use base64::Engine;
use flow_engine_core::context::ProxyState;
use flow_engine_core::node_config::ProxyMode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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

/// 构造 Proxy-Authorization 认证头（Basic Auth）
fn build_proxy_auth_header(username: &str, password: &str) -> String {
    let credentials =
        base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", username, password));
    format!("Basic {}", credentials)
}

// ============================================================================
// 新版代理连接函数（基于 ProxyState）
// ============================================================================

/// 通过代理建立到目标主机的 TCP 连接
///
/// 根据 `ProxyState.mode` 选择 HTTP CONNECT 或 SOCKS5 方式建立隧道。
/// 返回已建立隧道的 `TcpStream`，调用方可在此之上进行 TLS 握手。
///
/// # 参数
/// - `proxy_state`: 代理状态（从 `context.providers["proxy_state"]` 获取）
/// - `target_host`: 目标主机名
/// - `target_port`: 目标端口
/// - `timeout_ms`: 连接超时（毫秒）
pub async fn connect_via_proxy(
    proxy_state: &ProxyState,
    target_host: &str,
    target_port: u16,
    timeout_ms: u64,
) -> Result<TcpStream, String> {
    match proxy_state.mode {
        ProxyMode::Clear => Err("代理模式为 Clear，不应调用 connect_via_proxy".to_string()),
        ProxyMode::Http => {
            connect_via_http_proxy(proxy_state, target_host, target_port, timeout_ms).await
        }
        ProxyMode::Socks5 => {
            connect_via_socks5_proxy(proxy_state, target_host, target_port, timeout_ms).await
        }
    }
}

/// 通过 HTTP 代理（CONNECT 隧道）建立到目标主机的连接
///
/// 流程：
/// 1. TCP 连接到代理服务器
/// 2. 发送 CONNECT target:port HTTP/1.1 请求
/// 3. 解析代理响应，确认隧道建立（200）
/// 4. 返回 TcpStream（隧道已就绪，可进行 TLS 握手）
async fn connect_via_http_proxy(
    proxy_state: &ProxyState,
    target_host: &str,
    target_port: u16,
    timeout_ms: u64,
) -> Result<TcpStream, String> {
    let proxy_addr = format!("{}:{}", proxy_state.host, proxy_state.port);

    log::debug!(
        "通过 HTTP 代理 {} 连接到 {}:{}",
        proxy_addr,
        target_host,
        target_port
    );

    // TCP 连接到代理服务器
    let stream = tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        TcpStream::connect(&proxy_addr),
    )
    .await
    .map_err(|_| format!("连接 HTTP 代理超时 ({}ms): {}", timeout_ms, proxy_addr))?
    .map_err(|e| format!("连接 HTTP 代理失败 ({}): {}", proxy_addr, e))?;

    // 构造认证头
    let auth_header = match (&proxy_state.username, &proxy_state.password) {
        (Some(user), Some(pass)) if !user.is_empty() => {
            Some(build_proxy_auth_header(user, pass))
        }
        _ => None,
    };

    // 构建 CONNECT 请求
    let connect_req = build_connect_request(target_host, target_port, auth_header.as_deref());

    // 使用临时包装发送 CONNECT 并读取响应
    let mut stream = stream;
    stream
        .write_all(&connect_req)
        .await
        .map_err(|e| format!("发送 CONNECT 请求失败: {}", e))?;
    stream
        .flush()
        .await
        .map_err(|e| format!("刷新 CONNECT 请求失败: {}", e))?;

    // 读取代理响应（读到 \r\n\r\n 为止）
    let mut response_buf = vec![0u8; 4096];
    let mut total_read = 0usize;
    loop {
        if total_read >= response_buf.len() {
            return Err("代理响应过大，超出缓冲区".to_string());
        }
        let n = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            stream.read(&mut response_buf[total_read..]),
        )
        .await
        .map_err(|_| "读取代理响应超时".to_string())?
        .map_err(|e| format!("读取代理响应失败: {}", e))?;

        if n == 0 {
            return Err("代理关闭连接，未返回响应".to_string());
        }

        total_read += n;

        // 检查是否已读取完整的头部（以 \r\n\r\n 结尾）
        let response_so_far = &response_buf[..total_read];
        if response_so_far.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }

    // 解析响应
    parse_connect_response(&response_buf[..total_read])?;

    log::debug!(
        "HTTP 代理 CONNECT 隧道建立成功: {}:{} via {}",
        target_host,
        target_port,
        proxy_addr
    );

    Ok(stream)
}

/// 通过 SOCKS5 代理建立到目标主机的连接
///
/// 使用 `tokio-socks` crate 实现 SOCKS5 协议握手。
/// 返回的 `TcpStream` 已通过 SOCKS5 隧道连接到目标。
async fn connect_via_socks5_proxy(
    proxy_state: &ProxyState,
    target_host: &str,
    target_port: u16,
    timeout_ms: u64,
) -> Result<TcpStream, String> {
    let proxy_addr = format!("{}:{}", proxy_state.host, proxy_state.port);

    log::debug!(
        "通过 SOCKS5 代理 {} 连接到 {}:{}",
        proxy_addr,
        target_host,
        target_port
    );

    let socks5_stream = tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        connect_socks5(proxy_state, target_host, target_port),
    )
    .await
    .map_err(|_| {
        format!(
            "SOCKS5 代理连接超时 ({}ms): {}",
            timeout_ms, proxy_addr
        )
    })?
    .map_err(|e| {
        format!(
            "SOCKS5 代理连接失败 ({} -> {}:{}): {}",
            proxy_addr, target_host, target_port, e
        )
    })?;

    log::debug!(
        "SOCKS5 代理隧道建立成功: {}:{} via {}",
        target_host,
        target_port,
        proxy_addr
    );

    Ok(socks5_stream)
}

/// 执行 SOCKS5 连接握手，返回内部 TcpStream
async fn connect_socks5(
    proxy_state: &ProxyState,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream, String> {
    match (&proxy_state.username, &proxy_state.password) {
        (Some(user), Some(pass)) if !user.is_empty() => {
            // 带认证的 SOCKS5 连接
            let output = tokio_socks::tcp::Socks5Stream::connect_with_password(
                (proxy_state.host.as_str(), proxy_state.port),
                (target_host, target_port),
                user,
                pass,
            )
            .await
            .map_err(|e| format!("SOCKS5 认证连接失败: {}", e))?;
            Ok(output.into_inner())
        }
        _ => {
            // 无认证的 SOCKS5 连接
            let output = tokio_socks::tcp::Socks5Stream::connect(
                (proxy_state.host.as_str(), proxy_state.port),
                (target_host, target_port),
            )
            .await
            .map_err(|e| format!("SOCKS5 连接失败: {}", e))?;
            Ok(output.into_inner())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_connect_request() {
        let req = build_connect_request("target.com", 443, None);
        let req_str = String::from_utf8(req).unwrap();
        assert!(req_str.contains("CONNECT target.com:443"));
        assert!(req_str.contains("Host: target.com:443"));
    }

    #[test]
    fn test_parse_connect_response_200() {
        let response = b"HTTP/1.1 200 Connection established\r\n\r\n";
        assert!(parse_connect_response(response).is_ok());
    }

    #[test]
    fn test_parse_connect_response_407() {
        let response = b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n";
        assert!(parse_connect_response(response).is_err());
    }

    #[test]
    fn test_parse_connect_response_empty() {
        let response = b"";
        assert!(parse_connect_response(response).is_err());
    }

    #[test]
    fn test_build_proxy_auth_header() {
        let header = build_proxy_auth_header("user", "pass");
        assert!(header.starts_with("Basic "));
        // base64("user:pass") = "dXNlcjpwYXNz"
        assert_eq!(header, "Basic dXNlcjpwYXNz");
    }

    #[test]
    fn test_connect_via_proxy_clear_mode_returns_error() {
        let state = ProxyState {
            mode: ProxyMode::Clear,
            host: String::new(),
            port: 0,
            username: None,
            password: None,
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(connect_via_proxy(&state, "example.com", 443, 5000));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Clear"));
    }
}
