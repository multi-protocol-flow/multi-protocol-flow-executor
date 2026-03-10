// WebSocket URL 存储功能测试
#![allow(missing_docs)]

use flow_engine::context::ExecutionContext;

/// 测试基本的 URL 存储和检索
#[test]
fn test_ws_url_storage_basic() {
    let mut ctx = ExecutionContext::new("test-flow".to_string(), 1000);

    // 存储 URL
    ctx.set_ws_url("conn-001".to_string(), "wss://example.com/ws".to_string());

    // 检索 URL
    let url = ctx.get_ws_url("conn-001");
    assert_eq!(url, Some(&"wss://example.com/ws".to_string()));

    // 存储多个 URL
    ctx.set_ws_url(
        "conn-002".to_string(),
        "wss://api.example.com/stream".to_string(),
    );

    assert_eq!(
        ctx.get_ws_url("conn-001"),
        Some(&"wss://example.com/ws".to_string())
    );
    assert_eq!(
        ctx.get_ws_url("conn-002"),
        Some(&"wss://api.example.com/stream".to_string())
    );
}

/// 测试 URL 不存在时返回 None
#[test]
fn test_ws_url_storage_not_found() {
    let ctx = ExecutionContext::new("test-flow".to_string(), 1000);

    // 查询不存在的连接 ID
    let url = ctx.get_ws_url("nonexistent-conn");
    assert!(url.is_none());

    // 存储一个 URL 后再查询不存在的
    let mut ctx = ctx;
    ctx.set_ws_url("conn-001".to_string(), "wss://example.com/ws".to_string());

    assert!(ctx.get_ws_url("conn-002").is_none());
}

/// 测试覆盖已有 URL
#[test]
fn test_ws_url_storage_override() {
    let mut ctx = ExecutionContext::new("test-flow".to_string(), 1000);

    // 存储 URL
    ctx.set_ws_url("conn-001".to_string(), "wss://example.com/ws".to_string());
    assert_eq!(
        ctx.get_ws_url("conn-001"),
        Some(&"wss://example.com/ws".to_string())
    );

    // 覆盖 URL
    ctx.set_ws_url(
        "conn-001".to_string(),
        "wss://new.example.com/websocket".to_string(),
    );
    assert_eq!(
        ctx.get_ws_url("conn-001"),
        Some(&"wss://new.example.com/websocket".to_string())
    );

    // 确保只有一个条目
    assert_eq!(ctx.ws_connection_urls.len(), 1);
}
