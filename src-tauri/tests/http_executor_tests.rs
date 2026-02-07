// HTTP执行器集成测试
// 基于Task 17要求：编写HTTP执行器单元测试
// 测试覆盖：成功场景、失败场景、网络错误、超时、变量替换

use app_lib::{
    HttpNodeExecutor, Node, NodePosition, NodeType, ExecutionContext,
    HttpConfig, HttpMethod, NodeData, NodeExecutor
};
use serde_json::json;
use std::collections::HashMap;

// ============================================================================
// 测试辅助函数
// ============================================================================

/// 创建HTTP节点
fn create_http_node(
    uuid: &str,
    name: &str,
    url: &str,
    method: HttpMethod,
    headers: Option<HashMap<String, String>>,
    body: Option<serde_json::Value>,
) -> Node {
    Node {
        uuid: uuid.to_string(),
        node_type: NodeType::Http,
        name: name.to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::Http(HttpConfig {
            url: url.to_string(),
            method,
            headers,
            body,
        }),
        ports: None,
    }
}

/// 创建执行上下文
fn create_context(flow_id: &str, variables: serde_json::Value) -> ExecutionContext {
    ExecutionContext {
        flow_id: flow_id.to_string(),
        current_node_id: None,
        variables,
    }
}

// ============================================================================
// 测试：成功场景
// ============================================================================

