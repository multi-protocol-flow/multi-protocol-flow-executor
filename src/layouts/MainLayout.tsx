/**
 * MainLayout - 主布局组件
 * 整合所有面板：左侧流程列表/节点库、中间画布、右侧配置、底部历史
 */
import { FlowListPanel, NodeLibrary } from '@/components/sidebar';
import { FlowCanvas } from '@/components/canvas';
import { NodeConfigPanel } from '@/components/panels';
import { ExecuteButton, ExecutionReport, ExecutionHistory } from '@/components/execution';
import { NewFlowDialog } from '@/components/dialogs/NewFlowDialog';
import { useUIStore, useFlowStore } from '@/stores';
import { ChevronLeft, ChevronRight, ChevronUp, ChevronDown, Zap, PanelRightClose, PanelRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

export function MainLayout() {
  const {
    isFlowListPanelOpen,
    isHistoryPanelOpen,
    isConfigPanelOpen,
    toggleFlowListPanel,
    toggleHistoryPanel,
    toggleConfigPanel,
  } = useUIStore();

  const { isDirty, flow } = useFlowStore();

  return (
    <div className="h-screen flex flex-col bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900 text-slate-100 overflow-hidden">
      {/* ===== 顶部工具栏 ===== */}
      <header className="h-14 shrink-0 border-b border-slate-700/50 flex items-center justify-between px-4 bg-slate-900/80 backdrop-blur-sm">
        {/* 左侧：标题 */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2.5">
            {/* Logo/图标 */}
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-emerald-400 to-teal-500 flex items-center justify-center shadow-lg shadow-emerald-500/20">
              <Zap className="w-4 h-4 text-white" />
            </div>
            <div>
              <h1 className="text-base font-semibold tracking-tight text-slate-100">
                多协议流程执行器
              </h1>
              <p className="text-[10px] text-slate-500 leading-none">
                Flow Executor
              </p>
            </div>
          </div>

          {/* 当前流程名称 */}
          {flow && (
            <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-slate-800/50 border border-slate-700/50">
              <span className="text-sm text-slate-300">{flow.name}</span>
              {isDirty && (
                <span className="w-2 h-2 rounded-full bg-amber-400 animate-pulse" title="未保存" />
              )}
            </div>
          )}
        </div>

        {/* 右侧：操作按钮 */}
        <div className="flex items-center gap-2">
          {/* 执行按钮 */}
          <ExecuteButton />

          {/* 配置面板切换 */}
          <Button
            variant="ghost"
            size="icon"
            onClick={toggleConfigPanel}
            className={cn(
              'h-9 w-9',
              isConfigPanelOpen
                ? 'bg-slate-700/50 text-slate-200'
                : 'text-slate-400 hover:text-slate-200'
            )}
            title={isConfigPanelOpen ? '隐藏配置面板' : '显示配置面板'}
          >
            {isConfigPanelOpen ? (
              <PanelRightClose className="w-4 h-4" />
            ) : (
              <PanelRight className="w-4 h-4" />
            )}
          </Button>
        </div>
      </header>

      {/* ===== 主内容区 ===== */}
      <div className="flex-1 flex overflow-hidden relative">
        {/* ===== 左侧栏 ===== */}
        <aside
          className={cn(
            'shrink-0 border-r border-slate-700/50 flex flex-col bg-slate-900/60 backdrop-blur-sm',
            'transition-all duration-300 ease-out overflow-hidden',
            isFlowListPanelOpen ? 'w-64' : 'w-0'
          )}
        >
          {isFlowListPanelOpen && (
            <>
              {/* 流程列表 */}
              <FlowListPanel />
              {/* 节点库 */}
              <div className="flex-1 min-h-0">
                <NodeLibrary />
              </div>
            </>
          )}
        </aside>

        {/* 左侧折叠按钮 */}
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleFlowListPanel}
          className={cn(
            'absolute z-20 h-8 w-6 rounded-l-none rounded-r-lg',
            'bg-slate-800/80 hover:bg-slate-700 border border-l-0 border-slate-700/50',
            'transition-all duration-300 ease-out',
            isFlowListPanelOpen ? 'left-[256px]' : 'left-0'
          )}
          style={{ top: 'calc(50% - 16px)' }}
          title={isFlowListPanelOpen ? '收起侧边栏' : '展开侧边栏'}
        >
          {isFlowListPanelOpen ? (
            <ChevronLeft className="w-4 h-4 text-slate-400" />
          ) : (
            <ChevronRight className="w-4 h-4 text-slate-400" />
          )}
        </Button>

        {/* ===== 中间画布区 ===== */}
        <main className="flex-1 flex flex-col min-w-0 overflow-hidden">
          {/* 画布 */}
          <div className="flex-1 relative">
            <FlowCanvas />
          </div>

          {/* ===== 底部历史面板 ===== */}
          <div
            className={cn(
              'shrink-0 border-t border-slate-700/50 bg-slate-900/60 backdrop-blur-sm',
              'transition-all duration-300 ease-out overflow-hidden',
              isHistoryPanelOpen ? 'h-52' : 'h-0'
            )}
          >
            {isHistoryPanelOpen && (
              <div className="h-full flex flex-col">
                {/* 历史面板头部 */}
                <div className="flex items-center justify-between px-3 py-2 border-b border-slate-700/30">
                  <span className="text-sm font-medium text-slate-300">执行历史</span>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={toggleHistoryPanel}
                    className="h-7 w-7 hover:bg-slate-700/50"
                    title="收起面板"
                  >
                    <ChevronDown className="w-4 h-4 text-slate-400" />
                  </Button>
                </div>
                {/* 历史列表 */}
                <div className="flex-1 overflow-hidden">
                  <ExecutionHistory />
                </div>
              </div>
            )}
          </div>

          {/* 底部历史按钮（面板收起时显示）- 在画布底部 */}
          {!isHistoryPanelOpen && (
            <Button
              variant="ghost"
              size="sm"
              onClick={toggleHistoryPanel}
              className={cn(
                'absolute bottom-0 left-1/2 -translate-x-1/2 z-10',
                'h-6 px-3 rounded-t-lg rounded-b-none',
                'bg-slate-800/80 hover:bg-slate-700 border border-b-0 border-slate-700/50',
                'text-xs text-slate-300 hover:text-slate-100'
              )}
              title="展开历史面板"
            >
              <ChevronUp className="w-3.5 h-3.5 mr-1" />
              历史记录
            </Button>
          )}
        </main>

        {/* ===== 右侧配置面板 ===== */}
        <aside
          className={cn(
            'shrink-0 border-l border-slate-700/50 bg-slate-900/60 backdrop-blur-sm',
            'transition-all duration-300 ease-out overflow-hidden',
            isConfigPanelOpen ? 'w-80' : 'w-0'
          )}
        >
          {isConfigPanelOpen && (
            <div className="h-full w-80">
              <NodeConfigPanel />
            </div>
          )}
        </aside>
      </div>

      {/* ===== 执行报告弹窗 ===== */}
      <ExecutionReport />

      {/* ===== 新建流程对话框 ===== */}
      <NewFlowDialog />
    </div>
  );
}
