/**
 * 当前编辑流程状态管理
 */
import { create } from 'zustand';
import type { Flow, Node, NodeData, Connection, AnyNodeType, FlowTab, FlowTabSession } from '@/types';
import { useExecutionStore } from './execution';
import { useUIStore } from './ui';
import { checkFlowConflict } from '@/lib/flowConflict';
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

  /** 打开的流程标签列表 */
  tabs: FlowTab[];
  /** 当前激活的标签 UUID */
  activeTabUuid: string | null;
  /** 各标签的会话状态缓存（切换标签时保持每个流程独立的撤销重做栈与选中状态） */
  tabSessions: Record<string, FlowTabSession>;

  // ===== Tab Actions =====
  /** 打开流程标签（若已打开则切换） */
  openTab: (flow: Flow, filePath?: string, options?: { force?: boolean }) => boolean;
  /** 切换活跃标签 */
  switchTab: (targetUuid: string) => void;
  /** 关闭指定标签 */
  closeTab: (uuid: string) => void;
  /** 关闭其他标签 */
  closeOtherTabs: (keepUuid: string) => void;
  /** 关闭指定标签右侧的所有标签 */
  closeTabsToRight: (uuid: string) => void;
  /** 关闭所有标签 */
  closeAllTabs: () => void;
  /** 更新指定标签的名称 */
  updateTabName: (uuid: string, name: string) => void;

  // ===== Actions =====
  /** 设置当前流程 */
  setFlow: (flow: Flow | null) => void;
  /** 更新流程名称 */
  updateFlowName: (name: string) => void;
  /** 更新流程描述 */
  updateFlowDescription: (description: string) => void;
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
  /** 调试流程栈（用于进入子流程调试时隔离主流程画布状态） */
  debugFlowStack: Array<{ flow: Flow; parentNodeUuid?: string }>;
  /** 压入调试子流程（将子流程作为只读流程展示在画布，同时保留主流程） */
  pushDebugSubflow: (subflow: Flow, parentNodeUuid?: string) => void;
  /** 弹出调试子流程（画布平滑还原回上一级流程） */
  popDebugSubflow: () => void;
  /** 清空调试子流程栈，还原主流程 */
  clearDebugSubflows: () => void;
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
  debugFlowStack: [] as Array<{ flow: Flow; parentNodeUuid?: string }>,
  tabs: [] as FlowTab[],
  activeTabUuid: null as string | null,
  tabSessions: {} as Record<string, FlowTabSession>,
};

/** 标记当前活跃标签为未保存脏状态（若已经是 dirty 则返回原引用） */
function markTabDirty(tabs: FlowTab[], activeTabUuid: string | null): FlowTab[] {
  if (!activeTabUuid) return tabs;
  let changed = false;
  const newTabs = tabs.map((t) => {
    if (t.uuid === activeTabUuid && !t.isDirty) {
      changed = true;
      return { ...t, isDirty: true };
    }
    return t;
  });
  return changed ? newTabs : tabs;
}

