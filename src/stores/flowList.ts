/**
 * 流程列表状态管理
 */
import { create } from 'zustand';

/** 流程元数据（用于列表展示） */
export interface FlowMetadata {
  /** 流程唯一标识符 */
  uuid: string;
  /** 流程名称 */
  name: string;
  /** 最后更新时间（毫秒时间戳） */
  updated_at: number;
  /** 节点数量（可选，用于列表显示） */
  node_count?: number;
  /** 所属文件夹相对路径（空字符串表示根目录） */
  folder_path: string;
}

interface FlowListState {
  /** 流程列表 */
  flows: FlowMetadata[];
  /** 是否正在加载 */
  isLoading: boolean;
  /** 搜索关键词 */
  searchKeyword: string;
  /** 排序字段 */
  sortBy: 'name' | 'updated_at';
  /** 排序方向 */
  sortOrder: 'asc' | 'desc';
  /** 已展开的文件夹路径集合 */
  expandedFolders: Set<string>;

  // ===== Actions =====
  /** 设置流程列表 */
  setFlows: (flows: FlowMetadata[]) => void;
  /** 添加流程到列表 */
  addFlow: (flow: FlowMetadata) => void;
  /** 更新列表中的流程 */
  updateFlow: (uuid: string, updates: Partial<FlowMetadata>) => void;
  /** 从列表中移除流程 */
  removeFlow: (uuid: string) => void;
  /** 设置加载状态 */
  setLoading: (loading: boolean) => void;
  /** 设置搜索关键词 */
  setSearchKeyword: (keyword: string) => void;
  /** 设置排序方式 */
  setSortBy: (sortBy: 'name' | 'updated_at') => void;
  /** 设置排序方向 */
  setSortOrder: (order: 'asc' | 'desc') => void;
  /** 切换文件夹展开/折叠 */
  toggleFolder: (folderPath: string) => void;
  /** 重置状态 */
  reset: () => void;

  // ===== Computed (Selectors) =====
  /** 获取过滤和排序后的流程列表 */
  getFilteredFlows: () => FlowMetadata[];
  /** 获取所有文件夹路径（从流程列表中提取） */
  getFolders: () => string[];
}

const initialState = {
  flows: [],
  isLoading: false,
  searchKeyword: '',
  sortBy: 'updated_at' as const,
  sortOrder: 'desc' as const,
  expandedFolders: new Set<string>(),
};

export const useFlowListStore = create<FlowListState>((set, get) => ({
  ...initialState,

  // ===== Actions =====
  setFlows: (flows) => set({ flows }),

  addFlow: (flow) =>
    set((state) => ({
      flows: [flow, ...state.flows],
    })),

  updateFlow: (uuid, updates) =>
    set((state) => ({
      flows: state.flows.map((f) =>
        f.uuid === uuid ? { ...f, ...updates } : f
      ),
    })),

  removeFlow: (uuid) =>
    set((state) => ({
      flows: state.flows.filter((f) => f.uuid !== uuid),
    })),

  setLoading: (isLoading) => set({ isLoading }),

  setSearchKeyword: (searchKeyword) => set({ searchKeyword }),

  setSortBy: (sortBy) => set({ sortBy }),

  setSortOrder: (sortOrder) => set({ sortOrder }),

  toggleFolder: (folderPath) =>
    set((state) => {
      const next = new Set(state.expandedFolders);
      if (next.has(folderPath)) {
        next.delete(folderPath);
      } else {
        next.add(folderPath);
      }
      return { expandedFolders: next };
    }),

  reset: () => set(initialState),

  // ===== Computed (Selectors) =====
  getFilteredFlows: () => {
    const { flows, searchKeyword, sortBy, sortOrder } = get();

    // 过滤
    let filtered = flows;
    if (searchKeyword.trim()) {
      const keyword = searchKeyword.toLowerCase();
      filtered = flows.filter((f) =>
        f.name.toLowerCase().includes(keyword)
      );
    }

    // 排序
    const sorted = [...filtered].sort((a, b) => {
      let comparison = 0;
      if (sortBy === 'name') {
        comparison = a.name.localeCompare(b.name);
      } else {
        comparison = a.updated_at - b.updated_at;
      }
      return sortOrder === 'asc' ? comparison : -comparison;
    });

    return sorted;
  },

  getFolders: () => {
    const { flows } = get();
    const folders = new Set<string>();
    for (const flow of flows) {
      if (flow.folder_path) {
        folders.add(flow.folder_path);
        // 添加所有父级路径
        const parts = flow.folder_path.split('/');
        for (let i = 1; i < parts.length; i++) {
          folders.add(parts.slice(0, i).join('/'));
        }
      }
    }
    return Array.from(folders).sort();
  },
}));

// ===== Selectors =====
/** 选择流程列表 */
export const selectFlows = (state: FlowListState) => state.flows;
/** 选择加载状态 */
export const selectIsLoading = (state: FlowListState) => state.isLoading;
/** 选择搜索关键词 */
export const selectSearchKeyword = (state: FlowListState) => state.searchKeyword;
