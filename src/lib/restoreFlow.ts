/**
 * 流程快照恢复功能
 * 从快照恢复流程到画布，处理 UUID 冲突和脏画布保护
 */

import { getLL } from '@/i18n/i18n-lazy';
import type { Flow } from '@/types';

const ll = getLL();

/** 恢复结果 */
export type RestoreResult =
  | { success: true; action: 'restored' | 'overwritten' | 'created_copy'; flow: Flow }
  | { success: false; reason: 'dirty_cancelled' | 'conflict_cancelled' | 'load_failed'; error?: string };

/** 恢复操作所需的依赖（注入以提升可测试性） */
export interface RestoreDeps {
  /** 当前画布是否有未保存的修改 */
  isDirty: boolean;
  /** 显示确认对话框，返回用户是否确认 */
  showConfirm: (config: {
    title: string;
    description: string;
    confirmText?: string;
    cancelText?: string;
  }) => Promise<boolean>;
  /** 列出已有流程 */
  listFlows: () => Promise<Array<{ uuid: string }>>;
  /** 保存流程 */
  saveFlow: (flow: Flow) => Promise<void>;
  /** 将流程加载到画布 */
  setFlow: (flow: Flow) => void;
  /** 生成新的 UUID */
  generateUuid: () => string;
  /** 消息提示 */
  toast: {
    success: (msg: string) => void;
    error: (msg: string) => void;
  };
}

/**
 * 从快照恢复流程到画布
 *
 * 逻辑流程：
 * 1. 脏画布检查 — 有未保存修改时弹窗确认
 * 2. UUID 冲突检查 — 已存在同 UUID 流程时提供覆盖/新建副本/取消选项
 * 3. 直接恢复 — 无冲突时直接保存并加载
 */
export async function restoreFlowFromSnapshot(
  snapshot: Flow,
  deps: RestoreDeps,
): Promise<RestoreResult> {
  const { isDirty, showConfirm, listFlows, saveFlow, setFlow, generateUuid, toast } = deps;

  // 1. 脏画布检查
  if (isDirty) {
    const confirmed = await showConfirm({
      title: 'Restore Flow',
      description: 'The current canvas has unsaved changes. Continue restoring?',
      confirmText: 'Continue Restore',
      cancelText: 'Cancel',
    });
    if (!confirmed) {
      toast.error(ll.execution.toast.restore_cancelled());
      return { success: false, reason: 'dirty_cancelled' };
    }
  }

  // 2. UUID 冲突检查
  const existingFlows = await listFlows();
  const hasConflict = existingFlows.some((f) => f.uuid === snapshot.uuid);

  if (hasConflict) {
    const overwrite = await showConfirm({
      title: 'UUID Conflict',
      description: 'A flow with the same UUID already exists. Please choose an action:',
      confirmText: 'Overwrite',
      cancelText: 'Create Copy',
    });

    if (overwrite) {
      // 覆盖已有流程
      try {
        await saveFlow(snapshot);
        setFlow(snapshot);
        toast.success(ll.execution.toast.restore_overwritten());
        return { success: true, action: 'overwritten', flow: snapshot };
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        toast.error(ll.execution.toast.restore_failed() + ': ' + errorMsg);
        return { success: false, reason: 'load_failed', error: errorMsg };
      }
    } else {
      // 新建副本
      const newUuid = generateUuid();
      const copiedFlow: Flow = {
        ...snapshot,
        uuid: newUuid,
        name: `${snapshot.name} (Restored Copy)`,
      };
      try {
        await saveFlow(copiedFlow);
        setFlow(copiedFlow);
        toast.success(ll.execution.toast.restore_copy_created());
        return { success: true, action: 'created_copy', flow: copiedFlow };
      } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        toast.error(ll.execution.toast.restore_failed() + ': ' + errorMsg);
        return { success: false, reason: 'load_failed', error: errorMsg };
      }
    }
  }

  // 3. 直接恢复（无冲突）
  try {
    await saveFlow(snapshot);
    setFlow(snapshot);
    toast.success(ll.execution.toast.restore_completed());
    return { success: true, action: 'restored', flow: snapshot };
  } catch (err) {
    const errorMsg = err instanceof Error ? err.message : String(err);
    toast.error(ll.execution.toast.restore_failed() + ': ' + errorMsg);
    return { success: false, reason: 'load_failed', error: errorMsg };
  }
}
