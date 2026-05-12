//! HTTP 客户端与执行器 crate
//!
//! 基于 hyper 1.x + rustls 提供高性能 HTTP 客户端，支持 HTTP/1.1 和 HTTP/2，
//! 具备 DNS/TCP/TLS/TTFB/下载各阶段详细计时能力。
//! 同时包含 HTTP 节点执行器，用于流程引擎中的 HTTP 请求节点执行。

pub mod http;
/// HTTP 节点执行器，负责在流程引擎中执行 HTTP 请求节点
pub mod http_executor;
/// HTTP 节点插件，实现 `NodePlugin` trait
pub mod plugin;
