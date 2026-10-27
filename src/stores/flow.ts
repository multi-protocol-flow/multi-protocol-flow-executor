/**
 * 当前编辑流程状态管理
 */
import { create } from 'zustand';
import type { Flow, FlowSubGraph, Node, NodeData, Connection, AnyNodeType, FlowTab, FlowTabSession } from '@/types';
import { useExecutionStore } from './execution';
import { useUIStore } from './ui';
import { checkFlowConflict } from '@/lib/flowConflict';

/**
 * 流程生命周期阶段
 * - `setup`：准备阶段（测试前单次执行）
 * - `main`：核心业务流程（流程主体，压测并发循环执行）
 * - `teardown`：收尾阶段（测试后单次执行，可消费 `$report` 变量）
 */
export type FlowStage = 'setup' | 'main' | 'teardown';

/** 空子图（稳定的共享引用，避免每次调用产生新对象） */
const EMPTY_SUBGRAPH: FlowSubGraph = { nodes: [], connections: [] };
const EMPTY_NODES: Node[] = [];
const EMPTY_CONNECTIONS: Connection[] = [];

/**
 * 读取指定阶段的节点与连线（`main` 对应流程主体字段；流程为空时返回空子图）
 */
export function getStageGraph(flow: Flow | null, stage: FlowStage): FlowSubGraph {
  if (!flow) return EMPTY_SUBGRAPH;
  if (stage === 'main') return { nodes: flow.nodes, connections: flow.connections };
  return (stage === 'setup' ? flow.setup : flow.teardown) ?? EMPTY_SUBGRAPH;
}

/**
 * 将子图写回指定阶段的字段（保留另一阶段引用，保证撤销栈/渲染缓存身份稳定）
 */
function withStageGraph(flow: Flow, stage: FlowStage, graph: FlowSubGraph): Flow {
  if (stage === 'main') {
    return { ...flow, nodes: graph.nodes, connections: graph.connections };
  }
  return stage === 'setup' ? { ...flow, setup: graph } : { ...flow, teardown: graph };
}

/** 阶段化节点条目（含所属阶段，用于跨阶段导航） */
export interface StageNodeEntry {
  node: Node;
  stage: FlowStage;
}

const EMPTY_STAGE_NODES: StageNodeEntry[] = [];

/**
 * 收集三个阶段的全部节点（含所属阶段）
 *
 * 按 `(flow, setup, teardown)` 引用缓存结果：节点数组引用未变时复用同一数组，
 * 满足 zustand 的引用相等比较，避免无限重渲染。
 */
let stageNodesCacheKey: readonly [Flow | null, FlowSubGraph | undefined, FlowSubGraph | undefined] =
  [null, undefined, undefined];
let stageNodesCache: StageNodeEntry[] = EMPTY_STAGE_NODES;
export function getStageNodes(flow: Flow | null): StageNodeEntry[] {
  if (!flow) return EMPTY_STAGE_NODES;
  if (
    stageNodesCacheKey[0] === flow &&
    stageNodesCacheKey[1] === flow.setup &&
    stageNodesCacheKey[2] === flow.teardown
  ) {
    return stageNodesCache;
  }
  const result: StageNodeEntry[] = [];
  for (const node of flow.setup?.nodes ?? []) result.push({ node, stage: 'setup' });
  for (const node of flow.nodes) result.push({ node, stage: 'main' });
  for (const node of flow.teardown?.nodes ?? []) result.push({ node, stage: 'teardown' });
  stageNodesCacheKey = [flow, flow.setup, flow.teardown];
  stageNodesCache = result.length === 0 ? EMPTY_STAGE_NODES : result;
  return stageNodesCache;
}

let allNodesCacheKey: readonly [Flow | null, FlowSubGraph | undefined, FlowSubGraph | undefined] =
  [null, undefined, undefined];
let allNodesCache: Node[] = EMPTY_NODES;

/**
 * 收集三个阶段的全部节点列表（按 Setup -> Main -> Teardown 顺序）
 *
 * 按 `(flow, setup, teardown)` 引用缓存结果：节点数组引用未变时复用同一数组，
 * 满足 zustand 的引用相等比较，避免无限重渲染。
 */
