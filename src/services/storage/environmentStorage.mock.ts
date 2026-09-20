/**
 * Mock 环境文件存储服务 - 用于浏览器环境或离线测试
 */

import type { EnvironmentMetadata, EnvironmentSet } from '@/types/environment';

const STORAGE_KEY = 'mpe-mock-environments';

interface StoredEnvRecord {
  base: EnvironmentSet;
  local?: EnvironmentSet;
  updatedAt: number;
}

function getStore(): Record<string, StoredEnvRecord> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function saveStore(store: Record<string, StoredEnvRecord>): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(store));
}

export const environmentStorageMock = {
  async listEnvironments(): Promise<EnvironmentMetadata[]> {
    const store = getStore();
    return Object.entries(store).map(([name, record]) => ({
      name,
      filePath: `browser://environments/${name}.yaml`,
      variableCount: Object.keys(record.base.variables).length,
      hasLocalOverride: !!record.local,
      updatedAt: record.updatedAt,
    })).sort((a, b) => a.name.localeCompare(b.name));
  },

  async loadEnvironment(name: string, isLocal?: boolean): Promise<EnvironmentSet> {
    const store = getStore();
    const record = store[name];
    if (!record) {
      throw new Error(`Environment '${name}' not found`);
    }
    if (isLocal === true) {
      if (!record.local) {
        throw new Error(`Environment '${name}.local' not found`);
      }
      return record.local;
    }
    if (isLocal === false) {
      return record.base;
    }
    return {
      name,
      description: record.base.description,
      variables: {
        ...record.base.variables,
        ...(record.local?.variables ?? {}),
      },
    };
  },

  async saveEnvironment(env: EnvironmentSet, isLocal?: boolean): Promise<void> {
    const store = getStore();
    const existing = store[env.name] ?? {
      base: { name: env.name, variables: {} },
      updatedAt: Date.now(),
    };
    if (isLocal) {
      existing.local = env;
    } else {
      existing.base = env;
    }
    existing.updatedAt = Date.now();
    store[env.name] = existing;
    saveStore(store);
  },

  async deleteEnvironment(name: string): Promise<void> {
    const store = getStore();
    if (!store[name]) {
      throw new Error(`Environment '${name}' not found`);
    }
    delete store[name];
    saveStore(store);
  },

  async deleteEnvironmentLocalOverride(name: string): Promise<void> {
    const store = getStore();
    const record = store[name];
    if (!record) {
      throw new Error(`Environment '${name}' not found`);
    }
    if (!record.local) {
      throw new Error(`Local override for environment '${name}' not found`);
    }
    delete record.local;
    record.updatedAt = Date.now();
    saveStore(store);
  },

  async renameEnvironment(oldName: string, newName: string): Promise<EnvironmentMetadata> {
    const store = getStore();
    const record = store[oldName];
    if (!record) {
      throw new Error(`Environment '${oldName}' not found`);
    }
    // 与后端一致：改名为自身是合法 no-op，不应报「已存在」
    if (newName !== oldName && store[newName]) {
      throw new Error(`Environment '${newName}' already exists`);
    }
    record.base.name = newName;
    if (record.local) {
      record.local.name = newName;
    }
    delete store[oldName];
    store[newName] = record;
    saveStore(store);
    return {
      name: newName,
      filePath: `browser://environments/${newName}.yaml`,
      variableCount: Object.keys(record.base.variables).length,
      hasLocalOverride: !!record.local,
      updatedAt: record.updatedAt,
    };
  },

  reset(): void {
    localStorage.removeItem(STORAGE_KEY);
  },
};
