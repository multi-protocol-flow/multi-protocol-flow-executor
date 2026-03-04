use crate::connection::Connection;
use crate::context::ExecutionContext;
use crate::executor::NodeExecutor;
use crate::log_event::{LogEvent, LogLevel};
use crate::node::Node;
use std::collections::HashMap;
use std::time::Instant;
use serde_json_path::JsonPath;

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

/// Convert ContentType to Content-Type header value
fn content_type_to_string(content_type: &crate::node_config::ContentType) -> &'static str {
    match content_type {
        crate::node_config::ContentType::Json => "application/json",
        crate::node_config::ContentType::Text => "text/plain",
        crate::node_config::ContentType::Html => "text/html",
        crate::node_config::ContentType::Xml => "application/xml",
        crate::node_config::ContentType::Javascript => "application/javascript",
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
        _connections: &[Connection],
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
            crate::node_config::NodeData::Http(config) => config.clone(),
            _ => {
                let error_msg = format!("节点数据类型不是HttpConfig: {}", node.uuid);
                log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
                return Err(error_msg);
            }
        };

        // 使用replace_variables替换URL
        let url = crate::executor::replace_variables(&http_config.url, context)
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
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(
                http_config.timeout_ms
                    .map(|t| t.clamp(1000, 300000))
                    .unwrap_or(30000)
            ))
            .build()
            .map_err(|e| {
                let error_msg = format!("创建HTTP客户端失败: {}", e);
                log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
                error_msg
            })?;

        // 记录发送的请求头
        let mut sent_headers: HashMap<String, String> = HashMap::new();
        
        // 记录请求体信息（用于输出）
        let mut request_body_json: Option<serde_json::Value> = None;
        

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
                        log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
                        error_msg
                    })?;
                request = request.header(key, &replaced_value);
                sent_headers.insert(key.clone(), replaced_value);
            }
        }

        // Handle request body based on body_type
        // Content-Type priority: only set if user hasn't manually set Content-Type in headers
        let user_set_content_type = has_content_type_header(&http_config.headers);

        // 请求体类型字符串
        let body_type_str = match http_config.body_type {
            crate::node_config::HttpBodyType::None => "none",
            crate::node_config::HttpBodyType::Body => "body",
            crate::node_config::HttpBodyType::FormData => "form-data",
            crate::node_config::HttpBodyType::XWwwFormUrlencoded => "x-www-form-urlencoded",
            crate::node_config::HttpBodyType::Binary => "binary",
        };


        match http_config.body_type {
            crate::node_config::HttpBodyType::None => {
                // No request body
                log::info!("[流程 {}] [HTTP] body_type=none, 不添加请求体", context.flow_uuid);
            }
            crate::node_config::HttpBodyType::Body => {
                // Text body with optional variable replacement
                if let Some(body) = &http_config.body {
                    let replaced_body = crate::executor::replace_variables(body, context)
                        .map_err(|e| {
                            let error_msg = format!("请求体变量替换失败: {}", e);
                            log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", 
                                context.flow_uuid, node.uuid, node.name, error_msg);
                            error_msg
                        })?;
                    
                    // Set Content-Type header only if user hasn't set it manually
                    if !user_set_content_type {
                        let content_type_str = content_type_to_string(&http_config.content_type);
                        request = request.header("Content-Type", content_type_str);
                        sent_headers.insert("Content-Type".to_string(), content_type_str.to_string());
                        log::info!("[流程 {}] [HTTP] 自动设置 Content-Type: {}", 
                            context.flow_uuid, content_type_str);
                    }
                    
                    request = request.body(replaced_body.clone());
                    request_body_json = Some(replaced_body.into());
                    log::info!("[流程 {}] [HTTP] body_type=body, 请求体已添加", context.flow_uuid);
                } else {
                    log::info!("[流程 {}] [HTTP] body_type=body, 但 body 字段为空，跳过", context.flow_uuid);
                }
            }
            crate::node_config::HttpBodyType::FormData => {
                // Build multipart form from form_fields (text and file fields)
                if let Some(form_fields) = &http_config.form_fields {
                    let mut form = reqwest::multipart::Form::new();
                    let mut text_field_count = 0;
                    let mut file_field_count = 0;
                    
                    for field in form_fields {
                        match field.field_type {
                            crate::node_config::FormFieldType::Text => {
                                // Replace variables in key and value
                                let replaced_key = crate::executor::replace_variables(&field.key, context)
                                    .map_err(|e| format!("表单字段 key 变量替换失败: {}", e))?;
                                let replaced_value = crate::executor::replace_variables(&field.value, context)
                                    .map_err(|e| format!("表单字段 value 变量替换失败: {}", e))?;
                                
                                form = form.text(replaced_key, replaced_value);
                                text_field_count += 1;
                            }
                            crate::node_config::FormFieldType::File => {
                                if let Some(file_path) = &field.file_path {
                                    // Replace variables in file path
                                    let replaced_path = crate::executor::replace_variables(file_path, context)
                                        .map_err(|e| format!("文件路径变量替换失败: {}", e))?;
                                    
                                    // Resolve file path (supports relative paths)
                                    let resolved_path = context.resolve_file_path(&replaced_path);
                                    
                                    log::info!("[流程 {}] [HTTP] 读取表单文件: {:?}", context.flow_uuid, resolved_path);
                                    
                                    // Read file content
                                    let file_content = tokio::fs::read(&resolved_path).await
                                        .map_err(|e| {
                                            let error_msg = format!("读取文件失败 ({}): {}", resolved_path.display(), e);
                                            log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", 
                                                context.flow_uuid, node.uuid, node.name, error_msg);
                                            error_msg
                                        })?;
                                    
                                    // Detect MIME type
                                    let mime_type = mime_guess::from_path(&resolved_path)
                                        .first_or_octet_stream();
                                    
                                    // Extract file name (fallback to "upload")
                                    let file_name = resolved_path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("upload");
                                    
                                    // Replace variables in field key
                                    let replaced_key = crate::executor::replace_variables(&field.key, context)
                                        .map_err(|e| format!("表单字段 key 变量替换失败: {}", e))?;
                                    
                                    // Create multipart part
                                    let part = reqwest::multipart::Part::bytes(file_content)
                                        .file_name(file_name.to_string())
                                        .mime_str(mime_type.essence_str())
                                        .map_err(|e| format!("MIME 类型设置失败: {}", e))?;
                                    
                                    form = form.part(replaced_key, part);
                                    file_field_count += 1;
                                } else {
                                    log::warn!("[流程 {}] [HTTP] 文件字段 '{}' 缺少 file_path，跳过", 
                                        context.flow_uuid, field.key);
                                }
                            }
                        }
                    }
                    
                    // Note: reqwest will automatically set Content-Type: multipart/form-data; boundary=...
                    // But we only log if user hasn't set it manually
                    if !user_set_content_type {
                        sent_headers.insert("Content-Type".to_string(), "multipart/form-data (boundary auto-generated)".to_string());
                        log::info!("[流程 {}] [HTTP] 将自动设置 Content-Type: multipart/form-data", context.flow_uuid);
                    }
                    
                    // 记录 form-data 字段元数据
                    let fields_metadata: Vec<serde_json::Value> = form_fields.iter().map(|f| {
                        match f.field_type {
                            crate::node_config::FormFieldType::Text => {
                                serde_json::json!({
                                    "type": "text",
                                    "key": f.key,
                                    "value": f.value
                                })
                            }
                            crate::node_config::FormFieldType::File => {
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
                    
                    request = request.multipart(form);
                    log::info!("[流程 {}] [HTTP] body_type=form-data, {} 个文本字段, {} 个文件字段", 
                        context.flow_uuid, text_field_count, file_field_count);
                } else {
                    log::warn!("[流程 {}] [HTTP] body_type=form-data, 但 form_fields 为空", context.flow_uuid);
                }
            }
            crate::node_config::HttpBodyType::XWwwFormUrlencoded => {
                // Build form data from form_fields
                if let Some(form_fields) = &http_config.form_fields {
                    let mut form_data: Vec<(String, String)> = Vec::new();
                    
                    for field in form_fields {
                        // Only process text fields
                        if matches!(field.field_type, crate::node_config::FormFieldType::Text) {
                            // Replace variables in key and value
                            let replaced_key = crate::executor::replace_variables(&field.key, context)
                                .map_err(|e| format!("Form field key variable replacement failed: {}", e))?;
                            let replaced_value = crate::executor::replace_variables(&field.value, context)
                                .map_err(|e| format!("Form field value variable replacement failed: {}", e))?;
                            form_data.push((replaced_key, replaced_value));
                        }
                    }
                    
                    // Set Content-Type header only if user hasn't set it manually
                    if !user_set_content_type {
                        request = request.header("Content-Type", "application/x-www-form-urlencoded");
                        sent_headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
                        log::info!("[流程 {}] [HTTP] 自动设置 Content-Type: application/x-www-form-urlencoded", context.flow_uuid);
                    }
                    
                    // 记录 urlencoded 字段数据
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
                    
                    request = request.form(&form_data);
                    log::info!("[流程 {}] [HTTP] body_type=x-www-form-urlencoded, {} fields", context.flow_uuid, form_data.len());
                } else {
                    log::warn!("[流程 {}] [HTTP] body_type=x-www-form-urlencoded, but form_fields is empty", context.flow_uuid);
                }
            }
            crate::node_config::HttpBodyType::Binary => {
                // Binary file upload
                if let Some(file_path) = &http_config.file_path {
                    // Resolve file path (supports relative paths)
                    let resolved_path = context.resolve_file_path(file_path);
                    
                    log::info!("[流程 {}] [HTTP] body_type=binary, 读取文件: {:?}", context.flow_uuid, resolved_path);
                    
                    // Read file content
                    let file_content = match tokio::fs::read(&resolved_path).await {
                        Ok(content) => content,
                        Err(e) => {
                            let error_msg = format!("读取文件失败 ({}): {}", resolved_path.display(), e);
                            log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", 
                                context.flow_uuid, node.uuid, node.name, error_msg);
                            return Err(error_msg);
                        }
                    };
                    
                    // Warn for large files (> 10MB)
                    if file_content.len() > 10 * 1024 * 1024 {
                        log::warn!("[流程 {}] [HTTP] 文件较大 ({} bytes), 可能占用较多内存", 
                            context.flow_uuid, file_content.len());
                    }
                    
                    // Set Content-Type header only if user hasn't set it manually
                    if !user_set_content_type {
                        request = request.header("Content-Type", "application/octet-stream");
                        sent_headers.insert("Content-Type".to_string(), "application/octet-stream".to_string());
                        log::info!("[流程 {}] [HTTP] 自动设置 Content-Type: application/octet-stream", context.flow_uuid);
                    }
                    
                    let file_size = file_content.len();
                    request_body_json = Some(serde_json::json!({
                        "type": "binary",
                        "path": file_path,
                        "size": file_size
                    }));
                    request = request.body(file_content);
                    log::info!("[流程 {}] [HTTP] body_type=binary, 文件大小: {} bytes", context.flow_uuid, file_size);
                } else {
                    let error_msg = "body_type=binary 但 file_path 为空".to_string();
                    log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", 
                        context.flow_uuid, node.uuid, node.name, error_msg);
                    return Err(error_msg);
                }
            }
        }

        // 记录HTTP请求开始时间
        let http_start_time = Instant::now();

        // 发送请求并获取响应
        let response = match request.send().await {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = format!("HTTP请求失败: {}", e);
                log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
                // 发送请求失败日志到前端
                context.emit_log(LogEvent {
                    level: LogLevel::Error,
                    message: format!("HTTP 请求失败: {}", e),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("系统时间错误")
                        .as_millis() as u64,
                    node_uuid: Some(node.uuid.clone()),
                    node_name: Some(node.name.clone()),
                    flow_uuid: context.flow_uuid.clone(),
                }).await;
                return Err(error_msg);
            }
        };

        let status = response.status();
        let status_code = status.as_u16();
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

        // 验证HTTP状态码（2xx视为成功，其他视为失败）
        if !(200..300).contains(&status_code) {
            let error_msg = format!(
                "HTTP请求失败，状态码: {} {}",
                status_code,
                status.canonical_reason().unwrap_or("Unknown")
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
                log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
                error_msg
            })?;

        // 解析响应体为JSON（如果Content-Type是application/json）
        let body_json = if is_json {
            serde_json::from_str(&body_text)
                .map_err(|e| {
                    let error_msg = format!("解析JSON响应体失败: {}", e);
                    log::error!("[流程 {}] [错误] 节点ID: {}, 节点名称: {}, 错误: {}", context.flow_uuid, node.uuid, node.name, error_msg);
                    error_msg
                })?
        } else {
            serde_json::Value::String(body_text)
        };

         // 返回输出数据（向后兼容 + 新增请求信息）
        let output_data = serde_json::json!({
            // ===== 向后兼容：保留顶级字段 =====
            "status": status_code,
            "headers": headers_map,
            "body": body_json,
            
            // ===== 新增：请求信息 =====
            "request": {
                "url": url,
                "method": method_str,
                "headers": sent_headers,
                "body": request_body_json,
                "body_type": body_type_str
            }
        });

        // 应用输出映射，从响应中提取变量
        apply_output_mappings(&output_data, &http_config.output_mappings, context, &node.uuid);

        // 计算总执行时间
        let _duration_ms = start_time.elapsed().as_millis();

        // 执行后日志
        log::info!("[流程 {}] [完成] 节点UUID: {}, 耗时: {}ms, 状态: success", context.flow_uuid, node.uuid, _duration_ms);

        // 发送执行完成日志到前端
        context.emit_log(LogEvent {
            level: LogLevel::Info,
            message: format!("HTTP 节点 {} 完成", node.name),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时间错误")
                .as_millis() as u64,
            node_uuid: Some(node.uuid.clone()),
            node_name: Some(node.name.clone()),
            flow_uuid: context.flow_uuid.clone(),
        }).await;

        // 返回空列表（HTTP 节点不决定下一个节点）和输出数据
        Ok((vec![], Some(output_data)))
    }
}

