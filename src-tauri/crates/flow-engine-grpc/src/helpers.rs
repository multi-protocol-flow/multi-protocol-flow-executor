//! 辅助函数
//!
//! gRPC 路径构建、JSON ↔ DynamicMessage 转换、metadata 合并等工具函数。

use http::uri::PathAndQuery;
use prost_reflect::{DynamicMessage, SerializeOptions};
use serde_json::de::Deserializer;

/// 构建 gRPC 方法路径
///
/// gRPC 路径格式为 `/{package.Service}/{Method}`
pub(crate) fn build_method_path(service_name: &str, method_name: &str) -> Result<PathAndQuery, String> {
    let path = format!("/{}/{}", service_name, method_name);
    path.parse()
        .map_err(|e| format!("无效的 gRPC 路径 '{}': {}", path, e))
}

/// 将 JSON 字符串反序列化为 `DynamicMessage`
pub(crate) fn json_to_dynamic_message(
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
pub(crate) fn dynamic_message_to_json(msg: &DynamicMessage) -> Result<serde_json::Value, String> {
    let options = SerializeOptions::new().skip_default_fields(false);
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::new(&mut buf);
    msg.serialize_with_options(&mut ser, &options)
        .map_err(|e| format!("序列化响应消息失败: {}", e))?;
    serde_json::from_slice(&buf).map_err(|e| format!("解析响应 JSON 失败: {}", e))
}

/// 合并连接级默认 metadata 与调用级 metadata
///
/// 调用级 metadata 同名 key 覆盖连接级（后者优先）。
pub(crate) fn merge_metadata(
    default_metadata: &[(String, String)],
    call_metadata: Vec<(String, String)>,
) -> Vec<(String, String)> {
    default_metadata
        .iter()
        .filter(|(k, _)| !call_metadata.iter().any(|(mk, _)| mk == k))
        .chain(call_metadata.iter())
        .cloned()
        .collect()
}
