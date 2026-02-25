/**
 * 流程存储服务
 * 使用 localStorage 存储流程数据
 */

import type { Flow } from '@/types';

const STORAGE_KEYS = {
  FLOWS: 'mpe_flows',
  FLOW_LIST: 'mpe_flow_list',
  VERSION: 'mpe_version',
};

/** 当前数据版本 */
const CURRENT_VERSION = 1;

/** 流程元数据（用于列表展示） */
export interface FlowMetadata {
  uuid: string;
  name: string;
  updated_at: number;
}

/** 流程存储结构 */
interface FlowStorage {
  version: number;
  flows: Record<string, Flow>;
}

/**
 * 加载流程存储
 */
function loadFlowStorage(): FlowStorage {
  const json = localStorage.getItem(STORAGE_KEYS.FLOWS);
  if (json) {
    try {
      const storage = JSON.parse(json) as FlowStorage;
      // 版本检查，便于未来迁移
      if (storage.version === CURRENT_VERSION) {
        return storage;
      }
      // 未来可在此处理版本迁移
    } catch {
      // 解析失败，返回空存储
    }
  }
  return { version: CURRENT_VERSION, flows: {} };
}

/**
 * 更新流程列表
 */
function updateFlowList(flow: Flow): void {
  const list = listFlows();
  const index = list.findIndex((f) => f.uuid === flow.uuid);
  const metadata: FlowMetadata = {
    uuid: flow.uuid,
    name: flow.name,
    updated_at: Date.now(),
  };
  if (index >= 0) {
    list[index] = metadata;
  } else {
    list.push(metadata);
  }
  localStorage.setItem(STORAGE_KEYS.FLOW_LIST, JSON.stringify(list));
}

/**
 * 保存流程
 * @param flow 流程数据
 */
export function saveFlow(flow: Flow): void {
  const storage = loadFlowStorage();
  storage.flows[flow.uuid] = flow;
  localStorage.setItem(STORAGE_KEYS.FLOWS, JSON.stringify(storage));
  updateFlowList(flow);
}

/**
 * 加载流程
 * @param uuid 流程 UUID
 * @returns 流程数据，不存在返回 null
 */
export function loadFlow(uuid: string): Flow | null {
  const storage = loadFlowStorage();
  return storage.flows[uuid] || null;
}

/**
 * 列出所有流程（仅元数据）
 * @returns 流程元数据列表
 */
export function listFlows(): FlowMetadata[] {
  const json = localStorage.getItem(STORAGE_KEYS.FLOW_LIST);
  if (json) {
    try {
      return JSON.parse(json) as FlowMetadata[];
    } catch {
      return [];
    }
  }
  return [];
}

/**
 * 删除流程
 * @param uuid 流程 UUID
 */
export function deleteFlow(uuid: string): void {
  const storage = loadFlowStorage();
  delete storage.flows[uuid];
  localStorage.setItem(STORAGE_KEYS.FLOWS, JSON.stringify(storage));
  // 更新列表
  const list = listFlows().filter((f) => f.uuid !== uuid);
  localStorage.setItem(STORAGE_KEYS.FLOW_LIST, JSON.stringify(list));
}

/**
 * 清空所有流程数据
 */
export function clearFlows(): void {
  localStorage.removeItem(STORAGE_KEYS.FLOWS);
  localStorage.removeItem(STORAGE_KEYS.FLOW_LIST);
}
