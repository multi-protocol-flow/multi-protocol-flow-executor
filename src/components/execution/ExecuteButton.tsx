/**
 * 执行按钮组件
 * 点击后执行当前流程，显示执行状态
 */
import { useState } from 'react';
import { Play, Loader2, Zap } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useFlowStore, useExecutionStore, useUIStore } from '@/stores';
import { executeFlow } from '@/services/tauri';
import { saveExecutionHistory } from '@/services/storage/historyStorage';
import { cn } from '@/lib/utils';

export function ExecuteButton() {
  const [isExecuting, setIsExecuting] = useState(false);
  const flow = useFlowStore((state) => state.flow);
  const setReport = useExecutionStore((state) => state.setReport);
  const setError = useExecutionStore((state) => state.setError);
  const setConfigPanelOpen = useUIStore((state) => state.setConfigPanelOpen);

  const handleExecute = async () => {
    if (!flow || isExecuting) return;

    setIsExecuting(true);
    setConfigPanelOpen(false);

    try {
      const result = await executeFlow(flow);

      if (result.success && result.flow_report) {
        setReport(result.flow_report);

        // 保存执行历史
        saveExecutionHistory({
          uuid: crypto.randomUUID(),
          flow_uuid: flow.uuid,
          flow_name: flow.name,
          status: result.flow_report.status === 'success' ? 'success' : 'failed',
          duration_ms: result.flow_report.duration_ms,
          timestamp: Date.now(),
        });
      } else {
        setError(result.error || '执行失败');
      }
    } catch (error) {
      setError((error as Error).message);
    } finally {
      setIsExecuting(false);
    }
  };

  const hasValidFlow = flow && flow.nodes.length > 0;
  const hasEntryNode = flow?.nodes.some((n) => n.type === 'entry');

  return (
    <Button
      variant="ghost"
      onClick={handleExecute}
      disabled={!hasValidFlow || !hasEntryNode || isExecuting}
      className={cn(
        'relative overflow-hidden gap-2 min-w-[100px] h-9',
        'bg-white border-2 border-black',
        'hover:bg-slate-50 hover:border-black',
        'active:bg-slate-100',
        'text-black font-medium',
        'rounded-2xl',
        'disabled:bg-slate-100 disabled:border-slate-300',
        'disabled:text-slate-400',
        'transition-all duration-200'
      )}
    >
      {/* 背景动画效果 */}
      <div
        className={cn(
          'absolute inset-0 bg-slate-100',
          'translate-x-[-100%]',
          isExecuting && 'animate-shimmer'
        )}
      />

      {isExecuting ? (
        <>
          <Loader2 className="w-4 h-4 animate-spin" />
          <span className="relative z-10">执行中</span>
        </>
      ) : (
        <>
          <Zap className="w-4 h-4" />
          <span className="relative z-10">执行流程</span>
          <Play className="w-3 h-3 ml-1 opacity-70" />
        </>
      )}
    </Button>
  );
}
