//! Flow Engine gRPC 子 crate
//!
//! gRPC 连接管理器和执行器模块。
//! 支持运行时 .proto 文件解析（无编译时代码生成），
//! 通过 prost-reflect 实现动态消息编解码，
//! 通过 tonic 实现动态 gRPC 调用。

/// Proto 文件解析器
pub mod proto_parser;
/// TLS 辅助函数
pub mod tls;
/// Server Reflection 服务发现
pub mod reflection;
/// 动态 gRPC 编解码器
pub mod codec;
/// 辅助函数（路径构建、JSON 转换、metadata 合并）
pub mod helpers;
/// gRPC 错误详情解码（google.rpc.Status + google.protobuf.Any）
pub mod error_detail;
/// gRPC 连接池管理器
pub mod grpc_connection_manager;
/// gRPC 连接节点执行器
pub mod grpc_connect_executor;
/// gRPC 调用节点执行器
pub mod grpc_call_executor;
/// gRPC 关闭节点执行器
pub mod grpc_close_executor;
/// gRPC 节点插件（NodePlugin 实现）
pub mod plugin;