#[tokio::test]
/// 测试GET请求成功场景
async fn test_http_success_get() {
    let node = create_http_node(
        "test-get",
        "HTTP GET请求",
        "https://httpbin.org/get",
        HttpMethod::Get,
        None,
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    // 验证执行成功
    assert!(result.is_ok(), "GET请求应该成功");
    let (next_node_id, output_data) = result.unwrap();
    assert!(next_node_id.is_none());

    // 验证输出数据
    assert!(output_data.is_some(), "应该有输出数据");
    let output = output_data.unwrap();
    assert_eq!(output["status"], 200, "状态码应该是200");
    assert!(output["body"].is_object(), "响应体应该是JSON对象");
}

#[tokio::test]
/// 测试POST请求成功场景
async fn test_http_success_post() {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let node = create_http_node(
        "test-post",
        "HTTP POST请求",
        "https://httpbin.org/post",
        HttpMethod::Post,
        Some(headers),
        Some(json!({
            "test_key": "test_value",
            "nested": {
                "field": "value"
            }
        })),
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    // 验证执行成功
    assert!(result.is_ok(), "POST请求应该成功");
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    let output = output_data.unwrap();
    assert_eq!(output["status"], 200);

    // 验证响应体包含请求数据
    if let Some(body) = output["body"].as_object() {
        assert!(body.contains_key("json"), "响应应该包含json字段");
        if let Some(json_data) = body.get("json").and_then(|v| v.as_object()) {
            assert_eq!(json_data.get("test_key").and_then(|v| v.as_str()), Some("test_value"));
        }
    }
}

#[tokio::test]
/// 测试PUT请求成功场景
async fn test_http_success_put() {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let node = create_http_node(
        "test-put",
        "HTTP PUT请求",
        "https://httpbin.org/put",
        HttpMethod::Put,
        Some(headers),
        Some(json!({
            "key": "put_value"
        })),
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    assert!(result.is_ok(), "PUT请求应该成功");
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    let output = output_data.unwrap();
    assert_eq!(output["status"], 200);
}

#[tokio::test]
/// 测试DELETE请求成功场景
async fn test_http_success_delete() {
    let node = create_http_node(
        "test-delete",
        "HTTP DELETE请求",
        "https://httpbin.org/delete",
        HttpMethod::Delete,
        None,
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    assert!(result.is_ok(), "DELETE请求应该成功");
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    let output = output_data.unwrap();
    assert_eq!(output["status"], 200);
}

#[tokio::test]
/// 测试PATCH请求成功场景
async fn test_http_success_patch() {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let node = create_http_node(
        "test-patch",
        "HTTP PATCH请求",
        "https://httpbin.org/patch",
        HttpMethod::Patch,
        Some(headers),
        Some(json!({
            "key": "patch_value"
        })),
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    assert!(result.is_ok(), "PATCH请求应该成功");
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    let output = output_data.unwrap();
    assert_eq!(output["status"], 200);
}

// ============================================================================
// 测试：失败场景（非2xx状态码）
// ============================================================================

#[tokio::test]
/// 测试HTTP 404错误场景
async fn test_http_failure_404() {
    let node = create_http_node(
        "test-404",
        "HTTP 404错误",
        "https://httpbin.org/status/404",
        HttpMethod::Get,
        None,
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    // 验证执行失败
    assert!(result.is_err(), "404错误应该返回错误");
    let error = result.unwrap_err();
    assert!(error.contains("404"), "错误消息应该包含404");
}

#[tokio::test]
/// 测试HTTP 500服务器错误场景
async fn test_http_failure_500() {
    let node = create_http_node(
        "test-500",
        "HTTP 500错误",
        "https://httpbin.org/status/500",
        HttpMethod::Get,
        None,
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    // 验证执行失败
    assert!(result.is_err(), "500错误应该返回错误");
    let error = result.unwrap_err();
    assert!(error.contains("500"), "错误消息应该包含500");
}

#[tokio::test]
/// 测试HTTP 403禁止访问错误场景
async fn test_http_failure_403() {
    let node = create_http_node(
        "test-403",
        "HTTP 403错误",
        "https://httpbin.org/status/403",
        HttpMethod::Get,
        None,
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    assert!(result.is_err(), "403错误应该返回错误");
    let error = result.unwrap_err();
    assert!(error.contains("403"), "错误消息应该包含403");
}

#[tokio::test]
/// 测试HTTP 401未授权错误场景
async fn test_http_failure_401() {
    let node = create_http_node(
        "test-401",
        "HTTP 401错误",
        "https://httpbin.org/status/401",
        HttpMethod::Get,
        None,
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    assert!(result.is_err(), "401错误应该返回错误");
    let error = result.unwrap_err();
    assert!(error.contains("401"), "错误消息应该包含401");
}

// ============================================================================
// 测试：网络错误场景
// ============================================================================

#[tokio::test]
/// 测试DNS解析失败场景（无效域名）
async fn test_http_error_dns_failure() {
    let node = create_http_node(
        "test-dns",
        "DNS解析失败",
        "http://this-domain-does-not-exist-12345.invalid",
        HttpMethod::Get,
        None,
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    // 验证执行失败（网络错误）
    assert!(result.is_err(), "DNS解析失败应该返回错误");
    let error = result.unwrap_err();
    // 错误消息应该包含网络相关的错误信息
    assert!(
        error.contains("HTTP请求失败") ||
        error.contains("dns") ||
        error.contains("connect") ||
        error.contains("timeout") ||
        error.contains("no such host"),
        "错误消息应该包含网络错误信息: {}",
        error
    );
}

#[tokio::test]
/// 测试连接超时场景（使用不可路由的IP）
async fn test_http_error_connection_timeout() {
    // 使用192.0.2.1（RFC 3330定义的测试地址，不可路由）
    let node = create_http_node(
        "test-timeout",
        "连接超时",
        "http://192.0.2.1:81", // 使用非标准端口
        HttpMethod::Get,
        None,
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    // 验证执行失败（超时或连接错误）
    assert!(result.is_err(), "连接超时应该返回错误");
    let error = result.unwrap_err();
    assert!(
        error.contains("HTTP请求失败") ||
        error.contains("timeout") ||
        error.contains("connect") ||
        error.contains("timed out"),
        "错误消息应该包含超时或连接错误信息: {}",
        error
    );
}

// ============================================================================
// 测试：变量替换场景
// ============================================================================

#[tokio::test]
/// 测试URL变量替换
async fn test_http_variable_replacement_url() {
    let node = create_http_node(
        "test-var-url",
        "URL变量替换",
        "https://httpbin.org/get?param={{test_param}}&user={{user_id}}",
        HttpMethod::Get,
        None,
        None,
    );

    let variables = json!({
        "test_param": "value123",
        "user_id": "user456"
    });

    let context = create_context("test-flow", variables);
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    // 验证执行成功
    assert!(result.is_ok(), "变量替换应该成功");
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    let output = output_data.unwrap();
    assert_eq!(output["status"], 200);

    // 验证响应体包含替换后的变量值
    if let Some(body) = output["body"].as_object() {
        if let Some(args) = body.get("args").and_then(|v| v.as_object()) {
            assert_eq!(
                args.get("param").and_then(|v| v.as_str()),
                Some("value123"),
                "URL参数应该被替换"
            );
            assert_eq!(
                args.get("user").and_then(|v| v.as_str()),
                Some("user456"),
                "URL参数应该被替换"
            );
        }
    }
}

#[tokio::test]
/// 测试请求头变量替换
async fn test_http_variable_replacement_headers() {
    let mut headers = HashMap::new();
    headers.insert("X-Custom-Header".to_string(), "{{custom_header}}".to_string());
    headers.insert("Authorization".to_string(), "Bearer {{token}}".to_string());

    let node = create_http_node(
        "test-var-headers",
        "请求头变量替换",
        "https://httpbin.org/headers",
        HttpMethod::Get,
        Some(headers),
        None,
    );

    let variables = json!({
        "custom_header": "header_value_123",
        "token": "secret_token_456"
    });

    let context = create_context("test-flow", variables);
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    // 验证执行成功
    assert!(result.is_ok(), "请求头变量替换应该成功");
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    let output = output_data.unwrap();
    assert_eq!(output["status"], 200);

    // 验证响应体包含替换后的请求头值
    if let Some(body) = output["body"].as_object() {
        if let Some(response_headers) = body.get("headers").and_then(|v| v.as_object()) {
            assert_eq!(
                response_headers.get("X-Custom-Header").and_then(|v| v.as_str()),
                Some("header_value_123"),
                "请求头应该被替换"
            );
            assert!(
                response_headers.get("Authorization")
                    .and_then(|v| v.as_str())
                    .map(|s| s.contains("secret_token_456"))
                    .unwrap_or(false),
                "Authorization头应该包含替换后的token"
            );
        }
    }
}

#[tokio::test]
/// 测试请求体变量替换
async fn test_http_variable_replacement_body() {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let node = create_http_node(
        "test-var-body",
        "请求体变量替换",
        "https://httpbin.org/post",
        HttpMethod::Post,
        Some(headers),
        Some(json!({
            "username": "{{username}}",
            "email": "{{email}}",
            "metadata": {
                "id": "{{user_id}}"
            }
        })),
    );

    let variables = json!({
        "username": "john_doe",
        "email": "john@example.com",
        "user_id": "12345"
    });

    let context = create_context("test-flow", variables);
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    // 验证执行成功
    assert!(result.is_ok(), "请求体变量替换应该成功");
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    let output = output_data.unwrap();
    assert_eq!(output["status"], 200);

    // 验证响应体包含替换后的请求体数据
    if let Some(body) = output["body"].as_object() {
        if let Some(json_data) = body.get("json").and_then(|v| v.as_object()) {
            assert_eq!(
                json_data.get("username").and_then(|v| v.as_str()),
                Some("john_doe"),
                "请求体字段应该被替换"
            );
            assert_eq!(
                json_data.get("email").and_then(|v| v.as_str()),
                Some("john@example.com"),
                "请求体字段应该被替换"
            );
            if let Some(metadata) = json_data.get("metadata").and_then(|v| v.as_object()) {
                assert_eq!(
                    metadata.get("id").and_then(|v| v.as_str()),
                    Some("12345"),
                    "嵌套字段应该被替换"
                );
            }
        }
    }
}

#[tokio::test]
/// 测试URL、请求头和请求体同时进行变量替换
async fn test_http_variable_replacement_all() {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("X-Request-ID".to_string(), "{{request_id}}".to_string());

    let node = create_http_node(
        "test-var-all",
        "全部变量替换",
        "https://httpbin.org/post?env={{env}}",
        HttpMethod::Post,
        Some(headers),
        Some(json!({
            "user": "{{username}}",
            "action": "{{action}}"
        })),
    );

    let variables = json!({
        "env": "production",
        "request_id": "req-12345",
        "username": "alice",
        "action": "create"
    });

    let context = create_context("test-flow", variables);
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    // 验证执行成功
    assert!(result.is_ok(), "全部变量替换应该成功");
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    let output = output_data.unwrap();
    assert_eq!(output["status"], 200);

        // 验证URL参数被替换
        if let Some(body) = output["body"].as_object() {
            if let Some(args) = body.get("args").and_then(|v| v.as_object()) {
                assert_eq!(
                    args.get("env").and_then(|v| v.as_str()),
                    Some("production"),
                    "URL参数应该被替换"
                );
            }

            // 验证请求体被替换
            if let Some(json_data) = body.get("json").and_then(|v| v.as_object()) {
                assert_eq!(
                    json_data.get("user").and_then(|v| v.as_str()),
                    Some("alice"),
                    "请求体字段应该被替换"
                );
                assert_eq!(
                    json_data.get("action").and_then(|v| v.as_str()),
                    Some("create"),
                    "请求体字段应该被替换"
                );
            }
        }
}

// ============================================================================
// 测试：响应数据处理
// ============================================================================

#[tokio::test]
/// 测试JSON响应解析
async fn test_http_response_json_parsing() {
    let node = create_http_node(
        "test-json-response",
        "JSON响应解析",
        "https://httpbin.org/json",
        HttpMethod::Get,
        None,
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    assert!(result.is_ok());
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    let output = output_data.unwrap();

    // 验证响应体被解析为JSON对象
    assert!(output["body"].is_object(), "JSON响应应该被解析为对象");

    // 验证响应头存在
    assert!(output["headers"].is_object(), "响应头应该存在");
    let headers = output["headers"].as_object().unwrap();
    assert!(headers.contains_key("content-type"), "应该包含content-type头");
}

#[tokio::test]
/// 测试文本响应处理（非JSON）
async fn test_http_response_text_handling() {
    let node = create_http_node(
        "test-text-response",
        "文本响应处理",
        "https://httpbin.org/encoding/utf8",
        HttpMethod::Get,
        None,
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    assert!(result.is_ok());
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    let output = output_data.unwrap();

    // 验证文本响应被处理为字符串
    assert!(output["body"].is_string(), "文本响应应该被处理为字符串");
}

#[tokio::test]
/// 测试响应头收集
async fn test_http_response_headers_collection() {
    let node = create_http_node(
        "test-headers",
        "响应头收集",
        "https://httpbin.org/get",
        HttpMethod::Get,
        None,
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    assert!(result.is_ok());
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    let output = output_data.unwrap();

    // 验证响应头被收集
    assert!(output["headers"].is_object());
    let headers = output["headers"].as_object().unwrap();
    assert!(!headers.is_empty(), "响应头不应该为空");

    // 验证常见响应头存在
    assert!(headers.contains_key("content-type"), "应该包含content-type");
    assert!(headers.contains_key("date"), "应该包含date");
}

// ============================================================================
// 测试：边界情况
// ============================================================================

#[tokio::test]
/// 测试空请求头
async fn test_http_empty_headers() {
    let node = create_http_node(
        "test-empty-headers",
        "空请求头",
        "https://httpbin.org/get",
        HttpMethod::Get,
        None,
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    assert!(result.is_ok(), "空请求头应该成功");
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    assert_eq!(output_data.unwrap()["status"], 200);
}

#[tokio::test]
/// 测试空请求体
async fn test_http_empty_body() {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let node = create_http_node(
        "test-empty-body",
        "空请求体",
        "https://httpbin.org/post",
        HttpMethod::Post,
        Some(headers),
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    assert!(result.is_ok(), "空请求体应该成功");
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    assert_eq!(output_data.unwrap()["status"], 200);
}

#[tokio::test]
/// 测试URL编码处理
async fn test_http_url_encoding() {
    let node = create_http_node(
        "test-url-encoding",
        "URL编码",
        "https://httpbin.org/get?query=hello world&name=test user",
        HttpMethod::Get,
        None,
        None,
    );

    let context = create_context("test-flow", json!({}));
    let executor = HttpNodeExecutor::new();

    let result: Result<(Option<String>, Option<serde_json::Value>), String> =
        executor.execute(&node, &context, &[], &HashMap::new()).await;

    assert!(result.is_ok(), "URL编码应该被正确处理");
    let output_data = result.unwrap().1;
    assert!(output_data.is_some());
    let output = output_data.unwrap();

    // 验证请求成功
    assert_eq!(output["status"], 200);

    // 验证URL参数被正确解析
    if let Some(body) = output["body"].as_object() {
        if let Some(args) = body.get("args").and_then(|v| v.as_object()) {
            // httpbin会自动解码URL参数
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            assert!(query.contains("hello") || query.contains("world"));
            assert!(name.contains("test") || name.contains("user"));
        }
    }
}
