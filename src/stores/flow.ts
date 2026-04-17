/**
 * 当前编辑流程状态管理
 */
import { create } from 'zustand';
import type { Flow, Node, Connection, NodeType } from '@/types';

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

  // ===== Computed (Selectors) =====
  /** 获取当前选中的节点 */
  getSelectedNode: () => Node | null;
  /** 根据 UUID 获取节点 */
  getNodeByUuid: (uuid: string) => Node | null;
  /** 检查节点是否有断点 */
  hasBreakpoint: (uuid: string) => boolean;
}

const initialState = {
  flow: null,
  selectedNodeId: null,
  isDirty: false,
  pendingAddNodeType: null,
  focusNodeUuid: null,
  isLocked: false,
  lockReason: null,
};

export const useFlowStore = create<FlowState>((set, get) => ({
  ...initialState,

  // ===== Actions =====
  setFlow: (flow) => set({ flow, selectedNodeId: null, isDirty: false }),

  updateFlowName: (name) =>
    set((state) => ({
      flow: state.flow ? { ...state.flow, name } : null,
      isDirty: true,
    })),

  updateInitialVariables: (variables) =>
    set((state) => ({
      flow: state.flow
        ? { ...state.flow, initial_variables: variables }
        : null,
      isDirty: true,
    })),

  addNode: (node) =>
    set((state) => ({
      flow: state.flow
        ? { ...state.flow, nodes: [...state.flow.nodes, node] }
        : null,
      isDirty: true,
    })),

  updateNode: (uuid, updates) =>
    set((state) => ({
      flow: state.flow
        ? {
            ...state.flow,
            nodes: state.flow.nodes.map((n) =>
              n.uuid === uuid ? { ...n, ...updates } : n
            ),
          }
        : null,
      isDirty: true,
    })),

  batchUpdateNodePositions: (positions) =>
    set((state) => ({
      flow: state.flow
        ? {
            ...state.flow,
            nodes: state.flow.nodes.map((n) => {
              const pos = positions.get(n.uuid);
              return pos ? { ...n, position: { x: pos.x, y: pos.y } } : n;
            }),
          }
        : null,
      isDirty: true,
    })),

  removeNode: (uuid) =>
    set((state) => ({
      flow: state.flow
        ? {
            ...state.flow,
            nodes: state.flow.nodes.filter((n) => n.uuid !== uuid),
            connections: state.flow.connections.filter(
              (c) =>
                c.source_node_uuid !== uuid && c.target_node_uuid !== uuid
            ),
          }
        : null,
      selectedNodeId:
        state.selectedNodeId === uuid ? null : state.selectedNodeId,
      isDirty: true,
    })),

  addConnection: (connection) =>
    set((state) => ({
      flow: state.flow
        ? {
            ...state.flow,
            connections: [...state.flow.connections, connection],
          }
        : null,
      isDirty: true,
    })),

  removeConnection: (id) =>
    set((state) => ({
      flow: state.flow
        ? {
            ...state.flow,
            connections: state.flow.connections.filter(
              (c) => c.id !== id
            ),
          }
        : null,
      isDirty: true,
    })),

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
}));

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
