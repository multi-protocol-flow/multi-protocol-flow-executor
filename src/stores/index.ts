/**
 * 状态管理统一导出
 */
// Store hooks
export { useFlowStore } from './flow';
export { useFlowListStore } from './flowList';
export { useExecutionStore } from './execution';
export { useUIStore } from './ui';
export { useDebugStore } from './debug';
export { useStressTestStore } from './stressTest';
export { useWsStreamStore } from './wsStream';

// Flow store selectors
export {
  selectFlow,
  selectSelectedNodeId,
  selectIsDirty,
  selectNodes,
  selectConnections,
  selectCanUndo,
  selectCanRedo,
  selectHistoryVersion,
  selectCenterOnNodeUuid,
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
  selectShowSettingsDialog,
} from './ui';

// UI types
export type { PanelType, BottomPanelTab, PageType, DatasetReportPageState } from './ui';

// Debug store selectors
export {
  selectDebugStatus,
  selectCurrentNodeUuid,
  selectIsDebugging,
  selectIsPaused,
  selectIsDebugDataReadOnly,
} from './debug';

// StressTest store selectors
export {
  selectStressTestStatus,
  selectStressTestProgress,
  selectStressTestReport,
  selectIsStressTestRunning,
  selectStressTestError,
} from './stressTest';

// WsStream store selectors
export {
  selectWsStreams,
  selectActiveWsConnectionId,
} from './wsStream';

// Zoom store
export { useZoomStore, ZOOM_MIN, ZOOM_MAX, ZOOM_STEP, ZOOM_DEFAULT } from './zoom';
export { selectZoom } from './zoom';
