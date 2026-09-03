/**
 * 控制台面板状态管理
 * 管理日志数组、过滤条件和搜索功能
 */
import { create } from 'zustand';
import type { LogEvent, LogLevel } from '@/types/events';

export interface LogEntry extends LogEvent {
  id: string; // 唯一标识符，用于 React key
  /** 日志来源：flow 为流程执行，system 为系统/操作日志 */
  source?: 'flow' | 'system';
  /** 额外详情数据（对象、错误堆栈等） */
  details?: unknown;
}

export type LogSourceFilter = 'ALL' | 'flow' | 'system';

export interface SystemLogInput {
  level: LogLevel;
  message: string;
  category?: string; // e.g. 'FlowStorage', 'Plugin', 'Settings', 'IPC', 'Execution'
  flowUuid?: string;
  details?: unknown;
}

interface LogFilter {
  level: LogLevel | 'ALL';
  nodeUuid: string | 'ALL';
  source: LogSourceFilter;
  searchKeyword: string;
}

interface ConsoleState {
  // 状态
  logs: LogEntry[];
  filter: LogFilter;
  /** 日志最大条数 */
  MAX_LOGS: number;

  // Actions
  addLog: (log: LogEvent & { source?: 'flow' | 'system'; details?: unknown }) => void;
  addSystemLog: (input: SystemLogInput) => void;
  clearLogs: () => void;
  setFilter: (filter: Partial<LogFilter>) => void;
  setSearchKeyword: (keyword: string) => void;
  setSelectedNode: (nodeUuid: string | 'ALL') => void;
  reset: () => void;

  // Selectors (computed)
  getFilteredLogs: () => LogEntry[];
  getUniqueNodes: () => { uuid: string; name: string }[];
}

// 生成唯一 ID
const generateId = () =>
  `log-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

const initialState = {
  logs: [],
  filter: {
    level: 'ALL' as const,
    nodeUuid: 'ALL' as const,
    source: 'ALL' as const,
    searchKeyword: '',
  },
};

export const useConsoleStore = create<ConsoleState>((set, get) => ({
  ...initialState,

  /** 日志最大条数 */
  MAX_LOGS: 1000,

  // 添加日志
  // 添加流程执行日志
  addLog: (log) =>
    set((state) => {
      const newLogs = [
        ...state.logs,
        { ...log, id: generateId(), source: log.source ?? 'flow' },
      ];
      // 超出上限时丢弃最早的日志
      if (newLogs.length > state.MAX_LOGS) {
        return { logs: newLogs.slice(newLogs.length - state.MAX_LOGS) };
      }
      return { logs: newLogs };
    }),

  // 添加系统/操作日志
  addSystemLog: (input) =>
    set((state) => {
      const category = input.category ? input.category.trim() : undefined;
      const newEntry: LogEntry = {
        id: generateId(),
        level: input.level,
        message: input.message,
        timestamp: Date.now(),
        nodeUuid: category ? `system:${category.toLowerCase()}` : 'system',
        nodeName: category ? `[${category}]` : '[System]',
        flowUuid: input.flowUuid ?? 'system',
        source: 'system',
        details: input.details,
      };
      const newLogs = [...state.logs, newEntry];
      if (newLogs.length > state.MAX_LOGS) {
        return { logs: newLogs.slice(newLogs.length - state.MAX_LOGS) };
      }
      return { logs: newLogs };
    }),

  // 重置状态
  reset: () => set({ ...initialState, logs: [] }),

  // 清空日志
  clearLogs: () => set({ logs: [] }),

  // 设置过滤条件
  setFilter: (filter) =>
    set((state) => ({
      filter: { ...state.filter, ...filter },
    })),

  // 设置搜索关键词
  setSearchKeyword: (keyword) =>
    set((state) => ({
      filter: { ...state.filter, searchKeyword: keyword },
    })),

  // 设置选中节点
  setSelectedNode: (nodeUuid) =>
    set((state) => ({
      filter: { ...state.filter, nodeUuid },
    })),

  // 获取过滤后的日志
  getFilteredLogs: () => {
    const { logs, filter } = get();
    return logs.filter((log) => {
      // 来源过滤 (ALL / flow / system)
      if (filter.source !== 'ALL') {
        const logSource = log.source ?? 'flow';
        if (logSource !== filter.source) {
          return false;
        }
      }
      // 级别过滤
      if (filter.level !== 'ALL' && log.level !== filter.level) {
        return false;
      }
      // 节点过滤
      if (filter.nodeUuid !== 'ALL' && log.nodeUuid !== filter.nodeUuid) {
        return false;
      }
      // 关键词搜索
      if (
        filter.searchKeyword &&
        !log.message.toLowerCase().includes(filter.searchKeyword.toLowerCase())
      ) {
        return false;
      }
      return true;
    });
  },

  // 获取唯一节点列表
  getUniqueNodes: () => {
    const { logs } = get();
    const nodeMap = new Map<string, string>();
    logs.forEach((log) => {
      if (log.nodeUuid && log.nodeName && !nodeMap.has(log.nodeUuid)) {
        nodeMap.set(log.nodeUuid, log.nodeName);
      }
    });
    return Array.from(nodeMap.entries()).map(([uuid, name]) => ({
      uuid,
      name,
    }));
  },
}));

// ===== Selectors =====
/** 选择日志列表 */
export const selectLogs = (state: ConsoleState) => state.logs;
/** 选择过滤条件 */
export const selectFilter = (state: ConsoleState) => state.filter;
/** 选择搜索关键词 */
export const selectSearchKeyword = (state: ConsoleState) =>
  state.filter.searchKeyword;
/** 选择选中的节点 */
export const selectSelectedNode = (state: ConsoleState) =>
  state.filter.nodeUuid;
