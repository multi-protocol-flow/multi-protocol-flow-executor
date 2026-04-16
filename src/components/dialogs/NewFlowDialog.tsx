/**
 * NewFlowDialog - 新建流程对话框组件
 * 用于创建新的流程（entry 节点由 createFlow 自动创建）
 */
import { useState, useEffect } from 'react';
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
import { flowStorage } from '@/services/storage';
import type { FolderInfo } from '@/services/storage';

export function NewFlowDialog() {
  const showNewFlowDialog = useUIStore((state) => state.showNewFlowDialog);
  const closeNewFlowDialog = useUIStore((state) => state.closeNewFlowDialog);
  const setFlow = useFlowStore((state) => state.setFlow);
  const addFlow = useFlowListStore((state) => state.addFlow);

  const [flowName, setFlowName] = useState('');
  const [selectedFolder, setSelectedFolder] = useState('');
  const [folders, setFolders] = useState<FolderInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  // 对话框打开时加载文件夹列表
  useEffect(() => {
    if (showNewFlowDialog) {
      flowStorage.listFolders().then(setFolders).catch(() => setFolders([]));
    }
  }, [showNewFlowDialog]);

  const handleClose = () => {
    setFlowName('');
    setSelectedFolder('');
    closeNewFlowDialog();
  };

  const handleCreate = async () => {
    if (!flowName.trim()) return;

    setIsLoading(true);
    try {
      // 1. 创建流程（createFlow 已包含 entry 节点）
      const newFlow = await createFlow(flowName.trim());

      // 2. 持久化流程到存储
      await flowStorage.saveFlow(newFlow);

      // 3. 如果选择了文件夹，移动流程到该文件夹
      if (selectedFolder) {
        await flowStorage.moveFlowToFolder(newFlow.uuid, selectedFolder);
      }

      // 4. 设置当前流程
      setFlow(newFlow);

      // 5. 添加到流程列表
      addFlow({
        uuid: newFlow.uuid,
        name: newFlow.name,
        updated_at: Date.now(),
        folder_path: selectedFolder,
      });

      // 6. 关闭对话框
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
        <div className="py-4 space-y-3">
          <Input
            placeholder="请输入流程名称"
            value={flowName}
            onChange={(e) => setFlowName(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleCreate()}
            disabled={isLoading}
            autoFocus
          />
          <div className="space-y-1.5">
            <label htmlFor="folder-select" className="text-sm text-muted-foreground">
              所属文件夹
            </label>
            <select
              id="folder-select"
              value={selectedFolder}
              onChange={(e) => setSelectedFolder(e.target.value)}
              disabled={isLoading}
              className="w-full h-9 rounded-md border border-border bg-background px-3 py-1 text-sm transition-colors focus:outline-none focus:ring-1 focus:ring-primary disabled:cursor-not-allowed disabled:opacity-50"
            >
              <option value="">根目录</option>
              {folders.map(folder => (
                <option key={folder.path} value={folder.path}>
                  {folder.name}
                </option>
              ))}
            </select>
          </div>
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
