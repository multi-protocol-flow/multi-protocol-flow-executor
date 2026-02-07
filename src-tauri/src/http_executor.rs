use crate::connection::Connection;
use crate::executor::{ExecutionContext, NodeExecutor};
use crate::node::Node;
use std::collections::HashMap;

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
        log::info!("执行HTTP节点: {}", node.name);

        // 从Node的data字段提取HttpConfig
        let http_config = match &node.data {
            crate::node_config::NodeData::Http(config) => config.clone(),
            _ => return Err(format!("节点数据类型不是HttpConfig: {}", node.uuid)),
        };

        // 使用replace_variables替换URL
        let url = crate::executor::replace_variables(&http_config.url, context)
            .map_err(|e| format!("URL变量替换失败: {}", e))?;

        log::info!(
            "发送HTTP请求: {} {}",
            format!("{:?}", http_config.method).to_uppercase(),
            url
        );

        // 创建HTTP客户端（默认超时30秒）
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

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
                    .map_err(|e| format!("请求头变量替换失败 ({}): {}", key, e))?;
                request = request.header(key, replaced_value);
            }
        }

        // 添加请求体（替换变量）
        if let Some(body) = &http_config.body {
            let body_str = body.to_string();
            let replaced_body = crate::executor::replace_variables(&body_str, context)
                .map_err(|e| format!("请求体变量替换失败: {}", e))?;
            request = request.body(replaced_body);
        }

        // 发送请求并获取响应
        let response = request
            .send()
            .await
            .map_err(|e| format!("HTTP请求失败: {}", e))?;

        let status = response.status();
        let status_code = status.as_u16();

        log::info!(
            "HTTP响应状态码: {} {}",
            status_code,
            status.canonical_reason().unwrap_or("Unknown")
        );

        // 验证HTTP状态码（2xx视为成功，其他视为失败）
        if !(200..300).contains(&status_code) {
            let error_msg = format!(
                "HTTP请求失败，状态码: {} {}",
                status_code,
                status.canonical_reason().unwrap_or("Unknown")
            );
            log::error!("{}", error_msg);
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
            .map_err(|e| format!("读取响应体失败: {}", e))?;

        // 解析响应体为JSON（如果Content-Type是application/json）
        let body_json = if is_json {
            serde_json::from_str(&body_text)
                .map_err(|e| format!("解析JSON响应体失败: {}", e))?
        } else {
            serde_json::Value::String(body_text)
        };

        // 返回输出数据（status、headers、body）
        let output_data = serde_json::json!({
            "status": status_code,
            "headers": headers_map,
            "body": body_json
        });

        log::info!("HTTP请求成功完成: {}", url);

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
}
