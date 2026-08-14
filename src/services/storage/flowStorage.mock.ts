/**
 * 流程存储 Mock 服务
 * 用于非 Tauri 环境的测试
 */

import type { Flow } from '@/types/flow';
import type { FlowMetadata, FlowMeta, FolderInfo } from './flowStorage';

/** 带有 _meta 的流程存储结构 */
interface FlowWithMeta {
  flow: Flow;
  _meta?: FlowMeta;
  /** 所属文件夹路径（模拟） */
  folderPath: string;
  /** 最后保存时间（保存时固化，模拟文件 mtime） */
  updatedAt: number;
}

// 内存存储
const flowStore = new Map<string, FlowWithMeta>();

// 模拟文件夹存储
const folderStore = new Set<string>();

/**
 * 保存流程
 * @param flow 流程数据（可能携带 _meta）
 * @param _flowsDir 流程存储目录（Mock 中忽略）
 */
export async function saveFlow(flow: Flow & { _meta?: FlowMeta }, _flowsDir?: string): Promise<void> {
  const { _meta, ...flowData } = flow as Flow & { _meta?: FlowMeta };
  flowStore.set(flow.uuid, { flow: flowData, _meta, folderPath: '', updatedAt: Date.now() });
}

/**
 * 加载流程
 * @param uuid 流程 UUID
 * @param _flowsDir 流程存储目录（Mock 中忽略）
 * @returns 流程数据（运行时可能携带 _meta）
 */
export async function loadFlow(uuid: string, _flowsDir?: string): Promise<Flow> {
  const entry = flowStore.get(uuid);
  if (!entry) {
    throw new Error(`Flow not found: ${uuid}`);
  }
  // 将 _meta 附加到 flow 对象上
  const flow = { ...entry.flow } as Flow & { _meta?: FlowMeta };
  if (entry._meta) {
    flow._meta = entry._meta;
  }
  return flow;
}

/**
 * 列出所有流程元数据
 * @param _flowsDir 流程存储目录（Mock 中忽略）
 * @returns 流程元数据列表
 */
export async function listFlows(_flowsDir?: string): Promise<FlowMetadata[]> {
  return Array.from(flowStore.values()).map((entry) => ({
    uuid: entry.flow.uuid,
    name: entry.flow.name,
    filePath: `mock://flows/${entry.flow.uuid}.json`,
    updatedAt: entry.updatedAt,
    folderPath: entry.folderPath,
  }));
}

/**
 * 删除流程
 * @param uuid 流程 UUID
 * @param _flowsDir 流程存储目录（Mock 中忽略）
 */
export async function deleteFlow(uuid: string, _flowsDir?: string): Promise<void> {
  if (!flowStore.has(uuid)) {
    throw new Error(`Flow not found: ${uuid}`);
  }
  flowStore.delete(uuid);
}

/**
 * 复制流程
 * @param uuid 源流程 UUID
 * @param _flowsDir 流程存储目录（Mock 中忽略）
 * @returns 复制后的新流程
 */
export async function copyFlow(uuid: string, _flowsDir?: string): Promise<Flow> {
  const source = flowStore.get(uuid);
  if (!source) {
    throw new Error(`Flow not found: ${uuid}`);
  }
  const newUuid = `${uuid}-copy-${Date.now()}`;
  const copy: Flow = {
    ...source.flow,
    uuid: newUuid,
    name: `${source.flow.name} (副本)`,
    nodes: [...source.flow.nodes],
    connections: [...source.flow.connections],
  };
  flowStore.set(newUuid, { flow: copy, _meta: source._meta, folderPath: source.folderPath, updatedAt: Date.now() });
  return copy;
}

/**
 * 刷新流程列表（与 listFlows 相同）
 * @param _flowsDir 流程存储目录（Mock 中忽略）
 * @returns 流程元数据列表
 */
export async function refreshFlows(_flowsDir?: string): Promise<FlowMetadata[]> {
  return listFlows(_flowsDir);
}

// ===== 文件夹操作（Mock） =====

/**
 * 列出所有文件夹
 */
export async function listFolders(_flowsDir?: string): Promise<FolderInfo[]> {
  return Array.from(folderStore).map((path) => ({
    name: path.split('/').pop() ?? path,
    path,
  }));
}

/**
 * 创建文件夹
 */
export async function createFolder(folderPath: string, _flowsDir?: string): Promise<FolderInfo> {
  folderStore.add(folderPath);
  return {
    name: folderPath.split('/').pop() ?? folderPath,
    path: folderPath,
  };
}

/**
 * 删除文件夹
 */
export async function deleteFolder(folderPath: string, _flowsDir?: string): Promise<void> {
  folderStore.delete(folderPath);
}

/**
 * 重命名文件夹
 */
export async function renameFolder(folderPath: string, newName: string, _flowsDir?: string): Promise<FolderInfo> {
  folderStore.delete(folderPath);
  const parts = folderPath.split('/');
  parts[parts.length - 1] = newName;
  const newPath = parts.join('/');
  folderStore.add(newPath);
  return { name: newName, path: newPath };
}

/**
 * 移动流程到指定文件夹
 */
export async function moveFlowToFolder(uuid: string, targetFolderPath: string, _flowsDir?: string): Promise<void> {
  const entry = flowStore.get(uuid);
  if (entry) {
    entry.folderPath = targetFolderPath;
  }
}

/**
 * 移动文件夹到另一个文件夹（或根目录）
 */
export async function moveFolder(sourceFolderPath: string, targetFolderPath: string, _flowsDir?: string): Promise<void> {
  // 查找所有以 sourceFolderPath 开头的路径
  const pathsToMove = Array.from(folderStore).filter(p => p === sourceFolderPath || p.startsWith(sourceFolderPath + '/'));
  // 删除旧路径
  for (const p of pathsToMove) {
    folderStore.delete(p);
  }
  // 计算新路径并添加
  const folderName = sourceFolderPath.split('/').pop() ?? sourceFolderPath;
  const newBase = targetFolderPath ? `${targetFolderPath}/${folderName}` : folderName;
  folderStore.add(newBase);
  for (const p of pathsToMove) {
    if (p !== sourceFolderPath) {
      const relativePath = p.slice(sourceFolderPath.length);
      folderStore.add(`${newBase}${relativePath}`);
    }
  }
  // 同时更新相关流程的 folderPath
  for (const [, entry] of flowStore) {
    if (entry.folderPath === sourceFolderPath) {
      entry.folderPath = newBase;
    } else if (entry.folderPath.startsWith(sourceFolderPath + '/')) {
      entry.folderPath = `${newBase}${entry.folderPath.slice(sourceFolderPath.length)}`;
    }
  }
}

/**
 * 清空所有流程（测试辅助函数）
 */
export function clearMockFlows(): void {
  flowStore.clear();
  folderStore.clear();
}

/**
 * 获取存储的流程数量（测试辅助函数）
 */
export function getMockFlowCount(): number {
  return flowStore.size;
}
