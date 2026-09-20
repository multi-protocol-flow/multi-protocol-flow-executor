/**
 * 环境文件存储服务
 * 通过 Tauri IPC 调用后端环境文件存储命令
 *
 * 注意：生效的环境目录由后端按设置（`environmentsDir` → `<flows_dir>/environments`）解析，
 * 前端不传递目录参数，避免 IPC 层被用来读写任意路径。
 */

import { invoke } from '@tauri-apps/api/core';
import type { EnvironmentMetadata, EnvironmentSet } from '@/types/environment';
import { isTauriEnv } from '@/lib/platform';

export const environmentStorage = {
  /** 列出所有环境（仅元数据） */
  async listEnvironments(): Promise<EnvironmentMetadata[]> {
    if (isTauriEnv()) {
      return invoke<EnvironmentMetadata[]>('list_environment_files');
    }
    const { environmentStorageMock } = await import('./environmentStorage.mock');
    return environmentStorageMock.listEnvironments();
  },

  /** 加载环境（合并本地覆盖或加载原始文件） */
  async loadEnvironment(name: string, isLocal?: boolean): Promise<EnvironmentSet> {
    if (isTauriEnv()) {
      return invoke<EnvironmentSet>('load_environment_file', { name, isLocal });
    }
    const { environmentStorageMock } = await import('./environmentStorage.mock');
    return environmentStorageMock.loadEnvironment(name, isLocal);
  },

  /** 保存环境 */
  async saveEnvironment(env: EnvironmentSet, isLocal?: boolean): Promise<void> {
    if (isTauriEnv()) {
      return invoke<void>('save_environment_file', { env, isLocal });
    }
    const { environmentStorageMock } = await import('./environmentStorage.mock');
    return environmentStorageMock.saveEnvironment(env, isLocal);
  },

  /** 删除环境（同时删除关联的 .local 覆盖） */
  async deleteEnvironment(name: string): Promise<void> {
    if (isTauriEnv()) {
      return invoke<void>('delete_environment_file', { name });
    }
    const { environmentStorageMock } = await import('./environmentStorage.mock');
    return environmentStorageMock.deleteEnvironment(name);
  },

  /** 仅删除本地私有覆盖文件（保留主环境定义） */
  async deleteEnvironmentLocalOverride(name: string): Promise<void> {
    if (isTauriEnv()) {
      return invoke<void>('delete_environment_local_override', { name });
    }
    const { environmentStorageMock } = await import('./environmentStorage.mock');
    return environmentStorageMock.deleteEnvironmentLocalOverride(name);
  },

  /** 重命名环境 */
  async renameEnvironment(oldName: string, newName: string): Promise<EnvironmentMetadata> {
    if (isTauriEnv()) {
      return invoke<EnvironmentMetadata>('rename_environment_file', { oldName, newName });
    }
    const { environmentStorageMock } = await import('./environmentStorage.mock');
    return environmentStorageMock.renameEnvironment(oldName, newName);
  },

  /** 获取生效的环境目录绝对路径 */
  async getEffectiveEnvironmentsDir(): Promise<string> {
    if (isTauriEnv()) {
      return invoke<string>('get_environments_dir');
    }
    return 'browser://environments';
  },
};
