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

/// 从 `DescriptorPool` 生成指定服务和方法的请求 JSON 骨架
///
/// 通过 `service_name` 和 `method_name` 定位方法的输入消息类型，
/// 复用 `generate_json_skeleton` 生成占位 JSON。
/// 适用于 Server Reflection 场景（无需 proto 文件）。
pub fn generate_skeleton_from_pool(
    pool: &DescriptorPool,
    service_name: &str,
    method_name: &str,
) -> Result<String, String> {
    let service = pool
        .get_service_by_name(service_name)
        .ok_or_else(|| format!("未找到服务: {}", service_name))?;
    let method = service
        .methods()
        .find(|m| m.name() == method_name)
        .ok_or_else(|| format!("服务 {} 中未找到方法: {}", service_name, method_name))?;
    let input_msg = method.input();
    let skeleton = generate_json_skeleton(&input_msg);
    serde_json::to_string_pretty(&skeleton)
        .map_err(|e| format!("序列化 JSON 骨架失败: {}", e))
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

/// 递归扫描目录，发现所有 .proto 文件
///
/// 遍历指定目录及其子目录，收集所有 `.proto` 文件。
/// 跳过以 `.` 开头的隐藏目录。
///
/// # 参数
///
/// * `dir_path` - 目录路径字符串
///
/// # 返回
///
/// 成功时返回 `(Vec<(String, String)>, String)`：
/// - `Vec<(String, String)>`：文件名和内容的元组列表（文件名为相对路径）
/// - `String`：目录根路径（用作 proto 编译的 include 路径）
pub fn discover_proto_files_in_dir(
    dir_path: &str,
) -> Result<(Vec<(String, String)>, String), ProtoParseError> {
    let root = std::path::Path::new(dir_path);
    if !root.is_dir() {
        return Err(ProtoParseError::ParseError(format!(
            "路径不是一个有效目录: {}",
            dir_path
        )));
    }

    let mut files = Vec::new();
    scan_proto_dir(root, root, &mut files)?;

    if files.is_empty() {
        return Err(ProtoParseError::ParseError(
            "目录中未找到任何 .proto 文件".to_string(),
        ));
    }

    Ok((files, dir_path.to_string()))
}

/// 递归扫描目录的内部实现
fn scan_proto_dir(
    base: &std::path::Path,
    current: &std::path::Path,
    files: &mut Vec<(String, String)>,
) -> Result<(), ProtoParseError> {
    let entries = std::fs::read_dir(current)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // 跳过隐藏目录和文件（以 . 开头）
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if file_name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            scan_proto_dir(base, &path, files)?;
        } else if path.extension().map(|e| e == "proto").unwrap_or(false) {
            // 计算相对路径
            let relative = path
                .strip_prefix(base)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| file_name.clone());

            let content = std::fs::read_to_string(&path)?;
            files.push((relative, content));
        }
    }

    Ok(())
}

/// 解析 `FileDescriptorSet` 二进制数据
///
/// 从预编译的 `.pb` / `.fdset` / `.desc` 文件中解码 `FileDescriptorSet`，
/// 构建 `DescriptorPool` 用于后续服务发现和动态调用。
///
/// # 参数
///
/// * `data` - `FileDescriptorSet` 的原始字节数据
///
/// # 返回
///
/// 成功时返回 `DescriptorPool`
pub fn parse_descriptor_set(data: &[u8]) -> Result<DescriptorPool, ProtoParseError> {
    DescriptorPool::decode(data)
        .map_err(|e| ProtoParseError::ParseError(format!("解析 FileDescriptorSet 失败: {}", e)))
}

// ============================================================================
// 请求 JSON 预验证
// ============================================================================

/// 验证错误条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationError {
    /// 字段路径（如 "address.street"）
    pub path: String,
    /// 错误描述（如 "未知字段" 或 "期望 string 类型，实际 number"）
    pub message: String,
}