export function getAllNodes(flow: Flow | null): Node[] {
  if (!flow) return EMPTY_NODES;
  if (
    allNodesCacheKey[0] === flow &&
    allNodesCacheKey[1] === flow.setup &&
    allNodesCacheKey[2] === flow.teardown
  ) {
    return allNodesCache;
  }
  const result: Node[] = [];
  for (const node of flow.setup?.nodes ?? []) result.push(node);
  for (const node of flow.nodes) result.push(node);
  for (const node of flow.teardown?.nodes ?? []) result.push(node);
  allNodesCacheKey = [flow, flow.setup, flow.teardown];
  allNodesCache = result.length === 0 ? EMPTY_NODES : result;
  return allNodesCache;
}

/** 查找节点（跨 Setup / Main / Teardown 三阶段，未找到返回 null） */
export function findNodeInFlow(flow: Flow | null, uuid: string): Node | null {
  if (!flow) return null;
  return (
    flow.setup?.nodes.find((n) => n.uuid === uuid) ??
    flow.nodes.find((n) => n.uuid === uuid) ??
    flow.teardown?.nodes.find((n) => n.uuid === uuid) ??
    null
  );
}

/** 查找节点所属阶段（未找到返回 null） */
export function getNodeStage(flow: Flow, uuid: string): FlowStage | null {
  if (flow.setup?.nodes.some((n) => n.uuid === uuid)) return 'setup';
  if (flow.nodes.some((n) => n.uuid === uuid)) return 'main';
  if (flow.teardown?.nodes.some((n) => n.uuid === uuid)) return 'teardown';
  return null;
}

/** 规范化流程中的节点状态（对齐 disabled 与 on_error: 'bypass'） */
export function normalizeFlowNodes(flow: Flow): Flow {
  const normalizeNode = (n: Node): Node => {
    const isBypassed = Boolean(n.disabled || n.on_error === 'bypass');
    if (isBypassed && (!n.disabled || n.on_error !== 'bypass')) {
      return { ...n, disabled: true, on_error: 'bypass' };
    }
    return n;
  };
  return {
    ...flow,
    nodes: flow.nodes.map(normalizeNode),
    setup: flow.setup ? { ...flow.setup, nodes: flow.setup.nodes.map(normalizeNode) } : flow.setup,
    teardown: flow.teardown ? { ...flow.teardown, nodes: flow.teardown.nodes.map(normalizeNode) } : flow.teardown,
  };
}

/** 对指定阶段的子图施加变更，其余阶段引用保持原样 */
function mapStageGraph(flow: Flow, stage: FlowStage, fn: (graph: FlowSubGraph) => FlowSubGraph): Flow {
  return withStageGraph(flow, stage, fn(getStageGraph(flow, stage)));
}

/** 对包含指定节点的阶段施加变更（节点不在任何阶段时返回 null） */
function mapGraphOwningNode(flow: Flow, uuid: string, fn: (graph: FlowSubGraph) => FlowSubGraph): Flow | null {
  const stage = getNodeStage(flow, uuid);
  return stage ? mapStageGraph(flow, stage, fn) : null;
}

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

