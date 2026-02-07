use crate::connection::Connection;
use crate::executor::{ExecutionContext, NodeExecutor};
use crate::node::Node;
use std::collections::HashMap;
use std::time::Instant;

/// HTTP节点执行器
pub struct HttpNodeExecutor;

impl HttpNodeExecutor {
    /// 创建新的HTTP节点执行器
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl NodeExecutor for HttpNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &ExecutionContext,
        _connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Option<String>, Option<serde_json::Value>), String> {
        // 执行前日志
        let start_time = Instant::now();
        log::info!("[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: http", context.flow_id, node.uuid, node.name);

        // 从Node的data字段提取HttpConfig
        let http_config = match &node.data {
            crate::node_config::NodeData::Http(config) => config.clone(),
            _ => {
                let error_msg = format!("节点数据类型不是HttpConfig: {}", node.uuid);
                log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_id, node.uuid, node.name, error_msg);
                return Err(error_msg);
            }
        };

        // 使用replace_variables替换URL
        let url = crate::executor::replace_variables(&http_config.url, context)
            .map_err(|e| {
                let error_msg = format!("URL变量替换失败: {}", e);
                log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_id, node.uuid, node.name, error_msg);
                error_msg
            })?;

        // HTTP请求日志
        let method_str = format!("{:?}", http_config.method).to_uppercase();
        log::info!("[流程 {}] [HTTP请求] URL: {}, 方法: {}", context.flow_id, url, method_str);

        // 创建HTTP客户端（默认超时30秒）
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| {
                let error_msg = format!("创建HTTP客户端失败: {}", e);
                log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_id, node.uuid, node.name, error_msg);
                error_msg
            })?;

        // 构建请求
        let mut request = match http_config.method {
            crate::node_config::HttpMethod::Get => client.get(&url),
            crate::node_config::HttpMethod::Post => client.post(&url),
            crate::node_config::HttpMethod::Put => client.put(&url),
            crate::node_config::HttpMethod::Delete => client.delete(&url),
            crate::node_config::HttpMethod::Patch => client.patch(&url),
        };

        // 添加请求头（替换变量）
        if let Some(headers) = &http_config.headers {
            for (key, value) in headers {
                let replaced_value = crate::executor::replace_variables(value, context)
                    .map_err(|e| {
                        let error_msg = format!("请求头变量替换失败 ({}): {}", key, e);
                        log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_id, node.uuid, node.name, error_msg);
                        error_msg
                    })?;
                request = request.header(key, replaced_value);
            }
        }

        // 添加请求体（替换变量）
        if let Some(body) = &http_config.body {
            let body_str = body.to_string();
            let replaced_body = crate::executor::replace_variables(&body_str, context)
                .map_err(|e| {
                    let error_msg = format!("请求体变量替换失败: {}", e);
                    log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_id, node.uuid, node.name, error_msg);
                    error_msg
                })?;
            request = request.body(replaced_body);
        }

        // 记录HTTP请求开始时间
        let http_start_time = Instant::now();

        // 发送请求并获取响应
        let response = request
            .send()
            .await
            .map_err(|e| {
                let error_msg = format!("HTTP请求失败: {}", e);
                log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_id, node.uuid, node.name, error_msg);
                error_msg
            })?;

        let status = response.status();
        let status_code = status.as_u16();
        let http_duration_ms = http_start_time.elapsed().as_millis();

        // HTTP响应日志
        log::info!(
            "[流程 {}] [HTTP响应] 状态码: {}, 耗时: {}ms",
            context.flow_id,
            status_code,
            http_duration_ms
        );

        // 验证HTTP状态码（2xx视为成功，其他视为失败）
        if !(200..300).contains(&status_code) {
            let error_msg = format!(
                "HTTP请求失败，状态码: {} {}",
                status_code,
                status.canonical_reason().unwrap_or("Unknown")
            );
            log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_id, node.uuid, node.name, error_msg);
            return Err(error_msg);
        }

        // 获取响应头（在移动response之前）
        let headers_map: std::collections::HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // 检查Content-Type（在移动response之前）
        let is_json = response
            .headers()
            .get("content-type")
            .and_then(|ct| ct.to_str().ok())
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false);

        // 获取响应体
        let body_text = response
            .text()
            .await
            .map_err(|e| {
                let error_msg = format!("读取响应体失败: {}", e);
                log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_id, node.uuid, node.name, error_msg);
                error_msg
            })?;

        // 解析响应体为JSON（如果Content-Type是application/json）
        let body_json = if is_json {
            serde_json::from_str(&body_text)
                .map_err(|e| {
                    let error_msg = format!("解析JSON响应体失败: {}", e);
                    log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_id, node.uuid, node.name, error_msg);
                    error_msg
                })?
        } else {
            serde_json::Value::String(body_text)
        };

        // 返回输出数据（status、headers、body）
        let output_data = serde_json::json!({
            "status": status_code,
            "headers": headers_map,
            "body": body_json
        });

        // 计算总执行时间
        let duration_ms = start_time.elapsed().as_millis();

        // 执行后日志
        log::info!("[流程 {}] [完成] 节点ID: {}, 耗时: {}ms, 状态: success", context.flow_id, node.uuid, duration_ms);

        Ok((None, Some(output_data)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Node, NodePosition, NodeType};
    use crate::node_config::{HttpConfig, HttpMethod};
    use serde_json::json;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_http_executor_get_request() {
        // 创建HTTP GET请求节点
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP GET请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::Http(HttpConfig {
                url: "https://httpbin.org/get".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body: None,
            }),
            ports: None,
        };

        let context = crate::executor::ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({}),
        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok());
        let (next_node_id, output_data) = result.unwrap();
        assert!(next_node_id.is_none()); // 测试节点没有连接

        // 验证输出数据包含响应
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert!(output["status"].is_number());
        assert_eq!(output["status"], 200);
        assert!(output["body"].is_object());
    }

    #[tokio::test]
    async fn test_http_executor_post_request() {
        // 创建HTTP POST请求节点
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP POST请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::Http(HttpConfig {
                url: "https://httpbin.org/post".to_string(),
                method: HttpMethod::Post,
                headers: Some({
                    let mut headers = HashMap::new();
                    headers.insert("Content-Type".to_string(), "application/json".to_string());
                    headers
                }),
                body: Some(json!({
                    "test_key": "test_value"
                })),
            }),
            ports: None,
        };

        let context = crate::executor::ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({}),
        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok());
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);
    }

    #[tokio::test]
    async fn test_http_executor_404_error() {
        // 创建HTTP 404错误请求
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 404请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::Http(HttpConfig {
                url: "https://httpbin.org/status/404".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body: None,
            }),
            ports: None,
        };

        let context = crate::executor::ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({}),
        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &context, &[], &HashMap::new()).await;

        // 验证执行失败（404错误）
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("404"));
    }

    #[tokio::test]
    async fn test_http_executor_200_success() {
        // 测试HTTP 200成功状态码
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 200成功".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::Http(HttpConfig {
                url: "https://httpbin.org/status/200".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body: None,
            }),
            ports: None,
        };

        let context = crate::executor::ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({}),
        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok());
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);
    }

    #[tokio::test]
    async fn test_http_executor_500_error() {
        // 测试HTTP 500服务器错误
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 500错误".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::Http(HttpConfig {
                url: "https://httpbin.org/status/500".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body: None,
            }),
            ports: None,
        };

        let context = crate::executor::ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({}),
        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &context, &[], &HashMap::new()).await;

        // 验证执行失败（500错误）
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("500"));
    }

    #[tokio::test]
    async fn test_http_executor_variable_replacement() {
        // 测试变量替换（URL、headers、body）
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP变量替换".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::Http(HttpConfig {
                url: "https://httpbin.org/get?param={{test_param}}".to_string(),
                method: HttpMethod::Get,
                headers: Some({
                    let mut headers = HashMap::new();
                    headers.insert("X-Custom-Header".to_string(), "{{test_header}}".to_string());
                    headers
                }),
                body: Some(json!({
                    "key": "{{test_body}}"
                })),
            }),
            ports: None,
        };

        let context = crate::executor::ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({
                "test_param": "value123",
                "test_header": "header_value",
                "test_body": "body_value"
            }),
        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok());
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);
        
        // 验证响应体包含替换后的变量值
        if let Some(body) = output["body"].as_object() {
            if let Some(args) = body.get("args").and_then(|v| v.as_object()) {
                assert_eq!(args.get("param").and_then(|v| v.as_str()), Some("value123"));
            }
            if let Some(headers) = body.get("headers").and_then(|v| v.as_object()) {
                assert_eq!(headers.get("X-Custom-Header").and_then(|v| v.as_str()), Some("header_value"));
            }
        }
    }

    #[tokio::test]
    async fn test_http_executor_network_error() {
        // 测试网络错误（无效URL）
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP网络错误".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::Http(HttpConfig {
                url: "http://this-domain-does-not-exist-12345.invalid".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body: None,
            }),
            ports: None,
        };

        let context = crate::executor::ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({}),
        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &context, &[], &HashMap::new()).await;

        // 验证执行失败（网络错误）
        assert!(result.is_err());
        let error = result.unwrap_err();
        // 错误消息应该包含"HTTP请求失败"或相关的网络错误信息
        assert!(error.contains("HTTP请求失败") || error.contains("dns") || error.contains("connect") || error.contains("timeout"));
    }

    #[tokio::test]
    async fn test_http_executor_json_response() {
        // 测试JSON响应解析
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP JSON响应".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::Http(HttpConfig {
                url: "https://httpbin.org/json".to_string(),
                method: HttpMethod::Get,
                headers: Some({
                    let mut headers = HashMap::new();
                    headers.insert("Accept".to_string(), "application/json".to_string());
                    headers
                }),
                body: None,
            }),
            ports: None,
        };

        let context = crate::executor::ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({}),
        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok());
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);
        
        // 验证响应体被解析为JSON对象
        assert!(output["body"].is_object());
    }

    #[tokio::test]
    async fn test_http_executor_text_response() {
        // 测试非JSON响应处理
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP文本响应".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::Http(HttpConfig {
                url: "https://httpbin.org/encoding/utf8".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body: None,
            }),
            ports: None,
        };

        let context = crate::executor::ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({}),
        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok());
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);
        
        // 验证响应体为字符串（非JSON）
        assert!(output["body"].is_string());
    }

    #[tokio::test]
    async fn test_http_executor_put_request() {
        // 测试PUT请求
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP PUT请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::Http(HttpConfig {
                url: "https://httpbin.org/put".to_string(),
                method: HttpMethod::Put,
                headers: Some({
                    let mut headers = HashMap::new();
                    headers.insert("Content-Type".to_string(), "application/json".to_string());
                    headers
                }),
                body: Some(json!({
                    "test": "put_data"
                })),
            }),
            ports: None,
        };

        let context = crate::executor::ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({}),
        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok());
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);
    }

    #[tokio::test]
    async fn test_http_executor_delete_request() {
        // 测试DELETE请求
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP DELETE请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::Http(HttpConfig {
                url: "https://httpbin.org/delete".to_string(),
                method: HttpMethod::Delete,
                headers: None,
                body: None,
            }),
            ports: None,
        };

        let context = crate::executor::ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({}),
        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok());
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);
    }

    #[tokio::test]
    async fn test_http_executor_patch_request() {
        // 测试PATCH请求
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP PATCH请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::Http(HttpConfig {
                url: "https://httpbin.org/patch".to_string(),
                method: HttpMethod::Patch,
                headers: Some({
                    let mut headers = HashMap::new();
                    headers.insert("Content-Type".to_string(), "application/json".to_string());
                    headers
                }),
                body: Some(json!({
                    "test": "patch_data"
                })),
            }),
            ports: None,
        };

        let context = crate::executor::ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({}),
        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok());
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);
    }

    #[tokio::test]
    async fn test_http_executor_response_headers() {
        // 测试响应头处理
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP响应头".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: crate::node_config::NodeData::Http(HttpConfig {
                url: "https://httpbin.org/get".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body: None,
            }),
            ports: None,
        };

        let context = crate::executor::ExecutionContext {
            flow_id: flow_id.clone(),
            current_node_id: Some(http_node.uuid.clone()),
            variables: json!({}),
        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok());
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        
        // 验证响应头存在
        assert!(output["headers"].is_object());
        let headers = output["headers"].as_object().unwrap();
        assert!(headers.contains_key("content-type"));
    }
}
