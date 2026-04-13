/**
 * FlowListPanel - 流程列表面板
 * 显示已保存的流程列表，支持加载流程
 */
import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { Plus, FileJson, MoreVertical, Copy, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useFlowListStore, useFlowStore, useUIStore } from '@/stores';
import { listFlows, loadFlow, deleteFlow, saveFlow } from '@/services/storage/flowStorage';
import { DEMO_FLOW, WS_DEMO_FLOW, BUILTIN_FLOW_VERSION } from '@/templates/builtinFlows';
import { cn } from '@/lib/utils';
import type { Flow } from '@/types/flow';

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

export function FlowListPanel({ showHeader = true }: { showHeader?: boolean }) {
  const { setFlows, addFlow: addFlowToList, removeFlow: removeFlowFromList } = useFlowListStore();
  const { setFlow, flow: currentFlow, isDirty, markSaved } = useFlowStore();
  const { openNewFlowDialog, showConfirm } = useUIStore();

  // 加载流程列表 + 自动更新内置模板
  useEffect(() => {
    const flowMetas = listFlows();
    
    if (flowMetas.length === 0) {
      // 首次启动：加载模板
      saveFlow(DEMO_FLOW);
      saveFlow(WS_DEMO_FLOW);
      setFlow(DEMO_FLOW);
      setFlows([{
          uuid: DEMO_FLOW.uuid,
          name: DEMO_FLOW.name,
          updated_at: Date.now()
        },
        {
          uuid: WS_DEMO_FLOW.uuid,
          name: WS_DEMO_FLOW.name,
          updated_at: Date.now()
        }
      ]);
    } else {
      setFlows(flowMetas);
      
      // 检查内置模板是否需要更新
      const updateBuiltinFlow = (flow: Flow) => {
        const existingFlow = loadFlow(flow.uuid);
        if (existingFlow) {
          const existingVersion = (existingFlow as Flow & { _meta?: { version: number } })._meta?.version || 0;
          if (existingVersion < BUILTIN_FLOW_VERSION) {
            // 版本过期，更新内置模板
            saveFlow(flow);
            // 如果当前正在查看该流程，刷新显示
            if (currentFlow?.uuid === flow.uuid) {
              setFlow(flow);
            }
          }
        }
      };
      
      updateBuiltinFlow(DEMO_FLOW);
      updateBuiltinFlow(WS_DEMO_FLOW);
    }
  }, [setFlows, setFlow, currentFlow?.uuid]);

  const handleSelectFlow = (uuid: string) => {
    // 切换流程前，自动保存当前流程（如果有未保存的修改）
    if (currentFlow && isDirty) {
      saveFlow(currentFlow);
      markSaved();
      // 更新流程列表中的时间戳
      setFlows(listFlows());
    }

    const flow = loadFlow(uuid);
    if (flow) {
      setFlow(flow);
    }
  };

  const [openMenuUuid, setOpenMenuUuid] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [menuPosition, setMenuPosition] = useState<{ top: number; left: number }>({ top: 0, left: 0 });

  // 点击外部关闭菜单
  useEffect(() => {
    if (openMenuUuid === null) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpenMenuUuid(null);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [openMenuUuid]);

  const handleDeleteFlow = (uuid: string) => {
    setOpenMenuUuid(null);
    showConfirm({
      title: '删除流程',
      description: '确定要删除此流程吗？删除后无法恢复。',
      confirmText: '删除',
      onConfirm: () => {
        deleteFlow(uuid);
        removeFlowFromList(uuid);
        // 如果删除的是当前流程，清空画布
        if (currentFlow?.uuid === uuid) {
          setFlow(null);
        }
      },
    });
  };

  const handleDuplicateFlow = (uuid: string) => {
    setOpenMenuUuid(null);
    const sourceFlow = loadFlow(uuid);
    if (!sourceFlow) return;

    const newUuid = crypto.randomUUID();
    const duplicatedFlow: Flow = {
      ...structuredClone(sourceFlow),
      uuid: newUuid,
      name: `${sourceFlow.name} (副本)`,
    };
    saveFlow(duplicatedFlow);
    addFlowToList({
      uuid: newUuid,
      name: duplicatedFlow.name,
      updated_at: Date.now(),
    });
    // 切换到复制后的流程
    setFlow(duplicatedFlow);
  };

  const handleNewFlow = () => {
    openNewFlowDialog();
  };

  const flows = useFlowListStore((state) => state.flows);

  return (
    <div className={cn("flex flex-col h-full", showHeader && "h-52 border-b border-border")}>
      {/* 标题栏 */}
      {showHeader && (
        <div className="flex items-center justify-between px-3 py-2 border-b border-border">
          <span className="text-sm font-medium text-foreground">流程列表</span>
          <Button
            variant="ghost"
            size="icon"
            onClick={handleNewFlow}
            className="h-7 w-7 rounded-md bg-background border border-border hover:bg-accent"
            title="新建流程"
          >
            <Plus className="w-4 h-4 text-foreground" />
          </Button>
        </div>
      )}

      {/* 流程列表 */}
      <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden scrollbar-thin">
        <div className="p-2 space-y-1">
          {flows.map((flowMeta) => {
            const isSelected = currentFlow?.uuid === flowMeta.uuid;

            return (
              <div
                key={flowMeta.uuid}
                onClick={() => handleSelectFlow(flowMeta.uuid)}
                className={cn(
                  'group w-full flex items-center gap-2 px-3 h-12 rounded-lg text-sm text-left transition-all duration-150',
                  'bg-background border border-border',
                  'hover:bg-accent',
                  isSelected && 'bg-background border-border',
                  !isSelected && 'text-muted-foreground'
                )}
              >
                <FileJson
                  className={cn(
                    'w-4 h-4 shrink-0 transition-colors',
                    isSelected ? 'text-emerald-500' : 'text-muted-foreground'
                  )}
                />
                <div className="flex-1 min-w-0">
                  <p className="truncate font-medium">{flowMeta.name}</p>
                  <p className="text-xs text-muted-foreground truncate">
                    {formatRelativeTime(flowMeta.updated_at)}
                  </p>
                </div>

                {/* 更多操作菜单按钮 */}
                <div className="relative">
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={(e) => {
                      e.stopPropagation();
                      if (openMenuUuid === flowMeta.uuid) {
                        setOpenMenuUuid(null);
                      } else {
                        const rect = e.currentTarget.getBoundingClientRect();
                        setMenuPosition({ top: rect.bottom + 4, left: rect.right - 120 });
                        setOpenMenuUuid(flowMeta.uuid);
                      }
                    }}
                    className={cn(
                      'h-6 w-6 shrink-0 transition-opacity duration-150',
                      'opacity-0 group-hover:opacity-100',
                      'hover:bg-accent'
                    )}
                    title="更多操作"
                  >
                    <MoreVertical className="w-3.5 h-3.5" />
                  </Button>

                  {openMenuUuid === flowMeta.uuid && createPortal(
                    <div
                      ref={menuRef}
                      className="fixed z-[9999] min-w-[120px] rounded-md border border-border bg-background shadow-md animate-in fade-in-0 zoom-in-95"
                      style={{ top: menuPosition.top, left: menuPosition.left }}
                    >
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDuplicateFlow(flowMeta.uuid);
                        }}
                        className="flex w-full items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent transition-colors"
                      >
                        <Copy className="w-3.5 h-3.5" />
                        复制流程
                      </button>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDeleteFlow(flowMeta.uuid);
                        }}
                        className="flex w-full items-center gap-2 px-3 py-1.5 text-sm text-red-500 hover:bg-red-50 transition-colors"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                        删除流程
                      </button>
                    </div>,
                    document.body
                  )}
                </div>
              </div>
            );
          })}

          {flows.length === 0 && (
            <div className="flex flex-col items-center justify-center py-6 text-muted-foreground">
              <FileJson className="w-8 h-8 mb-2 opacity-40" />
              <p className="text-sm">暂无流程</p>
              <p className="text-xs mt-1 opacity-70">点击 + 创建新流程</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
