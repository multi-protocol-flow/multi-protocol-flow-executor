/**
 * 类型定义统一导出
 */

// 端口类型
export type { Port, PortType } from './port';

// 节点类型
export type {
  NodeType,
  NodePosition,
  HttpMethod,
  HttpBodyType,
  ContentType,
  FormFieldType,
  FormField,
  ConditionType,
  ConditionOperator,
  ExpectedValueType,
  EntryConfig,
  HttpConfig,
  ConditionalConfig,
  WsCollectMode,
  WsMatchType,
  WsMessageType,
  WsCollectCondition,
  WsConnectConfig,
  WsSendCollectConfig,
  WsCloseConfig,
  TcpFramingMode,
  FramingConfig,
  TcpConnectConfig,
  TcpSendConfig,
  TcpReceiveConfig,
  TcpCloseConfig,
  UdpBindConfig,
  UdpSendToConfig,
  UdpRecvFromConfig,
  UdpCloseConfig,
  RecordingDirection,
  RecordingEntry,
  TcpUdpRecording,
  VariableExtractorConfig,
  SseConnectConfig,
  SseListenConfig,
  SseDisconnectConfig,
  GraphqlConnectConfig,
  GraphqlQueryConfig,
  GraphqlSubscribeConfig,
  GraphqlIntrospectConfig,
  GraphqlDisconnectConfig,
  AssertionOperator,
  AssertionMode,
  OnFailureMode,
  Assertion,
  AssertionNodeConfig,
  AssertionResult,
  AssertionOutput,
  ScriptNodeConfig,
  NodeData,
  Node,
  AnyNodeType,
  AnyNodeData,
  AnyNode,
  PluginNodeData,
} from './node';
export {
  isEntryConfig,
  isHttpConfig,
  isConditionalConfig,
  isWsConnectConfig,
  isWsSendCollectConfig,
  isWsCloseConfig,
  isTcpConnectConfig,
  isTcpSendConfig,
  isTcpReceiveConfig,
  isTcpCloseConfig,
  isUdpBindConfig,
  isUdpSendToConfig,
  isUdpRecvFromConfig,
  isUdpCloseConfig,
  isAssertionConfig,
  isScriptConfig,
  isVariableExtractorConfig,
  isSseConnectConfig,
  isSseListenConfig,
  isSseDisconnectConfig,
  isGraphqlConnectConfig,
  isGraphqlQueryConfig,
  isGraphqlSubscribeConfig,
  isGraphqlIntrospectConfig,
  isGraphqlDisconnectConfig,
} from './node';



// 连接类型
export type { Connection } from './connection';

// 流程类型
export type { Flow, EnvironmentSet } from './flow';
export { getActiveVariables } from './flow';

// 错误类型
export type { ErrorKind, ErrorCode, ErrorDetail } from './error';
export { isFatalError } from './error';

// 报告类型
export type {
  ExecutionStatus,
  VariableChange,
  NodeExecutionReport,
  FlowStatistics,
  FlowExecutionReport,
  PluginStreamEvent,
  PluginStreamStats,
  PluginStreamRecord,
} from './report';

// 执行事件类型
export type { ExecutionEvent, PluginEventData } from './execution_event';

// IPC 类型
export type {
  LoadFlowCommandInput,
  LoadFlowCommandOutput,
  SaveFlowCommandInput,
  SaveFlowCommandOutput,
  CreateFlowCommandInput,
  CreateFlowCommandOutput,
  ExecuteFlowCommandInput,
  ExecuteFlowCommandOutput,
  UpdateNodeCommandInput,
  UpdateNodeCommandOutput,
  DeleteNodeCommandInput,
  DeleteNodeCommandOutput,
} from './ipc';

// 设置类型
export type {
  Settings,
  SettingsState,
  SettingsActions,
  SettingsStore,
} from './settings';

// 插件设置 / 插件市场类型
export type {
  PluginDirInfo,
  PluginDirSource,
  InstalledPlugin,
  MarketplacePlugin,
  MarketplacePlatformAsset,
} from './plugins';

// 变量类型
export type { Variable, VariablePool, OutputMapping } from './variable';

// 调试类型
export type {
  DebugStatus,
  PauseReason,
  EndReason,
  DebugCommand,
  ExecutionStackEntry,
  DebugEvent,
  DebugSessionState,
  GrpcConnectionInfo,
  GrpcMethodCallRecord,
} from './debug';
export { DEBUG_HEALTH_CONFIG } from './debug';

// WebSocket 流类型
export type {
  MessageDirection,
  MessageContent,
  WsMessage,
  WsStreamSegment,
  WsSegmentSummary,
  WsConnectionStream,
  WsStreamSummary,
} from './ws_stream';

// 压力测试类型
export type {
  StressTestStatus,
  StressTestConfig,
  StressTestProgress,
  ErrorSample,
  ResponseTimeDistribution,
  ErrorStatistics,
  StressTestStatistics,
  StressTestReport,
} from './stressTest';

// 数据集执行类型
export type {
  FailureStrategy,
  DatasetExecutionConfig,
  CsvInfo,
  RowStatus,
  RowSummary,
  DatasetExecutionReport,
  DatasetEvent,
  DatasetReportSummary,
} from './dataset';

// 钩子类型
export type {
  HookTrigger,
  WebhookHookAction,
  ScriptHookAction,
  NotifyHookAction,
  HookAction,
  FlowHook,
} from './hook';

// 插件 iframe 协议类型
export type {
  PluginIframeHostMessageType,
  PluginIframePluginMessageType,
  PluginIframeMessageType,
  PluginIframeHostApi,
  PluginIframeNodeSnapshot,
  PluginIframeInitPayload,
  PluginIframeConfigUpdatedPayload,
  PluginIframeActionResultPayload,
  PluginIframeReadyPayload,
  PluginIframeConfigChangedPayload,
  PluginIframeRequestActionPayload,
  PluginIframeResizePayload,
  PluginIframeErrorPayload,
  PluginIframeInitMessage,
  PluginIframeConfigUpdatedMessage,
  PluginIframeActionResultMessage,
  PluginIframeReadyMessage,
  PluginIframeConfigChangedMessage,
  PluginIframeRequestActionMessage,
  PluginIframeResizeMessage,
  PluginIframeErrorMessage,
  PluginIframeHostMessage,
  PluginIframePluginMessage,
  PluginIframeMessage,
} from './plugin-iframe';
export {
  isPluginIframeMessage,
  createInitMessage,
  createConfigUpdatedMessage,
  createActionResultMessage,
} from './plugin-iframe';
