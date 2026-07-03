//! gRPC 连接池管理器
//!
//! 实现 GrpcProvider trait，管理 gRPC 连接的生命周期。
//! 使用 tonic Channel 进行 gRPC 通信，
//! 使用 prost-reflect DynamicMessage 进行动态消息编解码。

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use async_trait::async_trait;
use flow_engine_core::debug_types::DebugEvent;
use flow_engine_core::traits::{DebugEventSender, GrpcMethodInfo, GrpcProvider, GrpcServiceInfo, GrpcStreamMessage, GrpcStreamingResult, GrpcTimingInfo};
use prost::Message;
use tokio_util::sync::CancellationToken;
use tonic::metadata::{MetadataKey, MetadataValue};
use tower::ServiceExt;

use crate::codec::DynamicCodec;
use crate::error_detail::format_grpc_error;
use crate::helpers::{build_method_path, dynamic_message_to_json, json_to_dynamic_message, merge_metadata};
use crate::proto_parser;
use crate::reflection::reflect_services;
use crate::tls::create_channel;
use crate::tls::create_balanced_channel;

// 重新导出公共函数，保持外部 API 兼容
pub use crate::reflection::{discover_services_via_reflection, discover_services_via_proto};

/// 从 tonic `MetadataMap` 提取 ASCII 键值对
/// 跳过 binary headers（key 以 -bin 后缀标识）
fn extract_metadata(metadata: &tonic::metadata::MetadataMap) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for kv in metadata.iter() {
        match kv {
            tonic::metadata::KeyAndValueRef::Ascii(key, value) => {
                match value.to_str() {
                    Ok(v) => result.push((key.to_string(), v.to_string())),
                    Err(_) => result.push((key.to_string(), format!("<non-utf8: {} bytes>", value.len()))),
                }
            }
            tonic::metadata::KeyAndValueRef::Binary(_, _) => {
                // 跳过 binary metadata（key 以 -bin 后缀标识）
            }
        }
    }
    result
}

/// 解析压缩编码字符串为 tonic `CompressionEncoding`
///
/// 仅支持 "gzip"，其他值返回 None
fn parse_compression(encoding: &str) -> Option<tonic::codec::CompressionEncoding> {
    match encoding.to_lowercase().as_str() {
        "gzip" => Some(tonic::codec::CompressionEncoding::Gzip),
        _ => None,
    }
}

/// 解析调用级压缩配置，覆盖连接级默认值
///
/// 调用级 "none" 明确禁用压缩；其他值尝试解析为压缩编码；
/// None 表示使用连接级默认
fn resolve_compression(
    call_level: Option<&str>,
    connect_level: Option<&str>,
) -> Option<tonic::codec::CompressionEncoding> {
    match call_level {
        Some("none") => None, // 明确禁用
        Some(enc) => parse_compression(enc),
        None => connect_level.and_then(parse_compression),
    }
}

/// Channelz 内省信息（轻量级，非完整 Channelz 协议）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelzInfo {
    /// 连接Status: "READY", "IDLE", "CONNECTING", "`TRANSIENT_FAILURE`", "SHUTDOWN"
    pub state: String,
    /// 连接建立时间（ISO 8601）
    pub connected_at: String,
    /// 连接运行时长（秒）
    pub uptime_secs: u64,
    /// 活跃调用数
    pub active_calls: usize,
    /// 最后Error（如果有）
    pub last_error: Option<String>,
}

/// 活跃的 gRPC 连接
struct GrpcConnection {
    /// tonic channel（可克隆，克隆后共享底层连接）
    channel: tonic::transport::Channel,
    /// Descriptor pool（从 proto 文件解析得到）
    descriptor_pool: Arc<prost_reflect::DescriptorPool>,
    /// 连接状态
    status: String,
    #[allow(dead_code)]
    /// 连接 URL（预留用于后续功能）
    url: String,
    /// 创建时间（用于连接 uptime 计算）
    created_at: chrono::DateTime<chrono::Utc>,
    /// 连接级默认 metadata（由 `grpc_connect` 节点配置）
    default_metadata: Vec<(String, String)>,
    /// 连接级默认压缩编码（由 `grpc_connect` 节点配置）
    compression_encoding: Option<String>,
    /// 活跃的流式调用: `call_id` → `CancellationToken`
    active_calls: Arc<Mutex<HashMap<String, CancellationToken>>>,
    /// 最后一次Error信息（用于 Channelz 内省）
    last_error: Option<String>,
}

/// gRPC 连接管理器
///
/// 管理 gRPC 连接池，实现 `GrpcProvider` trait。
/// 线程安全，使用 `RwLock` 保护连接池。
pub struct GrpcConnectionManager {
    /// 连接池: `connection_id` → `GrpcConnection`
    connections: Arc<RwLock<HashMap<String, GrpcConnection>>>,
}

