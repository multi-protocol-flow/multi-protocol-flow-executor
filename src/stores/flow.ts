/**
 * 当前编辑流程状态管理
 */
import { create } from 'zustand';
import type { Flow, Node, NodeData, Connection, EnvironmentSet, AnyNodeType } from '@/types';
import { useExecutionStore } from './execution';
import { useConsoleStore } from './console';

/** 深度比较两个值是否相等（属性顺序无关，undefined 值视为缺失） */
function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a == null || b == null) return a === b;
  if (typeof a !== typeof b) return false;
  if (Array.isArray(a)) {
    if (!Array.isArray(b)) return false;
    if (a.length !== b.length) return false;
    return a.every((item, i) => deepEqual(item, b[i]));
  }
  if (typeof a === 'object') {
    const aObj = a as Record<string, unknown>;
    const bObj = b as Record<string, unknown>;
    const aKeys = Object.keys(aObj).filter(k => aObj[k] !== undefined);
    const bKeys = Object.keys(bObj).filter(k => bObj[k] !== undefined);
    if (aKeys.length !== bKeys.length) return false;
    const bKeySet = new Set(bKeys);
    return aKeys.every(k => bKeySet.has(k) && deepEqual(aObj[k], bObj[k]));
  }
  return false;
}

/** 深度比较两个 Flow 对象是否相同（用于去重历史快照，属性顺序无关） */
function isFlowEqual(a: Flow, b: Flow): boolean {
  if (a.uuid !== b.uuid || a.name !== b.name) return false;
  if (!deepEqual(a.initial_variables, b.initial_variables)) return false;
  if (!deepEqual(a.environments, b.environments)) return false;
  if (a.active_environment !== b.active_environment) return false;
  if (a.nodes.length !== b.nodes.length) return false;
  if (a.connections.length !== b.connections.length) return false;
  for (let i = 0; i < a.nodes.length; i++) {
    if (a.nodes[i].uuid !== b.nodes[i].uuid) return false;
    if (!deepEqual(a.nodes[i], b.nodes[i])) return false;
  }
  for (let i = 0; i < a.connections.length; i++) {
    if (!deepEqual(a.connections[i], b.connections[i])) return false;
  }
  return true;
}

interface FlowState {
  /** 当前编辑的流程 */
  flow: Flow | null;
  /** 当前选中的节点 UUID */
  selectedNodeId: string | null;
  /** 流程是否已修改（未保存） */
  isDirty: boolean;
  /** 待添加的节点类型（由 NodePalette 设置，FlowCanvas 处理） */
  pendingAddNodeType: AnyNodeType | null;
  /** 需要聚焦的节点 UUID（由 DebugPanel 设置，FlowCanvas 处理） */
  focusNodeUuid: string | null;
  /** 需要居中定位的节点 UUID（由 FlowNodeList 设置，FlowCanvas 处理，仅居中不选中） */
  centerOnNodeUuid: string | null;
  /** 流程是否被锁定（禁止编辑） */
  isLocked: boolean;
  /** 锁定原因 */
  lockReason: string | null;
  /** 撤销历史栈（最多 10 步） */
  past: Flow[];
  /** 重做历史栈（最多 10 步） */
  future: Flow[];
  /** 历史 version，每次 undo/redo 递增，用于通知 UI 同步表单状态 */
  historyVersion: number;

