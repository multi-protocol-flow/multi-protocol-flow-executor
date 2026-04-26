//! 抽象接口模块
//!
//! 提供与 Tauri 解耦的 trait 定义，使得业务逻辑可以在无 Tauri 依赖的环境下运行。

mod app_dir;
mod db_provider;
mod debug_event_sender;
mod execution_event_sender;
mod directory_opener;
mod kafka_provider;
mod log_sender;
mod ws_provider;
mod grpc_provider;
mod mcp_provider;
mod mqtt_provider;
mod tcp_provider;
mod udp_provider;

pub use app_dir::{AppDirProvider, DefaultAppDirProvider};
pub use db_provider::{
    DatabasePool, DbProvider, IsolationLevel, QueryResult, TransactionState,
};
pub use debug_event_sender::DebugEventSender;
pub use execution_event_sender::{ExecutionEventSender, NoopExecutionEventSender};
pub use directory_opener::{DirectoryOpener, NoopDirectoryOpener};
pub use grpc_provider::{
    GrpcConnectionInfo, GrpcMethodInfo, GrpcPattern, GrpcProvider, GrpcServiceInfo,
    GrpcStreamingResult, GrpcTimingInfo,
};
pub use kafka_provider::{
    KafkaAdminOutput, KafkaBrokerInfo, KafkaCloseOutput, KafkaConnectOutput, KafkaConnectionInfo,
    KafkaConsumerGroupInfo, KafkaConsumeBatchOutput, KafkaConsumeOutput, KafkaMessageOutput,
    KafkaPartitionInfo, KafkaProduceOutput, KafkaProvider, KafkaSchemaInfo,
    KafkaSchemaRegistryOutput, KafkaTimingInfo, KafkaTopicInfo,
};
pub use mcp_provider::{
    McpConnectionInfo, McpProvider, McpPromptArgumentInfo, McpPromptInfo, McpPromptResult,
    McpResourceInfo, McpResourceReadResult, McpServerInfo, McpToolCallResult, McpToolInfo,
};
pub use mqtt_provider::{
    MqttBrokerInfo, MqttConnectOutput, MqttConnectionInfo, MqttMessageOutput, MqttProvider,
    MqttPublishOutput, MqttReceivedMessage, MqttSubscribeOutput, MqttTimingInfo,
};
pub use log_sender::{
    ConsoleLogSender, LogSender, LogWorker, NoopLogSender, QueuedLogSender, UnderlyingLogSender,
};
pub use tcp_provider::{FramingConfig, TcpConnectionInfo, TcpFramingMode, TcpProvider};
pub use udp_provider::{UdpProvider, UdpSocketInfo};
pub use ws_provider::WsProvider;
