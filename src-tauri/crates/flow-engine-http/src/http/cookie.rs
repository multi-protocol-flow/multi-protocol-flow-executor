//! Cookie 管理模块
//!
//! 提供简单的 Cookie Jar 实现，支持：
//! - 初始 Cookie 设置
//! - 从响应 Set-Cookie 头解析 Cookie
//! - 为请求生成 Cookie 头

use std::collections::HashMap;
use url::Url;

/// Cookie 条目
#[derive(Debug, Clone)]
pub struct Cookie {
    /// Cookie 名称
    pub name: String,
    /// Cookie 值
    pub value: String,
    /// 域名限制
    pub domain: Option<String>,
    /// 路径限制
    pub path: Option<String>,
    /// 是否仅 HTTPS
    pub secure: bool,
    /// 是否仅 HTTP（不可 JS 访问）
    pub http_only: bool,
}
fn normalize_domain(domain: Option<&str>) -> String {
    domain
        .map(|d| {
            let lower = d.to_lowercase();
            lower.strip_prefix('.').unwrap_or(&lower).to_string()
        })
        .unwrap_or_default()
}

fn normalize_path(path: Option<&str>) -> String {
    path.unwrap_or("/").to_string()
}

/// Cookie Jar（管理请求/响应的 Cookie）
#[derive(Debug, Clone, Default)]
pub struct CookieJar {
    /// Cookie 存储（(name, domain, path) -> Cookie）
    cookies: HashMap<(String, String, String), Cookie>,
}

impl CookieJar {
    /// 创建空的 Cookie Jar
    pub fn new() -> Self {
        Self::default()
    }

    /// 从初始 Cookie 列表创建 Jar
    pub fn from_initial(initial: &[flow_engine_core::node_config::CookieEntry]) -> Self {
        let mut jar = Self::new();
        for entry in initial {
            let domain_norm = normalize_domain(entry.domain.as_deref());
            let path_norm = normalize_path(entry.path.as_deref());
            jar.cookies.insert(
                (entry.name.clone(), domain_norm, path_norm),
                Cookie {
                    name: entry.name.clone(),
                    value: entry.value.clone(),
                    domain: entry.domain.clone(),
                    path: entry.path.clone(),
                    secure: false,
                    http_only: false,
                },
            );
        }
        jar
    }

    /// 为指定 URL 生成 Cookie 头值（按 domain 后缀匹配与 path 匹配过滤）
    pub fn cookie_header(&self, url: &str) -> String {
        let parsed_url = Url::parse(url).ok();
        let request_host = parsed_url
            .as_ref()
            .and_then(|u| u.host_str().map(|h| h.to_lowercase()));
        let request_path = parsed_url.as_ref().map(|u| u.path()).unwrap_or("/");

        self.cookies
            .values()
            .filter(|c| {
                // Domain 匹配
                let domain_matches = match (&c.domain, &request_host) {
                    // 无 domain 限制 → 总是发送（向后兼容手动配置）
                    (None, _) => true,
                    // 有 domain → 后缀匹配
                    (Some(cookie_domain), Some(host)) => {
                        let cookie_domain = cookie_domain.to_lowercase();
                        // 去掉前导点（.example.com → example.com）
                        let cookie_domain =
                            cookie_domain.strip_prefix('.').unwrap_or(&cookie_domain);
                        host == cookie_domain || host.ends_with(&format!(".{}", cookie_domain))
                    }
                    // 有 domain 但无法解析 host → 不发送
                    (Some(_), None) => false,
                };
                if !domain_matches {
                    return false;
                }

                // Path 匹配
                match &c.path {
                    Some(cookie_path) => request_path.starts_with(cookie_path),
                    None => true,
                }
            })
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// 从响应头解析 Set-Cookie 并存储（不记录来源域，仅用于测试/手动设置）
    pub fn store_from_response(&mut self, headers: &HashMap<String, String>) {
        for (key, value) in headers {
            if key.to_lowercase() == "set-cookie" {
                if let Some(cookie) = Self::parse_set_cookie(value) {
                    let domain_norm = normalize_domain(cookie.domain.as_deref());
                    let path_norm = normalize_path(cookie.path.as_deref());
                    self.cookies
                        .insert((cookie.name.clone(), domain_norm, path_norm), cookie);
                }
            }
        }
    }

    /// 从单条 Set-Cookie 头字符串解析并存入 Jar
    pub fn store_set_cookie_value(&mut self, header_value: &str, request_url: &str) {
        let request_host = Url::parse(request_url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()));

        if let Some(mut cookie) = Self::parse_set_cookie(header_value) {
            if cookie.domain.is_none() {
                cookie.domain = request_host;
            }
            let domain_norm = normalize_domain(cookie.domain.as_deref());
            let path_norm = normalize_path(cookie.path.as_deref());
            self.cookies
                .insert((cookie.name.clone(), domain_norm, path_norm), cookie);
        }
    }