/** 深度比较两个子图的节点与连线是否相同 */
function isGraphEqual(a: FlowSubGraph, b: FlowSubGraph): boolean {
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

/** 深度比较两个 Flow 对象是否相同（用于去重历史快照，属性顺序无关） */
function isFlowEqual(a: Flow, b: Flow): boolean {
  if (a.uuid !== b.uuid || a.name !== b.name) return false;
  if (!deepEqual(a.initial_variables, b.initial_variables)) return false;
  if (Boolean(a.setup) !== Boolean(b.setup) || Boolean(a.teardown) !== Boolean(b.teardown)) {
    return false;
  }
  if (a.setup && b.setup && !isGraphEqual(a.setup, b.setup)) return false;
  if (a.teardown && b.teardown && !isGraphEqual(a.teardown, b.teardown)) return false;
  return isGraphEqual(
    { nodes: a.nodes, connections: a.connections },
    { nodes: b.nodes, connections: b.connections },
  );
}

interface FlowState {
  /** 当前编辑的流程 */
  flow: Flow | null;
  /** 当前画布展示的生命周期阶段 */
  activeStage: FlowStage;
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
  /** 更新指定标签的文件绝对路径 */
  updateTabFilePath: (uuid: string, filePath: string) => void;

  // ===== Actions =====
  /** 设置当前流程（可附带文件绝对路径） */
  setFlow: (flow: Flow | null, filePath?: string) => void;
  /** 切换画布展示的生命周期阶段 */
  setActiveStage: (stage: FlowStage) => void;
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
  activeStage: 'main' as FlowStage,
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
    const set = new Set<string>();
    if (flow) {
      for (const stage of ['setup', 'main', 'teardown'] as const) {
        for (const node of getStageGraph(flow, stage).nodes) {
          if (node.hasBreakpoint) set.add(node.uuid);
        }
      }
    }
    breakpointCacheSet = set;
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
    setFlow: (flow, filePath) => {
      if (flow === null) {
        useExecutionStore.getState().clearExecutionState();
        return set({
          flow: null,
          activeStage: 'main',
          activeTabUuid: null,
          selectedNodeId: null,
          isDirty: false,
          past: [],
          future: [],
        });
      }
      return get().openTab(flow, filePath);
    },

    setActiveStage: (stage) =>
      set((state) => {
        if (state.activeStage === stage) return state;
        // 选中节点不属于目标阶段时清除选中，避免配置面板显示画布外的节点
        const selectedInStage =
          state.flow && state.selectedNodeId
            ? getStageGraph(state.flow, stage).nodes.some((n) => n.uuid === state.selectedNodeId)
            : false;
        return {
          activeStage: stage,
          selectedNodeId: selectedInStage ? state.selectedNodeId : null,
        };
      }),

    openTab: (rawFlow, filePath, options) => {
      const flow = normalizeFlowNodes(rawFlow);
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
            activeStage: 'main',
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
          activeStage: 'main',
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
          activeStage: state.activeStage,
          isDirty: state.isDirty,
          past: state.past,
          future: state.future,
          historyVersion: state.historyVersion,
        };
      }
      updatedSessions[flow.uuid] = {
        flow,
        selectedNodeId: null,
        activeStage: 'main',
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
        activeStage: 'main',
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
          activeStage: state.activeStage,
          isDirty: state.isDirty,
          past: state.past,
          future: state.future,
          historyVersion: state.historyVersion,
        };
      }

      useExecutionStore.getState().clearExecutionState();
      const targetStage = targetSession.activeStage ?? 'main';
      const isTargetNodeInStage = targetSession.selectedNodeId
        ? getStageGraph(targetSession.flow, targetStage).nodes.some(
            (n) => n.uuid === targetSession.selectedNodeId,
          )
        : false;

      set({
        tabSessions: updatedSessions,
        activeTabUuid: targetUuid,
        flow: targetSession.flow,
        activeStage: targetStage,
        selectedNodeId: isTargetNodeInStage ? targetSession.selectedNodeId : null,
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
          activeStage: 'main',
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
      const nextStage = nextSession?.activeStage ?? 'main';
      const isNextNodeInStage = nextSession && nextSession.selectedNodeId
        ? getStageGraph(nextSession.flow, nextStage).nodes.some(
            (n) => n.uuid === nextSession.selectedNodeId,
          )
        : false;

      useExecutionStore.getState().clearExecutionState();
      set({
        tabs: newTabs,
        activeTabUuid: nextTab.uuid,
        tabSessions: newSessions,
        flow: nextSession ? nextSession.flow : null,
        activeStage: nextStage,
        selectedNodeId: isNextNodeInStage ? nextSession!.selectedNodeId : null,
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
              activeStage: state.activeStage,
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

      const keepStage = keepSession.activeStage ?? 'main';
      const isKeepNodeInStage = keepSession.selectedNodeId
        ? getStageGraph(keepSession.flow, keepStage).nodes.some(
            (n) => n.uuid === keepSession.selectedNodeId,
          )
        : false;

      useExecutionStore.getState().clearExecutionState();
      set({
        tabs: [keepTab],
        activeTabUuid: keepUuid,
        tabSessions: newSessions,
        flow: keepSession.flow,
        activeStage: keepStage,
        selectedNodeId: isKeepNodeInStage ? keepSession.selectedNodeId : null,
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
          activeStage: state.activeStage,
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
        const targetStage = targetSession?.activeStage ?? 'main';
        const isTargetNodeInStage = targetSession && targetSession.selectedNodeId
          ? getStageGraph(targetSession.flow, targetStage).nodes.some(
              (n) => n.uuid === targetSession.selectedNodeId,
            )
          : false;

        useExecutionStore.getState().clearExecutionState();
        set({
          tabs: remainingTabs,
          activeTabUuid: uuid,
          tabSessions: updatedSessions,
          flow: targetSession ? targetSession.flow : null,
          activeStage: targetStage,
          selectedNodeId: isTargetNodeInStage ? targetSession!.selectedNodeId : null,
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
        activeStage: 'main',
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
    updateTabFilePath: (uuid, filePath) => {
      set((state) => {
        const newTabs = state.tabs.map((t) => (t.uuid === uuid ? { ...t, filePath } : t));
        return { tabs: newTabs };
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
        const currentStageGraph = getStageGraph(state.flow, state.activeStage);
        if (node.type === 'entry' && currentStageGraph.nodes.some((n) => n.type === 'entry')) {
          return state;
        }
        if (node.type === 'http:ingress' && currentStageGraph.nodes.some((n) => n.type === 'http:ingress')) {
          return state;
        }
        const newFlow = mapStageGraph(state.flow, state.activeStage, (graph) => ({
          ...graph,
          nodes: [...graph.nodes, node],
        }));
        const history = pushHistory(state, newFlow);
        const tabs = markTabDirty(state.tabs, state.activeTabUuid);
        if (!history) return { flow: newFlow, isDirty: true, tabs };
        return { ...history, flow: newFlow, isDirty: true, tabs };
      }),

    updateNode: (uuid, updates) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = mapGraphOwningNode(state.flow, uuid, (graph) => ({
          ...graph,
          nodes: graph.nodes.map((n) => {
            if (n.uuid !== uuid) return n;
            return updates.data && n.data
              ? { ...n, ...updates, data: { ...n.data, ...updates.data } as NodeData }
              : { ...n, ...updates };
          }),
        }));
        if (!newFlow) return state;
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
        const newFlow = mapStageGraph(state.flow, state.activeStage, (graph) => ({
          ...graph,
          nodes: graph.nodes.map((n) => {
            const pos = positions.get(n.uuid);
            return pos ? { ...n, position: { x: pos.x, y: pos.y } } : n;
          }),
        }));
        const history = pushHistory(state, newFlow);
        const tabs = markTabDirty(state.tabs, state.activeTabUuid);
        if (!history) return { flow: newFlow, isDirty: true, tabs };
        return { ...history, flow: newFlow, isDirty: true, tabs };
      }),

    removeNode: (uuid) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = mapGraphOwningNode(state.flow, uuid, (graph) => ({
          ...graph,
          nodes: graph.nodes.filter((n) => n.uuid !== uuid),
          connections: graph.connections.filter(
            (c) =>
              c.source_node_uuid !== uuid && c.target_node_uuid !== uuid
          ),
        }));
        if (!newFlow) return state;
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
        // 批量删除可能跨阶段（节点列表聚合展示三个阶段），逐阶段过滤
        let newFlow = state.flow;
        for (const stage of ['setup', 'main', 'teardown'] as const) {
          if (stage !== 'main' && !newFlow[stage]) continue;
          newFlow = mapStageGraph(newFlow, stage, (graph) => ({
            ...graph,
            nodes: graph.nodes.filter((n) => !uuidSet.has(n.uuid)),
            connections: graph.connections.filter(
              (c) => !uuidSet.has(c.source_node_uuid) && !uuidSet.has(c.target_node_uuid)
            ),
          }));
        }
        const history = pushHistory(state, newFlow);
        if (!history) return state;
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
        const newFlow = mapStageGraph(state.flow, state.activeStage, (graph) => ({
          ...graph,
          connections: [...graph.connections, connection],
        }));
        const history = pushHistory(state, newFlow);
        const tabs = markTabDirty(state.tabs, state.activeTabUuid);
        if (!history) return { flow: newFlow, isDirty: true, tabs };
        return { ...history, flow: newFlow, isDirty: true, tabs };
      }),

    removeConnection: (id) =>
      set((state) => {
        if (!state.flow) return state;
        const newFlow = mapStageGraph(state.flow, state.activeStage, (graph) => ({
          ...graph,
          connections: graph.connections.filter(
            (c) => c.id !== id
          ),
        }));
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
    stageNodesCacheKey = [null, undefined, undefined];
    stageNodesCache = EMPTY_STAGE_NODES;
    allNodesCacheKey = [null, undefined, undefined];
    allNodesCache = EMPTY_NODES;
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
      const stage = getNodeStage(state.flow, uuid);
      if (!stage) return state;
      const node = getStageGraph(state.flow, stage).nodes.find((n) => n.uuid === uuid);
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
          flow: mapStageGraph(state.flow, stage, (graph) => ({
            ...graph,
            nodes: graph.nodes.map((n) =>
              n.uuid === uuid ? { ...n, hasBreakpoint: newBreakpoint } : n
            ),
          })),
          isDirty: true,
          tabs: markTabDirty(state.tabs, state.activeTabUuid),
        };
      }),

  setBreakpoint: (uuid, enabled) =>
    set((state) => {
      if (!state.flow) return state;
      return {
        flow: mapGraphOwningNode(state.flow, uuid, (graph) => ({
          ...graph,
          nodes: graph.nodes.map((n) =>
            n.uuid === uuid ? { ...n, hasBreakpoint: enabled } : n
          ),
        })) ?? state.flow,
        isDirty: true,
        tabs: markTabDirty(state.tabs, state.activeTabUuid),
      };
    }),

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
    const stillExists = selectedNodeId ? getNodeStage(restoredFlow, selectedNodeId) !== null : false;
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
    const stillExists = selectedNodeId ? getNodeStage(restoredFlow, selectedNodeId) !== null : false;
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
    const stage = getNodeStage(flow, selectedNodeId);
    if (!stage) return null;
    return getStageGraph(flow, stage).nodes.find((n) => n.uuid === selectedNodeId) || null;
  },

  getNodeByUuid: (uuid: string) => {
    const { flow } = get();
    if (!flow) return null;
    const stage = getNodeStage(flow, uuid);
    if (!stage) return null;
    return getStageGraph(flow, stage).nodes.find((n) => n.uuid === uuid) || null;
  },

  hasBreakpoint: (uuid: string) => getBreakpointSet(get().flow).has(uuid),

  canUndo: () => get().past.length > 0,
  canRedo: () => get().future.length > 0,
  };
});

