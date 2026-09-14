/**
 * 流程存储服务
 * 通过 Tauri IPC 调用后端流程文件存储命令
 */

import { invoke } from '@tauri-apps/api/core';
import { getAppDataDir } from '@/services/tauri/settings';
import type { Flow } from '@/types/flow';
import { isTauriEnv } from '@/lib/platform';
import { notifyError } from '@/lib/notify';
import { getLL } from '@/i18n/i18n-lazy';
export interface FlowMetadata {
  uuid: string;
  name: string;
  filePath: string;
  updatedAt: number;
  /** 所属文件夹相对路径（相对于 flows_dir，空字符串表示根目录） */
  folderPath: string;
}

/** 内置模板元数据 */
export interface FlowMeta {
  version: number;
  isBuiltin: boolean;
}

/** 文件夹信息 */
export interface FolderInfo {
  /** 文件夹名称 */
  name: string;
  /** 文件夹相对路径 */
  path: string;
}

/** 后端 FlowFile 结构（包含 _version, flow, _meta） */
interface FlowFileResponse {
  _version: number;
  flow: Flow;
  _meta?: FlowMeta;
}

/**
 * 获取流程存储目录
 * 优先使用全局设置中的 flowsDir，未设置时默认为 {appDataDir}/flows/
 */
async function getFlowsDir(): Promise<string> {
  // 尝试从 Settings Store 缓存中读取自定义目录
  const { useSettingsStore } = await import('@/stores/settings');
  const cachedFlowsDir = useSettingsStore.getState().flowsDir;
  if (cachedFlowsDir) {
    return cachedFlowsDir;
  }
  // 未设置时使用默认目录
  const appDataDir = await getAppDataDir();
  return `${appDataDir}/flows/`;
}

