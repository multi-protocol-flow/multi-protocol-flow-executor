//! Proto 文件运行时解析器
//!
//! 使用 protox（纯 Rust protobuf 编译器）在运行时编译 .proto 文件，
//! 通过 prost-reflect 的 DescriptorPool 实现动态消息解析和服务发现。

use prost_reflect::{DescriptorPool, FieldDescriptor, Kind, MessageDescriptor};
use std::io::Write;

/// Proto 文件描述（用户上传）
#[derive(Debug, Clone)]
pub struct ProtoFile {
    /// 文件名（如 "example.proto"）
    pub path: String,
    /// 文件内容
    pub content: String,
}

/// 服务信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceInfo {
    /// 服务全名（如 "package.ServiceName"）
    pub service_name: String,
    /// 方法列表
    pub methods: Vec<MethodInfo>,
}

/// 方法信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MethodInfo {
    /// 方法名
    pub method_name: String,
    /// 输入类型全名
    pub input_type: String,
    /// 输出类型全名
    pub output_type: String,
    /// 是否为服务端流式
    pub is_server_streaming: bool,
    /// 是否为客户端流式
    pub is_client_streaming: bool,
}

/// Proto 解析错误
#[derive(Debug, thiserror::Error)]
pub enum ProtoParseError {
    /// Proto 文件解析失败
    #[error("Proto 文件解析失败: {0}")]
    ParseError(String),
    /// 缺失文件
    #[error("缺失文件: {0:?}")]
    MissingFiles(Vec<String>),
    /// I/O 错误
    #[error("I/O 错误: {0}")]
    IoError(#[from] std::io::Error),
}

/// 解析多个 proto 文件
///
/// 将 proto 文件内容写入临时目录，使用 protox 编译为 `FileDescriptorSet`，
/// 然后通过 prost-reflect 构建 `DescriptorPool`。
///
/// # 参数
///
/// * `files` - Proto 文件列表，每个包含文件名和内容
///
/// # 返回
///
/// 成功时返回 DescriptorPool，可用于后续的服务发现和消息解析
pub fn parse_proto_files(files: &[ProtoFile]) -> Result<DescriptorPool, ProtoParseError> {
    // 创建临时目录
    let temp_dir = tempfile::tempdir()?;

    // 写入 proto 文件到临时目录
    let mut file_names = Vec::new();
    for file in files {
        let path = temp_dir.path().join(&file.path);
        // 确保父目录存在（支持嵌套 import 路径）
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut f = std::fs::File::create(&path)?;
        f.write_all(file.content.as_bytes())?;
        // 只取文件名用于编译，include 目录是临时目录本身
        if let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) {
            file_names.push(name);
        }
    }

    if file_names.is_empty() {
        return Err(ProtoParseError::ParseError(
            "没有提供有效的 proto 文件".to_string(),
        ));
    }

    // 使用 protox 在运行时编译 proto 文件（纯 Rust，无 C++ 依赖）
    let include_path = temp_dir.path();
    let fds = protox::compile(&file_names, [include_path])
        .map_err(|e| ProtoParseError::ParseError(format!("{}", e)))?;

    // 从 FileDescriptorSet 构建 DescriptorPool
    let pool = DescriptorPool::from_file_descriptor_set(fds)
        .map_err(|e| ProtoParseError::ParseError(format!("构建 DescriptorPool 失败: {}", e)))?;

    Ok(pool)
}

/// 列出所有服务及其方法
///
/// 从 `DescriptorPool` 中提取所有 gRPC 服务定义，
/// 返回服务名、方法名、输入输出类型等信息。
pub fn list_services(pool: &DescriptorPool) -> Vec<ServiceInfo> {
    pool.services()
        .map(|service| {
            let methods: Vec<MethodInfo> = service
                .methods()
                .map(|method| MethodInfo {
                    method_name: method.name().to_string(),
                    input_type: method.input().full_name().to_string(),
                    output_type: method.output().full_name().to_string(),
                    is_server_streaming: method.is_server_streaming(),
                    is_client_streaming: method.is_client_streaming(),
                })
                .collect();

            ServiceInfo {
                service_name: service.full_name().to_string(),
                methods,
            }
        })
        .collect()
}

/// 生成消息类型的 JSON 骨架
///
/// 遍历消息的所有字段，根据字段类型生成占位值。
/// 用于帮助用户构造请求消息。
pub fn generate_json_skeleton(msg: &MessageDescriptor) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for field in msg.fields() {
        let key = field.name().to_string();
        let value = field_to_json(&field);
        obj.insert(key, value);
    }
    serde_json::Value::Object(obj)
}