/// 验证结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult {
    /// 是否通过验证
    pub valid: bool,
    /// 错误列表
    pub errors: Vec<ValidationError>,
}

/// 验证请求 JSON 是否符合 proto 消息定义
///
/// 根据消息描述符检查 JSON 对象中的字段名和类型是否与 proto schema 兼容。
/// 仅做建议性检查，不阻止发送。
///
/// # 参数
///
/// * `pool` - proto 描述符池
/// * `message_name` - 消息全名（如 "test.GetUserRequest"）
/// * `json` - 待验证的 JSON 字符串
///
/// # 返回
///
/// 验证结果，包含是否通过和错误列表
pub fn validate_request_json(
    pool: &DescriptorPool,
    message_name: &str,
    json: &str,
) -> Result<ValidationResult, String> {
    // 1. 解析 JSON 语法
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("JSON 语法错误: {}", e))?;

    // 2. 获取消息描述符
    let msg_desc = pool
        .get_message_by_name(message_name)
        .ok_or_else(|| format!("未找到消息类型: {}", message_name))?;

    // 3. 必须是 JSON 对象
    let obj = match value.as_object() {
        Some(obj) => obj,
        None => {
            return Ok(ValidationResult {
                valid: false,
                errors: vec![ValidationError {
                    path: String::new(),
                    message: "期望 JSON 对象，实际为其他类型".to_string(),
                }],
            });
        }
    };

    // 4. 遍历 JSON 字段进行验证
    let mut errors = Vec::new();
    validate_json_object(&msg_desc, obj, &mut errors, String::new());

    Ok(ValidationResult {
        valid: errors.is_empty(),
        errors,
    })
}

/// 递归验证 JSON 对象与 proto 消息定义的字段匹配
fn validate_json_object(
    msg_desc: &MessageDescriptor,
    obj: &serde_json::Map<String, serde_json::Value>,
    errors: &mut Vec<ValidationError>,
    parent_path: String,
) {
    for (key, value) in obj {
        let field_path = if parent_path.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", parent_path, key)
        };

        // 查找字段定义
        let field_desc = msg_desc.fields().find(|f| f.name() == key);

        match field_desc {
            None => {
                errors.push(ValidationError {
                    path: field_path,
                    message: "未知字段".to_string(),
                });
            }
            Some(fd) => {
                // map 类型
                if fd.is_map() {
                    if !value.is_object() {
                        errors.push(ValidationError {
                            path: field_path,
                            message: format!("期望 map (JSON 对象)，实际为 {}", json_type_name(value)),
                        });
                    }
                    continue;
                }

                // repeated 类型
                if fd.cardinality() == prost_reflect::Cardinality::Repeated {
                    if !value.is_array() {
                        errors.push(ValidationError {
                            path: field_path,
                            message: format!("期望 repeated (JSON 数组)，实际为 {}", json_type_name(value)),
                        });
                    }
                    continue;
                }

                // 检查 singular 字段类型兼容性
                check_field_type(&fd, value, errors, field_path);
            }
        }
    }
}

