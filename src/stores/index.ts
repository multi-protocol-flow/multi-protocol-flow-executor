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
export { usePluginsStore } from './plugins';
export { useDatasourceStore, connectionFor, connectionsForDatasource } from './datasource';
export { useAIStore } from './ai';
export { useEnvironmentStore } from './environment';

// Flow store selectors
export {
  selectFlow,
  selectActiveStage,
  selectSelectedNodeId,
  selectIsDirty,
  selectNodes,
  selectConnections,
  selectStageNodes,
  selectStageConnections,
  selectSetupNodeCount,
  selectMainNodeCount,
  selectTeardownNodeCount,
  selectAllStageNodes,
  selectAllNodes,
  selectCanUndo,
  selectCanRedo,
  selectHistoryVersion,
  selectCenterOnNodeUuid,
  selectTabs,
  selectActiveTabUuid,
  selectTabSessions,
} from './flow';
export type { FlowStage, StageNodeEntry } from './flow';
export { getStageGraph, getStageNodes, getNodeStage } from './flow';
export { getAllNodes, findNodeInFlow } from './flow';

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
  selectIsNodeRunning,
  selectRunningNodeUuid,
  selectIsExecutingFlag,
  selectSubFlowProgress,
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
  selectFlowTabDisplayMode,
  selectFlowConflict,
  selectShowFlowConflictDialog,
  selectEnvironmentManagerContext,
  selectShowExportFlowDialog,
  selectExportFlowTarget,
  selectLocateFlowTarget,
} from './ui';

// UI types
export type {
  PanelType,
  BottomPanelTab,
  PageType,
  DatasetReportPageState,
  FlowTabDisplayMode,
  FlowConflictState,
  EnvironmentManagerContext,
} from './ui';

// Debug store selectors
export {
  selectDebugStatus,
  selectCurrentNodeUuid,
  selectIsDebugging,
  selectIsPaused,
  selectIsNodeDebugPaused,
  selectIsDebugDataReadOnly,
  selectDebugCurrentNodeUuid,
  selectCallFrames,
  selectActiveFrameIndex,
  selectActiveCallFrame,
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
