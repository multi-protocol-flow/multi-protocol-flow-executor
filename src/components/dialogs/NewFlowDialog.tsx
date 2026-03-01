/**
 * NewFlowDialog - 新建流程对话框组件
 * 用于创建新的流程（entry 节点由 createFlow 自动创建）
 */
import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useUIStore, useFlowStore, useFlowListStore } from '@/stores';
import { createFlow } from '@/services/tauri';

export function NewFlowDialog() {
  const showNewFlowDialog = useUIStore((state) => state.showNewFlowDialog);
  const closeNewFlowDialog = useUIStore((state) => state.closeNewFlowDialog);
  const setFlow = useFlowStore((state) => state.setFlow);
  const addFlow = useFlowListStore((state) => state.addFlow);

  const [flowName, setFlowName] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const handleClose = () => {
    setFlowName('');
    closeNewFlowDialog();
  };

  const handleCreate = async () => {
    if (!flowName.trim()) return;

    setIsLoading(true);
    try {
      // 1. 创建流程（createFlow 已包含 entry 节点）
      const newFlow = await createFlow(flowName.trim());

      // 2. 设置当前流程
      setFlow(newFlow);

      // 3. 添加到流程列表
      addFlow({
        uuid: newFlow.uuid,
        name: newFlow.name,
        updated_at: Date.now(),
      });

      // 4. 关闭对话框
      handleClose();
    } finally {
      setIsLoading(false);
    }
  };

  if (!showNewFlowDialog) return null;

  return (
    <Dialog open={showNewFlowDialog} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent className="w-full max-w-[425px]">
        <DialogHeader>
          <DialogTitle>新建流程</DialogTitle>
        </DialogHeader>
        <div className="py-4">
          <Input
            placeholder="请输入流程名称"
            value={flowName}
            onChange={(e) => setFlowName(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleCreate()}
            disabled={isLoading}
            autoFocus
          />
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={handleClose} disabled={isLoading}>
            取消
          </Button>
          <Button onClick={handleCreate} disabled={isLoading || !flowName.trim()}>
            {isLoading ? '创建中...' : '创建'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