  // ===== Actions =====
  /** 设置当前流程 */
  setFlow: (flow: Flow | null) => void;
  /** 更新流程名称 */
  updateFlowName: (name: string) => void;
  /** 更新流程描述 */
  updateFlowDescription: (description: string) => void;
  /** 更新流程初始变量 */
  updateInitialVariables: (variables: Record<string, unknown>) => void;
  /** 设置环境列表 */
  setEnvironments: (environments: EnvironmentSet[]) => void;
  /** 设置激活的环境名称 */
  setActiveEnvironment: (name: string | null) => void;
  /** 添加环境 */
  addEnvironment: (env: EnvironmentSet) => void;
  /** 更新环境 */
  updateEnvironment: (name: string, env: EnvironmentSet) => void;
  /** 删除环境 */
  removeEnvironment: (name: string) => void;
  /** 添加节点 */
  addNode: (node: Node) => void;
  /** 更新节点 */
  updateNode: (uuid: string, updates: Partial<Node>) => void;
  /** 批量更新节点位置（自动布局用） */
  batchUpdateNodePositions: (positions: Map<string, { x: number; y: number }>) => void;
  /** 移除节点 */
  removeNode: (uuid: string) => void;
  /** 批量移除节点（单次撤销步骤） */
  removeNodes: (uuids: string[]) => void;
  /** 添加连接 */
  addConnection: (connection: Connection) => void;
  /** 移除连接 */
  removeConnection: (id: string) => void;
  /** 选中节点 */
  selectNode: (uuid: string | null) => void;
  /** 标记流程已保存 */
  markSaved: () => void;
  /** 重置流程状态 */
  reset: () => void;
  /** 设置待添加节点类型（用于跨组件通信，接受插件 type_id） */
  setPendingAddNodeType: (type: AnyNodeType | null) => void;
  /** 清除待添加节点类型 */
  clearPendingAddNodeType: () => void;
  /** 切换节点断点状态 */
  toggleBreakpoint: (uuid: string) => void;
  /** 设置节点断点状态 */
  setBreakpoint: (uuid: string, enabled: boolean) => void;
  /** 聚焦到指定节点（画布跳转） */
  focusOnNode: (uuid: string) => void;
  /** 清除节点聚焦状态 */
  clearFocusNode: () => void;
  /** 居中定位到指定节点（不触发选中） */
  centerOnNode: (uuid: string) => void;
  /** 清除居中定位状态 */
  clearCenterOnNode: () => void;
  /** 锁定流程 */
  lockFlow: (reason: string) => void;
  /** 解锁流程 */
  unlockFlow: () => void;
  /** 撤销 */
  undo: () => void;
  /** 重做 */
  redo: () => void;

  // ===== Computed (Selectors) =====
  /** 获取当前选中的节点 */
  getSelectedNode: () => Node | null;
  /** 根据 UUID 获取节点 */
  getNodeByUuid: (uuid: string) => Node | null;
  /** 检查节点是否有断点 */
  hasBreakpoint: (uuid: string) => boolean;
  /** 是否可撤销 */
  canUndo: () => boolean;
  /** 是否可重做 */
  canRedo: () => boolean;
}

const initialState = {
  flow: null,
  selectedNodeId: null,
  isDirty: false,
  pendingAddNodeType: null,
  focusNodeUuid: null,
  centerOnNodeUuid: null,
  isLocked: false,
  lockReason: null,
  past: [],
  future: [],
  historyVersion: 0,
};