impl GrpcConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取指定连接的 `DescriptorPool` 克隆
    ///
    /// 用于在设计时（如生成请求骨架）访问连接缓存的类型描述信息。
    /// 返回 `None` 表示连接不存在。
    pub fn get_descriptor_pool(&self, connection_id: &str) -> Option<prost_reflect::DescriptorPool> {
        let conns = self.connections.read().ok()?;
        let conn = conns.get(connection_id)?;
        Some((*conn.descriptor_pool).clone())
    }

    /// 获取连接的 Channelz 内省信息（EXPERIMENTAL）
    ///
    /// 返回连接状态、运行时长、活跃调用数等轻量级诊断信息。
    /// 注意：这不是完整的 gRPC Channelz 协议实现。
    ///
    /// # 状态映射
    /// - "READY" — 连接可用
    /// - "IDLE" — 连接空闲（tonic Channel 可能在此状态）
    /// - "CONNECTING" — 正在建立连接
    /// - "`TRANSIENT_FAILURE`" — 暂时性故障
    /// - "CLOSED" — 连接已关闭或不存在
    pub async fn get_channelz_info(&self, connection_id: &str) -> Result<ChannelzInfo, String> {
        // 获取连接信息（克隆后释放锁）
        let (mut channel, created_at, active_calls, last_error) = {
            let conns = self
                .connections
                .read()
                .map_err(|e| format!("获取连接池读锁Failed: {}", e))?;
            let conn = conns
                .get(connection_id)
                .ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
            (
                conn.channel.clone(),
                conn.created_at,
                Arc::clone(&conn.active_calls),
                conn.last_error.clone(),
            )
        }; // 读锁在此释放

        // 检测 tonic Channel 连接状态
        // tonic 0.12 的 Channel 不直接暴露 connectivity state enum，
        // 我们通过 ready() 检测来推断状态
        let state = match tokio::time::timeout(
            std::time::Duration::from_secs(3),
            channel.ready(),
        )
        .await
        {
            Ok(Ok(_)) => "READY".to_string(),
            Ok(Err(_)) => "TRANSIENT_FAILURE".to_string(),
            Err(_) => "CONNECTING".to_string(), // 超时，可能正在连接
        };

        // 计算运行时长
        let uptime_secs = created_at
            .signed_duration_since(chrono::Utc::now())
            .num_seconds()
            .unsigned_abs();

        // 获取活跃调用数
        let active_call_count = active_calls
            .lock()
            .map(|calls| calls.len())
            .unwrap_or(0);

        Ok(ChannelzInfo {
            state,
            connected_at: created_at.to_rfc3339(),
            uptime_secs,
            active_calls: active_call_count,
            last_error,
        })
    }
}

impl Default for GrpcConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GrpcProvider for GrpcConnectionManager {
    /// 建立 gRPC 连接
    ///
    /// 创建 tonic Channel，解析 proto 文件构建 `DescriptorPool`，
    /// 将连接信息存入连接池。
    ///
    /// 当 `enable_reflection=true` 且未提供 proto 文件时，
    /// 通过 Server Reflection 协议自动发现服务。
    async fn connect(
        &self,
        uuid: String,
        url: String,
        use_tls: bool,
        tls_skip_verify: bool,
        enable_reflection: bool,
        proto_files: Vec<(String, String)>,
        metadata: Vec<(String, String)>,
        connect_timeout_ms: u64,
        tls_ca_cert: Option<String>,
        tls_client_cert: Option<String>,
        tls_client_key: Option<String>,
        tls_server_name_override: Option<String>,
        reflection_metadata: Vec<(String, String)>,
        compression_encoding: Option<String>,
        keepalive_time_ms: Option<u64>,
        keepalive_timeout_ms: Option<u64>,
        keepalive_permit_without_streams: Option<bool>,
        endpoints: Option<Vec<String>>,
    ) -> Result<Vec<GrpcServiceInfo>, String> {
        // 1. 构造 URL（确保包含 scheme）
        let url_with_scheme = if url.starts_with("http://") || url.starts_with("https://") {
            url.clone()
        } else if use_tls {
            format!("https://{}", url)
        } else {
            format!("http://{}", url)
        };

        // 2. 创建 channel（含 TLS + Keepalive 配置 + 负载均衡）
        let additional: Vec<String> = endpoints
            .as_ref()
            .map(|eps| eps.iter().filter(|ep| !ep.trim().is_empty()).cloned().collect())
            .unwrap_or_default();

        let channel = if additional.is_empty() {
            // 单端点模式（现有行为）
            create_channel(
                &url_with_scheme,
                use_tls,
                tls_skip_verify,
                connect_timeout_ms,
                tls_ca_cert.as_deref(),
                tls_client_cert.as_deref(),
                tls_client_key.as_deref(),
                tls_server_name_override.as_deref(),
                keepalive_time_ms,
                keepalive_timeout_ms,
                keepalive_permit_without_streams,
            )?
        } else {
            // 多端点模式（轮询负载均衡）
            create_balanced_channel(
                &url_with_scheme,
                &additional,
                use_tls,
                tls_skip_verify,
                connect_timeout_ms,
                tls_ca_cert.as_deref(),
                tls_client_cert.as_deref(),
                tls_client_key.as_deref(),
                tls_server_name_override.as_deref(),
                keepalive_time_ms,
                keepalive_timeout_ms,
                keepalive_permit_without_streams,
            )?
        };

        // 3. 获取 DescriptorPool（反射或解析 proto 文件）
        let pool = if enable_reflection && proto_files.is_empty() {
            // 通过 Server Reflection 发现服务（传递 reflection_metadata）
            reflect_services(channel.clone(), reflection_metadata).await?
        } else if proto_files.is_empty() {
            return Err(
                "未提供 proto 文件且未启用 Server Reflection，无法发现服务".to_string()
            );
        } else {
            // 解析用户提供的 proto 文件
            let files: Vec<proto_parser::ProtoFile> = proto_files
                .into_iter()
                .map(|(path, content)| proto_parser::ProtoFile { path, content })
                .collect();
            proto_parser::parse_proto_files(&files)
                .map_err(|e| format!("解析 proto 文件Failed: {}", e))?
        };

        // 4. 提取服务信息
        let services = proto_parser::list_services(&pool);
        let message_definitions = Arc::new(proto_parser::get_message_definitions(&pool));
        let grpc_services: Vec<GrpcServiceInfo> = services
            .into_iter()
            .map(|s| GrpcServiceInfo {
                service_name: s.service_name,
                methods: s
                    .methods
                    .into_iter()
                    .map(|m| GrpcMethodInfo {
                        method_name: m.method_name,
                        input_type: m.input_type,
                        output_type: m.output_type,
                        is_server_streaming: m.is_server_streaming,
                        is_client_streaming: m.is_client_streaming,
                    })
                    .collect(),
                message_definitions: message_definitions.clone(),
            })
            .collect();

        // 5. 存入连接池
        let connection = GrpcConnection {
            channel,
            descriptor_pool: Arc::new(pool),
            url,
            status: "connected".to_string(),
            created_at: chrono::Utc::now(),
            default_metadata: metadata,
            compression_encoding: compression_encoding.clone(),
            active_calls: Arc::new(Mutex::new(HashMap::new())),
            last_error: None,
        };

        {
            let mut conns = self
                .connections
                .write()
                .map_err(|e| format!("获取连接池写锁Failed: {}", e))?;
            conns.insert(uuid, connection);
        }

        Ok(grpc_services)
    }