export const flowStorage = {
  /** 列出所有流程（仅元数据） */
  async listFlows(): Promise<FlowMetadata[]> {
    if (isTauriEnv()) {
      const flowsDir = await getFlowsDir();
      return await invoke<FlowMetadata[]>('list_flow_files', { flowsDir });
    }
    const { listFlows: mock } = await import('./flowStorage.mock');
    return await mock();
  },

  /** 加载流程 */
  async loadFlow(uuid: string): Promise<Flow> {
    if (isTauriEnv()) {
      const flowsDir = await getFlowsDir();
      const flowFile = await invoke<FlowFileResponse>('load_flow_file', { uuid, flowsDir });
      // 将 _meta 附加到 flow 对象上（运行时属性，TypeScript 类型不包含）
      const flow = flowFile.flow as Flow & { _meta?: FlowMeta };
      if (flowFile._meta) {
        flow._meta = flowFile._meta;
      }
      return flow;
    }
    const { loadFlow: mock } = await import('./flowStorage.mock');
    return await mock(uuid);
  },

  /** 保存流程 */
  async saveFlow(flow: Flow & { _meta?: FlowMeta }): Promise<void> {
    try {
      if (isTauriEnv()) {
        const flowsDir = await getFlowsDir();
        // 提取 _meta，其余作为 flow 发送
        const { _meta, ...flowData } = flow as Flow & { _meta?: FlowMeta };
        await invoke<void>('save_flow_file', { flow: flowData, flowsDir, meta: _meta ?? null });
        return;
      }
      const { saveFlow: mock } = await import('./flowStorage.mock');
      return await mock(flow);
    } catch (err) {
      notifyError(getLL().flow.save_flow_failed?.() ?? 'Failed to save flow', err, {
        category: 'FlowStorage',
        flowUuid: flow.uuid,
      });
      throw err;
    }
  },

  /** 删除流程 */
  async deleteFlow(uuid: string): Promise<void> {
    try {
      if (isTauriEnv()) {
        const flowsDir = await getFlowsDir();
        await invoke<void>('delete_flow_file', { uuid, flowsDir });
        return;
      }
      const { deleteFlow: mock } = await import('./flowStorage.mock');
      return await mock(uuid);
    } catch (err) {
      notifyError(getLL().flow.delete_flow_failed?.() ?? 'Failed to delete flow', err, {
        category: 'FlowStorage',
        flowUuid: uuid,
      });
      throw err;
    }
  },

  /** 复制流程 */
  async copyFlow(uuid: string): Promise<Flow> {
    try {
      if (isTauriEnv()) {
        const flowsDir = await getFlowsDir();
        return await invoke<Flow>('copy_flow_file', { uuid, flowsDir });
      }
      const { copyFlow: mock } = await import('./flowStorage.mock');
      return await mock(uuid);
    } catch (err) {
      notifyError(getLL().flow.copy_flow_failed?.() ?? 'Failed to copy flow', err, {
        category: 'FlowStorage',
        flowUuid: uuid,
      });
      throw err;
    }
  },

  /** 重命名流程 */
  async renameFlow(uuidOrPath: string, newName: string): Promise<FlowMetadata> {
    try {
      if (isTauriEnv()) {
        const flowsDir = await getFlowsDir();
        return await invoke<FlowMetadata>('rename_flow_file', {
          uuid: uuidOrPath,
          newName,
          flowsDir,
        });
      }
      const fullFlow = await this.loadFlow(uuidOrPath);
      fullFlow.name = newName;
      await this.saveFlow(fullFlow);
      return {
        uuid: uuidOrPath,
        name: newName,
        filePath: `mock://flows/${uuidOrPath}.json`,
        updatedAt: Date.now(),
        folderPath: '',
      };
    } catch (err) {
      notifyError(getLL().flow.save_flow_failed?.() ?? 'Failed to rename flow', err, {
        category: 'FlowStorage',
        flowUuid: uuidOrPath,
      });
      throw err;
    }
  },

  /** 刷新流程列表（与 listFlows 相同，便捷别名） */
  async refreshFlows(): Promise<FlowMetadata[]> {
    if (isTauriEnv()) {
      const flowsDir = await getFlowsDir();
      return invoke<FlowMetadata[]>('list_flow_files', { flowsDir });
    }
    const { refreshFlows: mock } = await import('./flowStorage.mock');
    return mock();
  },

  // ===== 文件夹操作 =====

  /** 列出所有文件夹 */
  async listFolders(): Promise<FolderInfo[]> {
    if (isTauriEnv()) {
      const flowsDir = await getFlowsDir();
      return invoke<FolderInfo[]>('list_folders', { flowsDir });
    }
    const { listFolders: mock } = await import('./flowStorage.mock');
    return mock();
  },

  /** 创建文件夹 */
  async createFolder(folderPath: string): Promise<FolderInfo> {
    try {
      if (isTauriEnv()) {
        const flowsDir = await getFlowsDir();
        return await invoke<FolderInfo>('create_folder', { flowsDir, folderPath });
      }
      const { createFolder: mock } = await import('./flowStorage.mock');
      return await mock(folderPath);
    } catch (err) {
      notifyError(getLL().flow.create_folder_failed?.() ?? 'Failed to create folder', err, {
        category: 'FlowStorage',
      });
      throw err;
    }
  },

  /** 删除文件夹（仅空文件夹） */
  async deleteFolder(folderPath: string): Promise<void> {
    try {
      if (isTauriEnv()) {
        const flowsDir = await getFlowsDir();
        await invoke<void>('delete_folder', { flowsDir, folderPath });
        return;
      }
      const { deleteFolder: mock } = await import('./flowStorage.mock');
      return await mock(folderPath);
    } catch (err) {
      notifyError(getLL().flow.delete_folder_failed?.() ?? 'Failed to delete folder', err, {
        category: 'FlowStorage',
      });
      throw err;
    }
  },

  /** 重命名文件夹 */
  async renameFolder(folderPath: string, newName: string): Promise<FolderInfo> {
    try {
      if (isTauriEnv()) {
        const flowsDir = await getFlowsDir();
        return await invoke<FolderInfo>('rename_folder', { flowsDir, folderPath, newName });
      }
      const { renameFolder: mock } = await import('./flowStorage.mock');
      return await mock(folderPath, newName);
    } catch (err) {
      notifyError(getLL().flow.rename_folder_failed?.() ?? 'Failed to rename folder', err, {
        category: 'FlowStorage',
      });
      throw err;
    }
  },

  /** 移动流程到指定文件夹 */
  async moveFlowToFolder(uuid: string, targetFolderPath: string): Promise<void> {
    try {
      if (isTauriEnv()) {
        const flowsDir = await getFlowsDir();
        await invoke<void>('move_flow_to_folder', { uuid, flowsDir, targetFolderPath });
        return;
      }
      const { moveFlowToFolder: mock } = await import('./flowStorage.mock');
      return await mock(uuid, targetFolderPath);
    } catch (err) {
      notifyError(getLL().flow.move_flow_failed?.() ?? 'Failed to move flow', err, {
        category: 'FlowStorage',
        flowUuid: uuid,
      });
      throw err;
    }
  },

  /** 移动文件夹到另一个文件夹（或根目录） */
  async moveFolder(sourceFolderPath: string, targetFolderPath: string): Promise<void> {
    try {
      if (isTauriEnv()) {
        const flowsDir = await getFlowsDir();
        await invoke<void>('move_folder', { flowsDir, sourceFolderPath, targetFolderPath });
        return;
      }
      const { moveFolder: mock } = await import('./flowStorage.mock');
      return await mock(sourceFolderPath, targetFolderPath);
    } catch (err) {
      notifyError(getLL().flow.move_folder_failed?.() ?? 'Failed to move folder', err, {
        category: 'FlowStorage',
      });
      throw err;
    }
  },
};