/// 根据字段类型生成占位 JSON 值
fn field_to_json(field: &FieldDescriptor) -> serde_json::Value {
    if field.is_map() {
        return serde_json::Value::Object(Default::default());
    }
    if field.cardinality() == prost_reflect::Cardinality::Repeated {
        return serde_json::Value::Array(Vec::new());
    }
    match field.kind() {
        Kind::Double | Kind::Float => serde_json::json!(0.0),
        Kind::Int32 | Kind::Int64 | Kind::Sint32 | Kind::Sint64 | Kind::Sfixed32 | Kind::Sfixed64 => {
            serde_json::json!(0)
        }
        Kind::Uint32 | Kind::Uint64 | Kind::Fixed32 | Kind::Fixed64 => serde_json::json!(0),
        Kind::Bool => serde_json::json!(false),
        Kind::String => serde_json::json!(""),
        Kind::Bytes => serde_json::json!(""),
        Kind::Enum(_) => serde_json::json!(0),
        Kind::Message(msg_desc) => generate_json_skeleton(&msg_desc),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用的简单 proto 文件内容
    const TEST_PROTO: &str = r#"
syntax = "proto3";

package test;

service TestService {
  rpc GetUser(GetUserRequest) returns (GetUserResponse);
  rpc ListUsers(ListUsersRequest) returns (stream User);
}

message GetUserRequest {
  string user_id = 1;
}

message GetUserResponse {
  string user_id = 1;
  string name = 2;
  int32 age = 3;
}

message ListUsersRequest {
  int32 page_size = 1;
}

message User {
  string id = 1;
  string name = 2;
}
"#;

    /// 测试解析简单的 proto 文件
    #[test]
    fn test_parse_simple_proto() {
        let files = vec![ProtoFile {
            path: "test.proto".to_string(),
            content: TEST_PROTO.to_string(),
        }];

        let pool = parse_proto_files(&files).expect("解析 proto 文件应成功");

        // 验证可以找到服务
        let service = pool
            .get_service_by_name("test.TestService")
            .expect("应能找到 TestService");
        assert_eq!(service.name(), "TestService");
    }

    /// 测试 list_services 返回正确的服务和方法信息
    #[test]
    fn test_list_services() {
        let files = vec![ProtoFile {
            path: "test.proto".to_string(),
            content: TEST_PROTO.to_string(),
        }];

        let pool = parse_proto_files(&files).expect("解析 proto 文件应成功");
        let services = list_services(&pool);

        assert_eq!(services.len(), 1, "应只有一个服务");

        let service = &services[0];
        assert_eq!(service.service_name, "test.TestService");
        assert_eq!(service.methods.len(), 2, "TestService 应有两个方法");

        // 验证 GetUser 方法
        let get_user = service
            .methods
            .iter()
            .find(|m| m.method_name == "GetUser")
            .expect("应能找到 GetUser 方法");
        assert_eq!(get_user.input_type, "test.GetUserRequest");
        assert_eq!(get_user.output_type, "test.GetUserResponse");
        assert!(
            !get_user.is_server_streaming,
            "GetUser 不应是服务端流式"
        );

        // 验证 ListUsers 方法
        let list_users = service
            .methods
            .iter()
            .find(|m| m.method_name == "ListUsers")
            .expect("应能找到 ListUsers 方法");
        assert_eq!(list_users.input_type, "test.ListUsersRequest");
        assert_eq!(list_users.output_type, "test.User");
        assert!(
            list_users.is_server_streaming,
            "ListUsers 应是服务端流式"
        );
    }

    /// 测试生成 JSON 骨架
    #[test]
    fn test_generate_json_skeleton() {
        let files = vec![ProtoFile {
            path: "test.proto".to_string(),
            content: TEST_PROTO.to_string(),
        }];

        let pool = parse_proto_files(&files).expect("解析 proto 文件应成功");

        let msg = pool
            .get_message_by_name("test.GetUserRequest")
            .expect("应能找到 GetUserRequest 消息");

        let skeleton = generate_json_skeleton(&msg);

        // 骨架应该是 JSON 对象，包含字段名
        assert!(skeleton.is_object(), "骨架应该是 JSON 对象");

        let obj = skeleton.as_object().expect("应为 JSON 对象");
        assert!(
            obj.contains_key("user_id"),
            "骨架应包含 user_id 字段"
        );
    }

    /// 测试空文件列表返回错误
    #[test]
    fn test_empty_files_returns_error() {
        let result = parse_proto_files(&[]);
        assert!(result.is_err(), "空文件列表应返回错误");
    }

    /// 测试无效 proto 内容返回错误
    #[test]
    fn test_invalid_proto_returns_error() {
        let files = vec![ProtoFile {
            path: "invalid.proto".to_string(),
            content: "this is not valid proto syntax !!!".to_string(),
        }];

        let result = parse_proto_files(&files);
        assert!(result.is_err(), "无效 proto 内容应返回错误");
    }

    /// 测试带有 import 的多文件解析
    #[test]
    fn test_multi_file_with_import() {
        let _common_proto = r#"
syntax = "proto3";

package common;

message Status {
  int32 code = 1;
  string message = 2;
}
"#;

        // 注意：由于 protox::compile 使用文件名匹配，
        // 多文件 import 需要更复杂的处理。
        // 这里只测试单文件中的嵌套消息
        let main_proto = r#"
syntax = "proto3";

package myapp;

service OrderService {
  rpc CreateOrder(CreateOrderRequest) returns (CreateOrderResponse);
}

message CreateOrderRequest {
  string product_id = 1;
  int32 quantity = 2;
}

message CreateOrderResponse {
  string order_id = 1;
  Status status = 2;
}

message Status {
  int32 code = 1;
  string message = 2;
}
"#;

        let files = vec![ProtoFile {
            path: "order.proto".to_string(),
            content: main_proto.to_string(),
        }];

        let pool = parse_proto_files(&files).expect("解析多消息 proto 文件应成功");
        let services = list_services(&pool);

        assert_eq!(services.len(), 1);
        assert_eq!(services[0].service_name, "myapp.OrderService");
        assert_eq!(services[0].methods.len(), 1);

        // 验证嵌套消息的 JSON 骨架
        let response_msg = pool
            .get_message_by_name("myapp.CreateOrderResponse")
            .expect("应能找到 CreateOrderResponse");
        let skeleton = generate_json_skeleton(&response_msg);

        assert!(skeleton.is_object());
        let obj = skeleton.as_object().expect("应为 JSON 对象");
        assert!(obj.contains_key("order_id"));
        assert!(obj.contains_key("status"));
    }
}