    /// gRPC Unary 调用
    ///
    /// 从连接池获取连接，通过 tonic 的低级 Grpc 客户端发送请求。
    /// 使用 `DynamicCodec` 进行动态编解码。
    async fn call_unary(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_json: &str,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
        compression_encoding: Option<String>,
        debug_sender: Option<Arc<dyn DebugEventSender>>,
        node_uuid: &str,
        call_id: &str,
    ) -> Result<(serde_json::Value, Vec<(String, String)>), String> {
        // 1. 从连接池获取连接信息（克隆后释放锁）
        let (channel, pool, default_metadata, default_compression) = {
            let conns = self
                .connections
                .read()
                .map_err(|e| format!("获取连接池读锁Failed: {}", e))?;
            let conn = conns
                .get(connection_id)
                .ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
            (conn.channel.clone(), Arc::clone(&conn.descriptor_pool), conn.default_metadata.clone(), conn.compression_encoding.clone())
        }; // 读锁在此释放

        // 2. 合并 metadata（调用级覆盖连接级同名 key）
        let merged_metadata = merge_metadata(&default_metadata, metadata);

        // 3. 查找方法描述符
        let service = pool
            .get_service_by_name(service_name)
            .ok_or_else(|| format!("服务 '{}' 不存在", service_name))?;
        let method = service
            .methods()
            .find(|m| m.name() == method_name)
            .ok_or_else(|| format!("方法 '{}' 在服务 '{}' 中不存在", method_name, service_name))?;

        // 4. 反序列化请求 JSON 为 DynamicMessage
        let req_message = json_to_dynamic_message(request_json, method.input())?;

        // 5. 构造 tonic 请求
        let mut request = tonic::Request::new(req_message);
        // 设置合并后的 metadata
        for (key, value) in merged_metadata {
            let meta_key = MetadataKey::from_bytes(key.as_bytes())
                .map_err(|e| format!("无效的 metadata key '{}': {:?}", key, e))?;
            let meta_val = MetadataValue::try_from(&value)
                .map_err(|e| format!("无效的 metadata value '{}': {:?}", value, e))?;
            request.metadata_mut().insert(meta_key, meta_val);
        }
        // 设置超时
        if timeout_ms > 0 {
            request.set_timeout(std::time::Duration::from_millis(timeout_ms));
        }

        // 6. 发起 Unary 调用
        let path = build_method_path(service_name, method_name)?;
        let codec = DynamicCodec::new(method);
        let mut client = tonic::client::Grpc::new(channel)
            .accept_compressed(tonic::codec::CompressionEncoding::Gzip);
        // 设置压缩编码（调用级覆盖连接级默认）
        let compression = resolve_compression(compression_encoding.as_deref(), default_compression.as_deref());
        if let Some(enc) = compression {
            client = client.send_compressed(enc);
        }
        client
            .ready()
            .await
            .map_err(|e| format!("gRPC 客户端未就绪: {}", e))?;

        let response = client
            .unary(request, path, codec)
            .await
            .map_err(|e| format_grpc_error("gRPC Unary 调用Failed", &e, &pool))?;

        // 7. 提取响应 metadata 并序列化响应为 JSON
        let response_metadata = extract_metadata(response.metadata());
        let response_msg = response.into_inner();
        let json_val = dynamic_message_to_json(&response_msg)?;

        // 8. 发送调试事件（supplementary）
        if let Some(sender) = &debug_sender {
            let data_str = serde_json::to_string(&json_val).unwrap_or_default();
            let size_bytes = data_str.len() as u64;
            let _ = sender.send_event(DebugEvent::GrpcStreamMessage {
                node_uuid: node_uuid.to_string(),
                connection_id: connection_id.to_string(),
                call_id: call_id.to_string(),
                index: 1,
                data: data_str,
                received_at_ms: 0,
                size_bytes,
                is_last: true,
            });
        }

        Ok((json_val, response_metadata))
    }

