/**
 * 当前编辑流程状态管理
 */
import { create } from 'zustand';
import type { Flow, Node, Connection } from '@/types';

interface FlowState {
  /** 当前编辑的流程 */
  flow: Flow | null;
  /** 当前选中的节点 UUID */
  selectedNodeId: string | null;
  /** 流程是否已修改（未保存） */
  isDirty: boolean;
  /** 待添加的节点类型（由 NodeLibrary 设置，FlowCanvas 处理） */
  pendingAddNodeType: string | null;

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
  setPendingAddNodeType: (type: string | null) => void;
  /** 清除待添加节点类型 */
  clearPendingAddNodeType: () => void;

  // ===== Computed (Selectors) =====
  /** 获取当前选中的节点 */
  getSelectedNode: () => Node | null;
  /** 根据 UUID 获取节点 */
  getNodeByUuid: (uuid: string) => Node | null;
}

const initialState = {
  flow: null,
  selectedNodeId: null,
  isDirty: false,
  pendingAddNodeType: null,
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
