/**
 * 流程存储服务
 * 通过 Tauri IPC 调用后端流程文件存储命令
 */

import { invoke } from '@tauri-apps/api/core';
import { getAppDataDir } from '@/services/tauri/settings';
import type { Flow } from '@/types/flow';

/** 流程元数据（用于列表展示） */
export interface FlowMetadata {
  uuid: string;
  name: string;
  filePath: string;
  updatedAt: number;
}

/** 内置模板元数据 */
export interface FlowMeta {
  version: number;
  isBuiltin: boolean;
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
    const flowsDir = await getFlowsDir();
    return invoke<FlowMetadata[]>('list_flow_files', { flowsDir });
  },

  /** 加载流程 */
  async loadFlow(uuid: string): Promise<Flow> {
    const flowsDir = await getFlowsDir();
    const flowFile = await invoke<FlowFileResponse>('load_flow_file', { uuid, flowsDir });
    // 将 _meta 附加到 flow 对象上（运行时属性，TypeScript 类型不包含）
    const flow = flowFile.flow as Flow & { _meta?: FlowMeta };
    if (flowFile._meta) {
      flow._meta = flowFile._meta;
    }
    return flow;
  },

  /** 保存流程 */
  async saveFlow(flow: Flow & { _meta?: FlowMeta }): Promise<void> {
    const flowsDir = await getFlowsDir();
    // 提取 _meta，其余作为 flow 发送
    const { _meta, ...flowData } = flow as Flow & { _meta?: FlowMeta };
    await invoke<void>('save_flow_file', { flow: flowData, flowsDir, meta: _meta ?? null });
  },

  /** 删除流程 */
  async deleteFlow(uuid: string): Promise<void> {
    const flowsDir = await getFlowsDir();
    await invoke<void>('delete_flow_file', { uuid, flowsDir });
  },

  /** 复制流程 */
  async copyFlow(uuid: string): Promise<Flow> {
    const flowsDir = await getFlowsDir();
    return invoke<Flow>('copy_flow_file', { uuid, flowsDir });
  },

  /** 刷新流程列表（与 listFlows 相同，便捷别名） */
  async refreshFlows(): Promise<FlowMetadata[]> {
    const flowsDir = await getFlowsDir();
    return invoke<FlowMetadata[]>('list_flow_files', { flowsDir });
  },
};
