/**
 * 流程 UUID / 节点 UUID 冲突检测工具
 */
import type { Flow, FlowTab, FlowTabSession } from '@/types';
import type { FlowConflictState } from '@/stores/ui';

/**
 * 检查新打开的流程与当前工作区已打开的标签是否存在 UUID / Node UUID 冲突
 */
export function checkFlowConflict(
  incomingFlow: Flow,
  incomingFilePath?: string,
  tabs: FlowTab[] = [],
  tabSessions: Record<string, FlowTabSession> = {},
): FlowConflictState | null {
  const incomingNodeUuids = new Set((incomingFlow.nodes || []).map((n) => n.uuid));

  for (const tab of tabs) {
    const isSameFlowUuid = tab.uuid === incomingFlow.uuid;

    // 情况 1：相同的 flow.uuid
    // 仅当明确属于不同的两个文件路径（例如文件系统直接复制产生的副本）时，才构成外部文件冲突！
    // 若两者文件路径一致，或者任一方缺失文件路径（内存切换、首次加载内置模板或尚未落盘），均属于同一流程实例，绝不构成冲突。
    if (isSameFlowUuid) {
      const isDifferentFilePath = Boolean(
        incomingFilePath && tab.filePath && incomingFilePath !== tab.filePath,
      );
      if (isDifferentFilePath) {
        const session = tabSessions[tab.uuid];
        const existingFlow = session?.flow;
        let duplicateNodeCount = 0;
        if (existingFlow) {
          for (const node of existingFlow.nodes || []) {
            if (incomingNodeUuids.has(node.uuid)) {
              duplicateNodeCount++;
            }
          }
        }
        return {
          incomingFlow,
          incomingFilePath,
          conflictingTabUuid: tab.uuid,
          conflictingTabName: tab.name,
          conflictingFilePath: tab.filePath,
          conflictType: duplicateNodeCount > 0 ? 'both' : 'flow_uuid',
          duplicateNodeCount: duplicateNodeCount > 0 ? duplicateNodeCount : undefined,
        };
      }
      // 同一流程实例，无需比对节点，直接检查下一个标签
      continue;
    }

    // 情况 2：不同 flow.uuid，但存在跨流程重复的 node.uuid（历史复制遗留脏数据）
    const session = tabSessions[tab.uuid];
    const existingFlow = session?.flow;
    let duplicateNodeCount = 0;
    if (existingFlow) {
      for (const node of existingFlow.nodes || []) {
        if (incomingNodeUuids.has(node.uuid)) {
          duplicateNodeCount++;
        }
      }
    }

    if (duplicateNodeCount > 0) {
      return {
        incomingFlow,
        incomingFilePath,
        conflictingTabUuid: tab.uuid,
        conflictingTabName: tab.name,
        conflictingFilePath: tab.filePath,
        conflictType: 'node_uuid',
        duplicateNodeCount,
      };
    }
  }

  return null;
}