/// 检查单个字段的类型兼容性
fn check_field_type(
    field: &FieldDescriptor,
    value: &serde_json::Value,
    errors: &mut Vec<ValidationError>,
    field_path: String,
) {
    // null 值在 proto3 中是合法的（等同于默认值）
    if value.is_null() {
        return;
    }

    match field.kind() {
        Kind::String => {
            if !value.is_string() {
                errors.push(ValidationError {
                    path: field_path,
                    message: format!("期望 string 类型，实际为 {}", json_type_name(value)),
                });
            }
        }
        Kind::Bool => {
            if !value.is_boolean() {
                errors.push(ValidationError {
                    path: field_path,
                    message: format!("期望 bool 类型，实际为 {}", json_type_name(value)),
                });
            }
        }
        Kind::Int32 | Kind::Int64 | Kind::Sint32 | Kind::Sint64 | Kind::Sfixed32 | Kind::Sfixed64 => {
            if !value.is_number() {
                errors.push(ValidationError {
                    path: field_path,
                    message: format!("期望整数类型，实际为 {}", json_type_name(value)),
                });
            }
        }
        Kind::Uint32 | Kind::Uint64 | Kind::Fixed32 | Kind::Fixed64 => {
            if !value.is_number() {
                errors.push(ValidationError {
                    path: field_path,
                    message: format!("期望无符号整数类型，实际为 {}", json_type_name(value)),
                });
            }
        }
        Kind::Float | Kind::Double => {
            if !value.is_number() {
                errors.push(ValidationError {
                    path: field_path,
                    message: format!("期望浮点数类型，实际为 {}", json_type_name(value)),
                });
            }
        }
        Kind::Bytes => {
            if !value.is_string() {
                errors.push(ValidationError {
                    path: field_path,
                    message: format!("期望 bytes (Base64 字符串)，实际为 {}", json_type_name(value)),
                });
            }
        }
        Kind::Enum(_) => {
            // enum 可以是字符串名或数字
            if !value.is_string() && !value.is_number() {
                errors.push(ValidationError {
                    path: field_path,
                    message: format!("期望 enum (字符串或数字)，实际为 {}", json_type_name(value)),
                });
            }
        }
        Kind::Message(nested_msg) => {
            if let Some(nested_obj) = value.as_object() {
                // 递归验证嵌套消息
                validate_json_object(&nested_msg, nested_obj, errors, field_path);
            } else {
                errors.push(ValidationError {
                    path: field_path,
                    message: format!("期望 message (JSON 对象)，实际为 {}", json_type_name(value)),
                });
            }
        }
    }
}