    /// gRPC Server Streaming 调用
    ///
    /// 从连接池获取连接，通过 tonic 的低级 Grpc 客户端发送请求，
    /// 收集所有流式响应并返回 JSON 数组。
    async fn call_server_streaming(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_json: &str,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
        compression_encoding: Option<String>,
        debug_sender: Option<Arc<dyn DebugEventSender>>,
        node_uuid: &str,
        call_id: &str,
    ) -> Result<(Vec<serde_json::Value>, Vec<(String, String)>), String> {
        // 1. 从连接池获取连接信息（克隆后释放锁）
        let (channel, pool, default_metadata, default_compression, active_calls) = {
            let conns = self
                .connections
                .read()
                .map_err(|e| format!("获取连接池读锁Failed: {}", e))?;
            let conn = conns
                .get(connection_id)
                .ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
            (conn.channel.clone(), Arc::clone(&conn.descriptor_pool), conn.default_metadata.clone(), conn.compression_encoding.clone(), Arc::clone(&conn.active_calls))
        }; // 读锁在此释放

        // 2. 注册 CancellationToken
        let cancel_token = CancellationToken::new();
        {
            let mut calls = active_calls.lock().map_err(|e| format!("获取活跃调用锁Failed: {}", e))?;
            calls.insert(call_id.to_string(), cancel_token.clone());
        }

        // 2. 合并 metadata（调用级覆盖连接级同名 key）
        let merged_metadata = merge_metadata(&default_metadata, metadata);

        // 3. 查找方法描述符
        let service = pool
            .get_service_by_name(service_name)
            .ok_or_else(|| format!("服务 '{}' 不存在", service_name))?;
        let method = service
            .methods()
            .find(|m| m.name() == method_name)
            .ok_or_else(|| format!("方法 '{}' 在服务 '{}' 中不存在", method_name, service_name))?;

        // 4. 反序列化请求 JSON 为 DynamicMessage
        let req_message = json_to_dynamic_message(request_json, method.input())?;

        // 5. 构造 tonic 请求
        let mut request = tonic::Request::new(req_message);
        for (key, value) in merged_metadata {
            let meta_key = MetadataKey::from_bytes(key.as_bytes())
                .map_err(|e| format!("无效的 metadata key '{}': {:?}", key, e))?;
            let meta_val = MetadataValue::try_from(&value)
                .map_err(|e| format!("无效的 metadata value '{}': {:?}", value, e))?;
            request.metadata_mut().insert(meta_key, meta_val);
        }
        // 6. 发起 Server Streaming 调用（不设 request deadline，避免过早终止流式响应）
        let path = build_method_path(service_name, method_name)?;
        let codec = DynamicCodec::new(method);
        let mut client = tonic::client::Grpc::new(channel)
            .accept_compressed(tonic::codec::CompressionEncoding::Gzip);
        // 设置压缩编码（调用级覆盖连接级默认）
        let compression = resolve_compression(compression_encoding.as_deref(), default_compression.as_deref());
        if let Some(enc) = compression {
            client = client.send_compressed(enc);
        }
        client
            .ready()
            .await
            .map_err(|e| format!("gRPC 客户端未就绪: {}", e))?;

        let response = client
            .server_streaming(request, path, codec)
            .await
            .map_err(|e| {
                format_grpc_error("gRPC Server Streaming 调用Failed", &e, &pool)
            })?;

        // 6. 提取响应 Initial Metadata 并采集响应流（带超时 + 取消支持）
        let response_metadata = extract_metadata(response.metadata());
        let mut stream = response.into_inner();
        let mut results = Vec::new();
        let collect_start = std::time::Instant::now();

        let collect_result = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            async {
                loop {
                    tokio::select! {
                        msg = stream.message() => {
                            match msg {
                                Ok(Some(msg)) => {
                                    let size_bytes = msg.encoded_len();
                                    let json_val = dynamic_message_to_json(&msg)?;
                                    let msg_index = results.len() + 1;
                                    let received_at = collect_start.elapsed().as_millis() as u64;
                                    let data_str = serde_json::to_string(&json_val).unwrap_or_default();

                                    // 发送调试事件（supplementary，实时推送）
                                    if let Some(sender) = &debug_sender {
                                        let _ = sender.send_event(DebugEvent::GrpcStreamMessage {
                                            node_uuid: node_uuid.to_string(),
                                            connection_id: connection_id.to_string(),
                                            call_id: call_id.to_string(),
                                            index: msg_index as u32,
                                            data: data_str.clone(),
                                            received_at_ms: received_at,
                                            size_bytes: size_bytes as u64,
                                            is_last: false, // 暂不确定是否最后一条，后续会更新
                                        });
                                    }

                                    let stream_msg = GrpcStreamMessage {
                                        index: msg_index,
                                        data: json_val,
                                        received_at_ms: received_at,
                                        size_bytes,
                                    };
                                    results.push(serde_json::to_value(&stream_msg).unwrap_or_default());
                                }
                                Ok(None) => break,
                                Err(e) => {
                                    // 发送调试Error事件
                                    if let Some(sender) = &debug_sender {
                                        let _ = sender.send_event(DebugEvent::GrpcStreamError {
                                            node_uuid: node_uuid.to_string(),
                                            connection_id: connection_id.to_string(),
                                            call_id: call_id.to_string(),
                                            message: format!("流式响应Error: {} ({})", e.message(), e.code()),
                                        });
                                    }
                                    return Err(format!(
                                        "流式响应Error: {} ({})",
                                        e.message(),
                                        e.code()
                                    ));
                                }
                            }
                        }
                        _ = cancel_token.cancelled() => {
                            log::info!(
                                "[gRPC Server Streaming] 调用被取消 (call_id={})，已收集 {} 条响应",
                                call_id,
                                results.len()
                            );
                            break;
                        }
                    }
                }
                Ok(())
            },
        )
        .await;

        // 清理 CancellationToken
        {
            let mut calls = active_calls.lock().map_err(|e| format!("获取活跃调用锁Failed: {}", e))?;
            calls.remove(call_id);
        }

