//! Flow Engine Core - 多协议流程编排引擎的基础类型与抽象接口
//!
//! 提供所有执行器共享的核心数据类型、trait 定义和工具函数。
//! 作为 workspace 的叶子 crate，不依赖任何其他 workspace crate，
//! 也不包含具体的协议客户端实现（HTTP/WS/DB 客户端）。

/// 节点间的连接定义，描述数据流向
pub mod connection;
/// 流程执行上下文，管理变量存储与运行时状态
pub mod context;
/// 数据集类型定义，包括 CSV 配置、行取策略和数据驱动执行报告
pub mod dataset;
/// 调试模式相关类型，支持断点、单步执行和执行栈追踪
pub mod debug_types;
/// 动态变量系统，提供类似 Postman 的随机数、时间戳、UUID 等变量即时生成
pub mod dynamic_variable;
/// `OAuth2` + SASL 认证工具模块（服务商预设配置、SASL 字符串构建）
/// Error类型定义，区分致命Error和可恢复Error
pub mod error;
/// 正常执行模式的事件类型，用于实时推送节点和流程完成状态到前端
pub mod execution_event;
/// 执行器 trait 定义，所有节点执行器的统一接口
pub mod executor;
/// 流程定义，包含节点、连接和初始变量的完整结构
pub mod flow;
/// 钩子类型定义，支持流程完成时自动触发动作
pub mod hook;
/// 日志事件类型，记录执行过程中的信息和诊断数据
pub mod log_event;
/// 节点定义，包括节点类型、端口和位置信息
pub mod node;
/// 节点配置枚举，14 种节点类型的差异化配置数据
pub mod node_config;
/// 节点类型常量，提供所有已注册节点类型的字符串常量
pub mod node_types;
/// 节点插件 trait 定义，所有节点类型的统一接口
pub mod plugin;
/// 节点注册表，管理所有已注册的节点插件
pub mod registry;
/// 执行报告结构，包含节点级和流程级的执行结果与统计数据
pub mod report;
/// 流程统计信息，记录执行时间、节点数等聚合指标
pub mod statistics;
/// 抽象 trait 集合，解耦核心逻辑与 Tauri/文件系统等外部依赖
pub mod traits;
/// 通用工具函数
pub mod utils;
/// 变量系统，支持 `{{path.to.value}}` 模板语法引用执行上下文中的数据
pub mod variable;
/// WebSocket 消息流追踪，支持消息方向、内容类型和所有权转移
pub mod ws_stream;

// Re-export key trait types
pub use traits::{
    AppDirProvider, ConsoleLogSender, DebugEventSender, DefaultAppDirProvider, DirectoryOpener,
    ExecutionEventSender, LogSender, LogWorker, NoopDirectoryOpener, NoopExecutionEventSender,
    NoopLogSender, QueuedLogSender, UnderlyingLogSender,
};

// Re-export debug types
pub use debug_types::{
    DebugCommand, DebugEvent, DebugSessionState, DebugStatus, EndReason, ExecutionStackEntry,
    PauseReason,
};

// Re-export execution event types
pub use execution_event::ExecutionEvent;

// Re-export ws_stream types
pub use ws_stream::{
    MessageContent, MessageDirection, WsConnectionStream, WsMessage, WsStreamSegment,
};

// Re-export hook types
pub use hook::{FlowHook, HookAction, HookContext, HookTrigger};

// Re-export hook executor trait
pub use traits::{HookExecutor, NoopHookExecutor};

// Re-export node types
pub use node::ExecutionErrorStrategy;
