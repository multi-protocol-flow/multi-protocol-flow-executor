/**
 * MainLayout - 主布局组件
 * 整合所有面板：左侧流程列表/节点库、中间画布、右侧配置、底部历史
 */
import { Toaster } from '@/components/ui/sonner';

import { FlowListPanel, NodeLibrary } from '@/components/sidebar';
import { FlowCanvas } from '@/components/canvas';
import { NodeConfigPanel } from '@/components/panels';
import { ExecuteButton, ExecutionReport, ExecutionHistory, ConsolePanel } from '@/components/execution';
import { NewFlowDialog } from '@/components/dialogs/NewFlowDialog';
import { SettingsDialog } from '@/components/settings/SettingsDialog';
import { useUIStore, useFlowStore } from '@/stores';
import { useConsoleStore } from '@/stores/console';
import { ChevronLeft, ChevronRight, ChevronUp, ChevronDown, Zap, Trash2, GripHorizontal, Settings } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { useState, useCallback, useRef, useEffect } from 'react';

export function MainLayout() {
  const {
    isFlowListPanelOpen,
    isHistoryPanelOpen,
    isConfigPanelOpen,
    bottomPanelTab,
    bottomPanelHeight,
    toggleFlowListPanel,
    toggleHistoryPanel,
    toggleConfigPanel,
    setBottomPanelTab,
    setBottomPanelHeight,
    openSettingsDialog,
  } = useUIStore();

  const { isDirty, flow } = useFlowStore();

  // Console store values
  const { filter, clearLogs, setFilter, setSearchKeyword, getUniqueNodes } = useConsoleStore();

  // 拖拽调整高度相关状态
  const [isDragging, setIsDragging] = useState(false);
  const startYRef = useRef(0);
  const startHeightRef = useRef(0);

  // 开始拖拽
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
    startYRef.current = e.clientY;
    startHeightRef.current = bottomPanelHeight;
    document.body.style.cursor = 'row-resize';
    document.body.style.userSelect = 'none';
  }, [bottomPanelHeight]);

  // 拖拽中
  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging) return;
    const deltaY = startYRef.current - e.clientY;
    const newHeight = Math.max(100, Math.min(500, startHeightRef.current + deltaY));
    setBottomPanelHeight(newHeight);
  }, [isDragging, setBottomPanelHeight]);

  // 结束拖拽
  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  // 使用 useEffect 管理事件监听
  useEffect(() => {
    if (!isDragging) return;

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, handleMouseMove, handleMouseUp]);

  return (
    <div className="h-screen flex flex-col bg-white text-slate-900 overflow-hidden">
      {/* ===== 顶部工具栏 ===== */}
      <header className="h-14 shrink-0 border-b border-border flex items-center justify-between px-4 bg-white">
        {/* 左侧：标题 */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2.5">
            {/* Logo/图标 */}
            <div className="w-8 h-8 rounded-lg bg-white border border-border flex items-center justify-center">
              <Zap className="w-4 h-4 text-slate-900" />
            </div>
            <div>
              <h1 className="text-base font-semibold tracking-tight text-slate-900">
                多协议流程执行器
              </h1>
              <p className="text-[10px] text-slate-500 leading-none">
                Flow Executor
              </p>
            </div>
          </div>

          {/* 当前流程名称 */}
          {flow && (
            <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-slate-100 border border-border">
              <span className="text-sm text-slate-600">{flow.name}</span>
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

          {/* 设置按钮 */}
          <Button
            variant="ghost"
            size="icon"
            onClick={openSettingsDialog}
            title="设置"
          >
            <Settings className="h-5 w-5" />
          </Button>

        </div>
      </header>

      {/* ===== 主内容区 ===== */}
      <div className="flex-1 flex overflow-hidden relative">
        {/* ===== 左侧栏 ===== */}
        <aside
          className={cn(
            'shrink-0 border-r border-border flex flex-col bg-white',
            'transition-all duration-300 ease-out overflow-hidden',
            isFlowListPanelOpen ? 'w-[256px] min-w-0 rounded-r-xl' : 'w-0'
          )}
        >
          {isFlowListPanelOpen && (
            <>
              <FlowListPanel />
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
            'absolute z-20 h-8 w-6 rounded-l-none rounded-r-lg top-[calc(50%-16px)]',
            'bg-white hover:bg-slate-50 border border-l-0 border-border',
            'transition-all duration-300 ease-out',
            isFlowListPanelOpen ? 'left-[256px]' : 'left-0'
          )}
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

          {/* ===== 底部面板（历史/控制台） ===== */}
          {/* 可拖拽调整高度的 handle */}
          {isHistoryPanelOpen && (
            <div
              onMouseDown={handleMouseDown}
              className={cn(
                'shrink-0 h-1.5 bg-transparent hover:bg-slate-200 active:bg-slate-300',
                'cursor-row-resize flex items-center justify-center',
                'transition-colors duration-150',
                isDragging && 'bg-slate-300'
              )}
              title="拖拽调整面板高度"
            >
              <GripHorizontal className="w-6 h-3 text-slate-400" />
            </div>
          )}
          <div
            className={cn(
              'shrink-0 bg-white overflow-hidden',
              isHistoryPanelOpen ? 'border-t border-border' : 'h-0'
            )}
            style={{ height: isHistoryPanelOpen ? bottomPanelHeight : 0 }}
          >
            {isHistoryPanelOpen && (
              <div className="h-full flex flex-col">
                {/* 面板头部：Tab 切换 + 关闭按钮 */}
                <div className="flex items-center justify-between px-3 py-1.5 border-b border-slate-100">
                  <Tabs value={bottomPanelTab} onValueChange={(v) => setBottomPanelTab(v as 'history' | 'console')} className="w-auto">
                    <TabsList className="h-7 p-0.5">
                      <TabsTrigger value="history" className="h-6 px-3 text-xs">
                        执行历史
                      </TabsTrigger>
                      <TabsTrigger value="console" className="h-6 px-3 text-xs">
                        控制台
                      </TabsTrigger>
                    </TabsList>
                  </Tabs>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={toggleHistoryPanel}
                    className="h-7 w-7 hover:bg-slate-100"
                    title="收起面板"
                  >
                    <ChevronDown className="w-4 h-4 text-slate-400" />
                  </Button>
                </div>

                {/* Tab 内容 */}
                <Tabs value={bottomPanelTab} onValueChange={(v) => setBottomPanelTab(v as 'history' | 'console')} className="flex-1 min-h-0">
                  <TabsContent value="history" className="m-0 h-full overflow-hidden">
                    <ExecutionHistory />
                  </TabsContent>
                  <TabsContent value="console" className="m-0 h-full overflow-hidden flex flex-col">
                    {/* 过滤控件 */}
                    <div className="flex items-center gap-2 px-2 py-1.5 border-b border-slate-100 shrink-0">
                      <Select value={filter.level} onValueChange={(v) => setFilter({ level: v as 'ALL' | 'INFO' | 'DEBUG' | 'WARN' | 'ERROR' })}>
                        <SelectTrigger className="w-20 h-7 text-xs">
                          <SelectValue placeholder="级别" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="ALL">全部</SelectItem>
                          <SelectItem value="INFO">INFO</SelectItem>
                          <SelectItem value="DEBUG">DEBUG</SelectItem>
                          <SelectItem value="WARN">WARN</SelectItem>
                          <SelectItem value="ERROR">ERROR</SelectItem>
                        </SelectContent>
                      </Select>
                      <Select value={filter.nodeUuid} onValueChange={(v) => setFilter({ nodeUuid: v })}>
                        <SelectTrigger className="w-28 h-7 text-xs">
                          <SelectValue placeholder="节点" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="ALL">全部节点</SelectItem>
                          {getUniqueNodes().map((node) => (
                            <SelectItem key={node.uuid} value={node.uuid}>
                              {node.name}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      <Input
                        placeholder="搜索..."
                        value={filter.searchKeyword}
                        onChange={(e) => setSearchKeyword(e.target.value)}
                        className="h-7 w-32 text-xs"
                      />
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={clearLogs}
                        className="h-7 px-2 text-xs text-slate-500 hover:text-slate-700"
                        title="清空日志"
                      >
                        <Trash2 className="w-3.5 h-3.5 mr-1" />
                        清空
                      </Button>
                    </div>
                    {/* 控制台内容 */}
                    <div className="flex-1 min-h-0 overflow-hidden">
                      <ConsolePanel />
                    </div>
                  </TabsContent>
                </Tabs>
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
                'bg-white hover:bg-slate-50 border border-b-0 border-border',
                'text-xs text-slate-500 hover:text-slate-700'
              )}
              title="展开底部面板"
            >
              <ChevronUp className="w-3.5 h-3.5 mr-1" />
              底部面板
            </Button>
          )}
        </main>

        {/* ===== 右侧配置面板 ===== */}
        <aside
          className={cn(
            'shrink-0 border-l border-border bg-white',
            'transition-all duration-300 ease-out overflow-hidden',
            isConfigPanelOpen ? 'w-72 min-w-0 rounded-l-xl' : 'w-0 min-w-0'
          )}
        >
          {isConfigPanelOpen && (
            <div className="h-full w-72 min-w-0 overflow-hidden">
              <NodeConfigPanel />
            </div>
          )}
        </aside>

        {/* 右侧折叠按钮 */}
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleConfigPanel}
          className={cn(
            'absolute z-20 h-8 w-6 rounded-r-none rounded-l-lg top-[calc(50%-16px)]',
            'bg-white hover:bg-slate-50 border border-r-0 border-border',
            'transition-all duration-300 ease-out',
            isConfigPanelOpen ? 'right-[288px]' : 'right-0'
          )}
          title={isConfigPanelOpen ? '收起配置面板' : '展开配置面板'}
        >
          {isConfigPanelOpen ? (
            <ChevronRight className="w-4 h-4 text-slate-400" />
          ) : (
            <ChevronLeft className="w-4 h-4 text-slate-400" />
          )}
        </Button>
      </div>

      {/* ===== 执行报告弹窗 ===== */}
      <ExecutionReport />

      {/* ===== 新建流程对话框 ===== */}
      <NewFlowDialog />

      {/* ===== 设置对话框 ===== */}
      <SettingsDialog />

      {/* ===== Toast 提示 ===== */}
      <Toaster />
    </div>
  );
}