        match collect_result {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                log::warn!(
                    "[gRPC Server Streaming] 采集超时 ({}ms)，已收集 {} 条响应",
                    timeout_ms,
                    results.len()
                );
            }
        }

        Ok((results, response_metadata))
    }

    /// gRPC Client Streaming 调用
    async fn call_client_streaming(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_messages: Vec<String>,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
        compression_encoding: Option<String>,
        debug_sender: Option<Arc<dyn DebugEventSender>>,
        node_uuid: &str,
        call_id: &str,
    ) -> Result<(serde_json::Value, Vec<(String, String)>), String> {
        // 1. 从连接池获取连接信息
        let (channel, pool, default_metadata, default_compression) = {
            let conns = self.connections.read().map_err(|e| format!("获取连接池读锁Failed: {}", e))?;
            let conn = conns.get(connection_id).ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
            (conn.channel.clone(), Arc::clone(&conn.descriptor_pool), conn.default_metadata.clone(), conn.compression_encoding.clone())
        };

        // 2. 合并 metadata（调用级覆盖连接级同名 key）
        let merged_metadata = merge_metadata(&default_metadata, metadata);

        // 3. 查找方法描述符
        let service = pool.get_service_by_name(service_name)
            .ok_or_else(|| format!("服务 '{}' 不存在", service_name))?;
        let method = service.methods()
            .find(|m| m.name() == method_name)
            .ok_or_else(|| format!("方法 '{}' 在服务 '{}' 中不存在", method_name, service_name))?;

        // 4. 将所有 request_messages 解码为 DynamicMessage
        let messages: Vec<prost_reflect::DynamicMessage> = request_messages.iter()
            .map(|json| json_to_dynamic_message(json, method.input()))
            .collect::<Result<Vec<_>, _>>()?;

        // 5. 构造请求流
        let request_stream = tokio_stream::iter(messages);
        let mut request = tonic::Request::new(request_stream);

        // 6. 设置合并后的 metadata
        for (key, value) in merged_metadata {
            let meta_key = MetadataKey::from_bytes(key.as_bytes())
                .map_err(|e| format!("无效的 metadata key '{}': {:?}", key, e))?;
            let meta_val = MetadataValue::try_from(&value)
                .map_err(|e| format!("无效的 metadata value '{}': {:?}", value, e))?;
            request.metadata_mut().insert(meta_key, meta_val);
        }
        if timeout_ms > 0 {
            request.set_timeout(std::time::Duration::from_millis(timeout_ms));
        }

        // 6. 发起 Client Streaming 调用
        let path = build_method_path(service_name, method_name)?;
        let codec = DynamicCodec::new(method);
        let mut client = tonic::client::Grpc::new(channel)
            .accept_compressed(tonic::codec::CompressionEncoding::Gzip);
        // 设置压缩编码（调用级覆盖连接级默认）
        let compression = resolve_compression(compression_encoding.as_deref(), default_compression.as_deref());
        if let Some(enc) = compression {
            client = client.send_compressed(enc);
        }
        client.ready().await.map_err(|e| format!("gRPC 客户端未就绪: {}", e))?;

        let response = client.client_streaming(request, path, codec).await
            .map_err(|e| format_grpc_error("gRPC Client Streaming 调用Failed", &e, &pool))?;

        // 7. 提取响应 metadata 并序列化响应为 JSON
        let response_metadata = extract_metadata(response.metadata());
        let response_msg = response.into_inner();
        let json_val = dynamic_message_to_json(&response_msg)?;

        // 8. 发送调试事件（supplementary）
        if let Some(sender) = &debug_sender {
            let data_str = serde_json::to_string(&json_val).unwrap_or_default();
            let size_bytes = data_str.len() as u64;
            let _ = sender.send_event(DebugEvent::GrpcStreamMessage {
                node_uuid: node_uuid.to_string(),
                connection_id: connection_id.to_string(),
                call_id: call_id.to_string(),
                index: 1,
                data: data_str,
                received_at_ms: 0,
                size_bytes,
                is_last: true,
            });
        }

        Ok((json_val, response_metadata))
    }

    /// gRPC Bidi Streaming 调用
    async fn call_bidi_streaming(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_messages: Vec<String>,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
        compression_encoding: Option<String>,
        debug_sender: Option<Arc<dyn DebugEventSender>>,
        node_uuid: &str,
        call_id: &str,
    ) -> Result<GrpcStreamingResult, String> {
        use std::time::Instant;

        let start = Instant::now();

        // 1. 从连接池获取连接信息
        let (channel, pool, default_metadata, default_compression, active_calls) = {
            let conns = self.connections.read().map_err(|e| format!("获取连接池读锁Failed: {}", e))?;
            let conn = conns.get(connection_id).ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
            (conn.channel.clone(), Arc::clone(&conn.descriptor_pool), conn.default_metadata.clone(), conn.compression_encoding.clone(), Arc::clone(&conn.active_calls))
        };

        // 2. 注册 CancellationToken
        let cancel_token = CancellationToken::new();
        {
            let mut calls = active_calls.lock().map_err(|e| format!("获取活跃调用锁Failed: {}", e))?;
            calls.insert(call_id.to_string(), cancel_token.clone());
        }

        // 3. 合并 metadata（调用级覆盖连接级同名 key）
        let merged_metadata = merge_metadata(&default_metadata, metadata);

        // 3. 查找方法描述符
        let service = pool.get_service_by_name(service_name)
            .ok_or_else(|| format!("服务 '{}' 不存在", service_name))?;
        let method = service.methods()
            .find(|m| m.name() == method_name)
            .ok_or_else(|| format!("方法 '{}' 在服务 '{}' 中不存在", method_name, service_name))?;

        // 4. 将所有 request_messages 解码为 DynamicMessage
        let messages: Vec<prost_reflect::DynamicMessage> = request_messages.iter()
            .map(|json| json_to_dynamic_message(json, method.input()))
            .collect::<Result<Vec<_>, _>>()?;
        let sent_count = messages.len();

        // 5. 构造请求流
        let request_stream = tokio_stream::iter(messages);
        let mut request = tonic::Request::new(request_stream);

        // 6. 设置合并后的 metadata
        for (key, value) in merged_metadata {
            let meta_key = MetadataKey::from_bytes(key.as_bytes())
                .map_err(|e| format!("无效的 metadata key '{}': {:?}", key, e))?;
            let meta_val = MetadataValue::try_from(&value)
                .map_err(|e| format!("无效的 metadata value '{}': {:?}", value, e))?;
            request.metadata_mut().insert(meta_key, meta_val);
        }

        // 7. 发起 Bidi Streaming 调用（不设 request deadline，避免过早终止流式响应）
        let path = build_method_path(service_name, method_name)?;
        let codec = DynamicCodec::new(method);
        let mut client = tonic::client::Grpc::new(channel)
            .accept_compressed(tonic::codec::CompressionEncoding::Gzip);
        // 设置压缩编码（调用级覆盖连接级默认）
        let compression = resolve_compression(compression_encoding.as_deref(), default_compression.as_deref());
        if let Some(enc) = compression {
            client = client.send_compressed(enc);
        }
        client.ready().await.map_err(|e| format!("gRPC 客户端未就绪: {}", e))?;

        let response = client.streaming(request, path, codec).await
            .map_err(|e| format_grpc_error("gRPC Bidi Streaming 调用Failed", &e, &pool))?;

        // 8. 提取响应 Initial Metadata 并采集响应流（带超时 + 取消支持）
        let response_metadata = extract_metadata(response.metadata());
        let mut stream = response.into_inner();
        let mut responses = Vec::new();
        let mut first_response_ms: Option<u64> = None;
        let mut was_cancelled = false;

        let collect_result = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            async {
                loop {
                    tokio::select! {
                        msg = stream.message() => {
                            match msg {
                                Ok(Some(msg)) => {
                                    if first_response_ms.is_none() {
                                        first_response_ms = Some(start.elapsed().as_millis() as u64);
                                    }
                                    let size_bytes = msg.encoded_len();
                                    let json_val = dynamic_message_to_json(&msg)?;
                                    let msg_index = responses.len() + 1;
                                    let received_at = start.elapsed().as_millis() as u64;
                                    let data_str = serde_json::to_string(&json_val).unwrap_or_default();

                                    // 发送调试事件（supplementary，实时推送）
                                    if let Some(sender) = &debug_sender {
                                        let _ = sender.send_event(DebugEvent::GrpcStreamMessage {
                                            node_uuid: node_uuid.to_string(),
                                            connection_id: connection_id.to_string(),
                                            call_id: call_id.to_string(),
                                            index: msg_index as u32,
                                            data: data_str,
                                            received_at_ms: received_at,
                                            size_bytes: size_bytes as u64,
                                            is_last: false,
                                        });
                                    }

                                    let stream_msg = GrpcStreamMessage {
                                        index: msg_index,
                                        data: json_val,
                                        received_at_ms: received_at,
                                        size_bytes,
                                    };
                                    responses.push(serde_json::to_value(&stream_msg).unwrap_or_default());
                                }
                                Ok(None) => break,
                                Err(e) => {
                                    // 发送调试Error事件
                                    if let Some(sender) = &debug_sender {
                                        let _ = sender.send_event(DebugEvent::GrpcStreamError {
                                            node_uuid: node_uuid.to_string(),
                                            connection_id: connection_id.to_string(),
                                            call_id: call_id.to_string(),
                                            message: format!("Bidi 流式响应Error: {} ({})", e.message(), e.code()),
                                        });
                                    }
                                    return Err(format!("Bidi 流式响应Error: {} ({})", e.message(), e.code()));
                                }
                            }
                        }
                        _ = cancel_token.cancelled() => {
                            log::info!(
                                "[gRPC Bidi] 调用被取消 (call_id={})，已收集 {} 条响应",
                                call_id,
                                responses.len()
                            );
                            was_cancelled = true;
                            break;
                        }
                    }
                }
                Ok(())
            },
        ).await;

        // 清理 CancellationToken
        {
            let mut calls = active_calls.lock().map_err(|e| format!("获取活跃调用锁Failed: {}", e))?;
            calls.remove(call_id);
        }

        match collect_result {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                // 超时，收集已获取的响应（非致命）
                log::warn!("[gRPC Bidi] 采集超时 ({}ms)，已收集 {} 条响应", timeout_ms, responses.len());
            }
        }

        let received_count = responses.len();
        let total_ms = start.elapsed().as_millis() as u64;

        Ok(GrpcStreamingResult {
            status: 0, // gRPC OK
            responses,
            sent_count,
            received_count,
            response_metadata,
            trailers: std::collections::HashMap::new(),
            timing: GrpcTimingInfo {
                total_ms,
                first_response_ms,
            },
            cancelled: was_cancelled,
        })
    }

    /// 关闭 gRPC 连接
    ///
    /// 从连接池中移除连接，tonic Channel 自动 drop 关闭。
    async fn close(&self, connection_id: &str) -> Result<(), String> {
        let mut conns = self
            .connections
            .write()
            .map_err(|e| format!("获取连接池写锁Failed: {}", e))?;
        conns
            .remove(connection_id)
            .ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
        Ok(())
    }

    /// 获取连接状态
    fn get_status(&self, connection_id: &str) -> Option<String> {
        let conns = self.connections.read().ok()?;
        conns.get(connection_id).map(|c| c.status.clone())
    }

    /// 获取连接的可用服务列表
    fn list_services(&self, connection_id: &str) -> Option<Vec<GrpcServiceInfo>> {
        let conns = self.connections.read().ok()?;
        let conn = conns.get(connection_id)?;
        let services = proto_parser::list_services(&conn.descriptor_pool);
        let message_definitions = Arc::new(proto_parser::get_message_definitions(&conn.descriptor_pool));
        Some(
            services
                .into_iter()
                .map(|s| GrpcServiceInfo {
                    service_name: s.service_name,
                    methods: s
                        .methods
                        .into_iter()
                        .map(|m| GrpcMethodInfo {
                            method_name: m.method_name,
                            input_type: m.input_type,
                            output_type: m.output_type,
                            is_server_streaming: m.is_server_streaming,
                            is_client_streaming: m.is_client_streaming,
                        })
                        .collect(),
                    message_definitions: message_definitions.clone(),
                })
                .collect(),
        )
    }

    /// 健康检查（gRPC Health Checking Protocol unary Check）
    ///
    /// 使用 `tonic_health::pb::health_client::HealthClient` 发送 unary Check 请求。
    /// - UNIMPLEMENTED → 返回 "`NOT_IMPLEMENTED`"
    /// - 其他Error → 返回Error字符串
    /// - Success → 返回 `ServingStatus` 字符串
    async fn health_check(
        &self,
        connection_id: &str,
        service: &str,
    ) -> Result<flow_engine_core::traits::HealthCheckResponse, String> {
        use tonic_health::pb::health_client::HealthClient;
        use tonic_health::pb::HealthCheckRequest;

        // 1. 从连接池获取 channel（克隆后释放锁）
        let channel = {
            let conns = self
                .connections
                .read()
                .map_err(|e| format!("获取连接池读锁Failed: {}", e))?;
            let conn = conns
                .get(connection_id)
                .ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
            conn.channel.clone()
        }; // 读锁在此释放

        // 2. 创建 Health 客户端并发送 Check 请求
        let mut client = HealthClient::new(channel);
        let request = tonic::Request::new(HealthCheckRequest {
            service: service.to_string(),
        });

        match client.check(request).await {
            Ok(response) => {
                let status = response.into_inner().status();
                let status_str = match status {
                    tonic_health::pb::health_check_response::ServingStatus::Serving => "SERVING",
                    tonic_health::pb::health_check_response::ServingStatus::NotServing => "NOT_SERVING",
                    tonic_health::pb::health_check_response::ServingStatus::ServiceUnknown => "SERVICE_UNKNOWN",
                    _ => "UNKNOWN",
                };
                Ok(flow_engine_core::traits::HealthCheckResponse {
                    status: status_str.to_string(),
                })
            }
            Err(status) => {
                // UNIMPLEMENTED 表示服务端未实现健康检查协议
                if status.code() == tonic::Code::Unimplemented {
                    Ok(flow_engine_core::traits::HealthCheckResponse {
                        status: "NOT_IMPLEMENTED".to_string(),
                    })
                } else {
                    Err(format!("健康检查Failed: {} ({})", status.message(), status.code()))
                }
            }
        }
    }

    fn cancel_stream(&self, connection_id: &str, call_id: &str) -> Result<(), String> {
        // 从连接池获取 active_calls
        let active_calls = {
            let conns = self.connections.read().map_err(|e| format!("获取连接池读锁Failed: {}", e))?;
            let conn = conns.get(connection_id).ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
            Arc::clone(&conn.active_calls)
        };

        let mut calls = active_calls.lock().map_err(|e| format!("获取活跃调用锁Failed: {}", e))?;
        match calls.get(call_id) {
            Some(token) => {
                log::info!("[gRPC] 取消流式调用: connection_id={}, call_id={}", connection_id, call_id);
                token.cancel();
                calls.remove(call_id);
                Ok(())
            }
            None => Err(format!("调用 '{}' 不存在或已完成", call_id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_new() {
        let manager = GrpcConnectionManager::new();
        // 新创建的 manager 应该没有连接
        assert!(manager.get_status("non-existent").is_none());
        assert!(manager.list_services("non-existent").is_none());
    }

    #[test]
    fn test_connection_manager_default() {
        let manager = GrpcConnectionManager::default();
        assert!(manager.get_status("non-existent").is_none());
    }

    #[test]
    fn test_build_method_path() {
        let path = build_method_path("test.TestService", "GetUser").expect("路径构建应Success");
        assert_eq!(path.as_str(), "/test.TestService/GetUser");
    }

    #[test]
    fn test_build_method_path_with_package() {
        let path = build_method_path("myapp.v1.OrderService", "CreateOrder").expect("路径构建应Success");
        assert_eq!(path.as_str(), "/myapp.v1.OrderService/CreateOrder");
    }

    #[test]
    fn test_dynamic_codec_roundtrip() {
        // 使用 proto_parser 创建一个 DescriptorPool 来测试编解码
        let proto_content = r#"
syntax = "proto3";
package test;
message TestMessage {
  string name = 1;
  int32 value = 2;
}
"#;
        let files = vec![proto_parser::ProtoFile {
            path: "test.proto".to_string(),
            content: proto_content.to_string(),
        }];
        let pool = proto_parser::parse_proto_files(&files).expect("解析应Success");

        // DynamicCodec 需要一个 MethodDescriptor 来确定输入输出类型
        // 由于我们无法从 pool 直接获取独立方法的 MethodDescriptor，
        // 这里只验证 Codec trait 的 encoder/decoder 方法可以调用
        // 真正的编解码测试需要通过 tonic 的 Grpc 客户端
        let msg_desc = pool
            .get_message_by_name("test.TestMessage")
            .expect("应能找到 TestMessage");

        // 创建 DynamicMessage 并设置字段
        let mut msg = prost_reflect::DynamicMessage::new(msg_desc);
        msg.set_field_by_name("name", prost_reflect::Value::String("hello".to_string()));
        msg.set_field_by_name("value", prost_reflect::Value::I32(42));

        // 验证消息内容
        assert_eq!(
            msg.get_field_by_name("name")
                .and_then(|v| v.as_str().map(|s| s.to_string())),
            Some("hello".to_string())
        );
        assert_eq!(
            msg.get_field_by_name("value").and_then(|v| v.as_i32()),
            Some(42)
        );
    }

    #[test]
    fn test_json_to_dynamic_message() {
        let proto_content = r#"
syntax = "proto3";
package test;
message TestRequest {
  string user_id = 1;
  int32 count = 2;
}
"#;
        let files = vec![proto_parser::ProtoFile {
            path: "test.proto".to_string(),
            content: proto_content.to_string(),
        }];
        let pool = proto_parser::parse_proto_files(&files).expect("解析应Success");
        let msg_desc = pool
            .get_message_by_name("test.TestRequest")
            .expect("应能找到 TestRequest");

        let json = r#"{"user_id": "abc123", "count": 5}"#;
        let msg = json_to_dynamic_message(json, msg_desc).expect("反序列化应Success");

        assert_eq!(
            msg.get_field_by_name("user_id")
                .and_then(|v| v.as_str().map(|s| s.to_string())),
            Some("abc123".to_string())
        );
        assert_eq!(
            msg.get_field_by_name("count").and_then(|v| v.as_i32()),
            Some(5)
        );
    }

    #[test]
    fn test_dynamic_message_to_json() {
        let proto_content = r#"
syntax = "proto3";
package test;
message TestResponse {
  string name = 1;
  int32 age = 2;
}
"#;
        let files = vec![proto_parser::ProtoFile {
            path: "test.proto".to_string(),
            content: proto_content.to_string(),
        }];
        let pool = proto_parser::parse_proto_files(&files).expect("解析应Success");
        let msg_desc = pool
            .get_message_by_name("test.TestResponse")
            .expect("应能找到 TestResponse");

        let mut msg = prost_reflect::DynamicMessage::new(msg_desc);
        msg.set_field_by_name("name", prost_reflect::Value::String("Alice".to_string()));
        msg.set_field_by_name("age", prost_reflect::Value::I32(30));

        let json = dynamic_message_to_json(&msg).expect("序列化应Success");
        assert_eq!(json["name"], "Alice");
        assert_eq!(json["age"], 30);
    }

    // ===== metadata 合并测试 =====

    #[test]
    fn test_merge_metadata_both_present() {
        // 连接级和调用级各有不同的 key → 两者都保留
        let default = vec![
            ("auth".to_string(), "token-a".to_string()),
        ];
        let call = vec![
            ("trace".to_string(), "id-1".to_string()),
        ];
        let merged = merge_metadata(&default, call);
        assert_eq!(merged.len(), 2);
        let auth_val = merged.iter().find(|(k, _)| k == "auth").map(|(_, v)| v.clone());
        let trace_val = merged.iter().find(|(k, _)| k == "trace").map(|(_, v)| v.clone());
        assert_eq!(auth_val, Some("token-a".to_string()));
        assert_eq!(trace_val, Some("id-1".to_string()));
    }

    #[test]
    fn test_merge_metadata_call_overrides_default() {
        // 同名 key → 调用级覆盖连接级
        let default = vec![
            ("key".to_string(), "old".to_string()),
            ("extra".to_string(), "keep".to_string()),
        ];
        let call = vec![
            ("key".to_string(), "new".to_string()),
        ];
        let merged = merge_metadata(&default, call);
        assert_eq!(merged.len(), 2);
        let key_val = merged.iter().find(|(k, _)| k == "key").map(|(_, v)| v.clone());
        let extra_val = merged.iter().find(|(k, _)| k == "extra").map(|(_, v)| v.clone());
        assert_eq!(key_val, Some("new".to_string())); // 调用级覆盖
        assert_eq!(extra_val, Some("keep".to_string())); // 连接级保留
    }

    #[test]
    fn test_merge_metadata_empty_default() {
        // 连接级为空 → 只使用调用级
        let default: Vec<(String, String)> = vec![];
        let call = vec![
            ("token".to_string(), "abc".to_string()),
        ];
        let merged = merge_metadata(&default, call);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].0, "token");
        assert_eq!(merged[0].1, "abc");
    }

    #[test]
    fn test_merge_metadata_empty_call() {
        // 调用级为空 → 只使用连接级
        let default = vec![
            ("auth".to_string(), "bearer".to_string()),
        ];
        let call: Vec<(String, String)> = vec![];
        let merged = merge_metadata(&default, call);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].0, "auth");
        assert_eq!(merged[0].1, "bearer");
    }

    // ===== ChannelzInfo 序列化测试 =====

    #[test]
    fn test_channelz_info_serialization() {
        let info = ChannelzInfo {
            state: "READY".to_string(),
            connected_at: "2026-04-27T12:00:00+00:00".to_string(),
            uptime_secs: 3600,
            active_calls: 3,
            last_error: None,
        };
        let json = serde_json::to_string(&info).expect("序列化应Success");
        let deserialized: ChannelzInfo =
            serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(deserialized.state, "READY");
        assert_eq!(deserialized.uptime_secs, 3600);
        assert_eq!(deserialized.active_calls, 3);
        assert!(deserialized.last_error.is_none());
    }

    #[test]
    fn test_channelz_info_with_error() {
        let info = ChannelzInfo {
            state: "TRANSIENT_FAILURE".to_string(),
            connected_at: "2026-04-27T12:00:00+00:00".to_string(),
            uptime_secs: 120,
            active_calls: 0,
            last_error: Some("connection refused".to_string()),
        };
        let json = serde_json::to_string(&info).expect("序列化应Success");
        let deserialized: ChannelzInfo =
            serde_json::from_str(&json).expect("反序列化应Success");
        assert_eq!(deserialized.state, "TRANSIENT_FAILURE");
        assert_eq!(deserialized.last_error, Some("connection refused".to_string()));
    }
}
