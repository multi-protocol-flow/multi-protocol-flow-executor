use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::executor::NodeExecutor;
use crate::http::{BodyType, HttpClient, HttpRequestConfig, MultipartBody};
use flow_engine_core::log_event::{LogEvent, LogLevel};
use flow_engine_core::node::Node;
use base64::Engine;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

// Re-export HttpTiming for external use
pub use crate::http::HttpTiming;

/// HTTP节点执行器
pub struct HttpNodeExecutor;

impl Default for HttpNodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpNodeExecutor {
    /// 创建新的HTTP节点执行器
    pub fn new() -> Self {
        Self
    }
}

/// Convert `ContentType` to Content-Type header value
fn content_type_to_string(content_type: &flow_engine_core::node_config::ContentType) -> &'static str {
    match content_type {
        flow_engine_core::node_config::ContentType::Json => "application/json",
        flow_engine_core::node_config::ContentType::Text => "text/plain",
        flow_engine_core::node_config::ContentType::Html => "text/html",
        flow_engine_core::node_config::ContentType::Xml => "application/xml",
        flow_engine_core::node_config::ContentType::Javascript => "application/javascript",
    }
}

/// Check if Content-Type header is already set in headers
fn has_content_type_header(headers: &Option<std::collections::HashMap<String, String>>) -> bool {
    match headers {
        Some(h) => h.keys().any(|k| k.to_lowercase() == "content-type"),
        None => false,
    }
}


#[async_trait::async_trait]
impl NodeExecutor for HttpNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), String> {
        // 执行前日志
        let start_time = Instant::now();
        log::info!("[流程 {}] [开始] 节点ID: {}, 节点名称: {}, 类型: http", context.flow_uuid, node.uuid, node.name);

        // 发送执行开始日志到前端
        context.emit_log(LogEvent {
            level: LogLevel::Info,
            message: format!("HTTP 节点 {} 开始执行", node.name),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            node_uuid: Some(node.uuid.clone()),
            node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone(),
        }).await;
        // 从Node的data字段提取HttpConfig
        let http_config = match &node.data {
            flow_engine_core::node_config::NodeData::Http(config) => config.clone(),
            _ => {
                let error_msg = format!("节点数据类型不是HttpConfig: {}", node.uuid);
                log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
                return Err(error_msg);
            }
        };

        // 使用replace_variables替换URL
        let mut url = flow_engine_core::executor::replace_variables(&http_config.url, context)
            .map_err(|e| {
                let error_msg = format!("URL变量替换失败: {}", e);
                log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
                error_msg
            })?;

        // HTTP请求日志
        let method_str = format!("{:?}", http_config.method).to_uppercase();
        log::info!("[流程 {}] [HTTP请求] URL: {}, 方法: {}", context.flow_uuid, url, method_str);

        // 发送HTTP请求日志到前端
        context.emit_log(LogEvent {
            level: LogLevel::Info,
            message: format!("HTTP {} {}", method_str, url),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            node_uuid: Some(node.uuid.clone()),
            node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone(),
        }).await;

        // 创建HTTP客户端（超时可配置，默认30秒，范围1-300秒）
        let timeout_ms = http_config.timeout_ms
            .map(|t| t.clamp(1000, 300000))
            .unwrap_or(30000);

        // 检查是否有代理状态（从 context.providers["proxy_state"] 读取）
        let proxy_state: Option<Arc<flow_engine_core::context::ProxyState>> =
            context.get_provider("proxy_state");

        let client = match (
            http_config.tls_skip_verify.unwrap_or(false),
            &http_config.tls_ca_cert_path,
            &http_config.tls_client_cert_path,
            &http_config.tls_client_key_path,
        ) {
            (true, None, None, None) => {
                use crate::http::connector::TlsOptions;
                let c = HttpClient::with_tls(timeout_ms, TlsOptions {
                    skip_verify: true, ca_cert_path: None, client_cert_path: None, client_key_path: None,
                });
                match proxy_state {
                    Some(ps) => c.with_proxy(ps),
                    None => c,
                }
            }
            (_, Some(_), _, _) | (_, _, Some(_), _) | (_, _, _, Some(_)) => {
                use crate::http::connector::TlsOptions;
                let c = HttpClient::with_tls(timeout_ms, TlsOptions {
                    skip_verify: http_config.tls_skip_verify.unwrap_or(false),
                    ca_cert_path: http_config.tls_ca_cert_path.clone(),
                    client_cert_path: http_config.tls_client_cert_path.clone(),
                    client_key_path: http_config.tls_client_key_path.clone(),
                });
                match proxy_state {
                    Some(ps) => c.with_proxy(ps),
                    None => c,
                }
            }
            _ => {
                let c = HttpClient::new(timeout_ms);
                match proxy_state {
                    Some(ps) => c.with_proxy(ps),
                    None => c,
                }
            }
        };

        // 记录发送的请求头
        let mut sent_headers: HashMap<String, String> = HashMap::new();
        
        // 记录请求体信息（用于输出）
        let mut request_body_json: Option<serde_json::Value> = None;
        
        // 处理请求头（替换变量）
        let mut headers: HashMap<String, String> = HashMap::new();
        if let Some(config_headers) = &http_config.headers {
            for (key, value) in config_headers {
                // 替换 header key 中的变量
                let replaced_key = flow_engine_core::executor::replace_variables(key, context)
                    .map_err(|e| {
                        let error_msg = format!("请求头 key 变量替换失败 ({}): {}", key, e);
                        log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
                        error_msg
                    })?;
                // 替换 header value 中的变量
                let replaced_value = flow_engine_core::executor::replace_variables(value, context)
                    .map_err(|e| {
                        let error_msg = format!("请求头 value 变量替换失败 ({}): {}", key, e);
                        log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
                        error_msg
                    })?;
                headers.insert(replaced_key.clone(), replaced_value.clone());
                sent_headers.insert(replaced_key, replaced_value);
            }
        }

        // 应用认证配置
        if let Some(auth) = &http_config.auth {
            match &auth.auth_type {
                flow_engine_core::node_config::HttpAuthType::None => {}
                flow_engine_core::node_config::HttpAuthType::Basic => {
                    if let (Some(username), Some(password)) = (&auth.username, &auth.password) {
                        let replaced_user = flow_engine_core::executor::replace_variables(username, context)
                            .map_err(|e| format!("认证用户名变量替换失败: {}", e))?;
                        let replaced_pass = flow_engine_core::executor::replace_variables(password, context)
                            .map_err(|e| format!("认证密码变量替换失败: {}", e))?;
                        let credentials = base64::engine::general_purpose::STANDARD
                            .encode(format!("{}:{}", replaced_user, replaced_pass));
                        headers.insert("Authorization".to_string(), format!("Basic {}", credentials));
                        sent_headers.insert("Authorization".to_string(), "[Basic Auth]".to_string());
                    }
                }
                flow_engine_core::node_config::HttpAuthType::Bearer => {
                    if let Some(token) = &auth.token {
                        let replaced = flow_engine_core::executor::replace_variables(token, context)
                            .map_err(|e| format!("Bearer Token 变量替换失败: {}", e))?;
                        headers.insert("Authorization".to_string(), format!("Bearer {}", replaced));
                        sent_headers.insert("Authorization".to_string(), "[Bearer Token]".to_string());
                    }
                }
                flow_engine_core::node_config::HttpAuthType::ApiKeyHeader => {
                    if let (Some(name), Some(value)) = (&auth.api_key_name, &auth.api_key_value) {
                        let rn = flow_engine_core::executor::replace_variables(name, context)
                            .map_err(|e| format!("API Key 名称变量替换失败: {}", e))?;
                        let rv = flow_engine_core::executor::replace_variables(value, context)
                            .map_err(|e| format!("API Key 值变量替换失败: {}", e))?;
                        headers.insert(rn.clone(), rv);
                        sent_headers.insert(rn, "[API Key]".to_string());
                    }
                }
                flow_engine_core::node_config::HttpAuthType::ApiKeyQuery => {
                    if let (Some(name), Some(value)) = (&auth.api_key_name, &auth.api_key_value) {
                        let rn = flow_engine_core::executor::replace_variables(name, context)
                            .map_err(|e| format!("API Key 名称变量替换失败: {}", e))?;
                        let rv = flow_engine_core::executor::replace_variables(value, context)
                            .map_err(|e| format!("API Key 值变量替换失败: {}", e))?;
                        let sep = if url.contains('?') { "&" } else { "?" };
                        url = format!("{}{}{}={}", url, sep, rn, rv);
                    }
                }
                flow_engine_core::node_config::HttpAuthType::Digest => {
                    log::warn!("[流程 {}] [HTTP] Digest 认证暂不支持", context.flow_uuid);
                }
            }
        }

