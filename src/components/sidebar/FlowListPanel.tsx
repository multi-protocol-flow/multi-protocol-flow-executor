/**
 * FlowListPanel - 流程列表面板
 * 显示已保存的流程列表，支持加载流程
 */
import { useEffect } from 'react';
import { Plus, FileJson, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useFlowListStore, useFlowStore, useUIStore } from '@/stores';
import { listFlows, loadFlow, deleteFlow } from '@/services/storage/flowStorage';
import { cn } from '@/lib/utils';

/** 格式化时间为相对时间 */
function formatRelativeTime(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);

  if (minutes < 1) return '刚刚';
  if (minutes < 60) return `${minutes} 分钟前`;
  if (hours < 24) return `${hours} 小时前`;
  if (days < 7) return `${days} 天前`;
  return new Date(timestamp).toLocaleDateString('zh-CN');
}

export function FlowListPanel() {
  const { setFlows, removeFlow: removeFlowFromList } = useFlowListStore();
  const { setFlow, flow: currentFlow } = useFlowStore();
  const { openNewFlowDialog } = useUIStore();

  // 加载流程列表
  useEffect(() => {
    const flows = listFlows();
    setFlows(flows);
  }, [setFlows]);

  const handleSelectFlow = (uuid: string) => {
    const flow = loadFlow(uuid);
    if (flow) {
      setFlow(flow);
    }
  };

  const handleDeleteFlow = (e: React.MouseEvent, uuid: string) => {
    e.stopPropagation();
    if (window.confirm('确定要删除此流程吗？')) {
      deleteFlow(uuid);
      removeFlowFromList(uuid);
      // 如果删除的是当前流程，清空画布
      if (currentFlow?.uuid === uuid) {
        setFlow(null);
      }
    }
  };

  const handleNewFlow = () => {
    openNewFlowDialog();
  };

  const flows = useFlowListStore((state) => state.flows);

  return (
    <div className="flex flex-col h-52 border-b border-border">
      {/* 标题栏 */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <span className="text-sm font-medium text-black">流程列表</span>
        <Button
          variant="ghost"
          size="icon"
          onClick={handleNewFlow}
          className="h-7 w-7 rounded-md bg-white border border-black hover:bg-slate-50"
          title="新建流程"
        >
          <Plus className="w-4 h-4 text-black" />
        </Button>
      </div>

      {/* 流程列表 */}
      <ScrollArea className="flex-1">
        <div className="p-1.5 space-y-0.5">
          {flows.map((flowMeta) => {
            const isSelected = currentFlow?.uuid === flowMeta.uuid;

            return (
              <div
                type="button"
                key={flowMeta.uuid}
                onClick={() => handleSelectFlow(flowMeta.uuid)}
                className={cn(
                  'group w-full flex items-center gap-2 px-3 h-12 rounded-lg text-sm text-left transition-all duration-150',
                  'bg-white border border-black',
                  'hover:bg-slate-50',
                  isSelected && 'bg-white border-black',
                  !isSelected && 'text-slate-600'
                )}
              >
                <FileJson
                  className={cn(
                    'w-4 h-4 shrink-0 transition-colors',
                    isSelected ? 'text-emerald-500' : 'text-slate-400'
                  )}
                />
                <div className="flex-1 min-w-0">
                  <p className="truncate font-medium">{flowMeta.name}</p>
                  <p className="text-xs text-slate-400 truncate">
                    {formatRelativeTime(flowMeta.updated_at)}
                  </p>
                </div>

                {/* 删除按钮 */}
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={(e) => handleDeleteFlow(e, flowMeta.uuid)}
                  className={cn(
                    'h-6 w-6 shrink-0 transition-opacity duration-150',
                    'opacity-0 group-hover:opacity-100',
                    'hover:bg-red-50 hover:text-red-500'
                  )}
                  title="删除流程"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </Button>
              </div>
            );
          })}

          {flows.length === 0 && (
            <div className="flex flex-col items-center justify-center py-6 text-slate-400">
              <FileJson className="w-8 h-8 mb-2 opacity-40" />
              <p className="text-sm">暂无流程</p>
              <p className="text-xs mt-1 opacity-70">点击 + 创建新流程</p>
            </div>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
