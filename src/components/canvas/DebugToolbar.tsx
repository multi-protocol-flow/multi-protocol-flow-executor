/**
 * DebugToolbar - 调试工具栏
 * 调试模式下显示的工具栏，包含调试控制按钮
 */
import { useState } from 'react';
import { Play, SkipForward, FastForward, Square, ArrowDownToLine } from 'lucide-react';
import { useDebugStore, selectIsPaused, selectCurrentNodeUuid } from '@/stores';
import { useFlowStore } from '@/stores';
import { SkipNodeDialog } from '@/components/dialogs/SkipNodeDialog';
import { cn } from '@/lib/utils';
import type { Port } from '@/types';

/**
 * 调试工具栏组件
 */
export function DebugToolbar() {
  const isPaused = useDebugStore(selectIsPaused);
  const currentNodeUuid = useDebugStore(selectCurrentNodeUuid);
  const continueExecution = useDebugStore((state) => state.continueExecution);
  const stepOver = useDebugStore((state) => state.stepOver);
  const skipNode = useDebugStore((state) => state.skipNode);
  const runToEnd = useDebugStore((state) => state.runToEnd);
  const endDebugSession = useDebugStore((state) => state.endDebugSession);

  const flow = useFlowStore((state) => state.flow);

  // 跳过节点对话框状态
  const [showSkipDialog, setShowSkipDialog] = useState(false);

  // 获取当前节点信息
  const currentNode = flow?.nodes.find((n) => n.uuid === currentNodeUuid);

  // 获取当前节点的输出端口
  const getOutputPorts = (): Array<{ id: string; label: string }> => {
    if (!currentNode) return [];

    // 根据节点类型返回不同的输出端口
    switch (currentNode.type) {
      case 'conditional':
      case 'ws_connect':
      case 'ws_send_collect':
      case 'ws_close':
        // 这些节点都有 true/false 两个分支
        return [
          { id: 'true', label: 'True 分支' },
          { id: 'false', label: 'False 分支' },
        ];
      case 'http':
        return [{ id: 'out', label: '输出 (out)' }];
      case 'entry':
        return [{ id: 'out', label: '输出' }];
      default:
        // 如果节点有自定义端口定义，使用它
        if (currentNode.ports) {
          return currentNode.ports
            .filter((p: Port) => p.type === 'output')
            .map((p: Port) => ({ id: p.id, label: p.label || p.id }));
        }
        return [{ id: 'output', label: '输出' }];
    }
  };

  // 处理跳过按钮点击
  const handleSkipClick = () => {
    const outputPorts = getOutputPorts();
    
    // 如果有多个输出端口，需要让用户选择分支
    if (outputPorts.length > 1) {
      setShowSkipDialog(true);
    } else if (outputPorts.length === 1) {
      // 只有一个输出端口，直接跳过
      skipNode(outputPorts[0].id);
    } else {
      // 没有输出端口（不应该发生），使用默认值
      skipNode('output');
    }
  };

  // 处理跳过节点对话框确认
  const handleSkipConfirm = (selectedPort: string, simulatedOutput?: unknown) => {
    skipNode(selectedPort, simulatedOutput);
    setShowSkipDialog(false);
  };

  return (
    <>
      {/* 方案1: 浅色背景 + 颜色图标 */}
      <div className="absolute top-4 left-1/2 -translate-x-1/2 z-10 flex items-center gap-1 bg-slate-50 border border-slate-200 rounded-xl px-3 py-2 shadow-lg">
        {/* 继续按钮 - 绿色 */}
        <button
          onClick={continueExecution}
          disabled={!isPaused}
          title={isPaused ? '继续执行到下一个断点' : '继续（仅在暂停时可用）'}
          className={cn(
            'flex items-center justify-center p-2 rounded-lg',
            'transition-all duration-150',
            'hover:bg-black/5',
            isPaused ? 'text-green-500 cursor-pointer' : 'opacity-40 cursor-not-allowed'
          )}
        >
          <Play className="w-5 h-5" fill="currentColor" stroke="none" />
        </button>

        {/* 单步按钮 - 蓝色 */}
        <button
          onClick={stepOver}
          disabled={!isPaused}
          title={isPaused ? '单步执行下一个节点' : '单步（仅在暂停时可用）'}
          className={cn(
            'flex items-center justify-center p-2 rounded-lg',
            'transition-all duration-150',
            'hover:bg-black/5',
            isPaused ? 'text-blue-500 cursor-pointer' : 'opacity-40 cursor-not-allowed'
          )}
        >
          <ArrowDownToLine className="w-5 h-5" />
        </button>

        {/* 跳过按钮 - 橙色 */}
        <button
          onClick={handleSkipClick}
          disabled={!isPaused}
          title={isPaused ? '跳过当前节点' : '跳过（仅在暂停时可用）'}
          className={cn(
            'flex items-center justify-center p-2 rounded-lg',
            'transition-all duration-150',
            'hover:bg-black/5',
            isPaused ? 'text-amber-500 cursor-pointer' : 'opacity-40 cursor-not-allowed'
          )}
        >
          <SkipForward className="w-5 h-5" />
        </button>

        {/* 执行到底按钮 - 蓝色 */}
        <button
          onClick={runToEnd}
          disabled={!isPaused}
          title={isPaused ? '执行到底（忽略后续断点）' : '执行到底（仅在暂停时可用）'}
          className={cn(
            'flex items-center justify-center p-2 rounded-lg',
            'transition-all duration-150',
            'hover:bg-black/5',
            isPaused ? 'text-blue-500 cursor-pointer' : 'opacity-40 cursor-not-allowed'
          )}
        >
          <FastForward className="w-5 h-5" />
        </button>

        {/* 分隔线 */}
        <div className="w-px h-6 bg-slate-200 mx-1" />

        {/* 结束调试按钮 - 红色 */}
        <button
          onClick={endDebugSession}
          title="结束调试会话"
          className={cn(
            'flex items-center justify-center p-2 rounded-lg',
            'transition-all duration-150',
            'hover:bg-black/5',
            'text-red-500 cursor-pointer'
          )}
        >
          <Square className="w-5 h-5" fill="currentColor" stroke="none" />
        </button>
      </div>

      {/* 跳过节点对话框 */}
      {currentNode && (
        <SkipNodeDialog
          open={showSkipDialog}
          nodeType={currentNode.type}
          nodeName={currentNode.name}
          outputPorts={getOutputPorts()}
          onConfirm={handleSkipConfirm}
          onCancel={() => setShowSkipDialog(false)}
        />
      )}
    </>
  );
}
