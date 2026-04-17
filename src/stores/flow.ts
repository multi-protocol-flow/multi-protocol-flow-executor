/**
 * 当前编辑流程状态管理
 */
import { create } from 'zustand';
import type { Flow, Node, Connection, NodeType } from '@/types';

/** 浅比较两个 Flow 对象是否相同（用于去重历史快照） */
function isFlowEqual(a: Flow, b: Flow): boolean {
  if (a.uuid !== b.uuid || a.name !== b.name) return false;
  if (JSON.stringify(a.initial_variables) !== JSON.stringify(b.initial_variables)) return false;
  if (a.nodes.length !== b.nodes.length) return false;
  if (a.connections.length !== b.connections.length) return false;
  // 逐节点比较（浅比较 JSON 序列化结果）
  for (let i = 0; i < a.nodes.length; i++) {
    if (a.nodes[i].uuid !== b.nodes[i].uuid) return false;
    if (JSON.stringify(a.nodes[i]) !== JSON.stringify(b.nodes[i])) return false;
  }
  for (let i = 0; i < a.connections.length; i++) {
    if (JSON.stringify(a.connections[i]) !== JSON.stringify(b.connections[i])) return false;
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
  pendingAddNodeType: NodeType | null;
  /** 需要聚焦的节点 UUID（由 DebugPanel 设置，FlowCanvas 处理） */
  focusNodeUuid: string | null;
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
  /** 更新流程初始变量 */
  updateInitialVariables: (variables: Record<string, unknown>) => void;
  /** 添加节点 */
  addNode: (node: Node) => void;
  /** 更新节点 */
  updateNode: (uuid: string, updates: Partial<Node>) => void;
  /** 批量更新节点位置（自动布局用） */
  batchUpdateNodePositions: (positions: Map<string, { x: number; y: number }>) => void;
  /** 移除节点 */
  removeNode: (uuid: string) => void;
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
  /** 设置待添加节点类型（用于跨组件通信） */
  setPendingAddNodeType: (type: NodeType | null) => void;
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
    setFlow: (flow) => set({ flow, selectedNodeId: null, isDirty: false, past: [], future: [] }),

    updateFlowName: (name) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = { ...state.flow, name };
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
          nodes: state.flow.nodes.map((n) =>
            n.uuid === uuid ? { ...n, ...updates } : n
          ),
        };
        const history = pushHistory(state, newFlow);
        if (!history) return { flow: newFlow, isDirty: true };
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
            console.error('[Flow] 同步断点到后端失败:', err);
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
/** 选择当前流程 */
export const selectFlow = (state: FlowState) => state.flow;
/** 选择选中的节点 ID */
export const selectSelectedNodeId = (state: FlowState) => state.selectedNodeId;
/** 选择是否已修改 */
export const selectIsDirty = (state: FlowState) => state.isDirty;
/** 选择节点列表 */
export const selectNodes = (state: FlowState) => state.flow?.nodes ?? [];
/** 选择连接列表 */
export const selectConnections = (state: FlowState) =>
  state.flow?.connections ?? [];
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