/** 断点节点 UUID 集合缓存：flow 对象身份变化时才重建，消除 27 个节点组件订阅导致的 O(N²) 扫描 */
let breakpointCacheFlow: Flow | null = null;
let breakpointCacheSet: ReadonlySet<string> = new Set<string>();
function getBreakpointSet(flow: Flow | null): ReadonlySet<string> {
  if (flow !== breakpointCacheFlow) {
    breakpointCacheFlow = flow;
    breakpointCacheSet = new Set(
      flow ? flow.nodes.filter((n) => n.hasBreakpoint).map((n) => n.uuid) : [],
    );
  }
  return breakpointCacheSet;
}

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
      if (flow === null) {
        useExecutionStore.getState().clearExecutionState();
        return set({
          flow: null,
          activeTabUuid: null,
          selectedNodeId: null,
          isDirty: false,
          past: [],
          future: [],
        });
      }
      return get().openTab(flow);
    },

    openTab: (flow, filePath, options) => {
      const state = get();
      if (!options?.force) {
        const conflict = checkFlowConflict(flow, filePath, state.tabs, state.tabSessions);
        if (conflict) {
          useUIStore.getState().openFlowConflictDialog(conflict);
          return false;
        }
      }
      const existingTabIndex = state.tabs.findIndex(
        (t) => t.uuid === flow.uuid || (filePath && t.filePath === filePath)
      );
      if (existingTabIndex >= 0) {
        const matchedTab = state.tabs[existingTabIndex];
        if (state.activeTabUuid === matchedTab.uuid) {
          useExecutionStore.getState().clearExecutionState();
          const updatedTabs = state.tabs.map((t) =>
            t.uuid === matchedTab.uuid
              ? { ...t, name: flow.name, isDirty: false, filePath: filePath ?? t.filePath }
              : t
          );
          const updatedSessions = {
            ...state.tabSessions,
            [flow.uuid]: {
              flow,
              selectedNodeId: null,
              isDirty: false,
              past: [],
              future: [],
              historyVersion: 0,
            },
          };
          set({
            tabs: updatedTabs,
            tabSessions: updatedSessions,
            flow,
            selectedNodeId: null,
            isDirty: false,
            past: [],
            future: [],
            historyVersion: 0,
          });
          return true;
        }

        const updatedSessions = { ...state.tabSessions };
        if (state.activeTabUuid && state.flow) {
          updatedSessions[state.activeTabUuid] = {
            flow: state.flow,
            selectedNodeId: state.selectedNodeId,
            isDirty: state.isDirty,
            past: state.past,
            future: state.future,
            historyVersion: state.historyVersion,
          };
        }
        updatedSessions[flow.uuid] = {
          flow,
          selectedNodeId: null,
          isDirty: false,
          past: [],
          future: [],
          historyVersion: 0,
        };
        const updatedTabs = state.tabs.map((t) =>
          t.uuid === matchedTab.uuid
            ? { ...t, name: flow.name, isDirty: false, filePath: filePath ?? t.filePath }
            : t
        );
        useExecutionStore.getState().clearExecutionState();
        set({
          tabs: updatedTabs,
          activeTabUuid: matchedTab.uuid,
          tabSessions: updatedSessions,
          flow,
          selectedNodeId: null,
          isDirty: false,
          past: [],
          future: [],
          historyVersion: 0,
        });
        return true;
      }

      const updatedSessions = { ...state.tabSessions };
      if (state.activeTabUuid && state.flow) {
        updatedSessions[state.activeTabUuid] = {
          flow: state.flow,
          selectedNodeId: state.selectedNodeId,
          isDirty: state.isDirty,
          past: state.past,
          future: state.future,
          historyVersion: state.historyVersion,
        };
      }
      updatedSessions[flow.uuid] = {
        flow,
        selectedNodeId: null,
        isDirty: false,
        past: [],
        future: [],
        historyVersion: 0,
      };
      const newTabs: FlowTab[] = [
        ...state.tabs,
        { uuid: flow.uuid, name: flow.name, isDirty: false, filePath },
      ];
      useExecutionStore.getState().clearExecutionState();
      set({
        tabs: newTabs,
        activeTabUuid: flow.uuid,
        tabSessions: updatedSessions,
        flow,
        selectedNodeId: null,
        isDirty: false,
        past: [],
        future: [],
        historyVersion: 0,
      });
      return true;
    },

    switchTab: (targetUuid) => {
      const state = get();
      if (targetUuid === state.activeTabUuid) return;
      const targetSession = state.tabSessions[targetUuid];
      if (!targetSession) return;

      const updatedSessions = { ...state.tabSessions };
      if (state.activeTabUuid && state.flow) {
        updatedSessions[state.activeTabUuid] = {
          flow: state.flow,
          selectedNodeId: state.selectedNodeId,
          isDirty: state.isDirty,
          past: state.past,
          future: state.future,
          historyVersion: state.historyVersion,
        };
      }

      useExecutionStore.getState().clearExecutionState();
      set({
        tabSessions: updatedSessions,
        activeTabUuid: targetUuid,
        flow: targetSession.flow,
        selectedNodeId: targetSession.selectedNodeId,
        isDirty: targetSession.isDirty,
        past: targetSession.past,
        future: targetSession.future,
        historyVersion: targetSession.historyVersion,
      });
    },

    closeTab: (uuid) => {
      const state = get();
      const index = state.tabs.findIndex((t) => t.uuid === uuid);
      if (index === -1) return;

      const newTabs = state.tabs.filter((t) => t.uuid !== uuid);
      const newSessions = { ...state.tabSessions };
      delete newSessions[uuid];

      if (uuid !== state.activeTabUuid) {
        set({
          tabs: newTabs,
          tabSessions: newSessions,
        });
        return;
      }

      if (newTabs.length === 0) {
        useExecutionStore.getState().clearExecutionState();
        set({
          tabs: [],
          activeTabUuid: null,
          tabSessions: {},
          flow: null,
          selectedNodeId: null,
          isDirty: false,
          past: [],
          future: [],
          historyVersion: 0,
        });
        return;
      }

      const nextTabIndex = Math.min(index, newTabs.length - 1);
      const nextTab = newTabs[nextTabIndex];
      const nextSession = newSessions[nextTab.uuid];

      useExecutionStore.getState().clearExecutionState();
      set({
        tabs: newTabs,
        activeTabUuid: nextTab.uuid,
        tabSessions: newSessions,
        flow: nextSession ? nextSession.flow : null,
        selectedNodeId: nextSession ? nextSession.selectedNodeId : null,
        isDirty: nextSession ? nextSession.isDirty : false,
        past: nextSession ? nextSession.past : [],
        future: nextSession ? nextSession.future : [],
        historyVersion: nextSession ? nextSession.historyVersion : 0,
      });
    },

    closeOtherTabs: (keepUuid) => {
      const state = get();
      const keepTab = state.tabs.find((t) => t.uuid === keepUuid);
      if (!keepTab) return;

      const keepSession =
        state.activeTabUuid === keepUuid && state.flow
          ? {
              flow: state.flow,
              selectedNodeId: state.selectedNodeId,
              isDirty: state.isDirty,
              past: state.past,
              future: state.future,
              historyVersion: state.historyVersion,
            }
          : state.tabSessions[keepUuid];

      if (!keepSession) return;

      const newSessions: Record<string, FlowTabSession> = {
        [keepUuid]: keepSession,
      };

      useExecutionStore.getState().clearExecutionState();
      set({
        tabs: [keepTab],
        activeTabUuid: keepUuid,
        tabSessions: newSessions,
        flow: keepSession.flow,
        selectedNodeId: keepSession.selectedNodeId,
        isDirty: keepSession.isDirty,
        past: keepSession.past,
        future: keepSession.future,
        historyVersion: keepSession.historyVersion,
      });
    },

    closeTabsToRight: (uuid) => {
      const state = get();
      const index = state.tabs.findIndex((t) => t.uuid === uuid);
      if (index === -1) return;

      const remainingTabs = state.tabs.slice(0, index + 1);
      const remainingUuids = new Set(remainingTabs.map((t) => t.uuid));

      const updatedSessions = { ...state.tabSessions };
      if (state.activeTabUuid && state.flow && remainingUuids.has(state.activeTabUuid)) {
        updatedSessions[state.activeTabUuid] = {
          flow: state.flow,
          selectedNodeId: state.selectedNodeId,
          isDirty: state.isDirty,
          past: state.past,
          future: state.future,
          historyVersion: state.historyVersion,
        };
      }
      for (const key of Object.keys(updatedSessions)) {
        if (!remainingUuids.has(key)) {
          delete updatedSessions[key];
        }
      }

      if (!remainingUuids.has(state.activeTabUuid ?? '')) {
        const targetSession = updatedSessions[uuid];
        useExecutionStore.getState().clearExecutionState();
        set({
          tabs: remainingTabs,
          activeTabUuid: uuid,
          tabSessions: updatedSessions,
          flow: targetSession ? targetSession.flow : null,
          selectedNodeId: targetSession ? targetSession.selectedNodeId : null,
          isDirty: targetSession ? targetSession.isDirty : false,
          past: targetSession ? targetSession.past : [],
          future: targetSession ? targetSession.future : [],
          historyVersion: targetSession ? targetSession.historyVersion : 0,
        });
      } else {
        set({
          tabs: remainingTabs,
          tabSessions: updatedSessions,
        });
      }
    },

    closeAllTabs: () => {
      useExecutionStore.getState().clearExecutionState();
      set({
        tabs: [],
        activeTabUuid: null,
        tabSessions: {},
        flow: null,
        selectedNodeId: null,
        isDirty: false,
        past: [],
        future: [],
        historyVersion: 0,
      });
    },

    updateTabName: (uuid, name) => {
      set((state) => {
        const newTabs = state.tabs.map((t) => (t.uuid === uuid ? { ...t, name } : t));
        const newSessions = { ...state.tabSessions };
        if (newSessions[uuid]) {
          newSessions[uuid] = {
            ...newSessions[uuid],
            flow: { ...newSessions[uuid].flow, name },
          };
        }
        const newFlow =
          state.flow && state.flow.uuid === uuid ? { ...state.flow, name } : state.flow;
        return {
          tabs: newTabs,
          tabSessions: newSessions,
          flow: newFlow,
        };
      });
    },

    updateFlowName: (name) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = { ...state.flow, name };
        const history = pushHistory(state, newFlow);
        const newTabs = state.tabs.map((t) =>
          t.uuid === state.activeTabUuid ? { ...t, name, isDirty: true } : t
        );
        if (!history) return { flow: newFlow, isDirty: true, tabs: newTabs };
        return { ...history, flow: newFlow, isDirty: true, tabs: newTabs };
      }),

    updateFlowDescription: (description) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = { ...state.flow, description };
        const history = pushHistory(state, newFlow);
        const tabs = markTabDirty(state.tabs, state.activeTabUuid);
        if (!history) return { flow: newFlow, isDirty: true, tabs };
        return { ...history, flow: newFlow, isDirty: true, tabs };
      }),

    updateInitialVariables: (variables) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = { ...state.flow, initial_variables: variables };
        const history = pushHistory(state, newFlow);
        const tabs = markTabDirty(state.tabs, state.activeTabUuid);
        if (!history) return { flow: newFlow, isDirty: true, tabs };
        return { ...history, flow: newFlow, isDirty: true, tabs };
      }),

    addNode: (node) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = { ...state.flow, nodes: [...state.flow.nodes, node] };
        const history = pushHistory(state, newFlow);
        const tabs = markTabDirty(state.tabs, state.activeTabUuid);
        if (!history) return { flow: newFlow, isDirty: true, tabs };
        return { ...history, flow: newFlow, isDirty: true, tabs };
      }),

    updateNode: (uuid, updates) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = {
          ...state.flow,
          nodes: state.flow.nodes.map((n) => {
            if (n.uuid !== uuid) return n;
            return updates.data && n.data
              ? { ...n, ...updates, data: { ...n.data, ...updates.data } as NodeData }
              : { ...n, ...updates };
          }),
        };
        const history = pushHistory(state, newFlow);
        if (!history) {
          return state;
        }
        const tabs = markTabDirty(state.tabs, state.activeTabUuid);
        return { ...history, flow: newFlow, isDirty: true, tabs };
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
        const tabs = markTabDirty(state.tabs, state.activeTabUuid);
        if (!history) return { flow: newFlow, isDirty: true, tabs };
        return { ...history, flow: newFlow, isDirty: true, tabs };
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
        const tabs = markTabDirty(state.tabs, state.activeTabUuid);
        return {
          ...(history ?? {}),
          flow: newFlow,
          selectedNodeId:
            state.selectedNodeId === uuid ? null : state.selectedNodeId,
          isDirty: true,
          tabs,
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
        const tabs = markTabDirty(state.tabs, state.activeTabUuid);
        return {
          ...(history ?? {}),
          flow: newFlow,
          selectedNodeId: selectedCleared ? null : state.selectedNodeId,
          isDirty: true,
          tabs,
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
        const tabs = markTabDirty(state.tabs, state.activeTabUuid);
        if (!history) return { flow: newFlow, isDirty: true, tabs };
        return { ...history, flow: newFlow, isDirty: true, tabs };
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
        const tabs = markTabDirty(state.tabs, state.activeTabUuid);
        if (!history) return { flow: newFlow, isDirty: true, tabs };
        return { ...history, flow: newFlow, isDirty: true, tabs };
      }),

  selectNode: (uuid) => set({ selectedNodeId: uuid }),

  markSaved: () =>
    set((state) => {
      const activeUuid = state.activeTabUuid;
      const newTabs = state.tabs.map((t) =>
        t.uuid === activeUuid ? { ...t, isDirty: false } : t
      );
      const newSessions =
        activeUuid && state.tabSessions[activeUuid]
          ? {
              ...state.tabSessions,
              [activeUuid]: {
                ...state.tabSessions[activeUuid],
                isDirty: false,
              },
            }
          : state.tabSessions;
      return { isDirty: false, tabs: newTabs, tabSessions: newSessions };
    }),


  reset: () => {
    breakpointCacheFlow = null;
    breakpointCacheSet = new Set<string>();
    set(initialState);
  },

  pushDebugSubflow: (subflow: Flow, parentNodeUuid?: string) => {
    const currentFlow = get().flow;
    if (!currentFlow) return;
    set((state) => ({
      debugFlowStack: [...state.debugFlowStack, { flow: currentFlow, parentNodeUuid }],
      flow: subflow,
      selectedNodeId: null,
      isLocked: true,
      lockReason: 'debugging_subflow',
    }));
  },

  popDebugSubflow: () => {
    const { debugFlowStack } = get();
    if (debugFlowStack.length === 0) return;
    const previous = debugFlowStack[debugFlowStack.length - 1];
    const newStack = debugFlowStack.slice(0, -1);
    set({
      debugFlowStack: newStack,
      flow: previous.flow,
      selectedNodeId: null,
      isLocked: newStack.length > 0,
      lockReason: newStack.length > 0 ? 'debugging_subflow' : null,
    });
  },

  clearDebugSubflows: () => {
    const { debugFlowStack } = get();
    if (debugFlowStack.length === 0) return;
    const root = debugFlowStack[0];
    set({
      debugFlowStack: [],
      flow: root.flow,
      selectedNodeId: null,
      isLocked: false,
      lockReason: null,
    });
  },
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
          tabs: markTabDirty(state.tabs, state.activeTabUuid),
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
      tabs: markTabDirty(state.tabs, state.activeTabUuid),
    })),

  focusOnNode: (uuid) => set({ focusNodeUuid: uuid }),

  clearFocusNode: () => set({ focusNodeUuid: null }),

  centerOnNode: (uuid) => set({ centerOnNodeUuid: uuid }),
  clearCenterOnNode: () => set({ centerOnNodeUuid: null }),

  lockFlow: (reason) => set({ isLocked: true, lockReason: reason }),
  unlockFlow: () => set({ isLocked: false, lockReason: null }),

  undo: () => {
    const { flow, past, future, selectedNodeId, historyVersion, tabs, activeTabUuid } = get();
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
      tabs: markTabDirty(tabs, activeTabUuid),
    });
  },

  redo: () => {
    const { flow, past, future, selectedNodeId, historyVersion, tabs, activeTabUuid } = get();
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
      tabs: markTabDirty(tabs, activeTabUuid),
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

  hasBreakpoint: (uuid: string) => getBreakpointSet(get().flow).has(uuid),

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
/** 选择打开的流程标签列表 */
export const selectTabs = (state: FlowState) => state.tabs;
/** 选择当前活跃标签 UUID */
export const selectActiveTabUuid = (state: FlowState) => state.activeTabUuid;
/** 选择各标签会话状态缓存 */
export const selectTabSessions = (state: FlowState) => state.tabSessions;
