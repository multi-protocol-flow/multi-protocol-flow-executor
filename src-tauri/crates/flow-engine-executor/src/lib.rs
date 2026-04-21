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
