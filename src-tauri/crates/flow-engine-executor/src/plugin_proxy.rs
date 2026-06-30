// 代理设置节点插件 — NodePlugin 实现

use flow_engine_core::connection::Connection;
use flow_engine_core::context::{ExecutionContext, ProxyState};
use flow_engine_core::node::Node;
use flow_engine_core::node_config::{NodeData, ProxyConfig, ProxyMode};
use flow_engine_core::plugin::{NodePlugin, PortDef};
use std::collections::HashMap;
use std::sync::Arc;

/// 代理设置节点插件
///
/// 设置或清除后续节点的代理配置。通过 `ExecutionContext` 的 provider 注册表
/// 存取 `ProxyState`，协议执行器可通过 `context.get_provider::<ProxyState>("proxy_state")` 读取。
pub struct ProxyPlugin;

#[async_trait::async_trait]
impl NodePlugin for ProxyPlugin {
    fn type_id(&self) -> &str {
        "proxy"
    }

    fn display_name(&self) -> &str {
        "代理设置"
    }

    fn category(&self) -> &str {
        "tools"
    }

    fn icon(&self) -> &str {
        "shield"
    }

    fn color(&self) -> &str {
        "#8B5CF6"
    }

    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                id: "in".into(),
                label: "输入".into(),
                position: "left".into(),
            },
            PortDef {
                id: "out".into(),
                label: "输出".into(),
                position: "right".into(),
            },
        ]
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "proxy",
            "mode": "clear"
        })
    }

    /// 代理节点固定返回 "out" 端口
    fn resolve_port(&self, _output: &Option<serde_json::Value>) -> String {
        "out".to_string()
    }

    /// 不做端口路由（始终走 out）
    fn is_port_routing(&self) -> bool {
        false
    }

    /// 报告输入数据
    fn build_report_input(&self, config: &serde_json::Value) -> Option<serde_json::Value> {
        let parsed: ProxyConfig = serde_json::from_value(config.clone()).ok()?;
        Some(serde_json::json!({
            "mode": match parsed.mode {
                ProxyMode::Clear => "clear",
                ProxyMode::Http => "http",
                ProxyMode::Socks5 => "socks5",
            },
            "proxy_url": parsed.proxy_url,
        }))
    }

    /// 执行代理设置或清除
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        _connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        let config = match &node.data {
            NodeData::Proxy(c) => c,
            _ => return Err("代理节点配置类型不匹配".to_string()),
        };

        match &config.mode {
            ProxyMode::Clear => {
                context.remove_provider("proxy_state");
                Ok((
                    vec!["out".to_string()],
                    Some(serde_json::json!({
                        "action": "clear",
                        "message": "已清除代理设置"
                    })),
                ))
            }
            ProxyMode::Http | ProxyMode::Socks5 => {
                let proxy_url = config
                    .proxy_url
                    .as_deref()
                    .ok_or_else(|| "Proxy address is required".to_string())?;

                // 字符串解析 proxy_url（格式: scheme://[user:pass@]host:port）
                let (host, port) = parse_proxy_url(proxy_url)?;

                let state = Arc::new(ProxyState {
                    mode: config.mode.clone(),
                    host,
                    port,
                    username: config.username.clone(),
                    password: config.password.clone(),
                });
                context.set_provider("proxy_state", state);

                let mode_str = match config.mode {
                    ProxyMode::Http => "HTTP",
                    ProxyMode::Socks5 => "SOCKS5",
                    ProxyMode::Clear => unreachable!(),
                };
                Ok((
                    vec!["out".to_string()],
                    Some(serde_json::json!({
                        "action": "set",
                        "mode": mode_str.to_lowercase(),
                        "proxy_url": proxy_url,
                        "message": format!("已设置{}代理: {}", mode_str, proxy_url)
                    })),
                ))
            }
        }
    }
}

/// 解析代理 URL，提取主机和端口
///
/// 支持格式:
/// - `http://host:port`
/// - `socks5://host:port`
/// - `http://user:pass@host:port`
/// - `host:port`（无 scheme 时默认端口 1080）
fn parse_proxy_url(url: &str) -> Result<(String, u16), String> {
    // 去除 scheme 部分（如有）
    let without_scheme = if let Some(rest) = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .or_else(|| url.strip_prefix("socks5://"))
        .or_else(|| url.strip_prefix("socks5h://"))
    {
        rest
    } else {
        url
    };

    // 去除用户认证部分（user:pass@）
    let host_port = if let Some(at_pos) = without_scheme.rfind('@') {
        &without_scheme[at_pos + 1..]
    } else {
        without_scheme
    };

    // 解析 host:port
    if let Some(colon_pos) = host_port.rfind(':') {
        let host = host_port[..colon_pos].to_string();
        let port_str = &host_port[colon_pos + 1..];
        let port: u16 = port_str
            .parse()
            .map_err(|_| format!("代理端口格式错误: {}", port_str))?;
        if host.is_empty() {
            return Err("代理地址缺少主机名".to_string());
        }
        Ok((host, port))
    } else {
        // 无端口时使用默认端口
        if host_port.is_empty() {
            return Err("代理地址缺少主机名".to_string());
        }
        Ok((host_port.to_string(), 1080))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proxy_url_with_scheme() {
        let (host, port) = parse_proxy_url("http://127.0.0.1:8080").unwrap();
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_parse_proxy_url_socks5() {
        let (host, port) = parse_proxy_url("socks5://192.168.1.1:1080").unwrap();
        assert_eq!(host, "192.168.1.1");
        assert_eq!(port, 1080);
    }

    #[test]
    fn test_parse_proxy_url_with_auth() {
        let (host, port) = parse_proxy_url("http://user:pass@proxy.example.com:3128").unwrap();
        assert_eq!(host, "proxy.example.com");
        assert_eq!(port, 3128);
    }

    #[test]
    fn test_parse_proxy_url_no_scheme() {
        let (host, port) = parse_proxy_url("proxy.example.com:9090").unwrap();
        assert_eq!(host, "proxy.example.com");
        assert_eq!(port, 9090);
    }

    #[test]
    fn test_parse_proxy_url_no_port() {
        let (host, port) = parse_proxy_url("proxy.example.com").unwrap();
        assert_eq!(host, "proxy.example.com");
        assert_eq!(port, 1080); // 默认端口
    }

    #[test]
    fn test_parse_proxy_url_empty_host() {
        let result = parse_proxy_url(":8080");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_proxy_url_invalid_port() {
        let result = parse_proxy_url("host:abc");
        assert!(result.is_err());
    }
}
