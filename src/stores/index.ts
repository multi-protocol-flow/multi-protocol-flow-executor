/**
 * 状态管理统一导出
 */
// Store hooks
export { useFlowStore } from './flow';
export { useFlowListStore } from './flowList';
export { useExecutionStore } from './execution';
export { useUIStore } from './ui';

// Flow store selectors
export {
  selectFlow,
  selectSelectedNodeId,
  selectIsDirty,
  selectNodes,
  selectConnections,
} from './flow';

// FlowList store selectors
export {
  selectFlows,
  selectIsLoading,
  selectSearchKeyword,
} from './flowList';

// FlowList types
export type { FlowMetadata } from './flowList';

// Execution store selectors
export {
  selectIsExecuting,
  selectReport,
  selectError,
  selectHistory,
  selectExecutionStatus,
} from './execution';

// UI store selectors
export {
  selectIsFlowListPanelOpen,
  selectIsHistoryPanelOpen,
  selectIsConfigPanelOpen,
  selectIsExecutionPanelOpen,
  selectShowNewFlowDialog,
  selectShowDeleteConfirmDialog,
} from './ui';

// UI types
export type { PanelType } from './ui';