/// 应用输出映射，从响应中提取变量
fn apply_output_mappings(
    response: &serde_json::Value,
    mappings: &[crate::variable::OutputMapping],
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

/// 按 JSONPath 从 JSON 中提取值
///
/// 支持标准 JSONPath 语法，如:
/// - `$.status` - 根字段
/// - `$.body.data.token` - 嵌套对象
/// - `$.body.items[0]` - 数组索引
/// - `$.body.items[-1]` - 负索引（从末尾）
fn extract_value_by_path(value: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
    if path.is_empty() {
        return None;
    }

    // 解析 JSONPath
    let json_path = match JsonPath::parse(path) {
        Ok(p) => p,
        Err(_) => {
            log::warn!("无效的 JSONPath 语法: {}", path);
            return None;
        }
    };

    // 执行查询
    let result = json_path.query(value);
    
    // 获取单个结果
    match result.exactly_one() {
        Ok(v) => Some(v.clone()),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use crate::context::VariableStorage;
    use crate::node_config::*;
    use crate::node::*;

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
            node_type: NodeType::Http,
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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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

        let mut context = create_context("test-flow", json!({}));
        let executor = HttpNodeExecutor::new();

        let result: Result<(Vec<String>, Option<serde_json::Value>), String> =
            executor.execute(&node, &mut context, &[], &HashMap::new()).await;

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
            node_type: NodeType::Http,
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
        };

        let mut context = ExecutionContext {
            flow_uuid: flow_id.clone(),
            current_node_uuid: Some(http_node.uuid.clone()),
            variables: VariableStorage::new(),
            node_reports: Vec::new(),
            flow_trigger_time: 1000,
            log_sender: None,
            flow_file_path: None,
        };

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
            node_type: NodeType::Http,
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
        };

        let mut context = ExecutionContext {
                            flow_uuid: flow_id.clone(),
                            current_node_uuid: Some(http_node.uuid.clone()),
                            variables: VariableStorage::new(),
                            node_reports: Vec::new(),
                            flow_trigger_time: 1000,
                            log_sender: None,
                            flow_file_path: None,
                        };

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
            node_type: NodeType::Http,
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
        };

        let mut context = ExecutionContext {
                            flow_uuid: flow_id.clone(),
                            current_node_uuid: Some(http_node.uuid.clone()),
                            variables: VariableStorage::new(),
                            node_reports: Vec::new(),
                            flow_trigger_time: 1000,
                            log_sender: None,
                            flow_file_path: None,
                        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

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
        };

        let mut context = ExecutionContext {
                            flow_uuid: flow_id.clone(),
                            current_node_uuid: Some(http_node.uuid.clone()),
                            variables: VariableStorage::new(),
                            node_reports: Vec::new(),
                            flow_trigger_time: 1000,
                            log_sender: None,
                            flow_file_path: None,
                        };

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
            node_type: NodeType::Http,
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
        };

        let mut context = ExecutionContext {
                            flow_uuid: flow_id.clone(),
                            current_node_uuid: Some(http_node.uuid.clone()),
                            variables: VariableStorage::new(),
                            node_reports: Vec::new(),
                            flow_trigger_time: 1000,
                            log_sender: None,
                            flow_file_path: None,
                        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

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
        };

        let mut storage = VariableStorage::new();
        storage.set_variable("test_param".to_string(), json!("value123"), String::new());
        storage.set_variable("test_header".to_string(), json!("header_value"), String::new());
        storage.set_variable("test_body".to_string(), json!("body_value"), String::new());
        let mut context = ExecutionContext {
                            flow_uuid: flow_id.clone(),
                            current_node_uuid: Some(http_node.uuid.clone()),
                            variables: storage,
                            node_reports: Vec::new(),
                            flow_trigger_time: 1000,
                            log_sender: None,
                            flow_file_path: None,
                        };

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
            node_type: NodeType::Http,
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
        };

        let mut context = ExecutionContext {
                            flow_uuid: flow_id.clone(),
                            current_node_uuid: Some(http_node.uuid.clone()),
                            variables: VariableStorage::new(),
                            node_reports: Vec::new(),
                            flow_trigger_time: 1000,
                            log_sender: None,
                            flow_file_path: None,
                        };

        let executor = HttpNodeExecutor::new();
        let result = executor.execute(&http_node, &mut context, &[], &HashMap::new()).await;

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
        };

        let mut context = ExecutionContext {
                            flow_uuid: flow_id.clone(),
                            current_node_uuid: Some(http_node.uuid.clone()),
                            variables: VariableStorage::new(),
                            node_reports: Vec::new(),
                            flow_trigger_time: 1000,
                            log_sender: None,
                            flow_file_path: None,
                        };

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
            node_type: NodeType::Http,
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
        };

        let mut context = ExecutionContext {
                            flow_uuid: flow_id.clone(),
                            current_node_uuid: Some(http_node.uuid.clone()),
                            variables: VariableStorage::new(),
                            node_reports: Vec::new(),
                            flow_trigger_time: 1000,
                            log_sender: None,
                            flow_file_path: None,
                        };

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
            node_type: NodeType::Http,
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
        };

        let mut context = ExecutionContext {
                            flow_uuid: flow_id.clone(),
                            current_node_uuid: Some(http_node.uuid.clone()),
                            variables: VariableStorage::new(),
                            node_reports: Vec::new(),
                            flow_trigger_time: 1000,
                            log_sender: None,
                            flow_file_path: None,
                        };

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
            node_type: NodeType::Http,
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
        };

        let mut context = ExecutionContext {
                            flow_uuid: flow_id.clone(),
                            current_node_uuid: Some(http_node.uuid.clone()),
                            variables: VariableStorage::new(),
                            node_reports: Vec::new(),
                            flow_trigger_time: 1000,
                            log_sender: None,
                            flow_file_path: None,
                        };

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
            node_type: NodeType::Http,
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
        };

        let mut context = ExecutionContext {
                            flow_uuid: flow_id.clone(),
                            current_node_uuid: Some(http_node.uuid.clone()),
                            variables: VariableStorage::new(),
                            node_reports: Vec::new(),
                            flow_trigger_time: 1000,
                            log_sender: None,
                            flow_file_path: None,
                        };

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
            node_type: NodeType::Http,
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
        };

        let mut context = ExecutionContext {
                            flow_uuid: flow_id.clone(),
                            current_node_uuid: Some(http_node.uuid.clone()),
                            variables: VariableStorage::new(),
                            node_reports: Vec::new(),
                            flow_trigger_time: 1000,
                            log_sender: None,
                            flow_file_path: None,
                        };

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
}
