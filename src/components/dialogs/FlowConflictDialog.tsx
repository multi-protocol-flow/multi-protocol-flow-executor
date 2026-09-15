/**
 * FlowConflictDialog - 流程/节点 UUID 冲突决策弹窗
 * 当从文件系统复制的副本文件或历史脏数据在多标签中打开时，由用户决策处理方式
 */
import { useState } from 'react';
import { AlertTriangle, Sparkles, ArrowRightCircle, Layers, Loader2 } from 'lucide-react';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { useUIStore, useFlowStore, selectFlowConflict } from '@/stores';
import { flowStorage } from '@/services/storage';
import { remapFlowUuids } from '@/lib/flowRemap';
import { getLL } from '@/i18n/i18n-lazy';

export function FlowConflictDialog() {
  const conflict = useUIStore(selectFlowConflict);
  const closeFlowConflictDialog = useUIStore((s) => s.closeFlowConflictDialog);
  const openTab = useFlowStore((s) => s.openTab);
  const switchTab = useFlowStore((s) => s.switchTab);
  const [isProcessing, setIsProcessing] = useState(false);

  if (!conflict) return null;

  const LL = getLL();
  const {
    incomingFlow,
    incomingFilePath,
    conflictingTabUuid,
    conflictingTabName,
    conflictType,
    duplicateNodeCount = 1,
  } = conflict;

  // 选项 1：全量重映射为全新独立流程
  const handleRemapAndOpen = async () => {
    setIsProcessing(true);
    try {
      const remappedFlow = remapFlowUuids(incomingFlow);
      if (incomingFilePath) {
        try {
          await flowStorage.saveFlow(remappedFlow);
        } catch (err) {
          console.warn('[FlowConflict] 保存重映射流程文件失败（非阻塞）:', err);
        }
      }
      openTab(remappedFlow, incomingFilePath);
      toast.success(LL.flow.conflict_remap_success());
      closeFlowConflictDialog();
    } catch (err) {
      console.error('[FlowConflict] 重映射失败:', err);
      toast.error(String(err));
    } finally {
      setIsProcessing(false);
    }
  };

  // 选项 2：直接切换至已打开的同 UUID 标签
  const handleSwitchToExisting = () => {
    switchTab(conflictingTabUuid);
    closeFlowConflictDialog();
  };

  // 选项 3：强制原样打开
  const handleForceOpen = () => {
    // 若 flow.uuid 碰撞，为避免 tab key 冲突，内存分配临时伪装识别
    if (conflictType === 'flow_uuid' || conflictType === 'both') {
      const clone = {
        ...incomingFlow,
        uuid: `${incomingFlow.uuid}#fork-${Date.now().toString(36)}`,
      };
      openTab(clone, incomingFilePath, { force: true });
    } else {
      openTab(incomingFlow, incomingFilePath, { force: true });
    }
    closeFlowConflictDialog();
  };

  // 冲突描述文案
  let conflictMessage = '';
  if (conflictType === 'flow_uuid') {
    conflictMessage = LL.flow.conflict_flow_uuid_msg({ name: conflictingTabName });
  } else if (conflictType === 'node_uuid') {
    conflictMessage = LL.flow.conflict_node_uuid_msg({
      name: conflictingTabName,
      count: duplicateNodeCount,
    });
  } else {
    conflictMessage = LL.flow.conflict_both_msg({
      name: conflictingTabName,
      count: duplicateNodeCount,
    });
  }

  return (
    <Dialog
      open={!!conflict}
      onOpenChange={(open) => {
        if (!open && !isProcessing) closeFlowConflictDialog();
      }}
    >
      <DialogContent className="max-w-[560px] w-[94vw] max-h-[90vh] flex flex-col p-6 overflow-hidden gap-0">
        <DialogHeader className="shrink-0 pb-3">
          <div className="flex items-center gap-2.5 text-amber-600 dark:text-amber-500">
            <AlertTriangle className="w-5 h-5 shrink-0" />
            <DialogTitle className="text-base font-semibold text-foreground">
              {LL.flow.conflict_dialog_title()}
            </DialogTitle>
          </div>
          <DialogDescription className="text-xs text-muted-foreground mt-1.5 leading-relaxed break-words">
            {LL.flow.conflict_dialog_desc()}
          </DialogDescription>
        </DialogHeader>

        {/* 可滚动的详情与选项内容区 */}
        <div className="flex-1 min-h-0 overflow-y-auto scrollbar-mini py-1 space-y-3 pr-1">
          {/* 冲突详情高亮卡片 */}
          <div className="p-3 rounded-lg bg-amber-500/10 border border-amber-500/30 text-xs text-amber-900 dark:text-amber-200 min-w-0">
            <p className="font-medium break-words leading-normal">{conflictMessage}</p>
            {incomingFilePath && (
              <p
                className="text-[11px] text-muted-foreground mt-1.5 break-all leading-normal"
                title={incomingFilePath}
              >
                {LL.flow.conflict_file_path({ path: incomingFilePath })}
              </p>
            )}
          </div>

          {/* 选项卡列表 */}
          <div className="space-y-2.5">
            {/* 方案 A：重映射为全新唯一流程 */}
            <button
              type="button"
              disabled={isProcessing}
              onClick={handleRemapAndOpen}
              className="flex items-start gap-3 w-full p-3 rounded-xl border-2 border-primary/40 hover:border-primary bg-primary/5 text-left transition-all cursor-pointer group min-w-0"
            >
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary/10 text-primary shrink-0 mt-0.5 group-hover:scale-105 transition-transform">
                {isProcessing ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <Sparkles className="w-4 h-4" />
                )}
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-xs font-semibold text-primary break-words">
                  {LL.flow.conflict_opt_remap_title()}
                </div>
                <p className="text-[11px] text-muted-foreground mt-0.5 leading-snug break-words">
                  {LL.flow.conflict_opt_remap_desc()}
                </p>
              </div>
            </button>

            {/* 方案 B：定位到已有流程（仅当 flow_uuid 相同时可用） */}
            {(conflictType === 'flow_uuid' || conflictType === 'both') && (
              <button
                type="button"
                disabled={isProcessing}
                onClick={handleSwitchToExisting}
                className="flex items-start gap-3 w-full p-3 rounded-xl border border-border hover:border-border/80 hover:bg-muted/40 text-left transition-all cursor-pointer min-w-0"
              >
                <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-muted text-muted-foreground shrink-0 mt-0.5">
                  <ArrowRightCircle className="w-4 h-4" />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-semibold text-foreground break-words">
                    {LL.flow.conflict_opt_switch_title()}
                  </div>
                  <p className="text-[11px] text-muted-foreground mt-0.5 leading-snug break-words">
                    {LL.flow.conflict_opt_switch_desc()}
                  </p>
                </div>
              </button>
            )}

            {/* 方案 C：强制原样打开 */}
            <button
              type="button"
              disabled={isProcessing}
              onClick={handleForceOpen}
              className="flex items-start gap-3 w-full p-3 rounded-xl border border-border/70 hover:border-border hover:bg-muted/30 text-left transition-all cursor-pointer min-w-0"
            >
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-muted/60 text-muted-foreground shrink-0 mt-0.5">
                <Layers className="w-4 h-4" />
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-xs font-medium text-foreground break-words">
                  {LL.flow.conflict_opt_force_title()}
                </div>
                <p className="text-[11px] text-muted-foreground mt-0.5 leading-snug break-words">
                  {LL.flow.conflict_opt_force_desc()}
                </p>
              </div>
            </button>
          </div>
        </div>

        <DialogFooter className="shrink-0 pt-3 mt-2 border-t border-border/40 flex justify-end">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={isProcessing}
            onClick={closeFlowConflictDialog}
            className="text-xs cursor-pointer"
          >
            {LL.common.CANCEL()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
