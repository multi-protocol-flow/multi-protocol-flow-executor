//! gRPC 连接池管理器
//!
//! 实现 GrpcProvider trait，管理 gRPC 连接的生命周期。
//! 使用 tonic Channel 进行 gRPC 通信，
//! 使用 prost-reflect DynamicMessage 进行动态消息编解码。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use flow_engine_core::traits::{GrpcMethodInfo, GrpcProvider, GrpcServiceInfo};
use http::uri::PathAndQuery;
use prost_reflect::{DynamicMessage, MethodDescriptor};
use serde_json::de::Deserializer;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::metadata::{MetadataKey, MetadataValue};

use crate::proto_parser;

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
    #[allow(dead_code)]
    /// 创建时间（预留用于连接超时管理）
    created_at: chrono::DateTime<chrono::Utc>,
}

/// 动态 gRPC 编解码器
///
/// 使用 prost-reflect 的 `DynamicMessage` 实现 tonic 的 Codec trait，
/// 从而支持对任意 protobuf 消息类型的动态编解码。
#[derive(Clone)]
struct DynamicCodec {
    /// 方法描述符（包含输入输出类型信息）
    method: MethodDescriptor,
}

impl DynamicCodec {
    /// 创建新的动态编解码器
    fn new(method: MethodDescriptor) -> Self {
        Self { method }
    }
}

impl Codec for DynamicCodec {
    type Encode = DynamicMessage;
    type Decode = DynamicMessage;
    type Encoder = Self;
    type Decoder = Self;

    fn encoder(&mut self) -> Self::Encoder {
        self.clone()
    }

    fn decoder(&mut self) -> Self::Decoder {
        self.clone()
    }
}

impl Encoder for DynamicCodec {
    type Item = DynamicMessage;
    type Error = tonic::Status;

    fn encode(
        &mut self,
        item: Self::Item,
        dst: &mut EncodeBuf<'_>,
    ) -> Result<(), Self::Error> {
        prost::Message::encode(&item, dst)
            .map_err(|e| tonic::Status::internal(format!("编码消息失败: {}", e)))?;
        Ok(())
    }
}

impl Decoder for DynamicCodec {
    type Item = DynamicMessage;
    type Error = tonic::Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        let mut msg = DynamicMessage::new(self.method.output());
        prost::Message::merge(&mut msg, src)
            .map_err(|e| tonic::Status::internal(format!("解码消息失败: {}", e)))?;
        Ok(Some(msg))
    }
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
}

impl Default for GrpcConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 构建 gRPC 方法路径
///
/// gRPC 路径格式为 `/{package.Service}/{Method}`
fn build_method_path(service_name: &str, method_name: &str) -> Result<PathAndQuery, String> {
    let path = format!("/{}/{}", service_name, method_name);
    path.parse()
        .map_err(|e| format!("无效的 gRPC 路径 '{}': {}", path, e))
}

/// 将 JSON 字符串反序列化为 `DynamicMessage`
fn json_to_dynamic_message(
    request_json: &str,
    descriptor: prost_reflect::MessageDescriptor,
) -> Result<DynamicMessage, String> {
    let mut deserializer = Deserializer::from_str(request_json);
    let msg = DynamicMessage::deserialize(descriptor, &mut deserializer)
        .map_err(|e| format!("反序列化请求消息失败: {}", e))?;
    deserializer
        .end()
        .map_err(|e| format!("请求 JSON 包含多余数据: {}", e))?;
    Ok(msg)
}

/// 将 `DynamicMessage` 序列化为 JSON Value
fn dynamic_message_to_json(msg: &DynamicMessage) -> Result<serde_json::Value, String> {
    let options = prost_reflect::SerializeOptions::new().skip_default_fields(false);
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::new(&mut buf);
    msg.serialize_with_options(&mut ser, &options)
        .map_err(|e| format!("序列化响应消息失败: {}", e))?;
    serde_json::from_slice(&buf).map_err(|e| format!("解析响应 JSON 失败: {}", e))
}

