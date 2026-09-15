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
    // 如果文件路径完全一致，表示用户只是点击了同一个已经打开的文件，不是外部复制冲突
    const isSameFile =
      incomingFilePath && tab.filePath && incomingFilePath === tab.filePath;
    if (isSameFile) {
      continue;
    }

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

    // 情况 1：相同的 flow.uuid 但不同文件路径（例如文件系统直接复制），且可能有重复节点
    if (isSameFlowUuid) {
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

    // 情况 2：不同 flow.uuid，但存在跨流程重复的 node.uuid（历史复制遗留脏数据）
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
