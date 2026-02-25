/**
 * 本地存储服务
 * 提供流程和执行历史的 CRUD 操作
 */

// 流程存储
export {
  saveFlow,
  loadFlow,
  listFlows,
  deleteFlow,
  clearFlows,
  type FlowMetadata,
} from './flowStorage';

// 执行历史存储
export {
  saveExecutionHistory,
  listExecutionHistory,
  clearExecutionHistory,
  deleteExecutionHistoryByFlow,
  type ExecutionHistoryItem,
} from './historyStorage';
