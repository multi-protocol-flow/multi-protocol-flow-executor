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
  ConditionType,
  ConditionOperator,
  EntryConfig,
  HttpConfig,
  ConditionalConfig,
  NodeData,
  Node,
} from './node';
export {
  isEntryConfig,
  isHttpConfig,
  isConditionalConfig,
} from './node';

// 连接类型
export type { Connection } from './connection';

// 流程类型
export type { Flow } from './flow';

// 错误类型
export type { ErrorKind, ErrorDetail } from './error';
export { isFatalError } from './error';

// 报告类型
export type {
  ExecutionStatus,
  NodeExecutionReport,
  FlowStatistics,
  FlowExecutionReport,
} from './report';

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