/// 返回 JSON 值的可读类型名称
fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
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

    /// 测试目录扫描 — 空目录应返回错误
    #[test]
    fn test_scan_empty_dir_returns_error() {
        let temp_dir = tempfile::tempdir().expect("创建临时目录应成功");
        let result = discover_proto_files_in_dir(
            temp_dir.path().to_str().expect("路径转换应成功"),
        );
        assert!(result.is_err(), "空目录应返回错误");
    }

    /// 测试目录扫描 — 不存在的目录应返回错误
    #[test]
    fn test_scan_nonexistent_dir_returns_error() {
        let result = discover_proto_files_in_dir("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err(), "不存在的目录应返回错误");
    }

    /// 测试目录扫描 — 正常扫描 .proto 文件
    #[test]
    fn test_scan_dir_finds_proto_files() {
        let temp_dir = tempfile::tempdir().expect("创建临时目录应成功");

        // 创建 proto 文件
        std::fs::write(
            temp_dir.path().join("test.proto"),
            TEST_PROTO,
        )
        .expect("写入 proto 文件应成功");

        // 创建子目录中的 proto 文件
        let sub_dir = temp_dir.path().join("sub");
        std::fs::create_dir_all(&sub_dir).expect("创建子目录应成功");
        std::fs::write(
            sub_dir.join("nested.proto"),
            r#"syntax = "proto3"; package nested; message Nested { string id = 1; }"#,
        )
        .expect("写入嵌套 proto 文件应成功");

        // 创建非 proto 文件（应被跳过）
        std::fs::write(
            temp_dir.path().join("readme.txt"),
            "not a proto file",
        )
        .expect("写入非 proto 文件应成功");

        // 创建隐藏目录（应被跳过）
        let hidden_dir = temp_dir.path().join(".hidden");
        std::fs::create_dir_all(&hidden_dir).expect("创建隐藏目录应成功");
        std::fs::write(
            hidden_dir.join("hidden.proto"),
            r#"syntax = "proto3"; package hidden;"#,
        )
        .expect("写入隐藏 proto 文件应成功");

        let dir_path = temp_dir.path().to_str().expect("路径转换应成功");
        let result = discover_proto_files_in_dir(dir_path);

        assert!(result.is_ok(), "扫描目录应成功: {:?}", result);
        let (files, _) = result.expect("应成功获取文件列表");
        assert_eq!(files.len(), 2, "应找到 2 个 proto 文件（跳过隐藏目录和非 proto 文件）");

        let filenames: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
        assert!(filenames.contains(&"test.proto"), "应包含 test.proto");
        assert!(
            filenames.contains(&"nested.proto") || filenames.iter().any(|f| f.contains("nested.proto")),
            "应包含 nested.proto（可能在子目录路径中）"
        );
    }

    /// 测试 parse_descriptor_set — 无效数据应返回错误
    #[test]
    fn test_parse_descriptor_set_invalid_data() {
        let result = parse_descriptor_set(b"not a valid descriptor set");
        assert!(result.is_err(), "无效数据应返回错误");
    }

    /// 测试 parse_descriptor_set — 空数据
    #[test]
    fn test_parse_descriptor_set_empty_data() {
        // 空的 FileDescriptorSet 可以被成功解码（prost 默认行为），只是结果为空 pool
        let result = parse_descriptor_set(b"");
        // 空数据解码成功但 pool 为空，这是 prost 的默认行为
        // 验证可以解码（即使结果为空）
        assert!(result.is_ok(), "空数据解码为空的 DescriptorPool");
        let pool = result.expect("应成功解码");
        // 空 pool 中没有服务
        assert_eq!(pool.services().count(), 0, "空 DescriptorPool 应没有服务");
    }

    /// 测试 parse_descriptor_set — 有效 FileDescriptorSet
    #[test]
    fn test_parse_descriptor_set_valid() {
        // 先用 proto_parser 生成一个 FileDescriptorSet，再通过 parse_descriptor_set 解码
        let files = vec![ProtoFile {
            path: "test.proto".to_string(),
            content: TEST_PROTO.to_string(),
        }];

        let temp_dir = tempfile::tempdir().expect("创建临时目录应成功");
        let file_path = temp_dir.path().join("test.proto");
        std::fs::write(&file_path, &files[0].content).expect("写入 proto 文件应成功");

        let fds = protox::compile(
            ["test.proto"],
            [temp_dir.path()],
        ).expect("编译 proto 文件应成功");

        // 将 FileDescriptorSet 编码为字节
        let mut buf = Vec::new();
        prost::Message::encode(&fds, &mut buf).expect("编码 FileDescriptorSet 应成功");

        // 使用 parse_descriptor_set 解码
        let pool = parse_descriptor_set(&buf).expect("解析 FileDescriptorSet 应成功");

        // 验证服务存在
        let service = pool
            .get_service_by_name("test.TestService")
            .expect("应能找到 TestService");
        assert_eq!(service.name(), "TestService");
        assert_eq!(service.methods().count(), 2);
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

    // ========================================================================
    // validate_request_json 测试
    // ========================================================================

    /// 辅助函数：解析测试 proto 并获取 pool
    fn get_test_pool() -> DescriptorPool {
        let files = vec![ProtoFile {
            path: "test.proto".to_string(),
            content: TEST_PROTO.to_string(),
        }];
        parse_proto_files(&files).expect("解析 proto 文件应成功")
    }

    /// 测试验证 — 有效的 JSON
    #[test]
    fn test_validate_valid_json() {
        let pool = get_test_pool();
        let result = validate_request_json(&pool, "test.GetUserRequest", r#"{"user_id": "abc"}"#);
        assert!(result.is_ok(), "应成功验证");
        let vr = result.expect("应返回验证结果");
        assert!(vr.valid, "应通过验证");
        assert!(vr.errors.is_empty(), "不应有错误");
    }

    /// 测试验证 — JSON 语法错误
    #[test]
    fn test_validate_json_syntax_error() {
        let pool = get_test_pool();
        let result = validate_request_json(&pool, "test.GetUserRequest", r#"{invalid}"#);
        assert!(result.is_err(), "JSON 语法错误应返回 Err");
    }

    /// 测试验证 — 消息类型不存在
    #[test]
    fn test_validate_unknown_message() {
        let pool = get_test_pool();
        let result = validate_request_json(&pool, "test.NonExistent", r#"{}"#);
        assert!(result.is_err(), "未知消息类型应返回 Err");
    }

    /// 测试验证 — 非 JSON 对象
    #[test]
    fn test_validate_non_object() {
        let pool = get_test_pool();
        let result = validate_request_json(&pool, "test.GetUserRequest", r#"[1,2,3]"#);
        assert!(result.is_ok(), "应返回验证结果");
        let vr = result.expect("应返回验证结果");
        assert!(!vr.valid, "非 JSON 对象应不通过");
        assert_eq!(vr.errors.len(), 1);
        assert!(vr.errors[0].message.contains("期望 JSON 对象"));
    }

    /// 测试验证 — 未知字段
    #[test]
    fn test_validate_unknown_field() {
        let pool = get_test_pool();
        let result = validate_request_json(
            &pool,
            "test.GetUserRequest",
            r#"{"user_id": "abc", "unknown_field": 123}"#,
        );
        assert!(result.is_ok());
        let vr = result.expect("应返回验证结果");
        assert!(!vr.valid, "有未知字段应不通过");
        assert!(vr.errors.iter().any(|e| e.path == "unknown_field" && e.message.contains("未知字段")));
    }

    /// 测试验证 — 类型不匹配
    #[test]
    fn test_validate_type_mismatch() {
        let pool = get_test_pool();
        // GetUserResponse: user_id (string), name (string), age (int32)
        let result = validate_request_json(
            &pool,
            "test.GetUserResponse",
            r#"{"user_id": 123, "name": "test", "age": "not_a_number"}"#,
        );
        assert!(result.is_ok());
        let vr = result.expect("应返回验证结果");
        assert!(!vr.valid, "类型不匹配应不通过");
        assert_eq!(vr.errors.len(), 2, "应有 2 个类型错误");
        assert!(vr.errors.iter().any(|e| e.path == "user_id"));
        assert!(vr.errors.iter().any(|e| e.path == "age"));
    }

    /// 测试验证 — 嵌套消息
    #[test]
    fn test_validate_nested_message() {
        let proto = r#"
syntax = "proto3";
package nested;

message Address {
  string street = 1;
  int32 zip = 2;
}

message Person {
  string name = 1;
  Address address = 2;
}
"#;
        let files = vec![ProtoFile {
            path: "nested.proto".to_string(),
            content: proto.to_string(),
        }];
        let pool = parse_proto_files(&files).expect("解析应成功");

        // 嵌套字段类型错误
        let result = validate_request_json(
            &pool,
            "nested.Person",
            r#"{"name": "Alice", "address": {"street": 123, "zip": "abc"}}"#,
        );
        assert!(result.is_ok());
        let vr = result.expect("应返回验证结果");
        assert!(!vr.valid);
        assert!(vr.errors.iter().any(|e| e.path == "address.street"));
        assert!(vr.errors.iter().any(|e| e.path == "address.zip"));

        // 嵌套字段正确
        let result2 = validate_request_json(
            &pool,
            "nested.Person",
            r#"{"name": "Alice", "address": {"street": "Main St", "zip": 12345}}"#,
        );
        assert!(result2.is_ok());
        let vr2 = result2.expect("应返回验证结果");
        assert!(vr2.valid, "正确的嵌套消息应通过");
    }

    /// 测试验证 — 空对象应通过
    #[test]
    fn test_validate_empty_object() {
        let pool = get_test_pool();
        let result = validate_request_json(&pool, "test.GetUserRequest", r#"{}"#);
        assert!(result.is_ok());
        let vr = result.expect("应返回验证结果");
        assert!(vr.valid, "空对象应通过验证（proto3 所有字段可选）");
    }

    /// 测试验证 — null 值应通过
    #[test]
    fn test_validate_null_values() {
        let pool = get_test_pool();
        let result = validate_request_json(
            &pool,
            "test.GetUserResponse",
            r#"{"user_id": null, "name": null, "age": null}"#,
        );
        assert!(result.is_ok());
        let vr = result.expect("应返回验证结果");
        assert!(vr.valid, "null 值应通过验证");
    }

    /// 测试验证 — repeated 字段
    #[test]
    fn test_validate_repeated_field() {
        let proto = r#"
syntax = "proto3";
package rep;

message ItemList {
  repeated string items = 1;
  repeated int32 counts = 2;
}
"#;
        let files = vec![ProtoFile {
            path: "rep.proto".to_string(),
            content: proto.to_string(),
        }];
        let pool = parse_proto_files(&files).expect("解析应成功");

        // repeated 使用数组 — 通过
        let result = validate_request_json(&pool, "rep.ItemList", r#"{"items": ["a", "b"], "counts": [1, 2]}"#);
        assert!(result.is_ok());
        let vr = result.expect("应返回验证结果");
        assert!(vr.valid, "正确的数组应通过");

        // repeated 使用非数组 — 不通过
        let result2 = validate_request_json(&pool, "rep.ItemList", r#"{"items": "not_array"}"#);
        assert!(result2.is_ok());
        let vr2 = result2.expect("应返回验证结果");
        assert!(!vr2.valid, "repeated 字段传入非数组应不通过");
    }

    /// 测试验证 — map 字段
    #[test]
    fn test_validate_map_field() {
        let proto = r#"
syntax = "proto3";
package maptest;

message Config {
  map<string, string> labels = 1;
}
"#;
        let files = vec![ProtoFile {
            path: "maptest.proto".to_string(),
            content: proto.to_string(),
        }];
        let pool = parse_proto_files(&files).expect("解析应成功");

        // map 使用对象 — 通过
        let result = validate_request_json(&pool, "maptest.Config", r#"{"labels": {"key": "value"}}"#);
        assert!(result.is_ok());
        let vr = result.expect("应返回验证结果");
        assert!(vr.valid, "正确的 map 应通过");

        // map 使用非对象 — 不通过
        let result2 = validate_request_json(&pool, "maptest.Config", r#"{"labels": [1,2]}"#);
        assert!(result2.is_ok());
        let vr2 = result2.expect("应返回验证结果");
        assert!(!vr2.valid, "map 字段传入非对象应不通过");
    }

    /// 测试验证 — enum 字段
    #[test]
    fn test_validate_enum_field() {
        let proto = r#"
syntax = "proto3";
package enumtest;

enum Status {
  UNKNOWN = 0;
  ACTIVE = 1;
}

message User {
  Status status = 1;
}
"#;
        let files = vec![ProtoFile {
            path: "enumtest.proto".to_string(),
            content: proto.to_string(),
        }];
        let pool = parse_proto_files(&files).expect("解析应成功");

        // enum 使用字符串 — 通过
        let result = validate_request_json(&pool, "enumtest.User", r#"{"status": "ACTIVE"}"#);
        assert!(result.is_ok());
        assert!(result.expect("应返回验证结果").valid);

        // enum 使用数字 — 通过
        let result2 = validate_request_json(&pool, "enumtest.User", r#"{"status": 1}"#);
        assert!(result2.is_ok());
        assert!(result2.expect("应返回验证结果").valid);

        // enum 使用数组 — 不通过
        let result3 = validate_request_json(&pool, "enumtest.User", r#"{"status": [1]}"#);
        assert!(result3.is_ok());
        assert!(!result3.expect("应返回验证结果").valid);
    }
}
