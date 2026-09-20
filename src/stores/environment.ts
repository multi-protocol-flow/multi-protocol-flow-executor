/**
 * 全局环境变量状态管理
 * 使用 Zustand 5 管理环境元数据列表与当前激活环境，支持本地缓存与测试隔离
 */

import { create } from 'zustand';
import type { EnvironmentMetadata, EnvironmentSet } from '@/types/environment';
import { environmentStorage } from '@/services/storage/environmentStorage';

const ACTIVE_ENV_STORAGE_KEY = 'mpe-active-environment';

function getStoredActiveEnvName(): string | null {
  try {
    return localStorage.getItem(ACTIVE_ENV_STORAGE_KEY);
  } catch {
    return null;
  }
}

function setStoredActiveEnvName(name: string | null): void {
  try {
    if (name) {
      localStorage.setItem(ACTIVE_ENV_STORAGE_KEY, name);
    } else {
      localStorage.removeItem(ACTIVE_ENV_STORAGE_KEY);
    }
  } catch {
    // ignore
  }
}

export interface EnvironmentState {
  /** 所有环境元数据列表 */
  environments: EnvironmentMetadata[];
  /** 当前激活的环境名称（null 表示无环境，仅使用流程内置变量） */
  activeEnvironmentName: string | null;
  /** 当前激活环境的数据缓存（变量集） */
  activeEnvironmentData: EnvironmentSet | null;
  /** 是否正在加载 */
  isLoading: boolean;
  /** 错误信息 */
  error: string | null;
}

export interface EnvironmentActions {
  /** 加载所有环境列表 */
  loadEnvironments: () => Promise<void>;
  /** 设置当前激活环境 */
  setActiveEnvironment: (name: string | null) => Promise<void>;
  /** 创建新环境 */
  createEnvironment: (name: string, initialVars?: Record<string, unknown>) => Promise<void>;
  /** 保存环境数据 */
  saveEnvironment: (env: EnvironmentSet, isLocal?: boolean) => Promise<void>;
  /** 删除环境（含本地私有覆盖） */
  deleteEnvironment: (name: string) => Promise<void>;
  /** 仅删除本地私有覆盖文件（保留主环境定义） */
  deleteEnvironmentLocalOverride: (name: string) => Promise<void>;
  /** 重命名环境 */
  renameEnvironment: (oldName: string, newName: string) => Promise<void>;
  /** 测试隔离重置 */
  reset: () => void;
}

export type EnvironmentStore = EnvironmentState & EnvironmentActions;

const initialState: EnvironmentState = {
  environments: [],
  activeEnvironmentName: getStoredActiveEnvName(),
  activeEnvironmentData: null,
  isLoading: false,
  error: null,
};

export const useEnvironmentStore = create<EnvironmentStore>((set, get) => ({
  ...initialState,

  loadEnvironments: async () => {
    set({ isLoading: true, error: null });
    try {
      const list = await environmentStorage.listEnvironments();
      const currentActive = get().activeEnvironmentName;
      let activeData: EnvironmentSet | null = null;

      // 若当前激活环境存在于列表中，重新拉取其完整变量数据
      if (currentActive && list.some((e) => e.name === currentActive)) {
        try {
          activeData = await environmentStorage.loadEnvironment(currentActive);
        } catch {
          activeData = null;
        }
      } else if (currentActive) {
        // 若之前激活的环境已被删除，清除激活项
        setStoredActiveEnvName(null);
        set({ activeEnvironmentName: null, activeEnvironmentData: null });
      }

      set({
        environments: list,
        activeEnvironmentData: activeData,
        isLoading: false,
      });
    } catch (e) {
      set({
        error: e instanceof Error ? e.message : String(e),
        isLoading: false,
      });
    }
  },

  setActiveEnvironment: async (name: string | null) => {
    if (!name) {
      setStoredActiveEnvName(null);
      set({ activeEnvironmentName: null, activeEnvironmentData: null });
      return;
    }

    // 先加载成功再落状态：加载失败（文件被删 / YAML 非法）时不能留下
    // 「已激活但无数据」的半成品状态，否则执行期会带着无效环境名跑到后端。
    try {
      const data = await environmentStorage.loadEnvironment(name);
      setStoredActiveEnvName(name);
      set({ activeEnvironmentName: name, activeEnvironmentData: data, error: null });
    } catch (e) {
      setStoredActiveEnvName(null);
      set({
        activeEnvironmentName: null,
        activeEnvironmentData: null,
        error: e instanceof Error ? e.message : String(e),
      });
      throw e;
    }
  },

  createEnvironment: async (name: string, initialVars?: Record<string, unknown>) => {
    const trimmed = name.trim();
    if (!trimmed) {
      throw new Error('Environment name cannot be empty');
    }
    // 保存为整体覆盖写入，重名创建会清空既有变量 → 必须显式拒绝
    if (get().environments.some((e) => e.name === trimmed)) {
      throw new Error(`Environment '${trimmed}' already exists`);
    }
    const newEnv: EnvironmentSet = {
      name: trimmed,
      variables: initialVars ?? {},
    };
    await environmentStorage.saveEnvironment(newEnv, false);
    await get().loadEnvironments();
  },

  saveEnvironment: async (env: EnvironmentSet, isLocal?: boolean) => {
    await environmentStorage.saveEnvironment(env, isLocal);
    // 若保存的是当前激活环境，同步刷新激活数据
    if (get().activeEnvironmentName === env.name) {
      try {
        const refreshed = await environmentStorage.loadEnvironment(env.name);
        set({ activeEnvironmentData: refreshed });
      } catch {
        // ignore
      }
    }
    await get().loadEnvironments();
  },

  deleteEnvironment: async (name: string) => {
    await environmentStorage.deleteEnvironment(name);
    if (get().activeEnvironmentName === name) {
      setStoredActiveEnvName(null);
      set({ activeEnvironmentName: null, activeEnvironmentData: null });
    }
    await get().loadEnvironments();
  },

  deleteEnvironmentLocalOverride: async (name: string) => {
    await environmentStorage.deleteEnvironmentLocalOverride(name);
    // 激活环境的有效变量集已变化，需要重新解析（去掉覆盖层）
    if (get().activeEnvironmentName === name) {
      try {
        const refreshed = await environmentStorage.loadEnvironment(name);
        set({ activeEnvironmentData: refreshed });
      } catch {
        set({ activeEnvironmentData: null });
      }
    }
    await get().loadEnvironments();
  },

  renameEnvironment: async (oldName: string, newName: string) => {
    const trimmed = newName.trim();
    if (!trimmed) {
      throw new Error('New environment name cannot be empty');
    }
    // 后端会对名称做字符清洗（如 ":" / "/" → "_"），必须采用清洗后的实际名称，
    // 否则激活态会指向一个列表中不存在的名字并被 loadEnvironments 立即清空。
    const meta = await environmentStorage.renameEnvironment(oldName, trimmed);
    if (get().activeEnvironmentName === oldName) {
      setStoredActiveEnvName(meta.name);
      set({ activeEnvironmentName: meta.name });
      try {
        const refreshed = await environmentStorage.loadEnvironment(meta.name);
        set({ activeEnvironmentData: refreshed });
      } catch {
        set({ activeEnvironmentData: null });
      }
    }
    await get().loadEnvironments();
  },

  reset: () => {
    set({
      environments: [],
      activeEnvironmentName: null,
      activeEnvironmentData: null,
      isLoading: false,
      error: null,
    });
    setStoredActiveEnvName(null);
  },
}));