    /// 从响应头解析 Set-Cookie 并存储，同时记录来源 URL 用于 domain 匹配
    ///
    /// 如果 Set-Cookie 没有显式 `Domain` 属性，则自动使用请求主机名作为 domain（host-only cookie）。
    /// 这确保了不同域名的 Cookie 不会相互串用。
    pub fn store_from_response_with_url(
        &mut self,
        headers: &HashMap<String, String>,
        request_url: &str,
    ) {
        for (key, value) in headers {
            if key.to_lowercase() == "set-cookie" {
                self.store_set_cookie_value(value, request_url);
            }
        }
    }

    /// 解析 Set-Cookie 头值
    fn parse_set_cookie(header_value: &str) -> Option<Cookie> {
        let parts: Vec<&str> = header_value.split(';').collect();
        let main_part = parts.first()?;

        let mut nv = main_part.splitn(2, '=');
        let name = nv.next()?.trim().to_string();
        let value = nv.next()?.trim().to_string();

        let mut cookie = Cookie {
            name,
            value,
            domain: None,
            path: None,
            secure: false,
            http_only: false,
        };

        for part in &parts[1..] {
            let part = part.trim();
            let lower = part.to_lowercase();
            if lower == "secure" {
                cookie.secure = true;
            } else if lower == "httponly" {
                cookie.http_only = true;
            } else if let Some(domain) = lower.strip_prefix("domain=") {
                cookie.domain = Some(domain.trim().to_string());
            } else if let Some(path) = lower.strip_prefix("path=") {
                cookie.path = Some(path.trim().to_string());
            }
        }

        Some(cookie)
    }

    /// 获取所有 Cookie 的快照
    pub fn all_cookies(&self) -> Vec<&Cookie> {
        self.cookies.values().collect()
    }

    /// Cookie 数量
    pub fn len(&self) -> usize {
        self.cookies.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.cookies.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_set_cookie() {
        let cookie = CookieJar::parse_set_cookie("session=abc123; Path=/; HttpOnly; Secure");
        assert!(cookie.is_some());
        let c = cookie.unwrap_or_else(|| panic!("expected Some"));
        assert_eq!(c.name, "session");
        assert_eq!(c.value, "abc123");
        assert_eq!(c.path.as_deref(), Some("/"));
        assert!(c.http_only);
        assert!(c.secure);
    }

    #[test]
    fn test_cookie_jar_header() {
        let mut jar = CookieJar::new();
        jar.store_from_response(&{
            let mut h = HashMap::new();
            h.insert("Set-Cookie".to_string(), "a=1".to_string());
            h
        });
        jar.store_from_response(&{
            let mut h = HashMap::new();
            h.insert("Set-Cookie".to_string(), "b=2".to_string());
            h
        });

        let header = jar.cookie_header("http://example.com");
        assert!(header.contains("a=1"));
        assert!(header.contains("b=2"));
    }

    #[test]
    fn test_from_initial() {
        use flow_engine_core::node_config::CookieEntry;
        let initial = vec![CookieEntry {
            name: "token".into(),
            value: "xyz".into(),
            domain: None,
            path: None,
        }];
        let jar = CookieJar::from_initial(&initial);
        assert_eq!(jar.len(), 1);
        let header = jar.cookie_header("http://example.com");
        assert_eq!(header, "token=xyz");
    }
}
