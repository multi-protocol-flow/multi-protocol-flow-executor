//! 抽象接口模块
//!
//! 提供与 Tauri 解耦的 trait 定义，使得业务逻辑可以在无 Tauri 依赖的环境下运行。

mod app_dir;
mod debug_event_sender;
mod directory_opener;
mod execution_event_sender;
mod graphql_provider;
mod hook_executor;
mod log_sender;
mod provider_key;
mod sse_provider;
mod tcp_provider;
mod udp_provider;
mod ws_provider;

pub use app_dir::{AppDirProvider, DefaultAppDirProvider};
pub use debug_event_sender::DebugEventSender;
pub use directory_opener::{DirectoryOpener, NoopDirectoryOpener};
pub use execution_event_sender::{ExecutionEventSender, NoopExecutionEventSender};
pub use graphql_provider::{
    GraphqlConnectOutput, GraphqlConnectionInfo, GraphqlIntrospectOutput, GraphqlProvider,
    GraphqlQueryOutput, GraphqlQueryTemplate, GraphqlSubscribeOutput, GraphqlSubscriptionEvent,
    GraphqlTimingInfo,
};
pub use hook_executor::{HookExecutor, NoopHookExecutor};
pub use log_sender::{
    ConsoleLogSender, LogSender, LogWorker, NoopLogSender, QueuedLogSender, UnderlyingLogSender,
};
pub use provider_key::{
    Graphql, ProviderKey, Sse, Tcp, Udp, Ws,
};
pub use sse_provider::{
    SseConnectOutput, SseConnectionInfo, SseEventItem, SseListenOutput, SseProvider,
};
pub use tcp_provider::{FramingConfig, TcpConnectionInfo, TcpFramingMode, TcpProvider};
pub use udp_provider::{UdpProvider, UdpSocketInfo};
pub use ws_provider::WsProvider;