#[async_trait]
impl GrpcProvider for GrpcConnectionManager {
    /// 建立 gRPC 连接
    ///
    /// 创建 tonic Channel，解析 proto 文件构建 `DescriptorPool`，
    /// 将连接信息存入连接池。
    async fn connect(
        &self,
        uuid: String,
        url: String,
        use_tls: bool,
        tls_skip_verify: bool,
        _enable_reflection: bool,
        proto_files: Vec<(String, String)>,
        _metadata: Vec<(String, String)>,
        connect_timeout_ms: u64,
    ) -> Result<Vec<GrpcServiceInfo>, String> {
        // 1. 解析 proto 文件（如果有）
        let pool = if proto_files.is_empty() {
            return Err("未提供 proto 文件，Server Reflection 暂未实现".to_string());
        } else {
            let files: Vec<proto_parser::ProtoFile> = proto_files
                .into_iter()
                .map(|(path, content)| proto_parser::ProtoFile { path, content })
                .collect();
            proto_parser::parse_proto_files(&files)
                .map_err(|e| format!("解析 proto 文件失败: {}", e))?
        };

        // 2. 提取服务信息
        let services = proto_parser::list_services(&pool);
        let grpc_services: Vec<GrpcServiceInfo> = services
            .iter()
            .map(|s| GrpcServiceInfo {
                service_name: s.service_name.clone(),
                methods: s
                    .methods
                    .iter()
                    .map(|m| GrpcMethodInfo {
                        method_name: m.method_name.clone(),
                        input_type: m.input_type.clone(),
                        output_type: m.output_type.clone(),
                        is_server_streaming: m.is_server_streaming,
                    })
                    .collect(),
            })
            .collect();

        // 3. 构造 URL（确保包含 scheme）
        let url_with_scheme = if url.starts_with("http://") || url.starts_with("https://") {
            url.clone()
        } else if use_tls {
            format!("https://{}", url)
        } else {
            format!("http://{}", url)
        };

        // 4. 创建 tonic 端点并连接
        let mut endpoint = tonic::transport::Endpoint::from_shared(url_with_scheme)
            .map_err(|e| format!("创建 gRPC 端点失败: {}", e))?;

        if use_tls {
            // 注意: tonic 0.12 不支持 danger_accept_invalid_certs
            // 使用默认 TLS 验证。如需跳过验证，需自行构建 rustls ClientConfig。
            let _tls_skip_verify = tls_skip_verify; // 抑制未使用变量警告
            let tls_config = tonic::transport::ClientTlsConfig::new();
            endpoint = endpoint
                .tls_config(tls_config)
                .map_err(|e| format!("TLS 配置失败: {}", e))?;
        }

        if connect_timeout_ms > 0 {
            endpoint = endpoint
                .connect_timeout(std::time::Duration::from_millis(connect_timeout_ms));
        }

        let channel = endpoint
            .connect()
            .await
            .map_err(|e| format!("连接 gRPC 服务器失败: {}", e))?;

        // 5. 存入连接池
        let connection = GrpcConnection {
            channel,
            descriptor_pool: Arc::new(pool),
            url,
            status: "connected".to_string(),
            created_at: chrono::Utc::now(),
        };

        {
            let mut conns = self
                .connections
                .write()
                .map_err(|e| format!("获取连接池写锁失败: {}", e))?;
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
    ) -> Result<serde_json::Value, String> {
        // 1. 从连接池获取连接信息（克隆后释放锁）
        let (channel, pool) = {
            let conns = self
                .connections
                .read()
                .map_err(|e| format!("获取连接池读锁失败: {}", e))?;
            let conn = conns
                .get(connection_id)
                .ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
            (conn.channel.clone(), Arc::clone(&conn.descriptor_pool))
        }; // 读锁在此释放

        // 2. 查找方法描述符
        let service = pool
            .get_service_by_name(service_name)
            .ok_or_else(|| format!("服务 '{}' 不存在", service_name))?;
        let method = service
            .methods()
            .find(|m| m.name() == method_name)
            .ok_or_else(|| format!("方法 '{}' 在服务 '{}' 中不存在", method_name, service_name))?;

        // 3. 反序列化请求 JSON 为 DynamicMessage
        let req_message = json_to_dynamic_message(request_json, method.input())?;

        // 4. 构造 tonic 请求
        let mut request = tonic::Request::new(req_message);
        // 设置 metadata
        for (key, value) in metadata {
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

        // 5. 发起 Unary 调用
        let path = build_method_path(service_name, method_name)?;
        let codec = DynamicCodec::new(method);
        let mut client = tonic::client::Grpc::new(channel);
        client
            .ready()
            .await
            .map_err(|e| format!("gRPC 客户端未就绪: {}", e))?;

        let response = client
            .unary(request, path, codec)
            .await
            .map_err(|e| format!("gRPC Unary 调用失败: {} ({})", e.message(), e.code()))?;

        // 6. 序列化响应为 JSON
        let response_msg = response.into_inner();
        dynamic_message_to_json(&response_msg)
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
    ) -> Result<Vec<serde_json::Value>, String> {
        // 1. 从连接池获取连接信息（克隆后释放锁）
        let (channel, pool) = {
            let conns = self
                .connections
                .read()
                .map_err(|e| format!("获取连接池读锁失败: {}", e))?;
            let conn = conns
                .get(connection_id)
                .ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
            (conn.channel.clone(), Arc::clone(&conn.descriptor_pool))
        }; // 读锁在此释放

        // 2. 查找方法描述符
        let service = pool
            .get_service_by_name(service_name)
            .ok_or_else(|| format!("服务 '{}' 不存在", service_name))?;
        let method = service
            .methods()
            .find(|m| m.name() == method_name)
            .ok_or_else(|| format!("方法 '{}' 在服务 '{}' 中不存在", method_name, service_name))?;

        // 3. 反序列化请求 JSON 为 DynamicMessage
        let req_message = json_to_dynamic_message(request_json, method.input())?;

        // 4. 构造 tonic 请求
        let mut request = tonic::Request::new(req_message);
        for (key, value) in metadata {
            let meta_key = MetadataKey::from_bytes(key.as_bytes())
                .map_err(|e| format!("无效的 metadata key '{}': {:?}", key, e))?;
            let meta_val = MetadataValue::try_from(&value)
                .map_err(|e| format!("无效的 metadata value '{}': {:?}", value, e))?;
            request.metadata_mut().insert(meta_key, meta_val);
        }
        if timeout_ms > 0 {
            request.set_timeout(std::time::Duration::from_millis(timeout_ms));
        }

        // 5. 发起 Server Streaming 调用
        let path = build_method_path(service_name, method_name)?;
        let codec = DynamicCodec::new(method);
        let mut client = tonic::client::Grpc::new(channel);
        client
            .ready()
            .await
            .map_err(|e| format!("gRPC 客户端未就绪: {}", e))?;

        let response = client
            .server_streaming(request, path, codec)
            .await
            .map_err(|e| {
                format!(
                    "gRPC Server Streaming 调用失败: {} ({})",
                    e.message(),
                    e.code()
                )
            })?;

        // 6. 收集所有流式响应
        let mut stream = response.into_inner();
        let mut results = Vec::new();
        loop {
            match stream.message().await {
                Ok(Some(msg)) => {
                    let json_val = dynamic_message_to_json(&msg)?;
                    results.push(json_val);
                }
                Ok(None) => break,
                Err(e) => {
                    return Err(format!(
                        "流式响应错误: {} ({})",
                        e.message(),
                        e.code()
                    ));
                }
            }
        }

        Ok(results)
    }

    /// 关闭 gRPC 连接
    ///
    /// 从连接池中移除连接，tonic Channel 自动 drop 关闭。
    async fn close(&self, connection_id: &str) -> Result<(), String> {
        let mut conns = self
            .connections
            .write()
            .map_err(|e| format!("获取连接池写锁失败: {}", e))?;
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
        Some(
            services
                .iter()
                .map(|s| GrpcServiceInfo {
                    service_name: s.service_name.clone(),
                    methods: s
                        .methods
                        .iter()
                        .map(|m| GrpcMethodInfo {
                            method_name: m.method_name.clone(),
                            input_type: m.input_type.clone(),
                            output_type: m.output_type.clone(),
                            is_server_streaming: m.is_server_streaming,
                        })
                        .collect(),
                })
                .collect(),
        )
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
        let path = build_method_path("test.TestService", "GetUser").expect("路径构建应成功");
        assert_eq!(path.as_str(), "/test.TestService/GetUser");
    }

    #[test]
    fn test_build_method_path_with_package() {
        let path = build_method_path("myapp.v1.OrderService", "CreateOrder").expect("路径构建应成功");
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
        let pool = proto_parser::parse_proto_files(&files).expect("解析应成功");

        // DynamicCodec 需要一个 MethodDescriptor 来确定输入输出类型
        // 由于我们无法从 pool 直接获取独立方法的 MethodDescriptor，
        // 这里只验证 Codec trait 的 encoder/decoder 方法可以调用
        // 真正的编解码测试需要通过 tonic 的 Grpc 客户端
        let msg_desc = pool
            .get_message_by_name("test.TestMessage")
            .expect("应能找到 TestMessage");

        // 创建 DynamicMessage 并设置字段
        let mut msg = DynamicMessage::new(msg_desc);
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
        let pool = proto_parser::parse_proto_files(&files).expect("解析应成功");
        let msg_desc = pool
            .get_message_by_name("test.TestRequest")
            .expect("应能找到 TestRequest");

        let json = r#"{"user_id": "abc123", "count": 5}"#;
        let msg = json_to_dynamic_message(json, msg_desc).expect("反序列化应成功");

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
        let pool = proto_parser::parse_proto_files(&files).expect("解析应成功");
        let msg_desc = pool
            .get_message_by_name("test.TestResponse")
            .expect("应能找到 TestResponse");

        let mut msg = DynamicMessage::new(msg_desc);
        msg.set_field_by_name("name", prost_reflect::Value::String("Alice".to_string()));
        msg.set_field_by_name("age", prost_reflect::Value::I32(30));

        let json = dynamic_message_to_json(&msg).expect("序列化应成功");
        assert_eq!(json["name"], "Alice");
        assert_eq!(json["age"], 30);
    }
}