export const useFlowStore = create<FlowState>((set, get) => {
  /** 历史栈最大步数 */
  const MAX_HISTORY = 10;

  /** 将当前 flow 推入历史栈（带去重：如果修改前后 flow 相同则跳过） */
  function pushHistory(state: FlowState, newFlow: Flow): { past: Flow[]; future: Flow[] } | null {
    // 去重：如果新 flow 和当前 flow（修改前）完全相同，说明没有实际变更，不推入
    if (isFlowEqual(state.flow!, newFlow)) {
      return null;
    }
    const newPast = [...state.past, state.flow!];
    if (newPast.length > MAX_HISTORY) newPast.shift();
    return { past: newPast, future: [] };
  }

  return {
    ...initialState,

    // ===== Actions =====
    setFlow: (flow) => {
      // 切换流程时清空上次执行的运行状态，避免底部面板残留旧流程的节点记录
      useExecutionStore.getState().clearExecutionState();
      useConsoleStore.getState().clearLogs();
      return set({ flow, selectedNodeId: null, isDirty: false, past: [], future: [] });
    },

    updateFlowName: (name) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = { ...state.flow, name };
        const history = pushHistory(state, newFlow);
        if (!history) return { flow: newFlow, isDirty: true };
        return { ...history, flow: newFlow, isDirty: true };
      }),

    updateFlowDescription: (description) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = { ...state.flow, description };
        const history = pushHistory(state, newFlow);
        if (!history) return { flow: newFlow, isDirty: true };
        return { ...history, flow: newFlow, isDirty: true };
      }),

    updateInitialVariables: (variables) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = { ...state.flow, initial_variables: variables };
        const history = pushHistory(state, newFlow);
        if (!history) return { flow: newFlow, isDirty: true };
        return { ...history, flow: newFlow, isDirty: true };
      }),

    setEnvironments: (environments) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = { ...state.flow, environments };
        const history = pushHistory(state, newFlow);
        if (!history) return { flow: newFlow, isDirty: true };
        return { ...history, flow: newFlow, isDirty: true };
      }),

    setActiveEnvironment: (name) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = { ...state.flow, active_environment: name ?? undefined };
        const history = pushHistory(state, newFlow);
        if (!history) return { flow: newFlow, isDirty: true };
        return { ...history, flow: newFlow, isDirty: true };
      }),

    addEnvironment: (env) =>
      set((state) => {
        if (!state.flow) return state;
        const environments = [...(state.flow.environments ?? []), env];
        const newFlow = { ...state.flow, environments };
        const history = pushHistory(state, newFlow);
        if (!history) return { flow: newFlow, isDirty: true };
        return { ...history, flow: newFlow, isDirty: true };
      }),

    updateEnvironment: (name, env) =>
      set((state) => {
        if (!state.flow) return state;
        const environments = (state.flow.environments ?? []).map((e) =>
          e.name === name ? env : e
        );
        // 如果改名了，也要更新 active_environment
        const active_environment =
          state.flow.active_environment === name ? env.name : state.flow.active_environment;
        const newFlow = { ...state.flow, environments, active_environment };
        const history = pushHistory(state, newFlow);
        if (!history) return { flow: newFlow, isDirty: true };
        return { ...history, flow: newFlow, isDirty: true };
      }),

    removeEnvironment: (name) =>
      set((state) => {
        if (!state.flow) return state;
        const environments = (state.flow.environments ?? []).filter((e) => e.name !== name);
        const active_environment =
          state.flow.active_environment === name ? undefined : state.flow.active_environment;
        const newFlow = { ...state.flow, environments, active_environment };
        const history = pushHistory(state, newFlow);
        if (!history) return { flow: newFlow, isDirty: true };
        return { ...history, flow: newFlow, isDirty: true };
      }),

    addNode: (node) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = { ...state.flow, nodes: [...state.flow.nodes, node] };
        const history = pushHistory(state, newFlow);
        if (!history) return { flow: newFlow, isDirty: true };
        return { ...history, flow: newFlow, isDirty: true };
      }),

    updateNode: (uuid, updates) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = {
          ...state.flow,
          nodes: state.flow.nodes.map((n) => {
            if (n.uuid !== uuid) return n;
            // 如果 updates 包含 data，与现有 data 浅合并，保留不在 updates.data 中的字段
            return updates.data && n.data
              ? { ...n, ...updates, data: { ...n.data, ...updates.data } as NodeData }
              : { ...n, ...updates };
          }),
        };
        // 检查是否有实际变更，无变更则不标记 isDirty（避免选中/取消选中触发无意义保存）
        const history = pushHistory(state, newFlow);
        if (!history) {
          // pushHistory 去重判定为无变更，不触发 isDirty
          return state;
        }
        return { ...history, flow: newFlow, isDirty: true };
      }),

    batchUpdateNodePositions: (positions) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = {
          ...state.flow,
          nodes: state.flow.nodes.map((n) => {
            const pos = positions.get(n.uuid);
            return pos ? { ...n, position: { x: pos.x, y: pos.y } } : n;
          }),
        };
        const history = pushHistory(state, newFlow);
        if (!history) return { flow: newFlow, isDirty: true };
        return { ...history, flow: newFlow, isDirty: true };
      }),

    removeNode: (uuid) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = {
          ...state.flow,
          nodes: state.flow.nodes.filter((n) => n.uuid !== uuid),
          connections: state.flow.connections.filter(
            (c) =>
              c.source_node_uuid !== uuid && c.target_node_uuid !== uuid
          ),
        };
        const history = pushHistory(state, newFlow);
        return {
          ...(history ?? {}),
          flow: newFlow,
          selectedNodeId:
            state.selectedNodeId === uuid ? null : state.selectedNodeId,
          isDirty: true,
        };
      }),

    removeNodes: (uuids) =>
      set((state) => {
        if (!state.flow || uuids.length === 0) return state;
        const uuidSet = new Set(uuids);
        const newFlow = {
          ...state.flow,
          nodes: state.flow.nodes.filter((n) => !uuidSet.has(n.uuid)),
          connections: state.flow.connections.filter(
            (c) => !uuidSet.has(c.source_node_uuid) && !uuidSet.has(c.target_node_uuid)
          ),
        };
        const history = pushHistory(state, newFlow);
        const selectedCleared = uuidSet.has(state.selectedNodeId ?? '');
        return {
          ...(history ?? {}),
          flow: newFlow,
          selectedNodeId: selectedCleared ? null : state.selectedNodeId,
          isDirty: true,
        };
      }),

    addConnection: (connection) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = {
          ...state.flow,
          connections: [...state.flow.connections, connection],
        };
        const history = pushHistory(state, newFlow);
        if (!history) return { flow: newFlow, isDirty: true };
        return { ...history, flow: newFlow, isDirty: true };
      }),

    removeConnection: (id) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = {
          ...state.flow,
          connections: state.flow.connections.filter(
            (c) => c.id !== id
          ),
        };
        const history = pushHistory(state, newFlow);
        if (!history) return { flow: newFlow, isDirty: true };
        return { ...history, flow: newFlow, isDirty: true };
      }),

  selectNode: (uuid) => set({ selectedNodeId: uuid }),

  markSaved: () => set({ isDirty: false }),

  reset: () => set(initialState),

  setPendingAddNodeType: (type) => set({ pendingAddNodeType: type }),

  clearPendingAddNodeType: () => set({ pendingAddNodeType: null }),

  toggleBreakpoint: (uuid) =>
    set((state) => {
      if (!state.flow) return state;
      const node = state.flow.nodes.find((n) => n.uuid === uuid);
      if (!node) return state;
      const currentBreakpoint = node.hasBreakpoint ?? false;
      const newBreakpoint = !currentBreakpoint;

      // 如果正在调试中，同步断点到后端
      // 使用动态导入避免循环依赖
      import('./debug').then(({ useDebugStore, selectIsDebugging }) => {
        const debugStore = useDebugStore.getState();
        if (selectIsDebugging(debugStore)) {
          debugStore.setBreakpointDynamic(uuid, newBreakpoint).catch((err) => {
            console.error('[Flow] Failed to sync breakpoint to backend:', err);
          });
        }
      });

      return {
        flow: {
          ...state.flow,
          nodes: state.flow.nodes.map((n) =>
            n.uuid === uuid ? { ...n, hasBreakpoint: newBreakpoint } : n
          ),
        },
        isDirty: true,
      };
    }),

  setBreakpoint: (uuid, enabled) =>
    set((state) => ({
      flow: state.flow
        ? {
            ...state.flow,
            nodes: state.flow.nodes.map((n) =>
              n.uuid === uuid ? { ...n, hasBreakpoint: enabled } : n
            ),
          }
        : null,
      isDirty: true,
    })),

  focusOnNode: (uuid) => set({ focusNodeUuid: uuid }),

  clearFocusNode: () => set({ focusNodeUuid: null }),

  centerOnNode: (uuid) => set({ centerOnNodeUuid: uuid }),
  clearCenterOnNode: () => set({ centerOnNodeUuid: null }),

  lockFlow: (reason) => set({ isLocked: true, lockReason: reason }),
  unlockFlow: () => set({ isLocked: false, lockReason: null }),

  undo: () => {
    const { flow, past, future, selectedNodeId, historyVersion } = get();
    if (!flow || past.length === 0) return;
    const restoredFlow = past[past.length - 1];
    // 保留选中状态，仅当节点不存在于恢复后的 flow 时才清除
    const stillExists = selectedNodeId
      ? restoredFlow.nodes.some((n) => n.uuid === selectedNodeId)
      : false;
    set({
      past: past.slice(0, -1),
      future: [...future, flow],
      flow: restoredFlow,
      selectedNodeId: stillExists ? selectedNodeId : null,
      isDirty: true,
      historyVersion: historyVersion + 1,
    });
  },

  redo: () => {
    const { flow, past, future, selectedNodeId, historyVersion } = get();
    if (!flow || future.length === 0) return;
    const restoredFlow = future[future.length - 1];
    // 保留选中状态，仅当节点不存在于恢复后的 flow 时才清除
    const stillExists = selectedNodeId
      ? restoredFlow.nodes.some((n) => n.uuid === selectedNodeId)
      : false;
    set({
      past: [...past, flow],
      future: future.slice(0, -1),
      flow: restoredFlow,
      selectedNodeId: stillExists ? selectedNodeId : null,
      isDirty: true,
      historyVersion: historyVersion + 1,
    });
  },

  // ===== Computed (Selectors) =====
  getSelectedNode: () => {
    const { flow, selectedNodeId } = get();
    if (!flow || !selectedNodeId) return null;
    return flow.nodes.find((n) => n.uuid === selectedNodeId) || null;
  },

  getNodeByUuid: (uuid: string) => {
    const { flow } = get();
    if (!flow) return null;
    return flow.nodes.find((n) => n.uuid === uuid) || null;
  },

  hasBreakpoint: (uuid: string) => {
    const { flow } = get();
    if (!flow) return false;
    const node = flow.nodes.find((n) => n.uuid === uuid);
    return node?.hasBreakpoint ?? false;
  },

  canUndo: () => get().past.length > 0,
  canRedo: () => get().future.length > 0,
  };
});