// ===== Selectors =====
/** 稳定的空数组引用，避免 useSyncExternalStore 无限循环 */

/** 选择当前流程 */
export const selectFlow = (state: FlowState) => state.flow;
/** 选择画布当前展示的生命周期阶段 */
export const selectActiveStage = (state: FlowState) => state.activeStage;
/** 选择选中的节点 ID */
export const selectSelectedNodeId = (state: FlowState) => state.selectedNodeId;
/** 选择是否已修改 */
export const selectIsDirty = (state: FlowState) => state.isDirty;
/** 选择 Main 核心流程的节点列表（流程主体，不含 Setup/Teardown） */
export const selectNodes = (state: FlowState) => state.flow?.nodes ?? EMPTY_NODES;
/** 选择 Main 核心流程的连接列表 */
export const selectConnections = (state: FlowState) =>
  state.flow?.connections ?? EMPTY_CONNECTIONS;
/** 选择当前阶段画布的节点列表 */
export const selectStageNodes = (state: FlowState) => getStageGraph(state.flow, state.activeStage).nodes;
/** 选择当前阶段画布的连接列表 */
export const selectStageConnections = (state: FlowState) =>
  getStageGraph(state.flow, state.activeStage).connections;
/** 选择 Setup 阶段节点数量 */
export const selectSetupNodeCount = (state: FlowState) => state.flow?.setup?.nodes.length ?? 0;
/** 选择 Main 阶段节点数量 */
export const selectMainNodeCount = (state: FlowState) => state.flow?.nodes.length ?? 0;
/** 选择 Teardown 阶段节点数量 */
export const selectTeardownNodeCount = (state: FlowState) => state.flow?.teardown?.nodes.length ?? 0;
/** 选择三阶段全部节点（含所属阶段） */
export const selectAllStageNodes = (state: FlowState) => getStageNodes(state.flow);
/** 选择三阶段全部节点（Setup -> Main -> Teardown 聚合列表） */
export const selectAllNodes = (state: FlowState) => getAllNodes(state.flow);
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
