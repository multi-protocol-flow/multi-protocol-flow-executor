//! Flow Engine Executor 子 crate
//!
//! 主执行器、流程编排、调试模式、压力测试、业务命令定义。

/// 主执行器，驱动流程节点的遍历与执行
pub mod executor;
/// 入口节点执行器，处理流程起始节点
pub mod entry_executor;
/// 条件分支节点执行器，根据表达式求值选择执行路径
pub mod conditional_executor;
pub mod debug;
pub mod debug_event_adapter;
/// 压力测试模块，支持并发执行与性能指标采集
pub mod stress_test;
pub mod commands;
/// 文件存储模块，管理流程文件的读写与持久化
pub mod storage;
/// 流程验证器，检查流程结构与配置的完整性
pub mod validator;
/// 命令定义，提供 CLI 和 IPC 层的输入输出类型
pub mod command;
/// 入口节点插件
pub mod plugin_entry;
/// 条件分支节点插件
pub mod plugin_conditional;
/// 默认节点注册表，注册全部 14 个内置插件
pub mod default_registry;
/// CSV 解析模块，支持多编码流式解析
pub mod csv_parser;
/// 数据驱动执行模块，逐行读取 CSV 注入 runtime_variables 执行流程
pub mod dataset_executor;
/// 变量提取节点执行器
pub mod variable_extractor_executor;
/// 变量提取节点插件
pub mod plugin_variable_extractor;
/// 默认钩子执行器，提供 Webhook/Script/Notify 三种动作的日志执行
pub mod hook_runner;