        // Handle request body based on body_type
        // Content-Type priority: only set if user hasn't manually set Content-Type in headers
        let user_set_content_type = has_content_type_header(&http_config.headers);

        // 请求体类型字符串
        let body_type_str = match http_config.body_type {
            flow_engine_core::node_config::HttpBodyType::None => "none",
            flow_engine_core::node_config::HttpBodyType::Body => "body",
            flow_engine_core::node_config::HttpBodyType::FormData => "form-data",
            flow_engine_core::node_config::HttpBodyType::XWwwFormUrlencoded => "x-www-form-urlencoded",
            flow_engine_core::node_config::HttpBodyType::Binary => "binary",
        };

        // 构建请求体
        let body: BodyType = match http_config.body_type {
            flow_engine_core::node_config::HttpBodyType::None => {
                log::info!("[流程 {}] [HTTP] body_type=none, 不添加请求体", context.flow_uuid);
                BodyType::None
            }
            flow_engine_core::node_config::HttpBodyType::Body => {
                if let Some(body_content) = &http_config.body {
                    let replaced_body = flow_engine_core::executor::replace_variables(body_content, context)
                        .map_err(|e| {
                            let error_msg = format!("请求体变量替换失败: {}", e);
                            log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", 
                                context.flow_uuid, node.uuid, node.name, error_msg);
                            error_msg
                        })?;
                    
                    // Set Content-Type header only if user hasn't set it manually
                    if !user_set_content_type {
                        let content_type_str = content_type_to_string(&http_config.content_type);
                        headers.insert("Content-Type".to_string(), content_type_str.to_string());
                        sent_headers.insert("Content-Type".to_string(), content_type_str.to_string());
                        log::info!("[流程 {}] [HTTP] 自动设置 Content-Type: {}", 
                            context.flow_uuid, content_type_str);
                    }
                    
                    request_body_json = Some(replaced_body.clone().into());
                    log::info!("[流程 {}] [HTTP] body_type=body, 请求体已添加", context.flow_uuid);
                    BodyType::raw(replaced_body.into_bytes())
                } else {
                    log::info!("[流程 {}] [HTTP] body_type=body, 但 body 字段为空，跳过", context.flow_uuid);
                    BodyType::None
                }
            }
            flow_engine_core::node_config::HttpBodyType::FormData => {
                if let Some(form_fields) = &http_config.form_fields {
                    let mut multipart = MultipartBody::new();
                    let mut text_field_count = 0;
                    let mut file_field_count = 0;
                    
                    for field in form_fields {
                        match field.field_type {
                            flow_engine_core::node_config::FormFieldType::Text => {
                                let replaced_key = flow_engine_core::executor::replace_variables(&field.key, context)
                                    .map_err(|e| format!("表单字段 key 变量替换失败: {}", e))?;
                                let replaced_value = flow_engine_core::executor::replace_variables(&field.value, context)
                                    .map_err(|e| format!("表单字段 value 变量替换失败: {}", e))?;
                                
                                multipart = multipart.text(replaced_key, replaced_value);
                                text_field_count += 1;
                            }
                            flow_engine_core::node_config::FormFieldType::File => {
                                if let Some(file_path) = &field.file_path {
                                    let replaced_path = flow_engine_core::executor::replace_variables(file_path, context)
                                        .map_err(|e| format!("文件路径变量替换失败: {}", e))?;
                                    
                                    let resolved_path = context.resolve_file_path(&replaced_path);
                                    
                                    log::info!("[流程 {}] [HTTP] 读取表单文件: {:?}", context.flow_uuid, resolved_path);
                                    
                                    let file_content = tokio::fs::read(&resolved_path).await
                                        .map_err(|e| {
                                            let error_msg = format!("读取文件失败 ({}): {}", resolved_path.display(), e);
                                            log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", 
                                                context.flow_uuid, node.uuid, node.name, error_msg);
                                            error_msg
                                        })?;
                                    
                                    let mime_type = mime_guess::from_path(&resolved_path)
                                        .first_or_octet_stream();
                                    
                                    let file_name = resolved_path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("upload");
                                    
                                    let replaced_key = flow_engine_core::executor::replace_variables(&field.key, context)
                                        .map_err(|e| format!("表单字段 key 变量替换失败: {}", e))?;
                                    
                                    multipart = multipart.file(
                                        replaced_key,
                                        file_name.to_string(),
                                        mime_type.essence_str().to_string(),
                                        file_content,
                                    );
                                    file_field_count += 1;
                                } else {
                                    log::warn!("[流程 {}] [HTTP] 文件字段 '{}' 缺少 file_path，跳过", 
                                        context.flow_uuid, field.key);
                                }
                            }
                        }
                    }
                    
                    if !user_set_content_type {
                        sent_headers.insert("Content-Type".to_string(), "multipart/form-data (boundary auto-generated)".to_string());
                        log::info!("[流程 {}] [HTTP] 将自动设置 Content-Type: multipart/form-data", context.flow_uuid);
                    }
                    
                    // 记录 form-data 字段元数据
                    let fields_metadata: Vec<serde_json::Value> = form_fields.iter().map(|f| {
                        match f.field_type {
                            flow_engine_core::node_config::FormFieldType::Text => {
                                serde_json::json!({
                                    "type": "text",
                                    "key": f.key,
                                    "value": f.value
                                })
                            }
                            flow_engine_core::node_config::FormFieldType::File => {
                                serde_json::json!({
                                    "type": "file",
                                    "key": f.key,
                                    "path": f.file_path
                                })
                            }
                        }
                    }).collect();
                    request_body_json = Some(serde_json::json!({
                        "type": "form-data",
                        "fields": fields_metadata
                    }));
                    
                    log::info!("[流程 {}] [HTTP] body_type=form-data, {} 个文本字段, {} 个文件字段", 
                        context.flow_uuid, text_field_count, file_field_count);
                    BodyType::multipart(multipart)
                } else {
                    log::warn!("[流程 {}] [HTTP] body_type=form-data, 但 form_fields 为空", context.flow_uuid);
                    BodyType::None
                }
            }
            flow_engine_core::node_config::HttpBodyType::XWwwFormUrlencoded => {
                if let Some(form_fields) = &http_config.form_fields {
                    let mut form_data: Vec<(String, String)> = Vec::new();
                    
                    for field in form_fields {
                        if matches!(field.field_type, flow_engine_core::node_config::FormFieldType::Text) {
                            let replaced_key = flow_engine_core::executor::replace_variables(&field.key, context)
                                .map_err(|e| format!("Form field key variable replacement failed: {}", e))?;
                            let replaced_value = flow_engine_core::executor::replace_variables(&field.value, context)
                                .map_err(|e| format!("Form field value variable replacement failed: {}", e))?;
                            form_data.push((replaced_key, replaced_value));
                        }
                    }
                    
                    if !user_set_content_type {
                        headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
                        sent_headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
                        log::info!("[流程 {}] [HTTP] 自动设置 Content-Type: application/x-www-form-urlencoded", context.flow_uuid);
                    }
                    
                    let fields_metadata: Vec<serde_json::Value> = form_data.iter().map(|(k, v)| {
                        serde_json::json!({
                            "key": k,
                            "value": v
                        })
                    }).collect();
                    request_body_json = Some(serde_json::json!({
                        "type": "urlencoded",
                        "fields": fields_metadata
                    }));
                    
                    log::info!("[流程 {}] [HTTP] body_type=x-www-form-urlencoded, {} fields", context.flow_uuid, form_data.len());
                    BodyType::urlencoded(form_data)
                } else {
                    log::warn!("[流程 {}] [HTTP] body_type=x-www-form-urlencoded, but form_fields is empty", context.flow_uuid);
                    BodyType::None
                }
            }
            flow_engine_core::node_config::HttpBodyType::Binary => {
                if let Some(file_path) = &http_config.file_path {
                    let resolved_path = context.resolve_file_path(file_path);
                    
                    log::info!("[流程 {}] [HTTP] body_type=binary, 读取文件: {:?}", context.flow_uuid, resolved_path);
                    
                    let file_content = match tokio::fs::read(&resolved_path).await {
                        Ok(content) => content,
                        Err(e) => {
                            let error_msg = format!("读取文件失败 ({}): {}", resolved_path.display(), e);
                            log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", 
                                context.flow_uuid, node.uuid, node.name, error_msg);
                            return Err(error_msg);
                        }
                    };
                    
                    if file_content.len() > 10 * 1024 * 1024 {
                        log::warn!("[流程 {}] [HTTP] 文件较大 ({} bytes), 可能占用较多内存", 
                            context.flow_uuid, file_content.len());
                    }
                    
                    if !user_set_content_type {
                        headers.insert("Content-Type".to_string(), "application/octet-stream".to_string());
                        sent_headers.insert("Content-Type".to_string(), "application/octet-stream".to_string());
                        log::info!("[流程 {}] [HTTP] 自动设置 Content-Type: application/octet-stream", context.flow_uuid);
                    }
                    
                    let file_size = file_content.len();
                    request_body_json = Some(serde_json::json!({
                        "type": "binary",
                        "path": file_path,
                        "size": file_size
                    }));
                    log::info!("[流程 {}] [HTTP] body_type=binary, 文件大小: {} bytes", context.flow_uuid, file_size);
                    BodyType::raw(file_content)
                } else {
                    let error_msg = "body_type=binary 但 file_path 为空".to_string();
                    log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", 
                        context.flow_uuid, node.uuid, node.name, error_msg);
                    return Err(error_msg);
                }
            }
        };

        // Cookie 管理：从初始 cookie_jar 创建 CookieJar 并注入请求头
        let cookie_jar = if http_config.enable_cookies.unwrap_or(false) {
            let jar = crate::http::cookie::CookieJar::from_initial(
                http_config.cookie_jar.as_deref().unwrap_or(&[]),
            );
            if !jar.is_empty() {
                let header = jar.cookie_header(&url);
                headers.insert("Cookie".to_string(), header.clone());
                sent_headers.insert("Cookie".to_string(), header);
            }
            Some(jar)
        } else {
            None
        };

        // 构建请求配置
        let config = HttpRequestConfig {
            url: url.clone(),
            method: method_str.clone(),
            headers,
            body,
            multipart_boundary: None,
            accept_encoding: http_config.accept_encoding.unwrap_or(true),
            follow_redirects: http_config.follow_redirects.unwrap_or(true),
            max_redirects: http_config.max_redirects.unwrap_or(10),
        };

        // 记录HTTP请求开始时间
        let http_start_time = Instant::now();

        // 重试配置
        let retry_max = http_config.retry_count.unwrap_or(0) as usize;
        let retry_delay = http_config.retry_delay_ms.unwrap_or(1000);
        let retry_statuses: Vec<u16> = http_config.retry_on_status.clone().unwrap_or_default();
        let mut actual_retry_count: usize = 0;

        // 带重试的请求执行循环
        let http_result: Result<crate::http::HttpResult, String> = loop {
            let result = tokio::time::timeout(
                std::time::Duration::from_millis(timeout_ms),
                client.execute(&config)
            ).await;

            match result {
                Ok(Ok(res)) => {
                    let status = res.response.status;
                    if retry_statuses.contains(&status) && actual_retry_count < retry_max {
                        actual_retry_count += 1;
                        log::info!("[流程 {}] [HTTP] 重试 {}/{}, 状态码: {}",
                            context.flow_uuid, actual_retry_count, retry_max, status);
                        if retry_delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(retry_delay)).await;
                        }
                        continue;
                    }
                    break Ok(res);
                }
                Ok(Err(e)) => {
                    if actual_retry_count < retry_max {
                        actual_retry_count += 1;
                        log::info!("[流程 {}] [HTTP] 网络错误重试 {}/{}: {}",
                            context.flow_uuid, actual_retry_count, retry_max, e);
                        if retry_delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(retry_delay)).await;
                        }
                        continue;
                    }
                    break Err(format!("HTTP请求失败: {}", e));
                }
                Err(_elapsed) => {
                    if actual_retry_count < retry_max {
                        actual_retry_count += 1;
                        log::info!("[流程 {}] [HTTP] 超时重试 {}/{}",
                            context.flow_uuid, actual_retry_count, retry_max);
                        if retry_delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(retry_delay)).await;
                        }
                        continue;
                    }
                    break Err(format!("HTTP请求超时 ({}ms)", timeout_ms));
                }
            }
        };
        
        // 处理超时和网络错误 - 走 false 端口
        let http_result = match http_result {
            Ok(res) => res,
            Err(error_msg) => {
                log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
                // 发送错误日志到前端
                context.emit_log(LogEvent {
                    level: LogLevel::Error,
                    message: error_msg.clone(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("系统时间错误")
                        .as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                }).await;
                
                // 错误：构建失败输出，走 false 端口
                let output_data = serde_json::json!({
                    "success": false,
                    "status": serde_json::Value::Null,
                    "body": serde_json::Value::Null,
                    "headers": serde_json::Value::Null,
                    "error": error_msg,
                    "request": {
                        "url": url,
                        "method": method_str,
                        "headers": sent_headers,
                        "body": request_body_json,
                        "body_type": body_type_str
                    },
                    "timing": HttpTiming::default(),
                    "retry_count": actual_retry_count
                });
                
                // 根据端口过滤连接
                let next_node_ids: Vec<String> = connections
                    .iter()
                    .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == "false")
                    .map(|conn| conn.target_node_uuid.clone())
                    .collect();
                
                return Ok((next_node_ids, Some(output_data)));
            }
        };

        let status_code = http_result.response.status;
        let http_duration_ms = http_start_time.elapsed().as_millis();

        // HTTP响应日志
        log::info!(
            "[流程 {}] [HTTP响应] 状态码: {}, 耗时: {}ms",
            context.flow_uuid,
            status_code,
            http_duration_ms
        );

        // 发送HTTP响应日志到前端
        context.emit_log(LogEvent {
            level: LogLevel::Info,
            message: format!("HTTP {} ({}ms)", status_code, http_duration_ms),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            node_uuid: Some(node.uuid.clone()),
            node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone(),
        }).await;

        // 获取响应头
        let headers_map = http_result.response.headers.clone();

        // 处理响应 Cookie：从 Set-Cookie 头解析并存储
        let cookies_received = if let Some(mut jar) = cookie_jar {
            jar.store_from_response(&headers_map);
            let cookies: Vec<serde_json::Value> = jar
                .all_cookies()
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "name": c.name,
                        "value": c.value,
                        "domain": c.domain,
                        "path": c.path,
                    })
                })
                .collect();
            Some(cookies)
        } else {
            None
        };

        // 检查Content-Type
        let is_json = headers_map
            .get("content-type")
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false);

        // 获取响应体
        let body_text = String::from_utf8(http_result.response.body.clone())
            .unwrap_or_else(|e| format!("无效的 UTF-8: {}", e));

        // 解析响应体为JSON（如果Content-Type是application/json）
        let body_json = if is_json {
            serde_json::from_str(&body_text)
                .unwrap_or_else(|e| {
                    log::warn!("[流程 {}] [警告] 节点ID: {}, 节点名称: {}, JSON解析失败: {}", context.flow_uuid, node.uuid, node.name, e);
                    serde_json::Value::String(body_text)
                })
        } else {
            serde_json::Value::String(body_text)
        };

        // 判断是否成功（2xx 状态码）
        let is_success = (200..300).contains(&status_code);
        let port_id = if is_success { "true" } else { "false" };
        
        // 非2xx 状态码日志
        if !is_success {
            let error_msg = format!(
                "HTTP请求失败，状态码: {}",
                status_code
            );
            log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
            // 发送HTTP状态码错误日志到前端
            context.emit_log(LogEvent {
                level: LogLevel::Error,
                message: format!("HTTP 请求失败: {}", error_msg),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间错误")
                    .as_millis() as u64,
                node_uuid: Some(node.uuid.clone()),
                node_name: Some(node.name.clone()),
                flow_uuid: context.flow_uuid.clone(),
            }).await;
        }

        // 使用来自 HttpClient 的真实计时数据
        let timing = http_result.timing;

        // 构建统一输出数据结构
        let output_data = serde_json::json!({
            "success": is_success,
            "status": status_code,
            "headers": headers_map,
            "body": body_json,
            "cookies_received": cookies_received,
            "redirect_history": &http_result.redirect_history,
            "retry_count": actual_retry_count,
            "request": {
                "url": url,
                "method": method_str,
                "headers": sent_headers,
                "body": request_body_json,
                "body_type": body_type_str
            },
            "timing": timing
        });

        // 应用输出映射，从响应中提取变量
        apply_output_mappings(&output_data, &http_config.output_mappings, context, &node.uuid);

        // 计算总执行时间
        let _duration_ms = start_time.elapsed().as_millis();

        // 执行后日志
        log::info!("[流程 {}] [完成] 节点UUID: {}, 耗时: {}ms, 端口: {}", context.flow_uuid, node.uuid, _duration_ms, port_id);

        // 发送执行完成日志到前端
        context.emit_log(LogEvent {
            level: LogLevel::Info,
            message: format!("HTTP 节点 {} 完成 ({})", node.name, if is_success { "成功" } else { "失败" }),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            node_uuid: Some(node.uuid.clone()),
            node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone(),
        }).await;

        // 根据端口过滤连接，返回下一个节点
        let next_node_ids: Vec<String> = connections
            .iter()
            .filter(|conn| conn.source_node_uuid == node.uuid && conn.source_port_id == port_id)
            .map(|conn| conn.target_node_uuid.clone())
            .collect();

        Ok((next_node_ids, Some(output_data)))
    }
}

