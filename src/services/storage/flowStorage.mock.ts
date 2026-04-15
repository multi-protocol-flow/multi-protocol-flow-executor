/**
 * 流程存储 Mock 服务
 * 用于非 Tauri 环境的测试
 */

import type { Flow } from '@/types/flow';
import type { FlowMetadata, FlowMeta } from './flowStorage';

/** 带有 _meta 的流程存储结构 */
interface FlowWithMeta {
  flow: Flow;
  _meta?: FlowMeta;
}

// 内存存储
const flowStore = new Map<string, FlowWithMeta>();

/**
 * 保存流程
 * @param flow 流程数据（可能携带 _meta）
 * @param _flowsDir 流程存储目录（Mock 中忽略）
 */
export async function saveFlow(flow: Flow & { _meta?: FlowMeta }, _flowsDir?: string): Promise<void> {
  const { _meta, ...flowData } = flow as Flow & { _meta?: FlowMeta };
  flowStore.set(flow.uuid, { flow: flowData, _meta });
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
    throw new Error(`流程不存在: ${uuid}`);
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
    updatedAt: Date.now(),
  }));
}

/**
 * 删除流程
 * @param uuid 流程 UUID
 * @param _flowsDir 流程存储目录（Mock 中忽略）
 */
export async function deleteFlow(uuid: string, _flowsDir?: string): Promise<void> {
  if (!flowStore.has(uuid)) {
    throw new Error(`流程不存在: ${uuid}`);
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
    throw new Error(`流程不存在: ${uuid}`);
  }
  const newUuid = `${uuid}-copy-${Date.now()}`;
  const copy: Flow = {
    ...source.flow,
    uuid: newUuid,
    name: `${source.flow.name} (副本)`,
    nodes: [...source.flow.nodes],
    connections: [...source.flow.connections],
  };
  flowStore.set(newUuid, { flow: copy, _meta: source._meta });
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

/**
 * 清空所有流程（测试辅助函数）
 */
export function clearMockFlows(): void {
  flowStore.clear();
}

/**
 * 获取存储的流程数量（测试辅助函数）
 */
export function getMockFlowCount(): number {
  return flowStore.size;
}
