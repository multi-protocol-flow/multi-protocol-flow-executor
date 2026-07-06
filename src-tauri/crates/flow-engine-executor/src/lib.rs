//! Flow Engine Executor 子 crate
//!
//! 主执行器、流程编排、调试模式、压力测试、业务命令定义。

/// 命令定义，提供 CLI 和 IPC 层的输入输出类型
pub mod command;
pub mod commands;
/// 条件分支节点执行器，根据表达式求值选择执行路径
pub mod conditional_executor;
/// CSV 解析模块，支持多编码流式解析
pub mod csv_parser;
/// 数据驱动执行模块，逐行读取 CSV 注入 runtime_variables 执行流程
pub mod dataset_executor;
pub mod debug;
pub mod debug_event_adapter;
/// 默认节点注册表，注册全部 14 个内置插件
pub mod default_registry;
/// 入口节点执行器，处理流程起始节点
pub mod entry_executor;
/// 主执行器，驱动流程节点的遍历与执行
pub mod executor;
/// 默认钩子执行器，提供 Webhook/Script/Notify 三种动作的日志执行
pub mod hook_runner;
/// 条件分支节点插件
pub mod plugin_conditional;
/// 结束节点插件
pub mod plugin_end;
/// 入口节点插件
pub mod plugin_entry;
/// 代理设置节点插件
pub mod plugin_proxy;
/// 变量提取节点插件
pub mod plugin_variable_extractor;
/// 协议 provider 构造（DRY: 共享给 executor 和 debug executor）
pub mod provider_builder;
/// 文件存储模块，管理流程文件的读写与持久化
pub mod storage;
/// 压力测试模块，支持并发执行与性能指标采集
pub mod stress_test;
/// 流程验证器，检查流程结构与配置的完整性
pub mod validator;
/// 变量提取节点执行器
pub mod variable_extractor_executor;

// ============================================================================
// 协议 crate 公共 re-export（供 facade 层条件转发）
// ============================================================================
#[cfg(feature = "protocol-amqp")]
pub use flow_engine_amqp;
#[cfg(feature = "protocol-assertion")]
pub use flow_engine_assertion;
#[cfg(feature = "protocol-db")]
pub use flow_engine_db;
#[cfg(feature = "protocol-graphql")]
pub use flow_engine_graphql;
#[cfg(feature = "protocol-grpc")]
pub use flow_engine_grpc;
#[cfg(feature = "protocol-http")]
pub use flow_engine_http;
#[cfg(feature = "protocol-imap")]
pub use flow_engine_imap;
#[cfg(feature = "protocol-kafka")]
pub use flow_engine_kafka;
#[cfg(feature = "protocol-mcp")]
pub use flow_engine_mcp;
#[cfg(feature = "protocol-mqtt")]
pub use flow_engine_mqtt;
#[cfg(feature = "protocol-redis")]
pub use flow_engine_redis;
#[cfg(feature = "protocol-script")]
pub use flow_engine_script;
#[cfg(feature = "protocol-smtp")]
pub use flow_engine_smtp;
#[cfg(feature = "protocol-sse")]
pub use flow_engine_sse;
#[cfg(feature = "protocol-tcp")]
pub use flow_engine_tcp;
#[cfg(feature = "protocol-ws")]
pub use flow_engine_ws;