// ===== Selectors =====
/** 稳定的空数组引用，避免 useSyncExternalStore 无限循环 */
const EMPTY_NODES: Node[] = [];
const EMPTY_CONNECTIONS: Connection[] = [];

/** 选择当前流程 */
export const selectFlow = (state: FlowState) => state.flow;
/** 选择选中的节点 ID */
export const selectSelectedNodeId = (state: FlowState) => state.selectedNodeId;
/** 选择是否已修改 */
export const selectIsDirty = (state: FlowState) => state.isDirty;
/** 选择节点列表 */
export const selectNodes = (state: FlowState) => state.flow?.nodes ?? EMPTY_NODES;
/** 选择连接列表 */
export const selectConnections = (state: FlowState) =>
  state.flow?.connections ?? EMPTY_CONNECTIONS;
/** 选择流程是否被锁定 */
export const selectIsLocked = (state: FlowState) => state.isLocked;
/** 选择锁定原因 */
export const selectLockReason = (state: FlowState) => state.lockReason;
/** 选择是否可撤销 */
export const selectCanUndo = (state: FlowState) => state.past.length > 0;
/** 选择是否可重做 */
export const selectCanRedo = (state: FlowState) => state.future.length > 0;
/** 选择历史版本号 */
export const selectHistoryVersion = (state: FlowState) => state.historyVersion;
/** 选择居中定位节点 UUID */
export const selectCenterOnNodeUuid = (state: FlowState) => state.centerOnNodeUuid;