/// 应用输出映射，从响应中提取变量
fn apply_output_mappings(
    response: &serde_json::Value,
    mappings: &[flow_engine_core::variable::OutputMapping],
    context: &mut ExecutionContext,
    node_uuid: &str,
) {
    for mapping in mappings {
        if let Some(value) = extract_value_by_path(response, &mapping.source) {
            log::info!(
                "[流程 {}] [输出映射] 节点: {}, 路径: {} -> 变量: {}",
                context.flow_uuid,
                node_uuid,
                mapping.source,
                mapping.target
            );
            context.variables.set_variable(
                mapping.target.clone(),
                value,
                node_uuid.to_string(),
            );
        } else {
            log::warn!(
                "[流程 {}] [输出映射] 节点: {}, 路径 '{}' 未找到值",
                context.flow_uuid,
                node_uuid,
                mapping.source
            );
        }
    }
}

// Re-export extract_value_by_path from core utils
pub use flow_engine_core::utils::extract_value_by_path;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;
    use flow_engine_core::context::VariableStorage;
    use flow_engine_core::node_config::*;
    use flow_engine_core::node::*;

    // ============================================================================
    // 测试辅助函数
    // ============================================================================

    /// Create HTTP node for testing
    fn create_http_node(
        uuid: &str,
        name: &str,
        url: &str,
        method: HttpMethod,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    ) -> Node {
        // Derive body_type from whether body is present
        let body_type = if body.is_some() {
            HttpBodyType::Body
        } else {
            HttpBodyType::None
        };
        
        Node {
            uuid: uuid.to_string(),
            node_type: "http".to_string(),
            name: name.to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: url.to_string(),
                method,
                headers,
                body_type,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body,
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        }
    }

    /// 创建执行上下文
    fn create_context(flow_id: &str, variables: serde_json::Value) -> ExecutionContext {
        let mut var_storage = VariableStorage::new();
        if let Some(obj) = variables.as_object() {
            for (key, value) in obj {
                var_storage.set_variable(key.clone(), value.clone(), String::new());
            }
        }
        ExecutionContext {
            flow_uuid: flow_id.to_string(),
            current_node_uuid: None,
            variables: var_storage,
            node_reports: vec![],
            flow_trigger_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            log_sender: None,
            flow_file_path: None,
            ws_manager: None,
            ws_connection_urls: HashMap::new(),
            ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            db_manager: None,
            grpc_manager: None,
            mcp_manager: None,
            tcp_manager: None,
            udp_manager: None,
            mqtt_manager: None,
            kafka_manager: None,
            sse_manager: None,
            graphql_manager: None,
            amqp_manager: None,
            smtp_manager: None,
            imap_manager: None,
            last_node_output: None,
            debug_event_sender: None,
            execution_event_sender: None,
            cancel_receiver: None,
            providers: HashMap::new(),
        }
    }

    // ============================================================================
    // 测试：成功场景
    // ============================================================================

    #[tokio::test]
    /// 测试GET请求成功场景（2xx -> true 端口）
    async fn test_http_success_get() {
        let node = create_http_node(
            "test-get",
            "HTTP GET请求",
            "https://httpbin.org/get",
            HttpMethod::Get,
            None,
            None,
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok(), "GET请求应该成功");
        let (next_node_ids, output_data) = result.unwrap();
        assert!(next_node_ids.is_empty());

        // 验证输出数据
        assert!(output_data.is_some(), "应该有输出数据");
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200, "状态码应该是200");
        assert_eq!(output["success"], true, "success 应该为 true");
        assert!(output["body"].is_object(), "响应体应该是JSON对象");
    }

    #[tokio::test]
    /// 测试POST请求成功场景（2xx -> true 端口）
    async fn test_http_success_post() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let node = create_http_node(
            "test-post",
            "HTTP POST请求",
            "https://httpbin.org/post",
            HttpMethod::Post,
            Some(headers),
            None,
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok(), "POST请求应该成功");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);
        assert_eq!(output["success"], true, "success 应该为 true");

        // 验证响应体包含请求数据
        if let Some(body) = output["body"].as_object() {
            assert!(body.contains_key("json"), "响应应该包含json字段");
            if let Some(json_data) = body.get("json").and_then(|v| v.as_object()) {
                assert_eq!(json_data.get("test_key").and_then(|v| v.as_str()), Some("test_value"));
            }
        }
    }

    #[tokio::test]
    /// 测试PUT请求成功场景（2xx -> true 端口）
    async fn test_http_success_put() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let node = create_http_node(
            "test-put",
            "HTTP PUT请求",
            "https://httpbin.org/put",
            HttpMethod::Put,
            Some(headers),
            None,
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        assert!(result.is_ok(), "PUT请求应该成功");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);
        assert_eq!(output["success"], true, "success 应该为 true");
    }

    #[tokio::test]
    /// 测试DELETE请求成功场景（2xx -> true 端口）
    async fn test_http_success_delete() {
        let node = create_http_node(
            "test-delete",
            "HTTP DELETE请求",
            "https://httpbin.org/delete",
            HttpMethod::Delete,
            None,
            None,
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        assert!(result.is_ok(), "DELETE请求应该成功");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);
        assert_eq!(output["success"], true, "success 应该为 true");
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
            None,
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        assert!(result.is_ok(), "PATCH请求应该成功");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);
        assert_eq!(output["success"], true, "success 应该为 true");
    }

    // ============================================================================
    // 测试：失败场景（非2xx状态码 -> false 端口）
    // ============================================================================

    #[tokio::test]
    /// 测试HTTP 404错误场景（非2xx -> false 端口）
    async fn test_http_failure_404() {
        let node = create_http_node(
            "test-404",
            "HTTP 404错误",
            "https://httpbin.org/status/404",
            HttpMethod::Get,
            None,
            None,
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功（不返回 Err，而是返回 Ok 并标记 success: false）
        assert!(result.is_ok(), "404错误应该返回Ok（走false端口）");
        let (next_node_ids, output_data) = result.unwrap();
        assert!(next_node_ids.is_empty(), "没有连接时next_node_ids应该为空");
        
        // 验证输出数据
        assert!(output_data.is_some(), "应该有输出数据");
        let output = output_data.unwrap();
        assert_eq!(output["success"], false, "success 应该为 false");
        assert_eq!(output["status"], 404, "状态码应该是404");
    }

    #[tokio::test]
    /// 测试HTTP 500服务器错误场景（非2xx -> false 端口）
    async fn test_http_failure_500() {
        let node = create_http_node(
            "test-500",
            "HTTP 500错误",
            "https://httpbin.org/status/500",
            HttpMethod::Get,
            None,
            None,
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功（不返回 Err，而是返回 Ok 并标记 success: false）
        assert!(result.is_ok(), "500错误应该返回Ok（走false端口）");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["success"], false, "success 应该为 false");
        assert_eq!(output["status"], 500, "状态码应该是500");
    }

    #[tokio::test]
    /// 测试HTTP 403禁止访问错误场景（非2xx -> false 端口）
    async fn test_http_failure_403() {
        let node = create_http_node(
            "test-403",
            "HTTP 403错误",
            "https://httpbin.org/status/403",
            HttpMethod::Get,
            None,
            None,
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功（不返回 Err，而是返回 Ok 并标记 success: false）
        assert!(result.is_ok(), "403错误应该返回Ok（走false端口）");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["success"], false, "success 应该为 false");
        assert_eq!(output["status"], 403, "状态码应该是403");
    }

    #[tokio::test]
    /// 测试HTTP 401未授权错误场景（非2xx -> false 端口）
    async fn test_http_failure_401() {
        let node = create_http_node(
            "test-401",
            "HTTP 401错误",
            "https://httpbin.org/status/401",
            HttpMethod::Get,
            None,
            None,
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功（不返回 Err，而是返回 Ok 并标记 success: false）
        assert!(result.is_ok(), "401错误应该返回Ok（走false端口）");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["success"], false, "success 应该为 false");
        assert_eq!(output["status"], 401, "状态码应该是401");
    }

    // ============================================================================
    // 测试：网络错误场景（-> false 端口）
    // ============================================================================

    #[tokio::test]
    /// 测试DNS解析失败场景（网络错误 -> false 端口）
    async fn test_http_error_dns_failure() {
        let node = create_http_node(
            "test-dns",
            "DNS解析失败",
            "http://this-domain-does-not-exist-12345.invalid",
            HttpMethod::Get,
            None,
            None,
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功（不返回 Err，而是返回 Ok 并标记 success: false）
        assert!(result.is_ok(), "DNS解析失败应该返回Ok（走false端口）");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some(), "应该有输出数据");
        let output = output_data.unwrap();
        assert_eq!(output["success"], false, "success 应该为 false");
        assert!(output["status"].is_null(), "网络错误时 status 应该为 null");
        assert!(output["body"].is_null(), "网络错误时 body 应该为 null");
        assert!(output["headers"].is_null(), "网络错误时 headers 应该为 null");
        assert!(output["error"].is_string(), "网络错误时应该有 error 字段");
        let error_msg = output["error"].as_str().unwrap();
        assert!(
            error_msg.contains("HTTP请求失败") ||
            error_msg.contains("dns") ||
            error_msg.contains("connect") ||
            error_msg.contains("timeout") ||
            error_msg.contains("no such host"),
            "错误消息应该包含网络错误信息: {}",
            error_msg
        );
    }

    #[tokio::test]
    /// 测试连接超时场景（网络错误 -> false 端口）
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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功（不返回 Err，而是返回 Ok 并标记 success: false）
        assert!(result.is_ok(), "连接超时应该返回Ok（走false端口）");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some(), "应该有输出数据");
        let output = output_data.unwrap();
        assert_eq!(output["success"], false, "success 应该为 false");
        assert!(output["status"].is_null(), "网络错误时 status 应该为 null");
        assert!(output["body"].is_null(), "网络错误时 body 应该为 null");
        assert!(output["headers"].is_null(), "网络错误时 headers 应该为 null");
        assert!(output["error"].is_string(), "网络错误时应该有 error 字段");
        let error_msg = output["error"].as_str().unwrap();
        assert!(
            error_msg.contains("HTTP请求失败") ||
            error_msg.contains("timeout") ||
            error_msg.contains("connect") ||
            error_msg.contains("timed out"),
            "错误消息应该包含超时或连接错误信息: {}",
            error_msg
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

        let mut context = create_context("test-flow", variables);
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

        let mut context = create_context("test-flow", variables);
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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
            }).to_string())
        );

        let variables = json!({
            "username": "john_doe",
            "email": "john@example.com",
            "user_id": "12345"
        });

        let mut context = create_context("test-flow", variables);
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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
            }).to_string())
        );

        let variables = json!({
            "env": "production",
            "request_id": "req-12345",
            "username": "alice",
            "action": "create"
        });

        let mut context = create_context("test-flow", variables);
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

    // ============================================================================
    // 测试：来自 http_executor_unit_tests.rs 的单元测试
    // ============================================================================

    #[tokio::test]
    async fn test_http_executor_get_request() {
        // 创建HTTP GET请求节点
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: "http".to_string(),
            name: "HTTP GET请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/get".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body_type: HttpBodyType::None,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body: None,
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext { flow_uuid: flow_id.clone(), current_node_uuid: Some(http_node.uuid.clone()), variables: VariableStorage::new(), node_reports: Vec::new(), flow_trigger_time: 1000, log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())), db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None, mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(), };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok());
        let (next_node_ids, output_data) = result.unwrap();
        assert!(next_node_ids.is_empty()); // 测试节点没有连接

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
            node_type: "http".to_string(),
            name: "HTTP POST请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/post".to_string(),
                method: HttpMethod::Post,
                headers: Some({
                    let mut headers = HashMap::new();
                    headers.insert("Content-Type".to_string(), "application/json".to_string());
                    headers
                }),
                body_type: HttpBodyType::Body,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body: Some(r#"{"test_key":"test_value"}"#.to_string()),
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext { flow_uuid: flow_id.clone(), current_node_uuid: Some(http_node.uuid.clone()), variables: VariableStorage::new(), node_reports: Vec::new(), flow_trigger_time: 1000, log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())), db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None, mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(), };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

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
            node_type: "http".to_string(),
            name: "HTTP 404请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/status/404".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body_type: HttpBodyType::None,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body: None,
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext { flow_uuid: flow_id.clone(), current_node_uuid: Some(http_node.uuid.clone()), variables: VariableStorage::new(), node_reports: Vec::new(), flow_trigger_time: 1000, log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())), db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None, mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(), };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功（不返回 Err，而是返回 Ok 并标记 success: false）
        assert!(result.is_ok(), "404错误应该返回Ok（走false端口）");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["success"], false, "success 应该为 false");
        assert_eq!(output["status"], 404, "状态码应该是404");
    }

    #[tokio::test]
    async fn test_http_executor_200_success() {
        // 测试HTTP 200成功状态码
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: "http".to_string(),
            name: "HTTP 200成功".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/status/200".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body_type: HttpBodyType::None,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body: None,
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext { flow_uuid: flow_id.clone(), current_node_uuid: Some(http_node.uuid.clone()), variables: VariableStorage::new(), node_reports: Vec::new(), flow_trigger_time: 1000, log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())), db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None, mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(), };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

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
            node_type: "http".to_string(),
            name: "HTTP 500错误".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/status/500".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body_type: HttpBodyType::None,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body: None,
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext { flow_uuid: flow_id.clone(), current_node_uuid: Some(http_node.uuid.clone()), variables: VariableStorage::new(), node_reports: Vec::new(), flow_trigger_time: 1000, log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())), db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None, mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(), };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功（不返回 Err，而是返回 Ok 并标记 success: false）
        assert!(result.is_ok(), "500错误应该返回Ok（走false端口）");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["success"], false, "success 应该为 false");
        assert_eq!(output["status"], 500, "状态码应该是500");
    }

    #[tokio::test]
    async fn test_http_executor_variable_replacement() {
        // 测试变量替换（URL、headers、body）
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: "http".to_string(),
            name: "HTTP变量替换".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/get?param={{test_param}}".to_string(),
                method: HttpMethod::Get,
                headers: Some({
                    let mut headers = HashMap::new();
                    headers.insert("X-Custom-Header".to_string(), "{{test_header}}".to_string());
                    headers
                }),
                body_type: HttpBodyType::None,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body: None,
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        };

        let mut storage = VariableStorage::new();
        storage.set_variable("test_param".to_string(), json!("value123"), String::new());
        storage.set_variable("test_header".to_string(), json!("header_value"), String::new());
        storage.set_variable("test_body".to_string(), json!("body_value"), String::new());
        let mut context = ExecutionContext { flow_uuid: flow_id.clone(), current_node_uuid: Some(http_node.uuid.clone()), variables: storage, node_reports: Vec::new(), flow_trigger_time: 1000, log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())), db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None, mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(), };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

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
            node_type: "http".to_string(),
            name: "HTTP网络错误".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "http://this-domain-does-not-exist-12345.invalid".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body_type: HttpBodyType::None,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body: None,
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext { flow_uuid: flow_id.clone(), current_node_uuid: Some(http_node.uuid.clone()), variables: VariableStorage::new(), node_reports: Vec::new(), flow_trigger_time: 1000, log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())), db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None, mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(), };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功（不返回 Err，而是返回 Ok 并标记 success: false）
        assert!(result.is_ok(), "网络错误应该返回Ok（走false端口）");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some(), "应该有输出数据");
        let output = output_data.unwrap();
        assert_eq!(output["success"], false, "success 应该为 false");
        assert!(output["status"].is_null(), "网络错误时 status 应该为 null");
        assert!(output["body"].is_null(), "网络错误时 body 应该为 null");
        assert!(output["headers"].is_null(), "网络错误时 headers 应该为 null");
        assert!(output["error"].is_string(), "网络错误时应该有 error 字段");
        let error_msg = output["error"].as_str().unwrap();
        assert!(
            error_msg.contains("HTTP请求失败") ||
            error_msg.contains("dns") ||
            error_msg.contains("connect") ||
            error_msg.contains("timeout"),
            "错误消息应该包含网络错误信息: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_http_executor_json_response() {
        // 测试JSON响应解析
        let flow_id = "test-flow".to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: "http".to_string(),
            name: "HTTP JSON响应".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/json".to_string(),
                method: HttpMethod::Get,
                headers: Some({
                    let mut headers = HashMap::new();
                    headers.insert("Accept".to_string(), "application/json".to_string());
                    headers
                }),
                body_type: HttpBodyType::None,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body: None,
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext { flow_uuid: flow_id.clone(), current_node_uuid: Some(http_node.uuid.clone()), variables: VariableStorage::new(), node_reports: Vec::new(), flow_trigger_time: 1000, log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())), db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None, mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(), };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

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
            node_type: "http".to_string(),
            name: "HTTP文本响应".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/encoding/utf8".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body_type: HttpBodyType::None,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body: None,
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext { flow_uuid: flow_id.clone(), current_node_uuid: Some(http_node.uuid.clone()), variables: VariableStorage::new(), node_reports: Vec::new(), flow_trigger_time: 1000, log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())), db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None, mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(), };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

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
            node_type: "http".to_string(),
            name: "HTTP PUT请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/put".to_string(),
                method: HttpMethod::Put,
                headers: Some({
                    let mut headers = HashMap::new();
                    headers.insert("Content-Type".to_string(), "application/json".to_string());
                    headers
                }),
                body_type: HttpBodyType::Body,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body: Some(r#"{"test":"put_data"}"#.to_string()),
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext { flow_uuid: flow_id.clone(), current_node_uuid: Some(http_node.uuid.clone()), variables: VariableStorage::new(), node_reports: Vec::new(), flow_trigger_time: 1000, log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())), db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None, mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(), };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

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
            node_type: "http".to_string(),
            name: "HTTP DELETE请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/delete".to_string(),
                method: HttpMethod::Delete,
                headers: None,
                body_type: HttpBodyType::None,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body: None,
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext { flow_uuid: flow_id.clone(), current_node_uuid: Some(http_node.uuid.clone()), variables: VariableStorage::new(), node_reports: Vec::new(), flow_trigger_time: 1000, log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())), db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None, mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(), };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

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
            node_type: "http".to_string(),
            name: "HTTP PATCH请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/patch".to_string(),
                method: HttpMethod::Patch,
                headers: Some({
                    let mut headers = HashMap::new();
                    headers.insert("Content-Type".to_string(), "application/json".to_string());
                    headers
                }),
                body_type: HttpBodyType::Body,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body: Some(r#"{"test":"patch_data"}"#.to_string()),
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext { flow_uuid: flow_id.clone(), current_node_uuid: Some(http_node.uuid.clone()), variables: VariableStorage::new(), node_reports: Vec::new(), flow_trigger_time: 1000, log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())), db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None, mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(), };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

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
            node_type: "http".to_string(),
            name: "HTTP响应头".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::Http(HttpConfig {
                url: "https://httpbin.org/get".to_string(),
                method: HttpMethod::Get,
                headers: None,
                body_type: HttpBodyType::None,
                content_type: ContentType::Json,
                form_fields: None,
                file_path: None,
                body: None,
                timeout_ms: None,
                output_mappings: Vec::new(),
            }),
            ports: None,
        has_breakpoint: None,
        };

        let mut context = ExecutionContext { flow_uuid: flow_id.clone(), current_node_uuid: Some(http_node.uuid.clone()), variables: VariableStorage::new(), node_reports: Vec::new(), flow_trigger_time: 1000, log_sender: None, flow_file_path: None, ws_manager: None, ws_connection_urls: HashMap::new(), ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())), db_manager: None, grpc_manager: None, mcp_manager: None, tcp_manager: None, udp_manager: None, mqtt_manager: None, kafka_manager: None, sse_manager: None, graphql_manager: None, amqp_manager: None, smtp_manager: None, imap_manager: None, last_node_output: None, debug_event_sender: None, execution_event_sender: None, cancel_receiver: None, providers: HashMap::new(), };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

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

    // ============================================================================
    // 测试：request 字段验证（新增功能）
    // ============================================================================

    #[tokio::test]
    /// 测试 output_data 包含 request 字段（向后兼容 + 新增）
    async fn test_output_contains_request_info() {
        let node = create_http_node(
            "test-request-info",
            "HTTP GET请求",
            "https://httpbin.org/get",
            HttpMethod::Get,
            None,
            None,
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok(), "GET请求应该成功");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();

        // ===== 向后兼容性验证 =====
        assert!(output.get("status").is_some(), "output_data 应包含 status 字段");
        assert!(output.get("headers").is_some(), "output_data 应包含 headers 字段");
        assert!(output.get("body").is_some(), "output_data 应包含 body 字段");
        assert_eq!(output["status"], 200, "状态码应该是 200");

        // ===== 新增 request 字段验证 =====
        assert!(output.get("request").is_some(), "output_data 应包含 request 字段");
        let request = output.get("request").unwrap();
        
        // 验证 request.url
        assert!(request.get("url").is_some(), "request 应包含 url 字段");
        let url = request.get("url").unwrap().as_str().unwrap();
        assert!(url.contains("httpbin.org/get"), "request.url 应包含请求地址");
        
        // 验证 request.method
        assert!(request.get("method").is_some(), "request 应包含 method 字段");
        assert_eq!(request.get("method").unwrap().as_str().unwrap(), "GET", "request.method 应该是 GET");
        
        // 验证 request.headers
        assert!(request.get("headers").is_some(), "request 应包含 headers 字段");
        assert!(request.get("headers").unwrap().is_object(), "request.headers 应该是对象");
        
        // 验证 request.body_type
        assert!(request.get("body_type").is_some(), "request 应包含 body_type 字段");
        assert_eq!(request.get("body_type").unwrap().as_str().unwrap(), "none", "GET 请求的 body_type 应该是 none");
        
        // 验证 request.body (GET 请求应该是 null)
        assert!(request.get("body").is_some(), "request 应包含 body 字段");
        assert!(request.get("body").unwrap().is_null(), "GET 请求的 request.body 应该是 null");
    }

    #[tokio::test]
    /// 测试 POST 请求的 request.body 记录
    async fn test_request_info_body_for_post() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

        let node = create_http_node(
            "test-post-body",
            "HTTP POST请求",
            "https://httpbin.org/post",
            HttpMethod::Post,
            Some(headers),
            Some(r#"{"test_key":"test_value"}"#.to_string()),
        );

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        assert!(result.is_ok(), "POST请求应该成功");
        let output = result.unwrap().1.unwrap();

        // 验证 request 字段
        let request = output.get("request").unwrap();
        
        // 验证 method
        assert_eq!(request.get("method").unwrap().as_str().unwrap(), "POST");
        
        // 验证 headers 包含自定义头
        let req_headers = request.get("headers").unwrap().as_object().unwrap();
        assert_eq!(req_headers.get("X-Custom-Header").unwrap().as_str().unwrap(), "custom-value");
        assert_eq!(req_headers.get("Content-Type").unwrap().as_str().unwrap(), "application/json");
        
        // 验证 body_type
        assert_eq!(request.get("body_type").unwrap().as_str().unwrap(), "body");
        
        // 验证 body 包含发送的数据
        let req_body = request.get("body").unwrap().as_str().unwrap();
        assert!(req_body.contains("test_key"));
        assert!(req_body.contains("test_value"));
    }

    #[tokio::test]
    /// 测试变量替换后的 URL 记录在 request.url
    async fn test_request_info_url_after_variable_replacement() {
        let node = create_http_node(
            "test-var-url",
            "URL变量替换",
            "https://httpbin.org/get?param={{test_param}}",
            HttpMethod::Get,
            None,
            None,
        );

        let variables = json!({ "test_param": "replaced_value" });
        let mut context = create_context("test-flow", variables);
        let executor = HttpNodeExecutor::new();

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        assert!(result.is_ok());
        let output = result.unwrap().1.unwrap();
        let request = output.get("request").unwrap();
        
        // 验证 request.url 是变量替换后的值
        let url = request.get("url").unwrap().as_str().unwrap();
        assert!(url.contains("param=replaced_value"), "request.url 应该包含替换后的变量值");
        assert!(!url.contains("{{test_param}}"), "request.url 不应该包含未替换的变量");
    }

    #[tokio::test]
    /// 测试请求头变量替换记录在 request.headers
    async fn test_request_info_headers_includes_replaced_variables() {
        let mut headers = HashMap::new();
        headers.insert("X-Auth-Token".to_string(), "Bearer {{api_token}}".to_string());

        let node = create_http_node(
            "test-var-headers",
            "请求头变量替换",
            "https://httpbin.org/get",
            HttpMethod::Get,
            Some(headers),
            None,
        );

        let variables = json!({ "api_token": "secret_token_123" });
        let mut context = create_context("test-flow", variables);
        let executor = HttpNodeExecutor::new();

        let result = executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        assert!(result.is_ok());
        let output = result.unwrap().1.unwrap();
        let request = output.get("request").unwrap();
        
        // 验证 request.headers 包含替换后的值
        let req_headers = request.get("headers").unwrap().as_object().unwrap();
        let auth_header = req_headers.get("X-Auth-Token").unwrap().as_str().unwrap();
        assert!(auth_header.contains("secret_token_123"), "request.headers 应该包含替换后的变量值");
        assert!(!auth_header.contains("{{api_token}}"), "request.headers 不应该包含未替换的变量");
    }

    // ============================================================================
    // 测试：JSONPath 提取功能（TDD: 先写测试）
    // ============================================================================

    /// 测试 extract_value_by_path 使用 JSONPath 语法
    #[test]
    fn test_extract_value_by_jsonpath_basic() {
        let data = json!({
            "status": 200,
            "headers": {
                "content-type": "application/json",
                "x-request-id": "abc123"
            },
            "body": {
                "data": {
                    "token": "secret_token",
                    "user": {
                        "id": 1,
                        "name": "alice"
                    }
                }
            }
        });

        // 测试基本路径
        assert_eq!(
            extract_value_by_path(&data, "$.status"),
            Some(json!(200)),
            "$.status 应该返回 200"
        );

        // 测试嵌套对象
        assert_eq!(
            extract_value_by_path(&data, "$.body.data.token"),
            Some(json!("secret_token")),
            "$.body.data.token 应该返回 secret_token"
        );

        // 测试深层嵌套
        assert_eq!(
            extract_value_by_path(&data, "$.body.data.user.name"),
            Some(json!("alice")),
            "$.body.data.user.name 应该返回 alice"
        );

        // 测试不存在的路径
        assert_eq!(
            extract_value_by_path(&data, "$.not.exist"),
            None,
            "$.not.exist 应该返回 None"
        );
    }

    /// 测试 JSONPath 数组索引访问
    #[test]
    fn test_extract_value_by_jsonpath_array_index() {
        let data = json!({
            "body": {
                "items": [
                    {"id": 1, "name": "Item1"},
                    {"id": 2, "name": "Item2"},
                    {"id": 3, "name": "Item3"}
                ],
                "users": ["alice", "bob", "charlie"]
            }
        });

        // 测试数组索引 [0]
        assert_eq!(
            extract_value_by_path(&data, "$.body.items[0]"),
            Some(json!({"id": 1, "name": "Item1"})),
            "$.body.items[0] 应该返回第一个元素"
        );

        // 测试数组索引 [1]
        assert_eq!(
            extract_value_by_path(&data, "$.body.items[1].name"),
            Some(json!("Item2")),
            "$.body.items[1].name 应该返回 Item2"
        );

        // 测试简单数组索引
        assert_eq!(
            extract_value_by_path(&data, "$.body.users[2]"),
            Some(json!("charlie")),
            "$.body.users[2] 应该返回 charlie"
        );

        // 测试负索引 [-1]（最后一个元素）
        assert_eq!(
            extract_value_by_path(&data, "$.body.items[-1].name"),
            Some(json!("Item3")),
            "$.body.items[-1].name 应该返回最后一个元素的 name"
        );

        // 测试负索引 [-2]
        assert_eq!(
            extract_value_by_path(&data, "$.body.users[-2]"),
            Some(json!("bob")),
            "$.body.users[-2] 应该返回倒数第二个元素"
        );
    }

    /// 测试 JSONPath 边界情况
    #[test]
    fn test_extract_value_by_jsonpath_edge_cases() {
        let data = json!({
            "status": 200,
            "body": null,
            "items": []
        });

        // 测试空路径
        assert_eq!(
            extract_value_by_path(&data, ""),
            None,
            "空路径应该返回 None"
        );

        // 测试根路径
        assert_eq!(
            extract_value_by_path(&data, "$"),
            Some(data.clone()),
            "$ 应该返回整个对象"
        );

        // 测试 null 值
        assert_eq!(
            extract_value_by_path(&data, "$.body"),
            Some(json!(null)),
            "$.body 应该返回 null"
        );

        // 测试空数组索引
        assert_eq!(
            extract_value_by_path(&data, "$.items[0]"),
            None,
            "$.items[0] 空数组应该返回 None"
        );

        // 测试越界索引
        let data_with_items = json!({"items": [1, 2]});
        assert_eq!(
            extract_value_by_path(&data_with_items, "$.items[10]"),
            None,
            "越界索引应该返回 None"
        );
    }

    /// 测试 JSONPath 无效语法处理
    #[test]
    fn test_extract_value_by_jsonpath_invalid_syntax() {
        let data = json!({"status": 200});

        // 无效 JSONPath 应该返回 None（不 panic）
        assert_eq!(
            extract_value_by_path(&data, "invalid["),
            None,
            "无效 JSONPath 应该返回 None"
        );

        assert_eq!(
            extract_value_by_path(&data, "$..[["),
            None,
            "无效 JSONPath 应该返回 None"
        );
    }

    // ============================================================================
    // Header Key 变量替换测试
    // ============================================================================

    #[tokio::test]
    /// 测试请求头 key 变量替换
    async fn test_http_header_key_variable_replacement() {
        let mut headers = HashMap::new();
        // 使用变量作为 header key
        headers.insert("{{header_name}}".to_string(), "header_value".to_string());
        headers.insert("X-Static-Header".to_string(), "{{header_value}}".to_string());

        let node = create_http_node(
            "test-header-key-var",
            "Header Key 变量替换",
            "https://httpbin.org/headers",
            HttpMethod::Get,
            Some(headers),
            None,
        );

        let variables = json!({
            "header_name": "X-Custom-Header",
            "header_value": "dynamic_value"
        });

        let mut context = create_context("test-flow", variables);
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok(), "Header key 变量替换应该成功");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);

        // 验证响应体包含替换后的 header key
        if let Some(body) = output["body"].as_object() {
            if let Some(response_headers) = body.get("headers").and_then(|v| v.as_object()) {
                assert_eq!(
                    response_headers.get("X-Custom-Header").and_then(|v| v.as_str()),
                    Some("header_value"),
                    "Header key 应该被替换"
                );
                assert_eq!(
                    response_headers.get("X-Static-Header").and_then(|v| v.as_str()),
                    Some("dynamic_value"),
                    "Header value 应该被替换"
                );
            }
        }
    }

    #[tokio::test]
    /// 测试请求头 key 部分变量替换
    async fn test_http_header_key_partial_variable_replacement() {
        let mut headers = HashMap::new();
        // 使用部分变量作为 header key
        headers.insert("X-{{suffix}}".to_string(), "test_value".to_string());

        let node = create_http_node(
            "test-header-key-partial-var",
            "Header Key 部分变量替换",
            "https://httpbin.org/headers",
            HttpMethod::Get,
            Some(headers),
            None,
        );

        let variables = json!({
            "suffix": "Request-Id"
        });

        let mut context = create_context("test-flow", variables);
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

        // 验证执行成功
        assert!(result.is_ok(), "Header key 部分变量替换应该成功");
        let output_data = result.unwrap().1;
        assert!(output_data.is_some());
        let output = output_data.unwrap();
        assert_eq!(output["status"], 200);

        // 验证 request.headers 中包含替换后的 header key
        // 注意：httpbin.org 的 /headers 端点可能不会返回所有发送的 headers
        // 所以我们验证 request.headers 而不是 body.headers
        if let Some(request) = output.get("request") {
            if let Some(sent_headers) = request.get("headers").and_then(|v| v.as_object()) {
                assert_eq!(
                    sent_headers.get("X-Request-Id").and_then(|v| v.as_str()),
                    Some("test_value"),
                    "Request header key 部分变量应该被替换"
                );
            }
        }
    }

    // ============================================================================
    // 测试：双端口路由功能
    // ============================================================================

    #[tokio::test]
    /// 测试成功响应（2xx）路由到 true 端口
    async fn test_http_port_routing_success_to_true() {
        use flow_engine_core::connection::Connection;

        let node = create_http_node(
            "test-routing-success",
            "HTTP GET请求",
            "https://httpbin.org/get",
            HttpMethod::Get,
            None,
            None,
        );

        // 创建连接：true 端口 -> node-true, false 端口 -> node-false
        let connections = vec![
            Connection {
                id: "conn-true".to_string(),
                source_node_uuid: "test-routing-success".to_string(),
                target_node_uuid: "node-true".to_string(),
                source_port_id: "true".to_string(),
                target_port_id: "in".to_string(),
            },
            Connection {
                id: "conn-false".to_string(),
                source_node_uuid: "test-routing-success".to_string(),
                target_node_uuid: "node-false".to_string(),
                source_port_id: "false".to_string(),
                target_port_id: "in".to_string(),
            },
        ];

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result = executor.execute(&node, &mut context, &connections, &HashMap::new()).await;

        assert!(result.is_ok(), "请求应该成功");
        let (next_node_ids, output_data) = result.unwrap();

        // 验证：应该路由到 true 端口连接的节点
        assert_eq!(next_node_ids, vec!["node-true"], "2xx 响应应该路由到 true 端口");

        // 验证输出数据
        let output = output_data.unwrap();
        assert_eq!(output["success"], true);
        assert_eq!(output["status"], 200);
    }

    #[tokio::test]
    /// 测试失败响应（非2xx）路由到 false 端口
    async fn test_http_port_routing_failure_to_false() {
        use flow_engine_core::connection::Connection;

        let node = create_http_node(
            "test-routing-failure",
            "HTTP 404请求",
            "https://httpbin.org/status/404",
            HttpMethod::Get,
            None,
            None,
        );

        // 创建连接：true 端口 -> node-true, false 端口 -> node-false
        let connections = vec![
            Connection {
                id: "conn-true".to_string(),
                source_node_uuid: "test-routing-failure".to_string(),
                target_node_uuid: "node-true".to_string(),
                source_port_id: "true".to_string(),
                target_port_id: "in".to_string(),
            },
            Connection {
                id: "conn-false".to_string(),
                source_node_uuid: "test-routing-failure".to_string(),
                target_node_uuid: "node-false".to_string(),
                source_port_id: "false".to_string(),
                target_port_id: "in".to_string(),
            },
        ];

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result = executor.execute(&node, &mut context, &connections, &HashMap::new()).await;

        assert!(result.is_ok(), "请求应该返回Ok");
        let (next_node_ids, output_data) = result.unwrap();

        // 验证：应该路由到 false 端口连接的节点
        assert_eq!(next_node_ids, vec!["node-false"], "非2xx 响应应该路由到 false 端口");

        // 验证输出数据
        let output = output_data.unwrap();
        assert_eq!(output["success"], false);
        assert_eq!(output["status"], 404);
    }

    #[tokio::test]
    /// 测试网络错误路由到 false 端口
    async fn test_http_port_routing_network_error_to_false() {
        use flow_engine_core::connection::Connection;

        let node = create_http_node(
            "test-routing-network-error",
            "DNS解析失败",
            "http://this-domain-does-not-exist-12345.invalid",
            HttpMethod::Get,
            None,
            None,
        );

        // 创建连接：true 端口 -> node-true, false 端口 -> node-false
        let connections = vec![
            Connection {
                id: "conn-true".to_string(),
                source_node_uuid: "test-routing-network-error".to_string(),
                target_node_uuid: "node-true".to_string(),
                source_port_id: "true".to_string(),
                target_port_id: "in".to_string(),
            },
            Connection {
                id: "conn-false".to_string(),
                source_node_uuid: "test-routing-network-error".to_string(),
                target_node_uuid: "node-false".to_string(),
                source_port_id: "false".to_string(),
                target_port_id: "in".to_string(),
            },
        ];

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result = executor.execute(&node, &mut context, &connections, &HashMap::new()).await;

        assert!(result.is_ok(), "网络错误应该返回Ok");
        let (next_node_ids, output_data) = result.unwrap();

        // 验证：应该路由到 false 端口连接的节点
        assert_eq!(next_node_ids, vec!["node-false"], "网络错误应该路由到 false 端口");

        // 验证输出数据
        let output = output_data.unwrap();
        assert_eq!(output["success"], false);
        assert!(output["status"].is_null(), "网络错误时 status 应该为 null");
        assert!(output["error"].is_string(), "网络错误时应该有 error 字段");
    }
}
