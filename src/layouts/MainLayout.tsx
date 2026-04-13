/**
 * MainLayout - 主布局组件
 * 整合所有面板：左侧流程列表/节点库、中间画布、右侧配置、底部历史
 */
import { Toaster } from '@/components/ui/sonner';

import { LeftSidebarPanel } from '@/components/sidebar';
import { FlowCanvas } from '@/components/canvas';
import { NodeConfigPanel, SettingsPanel, DebugPanel } from '@/components/panels';
import { ExecutionReport, ExecutionHistory, ConsolePanel, ExecutionPanel } from '@/components/execution';
import { NewFlowDialog } from '@/components/dialogs/NewFlowDialog';
import { ConfirmDialog } from '@/components/dialogs/ConfirmDialog';
import { ConnectionDialog } from '@/components/dialogs/ConnectionDialog';
import { useUIStore, useFlowStore, useDebugStore, selectIsDebugging, selectIsDebugDataReadOnly } from '@/stores';
import { useConsoleStore } from '@/stores/console';
import { ChevronLeft, ChevronRight, ChevronUp, ChevronDown, Zap, Trash2, Settings, GripVertical, GripHorizontal } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { useState, useCallback, useRef, useEffect } from 'react';

/**
 * 确认对话框处理器组件
 * 从 store 读取状态并渲染 ConfirmDialog
 */
function ConfirmDialogHandler() {
  const showConfirmDialog = useUIStore((state) => state.showConfirmDialog);
  const confirmDialogConfig = useUIStore((state) => state.confirmDialogConfig);
  const hideConfirm = useUIStore((state) => state.hideConfirm);

  if (!showConfirmDialog || !confirmDialogConfig) {
    return null;
  }

  return (
    <ConfirmDialog
      open={showConfirmDialog}
      title={confirmDialogConfig.title}
      description={confirmDialogConfig.description}
      confirmText={confirmDialogConfig.confirmText}
      onConfirm={() => {
        confirmDialogConfig.onConfirm();
        hideConfirm();
      }}
      onCancel={hideConfirm}
    />
  );
}

export function MainLayout() {
  const isFlowListPanelOpen = useUIStore((state) => state.isFlowListPanelOpen);
  const isHistoryPanelOpen = useUIStore((state) => state.isHistoryPanelOpen);
  const isConfigPanelOpen = useUIStore((state) => state.isConfigPanelOpen);
  const configPanelMode = useUIStore((state) => state.configPanelMode);
  const bottomPanelTab = useUIStore((state) => state.bottomPanelTab);
  const bottomPanelHeight = useUIStore((state) => state.bottomPanelHeight);
  const configPanelWidth = useUIStore((state) => state.configPanelWidth);
  const sidebarWidth = useUIStore((state) => state.sidebarWidth);
  const toggleFlowListPanel = useUIStore((state) => state.toggleFlowListPanel);
  const toggleHistoryPanel = useUIStore((state) => state.toggleHistoryPanel);
  const toggleConfigPanel = useUIStore((state) => state.toggleConfigPanel);
  const setBottomPanelTab = useUIStore((state) => state.setBottomPanelTab);
  const setBottomPanelHeight = useUIStore((state) => state.setBottomPanelHeight);
  const setConfigPanelWidth = useUIStore((state) => state.setConfigPanelWidth);
  const setSidebarWidth = useUIStore((state) => state.setSidebarWidth);
  const openSettingsDialog = useUIStore((state) => state.openSettingsDialog);
  const showConfirm = useUIStore((state) => state.showConfirm);

  const isDirty = useFlowStore((state) => state.isDirty);
  const flow = useFlowStore((state) => state.flow);

  // Debug store values
  const isDebugging = useDebugStore(selectIsDebugging);
  const isDebugDataReadOnly = useDebugStore(selectIsDebugDataReadOnly);
  
  // 是否显示调试面板（调试中或调试数据只读时）
  const showDebugPanel = isDebugging || isDebugDataReadOnly;

  // Console store values
  const filter = useConsoleStore((state) => state.filter);
  const clearLogs = useConsoleStore((state) => state.clearLogs);
  const setFilter = useConsoleStore((state) => state.setFilter);
  const setSearchKeyword = useConsoleStore((state) => state.setSearchKeyword);
  const getUniqueNodes = useConsoleStore((state) => state.getUniqueNodes);

  // 清空控制台（带确认）
  const handleClearConsole = useCallback(() => {
    showConfirm({
      title: '清空控制台',
      description: '确定要清空所有日志吗？清空后无法恢复。',
      confirmText: '清空',
      onConfirm: () => {
        clearLogs();
      },
    });
  }, [clearLogs, showConfirm]);

  // 底部面板拖拽调整高度相关状态
  const [isDragging, setIsDragging] = useState(false);
  const startYRef = useRef(0);
  const startHeightRef = useRef(0);

  // 底部面板内容延迟渲染：展开动画结束后再渲染内容
  const [bottomPanelContentReady, setBottomPanelContentReady] = useState(isHistoryPanelOpen);
  const bottomPanelTransitionRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 右侧面板拖拽调整宽度相关状态
  const [isRightPanelDragging, setIsRightPanelDragging] = useState(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(0);

  // 左侧面板拖拽调整宽度相关状态
  const [isLeftPanelDragging, setIsLeftPanelDragging] = useState(false);
  const startLeftXRef = useRef(0);
  const startLeftWidthRef = useRef(0);

  // 底部面板开始拖拽
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
    startYRef.current = e.clientY;
    startHeightRef.current = bottomPanelHeight;
    document.body.style.cursor = 'row-resize';
    document.body.style.userSelect = 'none';
  }, [bottomPanelHeight]);

  // 底部面板拖拽中
  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging) return;
    const deltaY = startYRef.current - e.clientY;
    const newHeight = Math.max(100, Math.min(500, startHeightRef.current + deltaY));
    setBottomPanelHeight(newHeight);
  }, [isDragging, setBottomPanelHeight]);

  // 底部面板结束拖拽
  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  // 右侧面板开始拖拽
  const handleRightPanelMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsRightPanelDragging(true);
    startXRef.current = e.clientX;
    startWidthRef.current = configPanelWidth;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }, [configPanelWidth]);

  // 右侧面板拖拽中
  const handleRightPanelMouseMove = useCallback((e: MouseEvent) => {
    if (!isRightPanelDragging) return;
    const deltaX = startXRef.current - e.clientX;
    const newWidth = Math.max(250, Math.min(500, startWidthRef.current + deltaX));
    setConfigPanelWidth(newWidth);
  }, [isRightPanelDragging, setConfigPanelWidth]);

  // 右侧面板结束拖拽
  const handleRightPanelMouseUp = useCallback(() => {
    setIsRightPanelDragging(false);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  // 左侧面板开始拖拽
  const handleLeftPanelMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsLeftPanelDragging(true);
    startLeftXRef.current = e.clientX;
    startLeftWidthRef.current = sidebarWidth;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }, [sidebarWidth]);

  // 左侧面板拖拽中
  const handleLeftPanelMouseMove = useCallback((e: MouseEvent) => {
    if (!isLeftPanelDragging) return;
    const deltaX = e.clientX - startLeftXRef.current;
    const newWidth = Math.max(200, Math.min(400, startLeftWidthRef.current + deltaX));
    setSidebarWidth(newWidth);
  }, [isLeftPanelDragging, setSidebarWidth]);

  // 左侧面板结束拖拽
  const handleLeftPanelMouseUp = useCallback(() => {
    setIsLeftPanelDragging(false);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  // 使用 useEffect 管理底部面板事件监听
  useEffect(() => {
    if (!isDragging) return;

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, handleMouseMove, handleMouseUp]);

  // 使用 useEffect 管理右侧面板事件监听
  useEffect(() => {
    if (!isRightPanelDragging) return;

    window.addEventListener('mousemove', handleRightPanelMouseMove);
    window.addEventListener('mouseup', handleRightPanelMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleRightPanelMouseMove);
      window.removeEventListener('mouseup', handleRightPanelMouseUp);
    };
  }, [isRightPanelDragging, handleRightPanelMouseMove, handleRightPanelMouseUp]);

  // 使用 useEffect 管理左侧面板事件监听
  useEffect(() => {
    if (!isLeftPanelDragging) return;

    window.addEventListener('mousemove', handleLeftPanelMouseMove);
    window.addEventListener('mouseup', handleLeftPanelMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleLeftPanelMouseMove);
      window.removeEventListener('mouseup', handleLeftPanelMouseUp);
    };
  }, [isLeftPanelDragging, handleLeftPanelMouseMove, handleLeftPanelMouseUp]);

  // 调试进行中时自动打开右侧面板（只读模式下不自动打开，允许用户折叠）
  useEffect(() => {
    if (isDebugging && !isConfigPanelOpen) {
      toggleConfigPanel();
    }
  }, [isDebugging, isConfigPanelOpen, toggleConfigPanel]);

  // 底部面板内容延迟渲染：展开动画结束后渲染，收起时立即移除
  useEffect(() => {
    // 清除之前的定时器
    if (bottomPanelTransitionRef.current) {
      clearTimeout(bottomPanelTransitionRef.current);
      bottomPanelTransitionRef.current = null;
    }

    if (isHistoryPanelOpen) {
      // 展开：等待动画结束后渲染内容
      bottomPanelTransitionRef.current = setTimeout(() => {
        setBottomPanelContentReady(true);
      }, 300);
    } else {
      // 收起：立即移除内容
      setBottomPanelContentReady(false);
    }

    return () => {
      if (bottomPanelTransitionRef.current) {
        clearTimeout(bottomPanelTransitionRef.current);
      }
    };
  }, [isHistoryPanelOpen]);

  return (
    <div className="h-screen flex flex-col bg-background text-foreground overflow-hidden">
      {/* ===== 顶部工具栏 ===== */}
      <header className="h-14 shrink-0 border-b border-border flex items-center justify-between px-4 bg-background">
        {/* 左侧：标题 */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2.5">
            {/* Logo/图标 */}
            <div className="w-8 h-8 rounded-lg bg-background border border-border flex items-center justify-center">
              <Zap className="w-4 h-4 text-foreground" />
            </div>
            <div>
              <h1 className="text-base font-semibold tracking-tight text-foreground">
                多协议流程执行器
              </h1>
              <p className="text-[10px] text-muted-foreground leading-none">
                Flow Executor
              </p>
            </div>
          </div>

          {/* 当前流程名称 */}
          {flow && (
            <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-accent border border-border">
              <span className="text-sm text-muted-foreground">{flow.name}</span>
              {isDirty && (
                <span className="w-2 h-2 rounded-full bg-amber-400 animate-pulse" title="未保存" />
              )}
            </div>
          )}
        </div>

        {/* 右侧：操作按钮 */}
        <div className="flex items-center gap-2">

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
            'shrink-0 border-r border-border flex flex-col bg-background overflow-hidden relative',
            'transition-all duration-300 ease-out',
            isFlowListPanelOpen ? 'min-w-0' : 'w-0 min-w-0',
            isLeftPanelDragging && '!transition-none'
          )}
          style={{ width: isFlowListPanelOpen ? sidebarWidth : 0 }}
        >
          {/* 可拖拽调整宽度的 handle - 放在面板内部右边缘 */}
          {isFlowListPanelOpen && (
            <div
              onMouseDown={handleLeftPanelMouseDown}
              className={cn(
                'absolute right-0 top-0 bottom-0 w-2 z-10',
                'bg-transparent hover:bg-border active:bg-input',
                'cursor-col-resize flex items-center justify-center',
                'transition-colors duration-150',
                isLeftPanelDragging && 'bg-input'
              )}
              title="拖拽调整面板宽度"
            >
              <GripVertical className="w-2.5 h-6 text-muted-foreground" />
            </div>
          )}
          {isFlowListPanelOpen && (
            <div className="h-full min-w-0 overflow-hidden pr-2" style={{ width: sidebarWidth }}>
              <LeftSidebarPanel />
            </div>
          )}
        </aside>

        {/* 左侧折叠按钮 */}
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleFlowListPanel}
          className={cn(
            'absolute z-20 h-8 w-6 rounded-l-none rounded-r-lg top-[calc(50%-16px)]',
            'bg-background hover:bg-accent border border-l-0 border-border',
            'transition-colors duration-150'
          )}
          style={{ left: isFlowListPanelOpen ? sidebarWidth : 0 }}
          title={isFlowListPanelOpen ? '收起侧边栏' : '展开侧边栏'}
        >
          {isFlowListPanelOpen ? (
            <ChevronLeft className="w-4 h-4 text-muted-foreground" />
          ) : (
            <ChevronRight className="w-4 h-4 text-muted-foreground" />
          )}
        </Button>

        {/* ===== 中间画布区 ===== */}
        <main className="flex-1 flex flex-col min-w-0 overflow-hidden">
          {/* 画布 */}
          <div className="flex-1 relative">
            <FlowCanvas />
          </div>

          {/* ===== 底部面板（历史/控制台） ===== */}
          <div
            className={cn(
              'shrink-0 border-t border-border bg-background overflow-hidden relative',
              'transition-all duration-300 ease-out',
              isDragging && '!transition-none'
            )}
            style={{ height: isHistoryPanelOpen ? bottomPanelHeight : 0 }}
          >
            {/* 可拖拽调整高度的 handle - 放在面板内部上边缘 */}
            <div
              onMouseDown={handleMouseDown}
              className={cn(
                'absolute left-0 right-0 top-0 h-2 z-10',
                'bg-transparent hover:bg-border active:bg-input',
                'cursor-row-resize flex items-center justify-center',
                'transition-colors duration-150',
                isDragging && 'bg-input'
              )}
              title="拖拽调整面板高度"
            >
              <GripHorizontal className="w-6 h-2.5 text-muted-foreground" />
            </div>
            {/* 内容延迟渲染：展开动画结束后再渲染，避免卡顿 */}
            {bottomPanelContentReady && (
              <div className="h-full flex flex-col pt-2">
                {/* 面板头部：Tab 切换 + 关闭按钮 */}
                <div className="flex items-center justify-between px-3 py-1.5 border-b border-border">
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
                      className="h-7 w-7 hover:bg-accent"
                      title="收起面板"
                    >
                      <ChevronDown className="w-4 h-4 text-muted-foreground" />
                    </Button>
                  </div>

                  {/* Tab 内容 */}
                  <Tabs value={bottomPanelTab} onValueChange={(v) => setBottomPanelTab(v as 'history' | 'console')} className="flex-1 min-h-0">
                    <TabsContent value="history" className="m-0 h-full overflow-hidden">
                      <ExecutionHistory />
                    </TabsContent>
                    <TabsContent value="console" className="m-0 h-full overflow-hidden flex flex-col">
                      {/* 过滤控件 */}
                      <div className="flex items-center gap-2 px-2 py-1.5 border-b border-border shrink-0">
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
                          onClick={handleClearConsole}
                          className="h-7 px-2 text-xs text-muted-foreground hover:text-accent-foreground"
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
                'bg-background hover:bg-accent border border-b-0 border-border',
                'text-xs text-muted-foreground hover:text-accent-foreground'
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
            'shrink-0 border-l border-border bg-background overflow-hidden relative',
            'transition-all duration-300 ease-out',
            isConfigPanelOpen ? 'min-w-0' : 'w-0 min-w-0',
            isRightPanelDragging && '!transition-none'
          )}
          style={{ width: isConfigPanelOpen ? configPanelWidth : 0 }}
        >
          {/* 可拖拽调整宽度的 handle - 放在面板内部左边缘 */}
          {isConfigPanelOpen && (
            <div
              onMouseDown={handleRightPanelMouseDown}
              className={cn(
                'absolute left-0 top-0 bottom-0 w-2 z-10',
                'bg-transparent hover:bg-border active:bg-input',
                'cursor-col-resize flex items-center justify-center',
                'transition-colors duration-150',
                isRightPanelDragging && 'bg-input'
              )}
              title="拖拽调整面板宽度"
            >
              <GripVertical className="w-2.5 h-6 text-muted-foreground" />
            </div>
          )}
          {isConfigPanelOpen && (
            <div className="h-full min-w-0 overflow-hidden pl-2" style={{ width: configPanelWidth }}>
              {showDebugPanel ? (
                <DebugPanel />
              ) : configPanelMode === 'settings' ? (
                <SettingsPanel />
              ) : (
                <NodeConfigPanel />
              )}
            </div>
          )}
        </aside>

        {/* 右侧折叠按钮 - 调试进行中时禁用（只读模式允许折叠） */}
        {!isDebugging && (
          <Button
            variant="ghost"
            size="icon"
            onClick={toggleConfigPanel}
            className={cn(
              'absolute z-20 h-8 w-6 rounded-r-none rounded-l-lg top-[calc(50%-16px)]',
              'bg-background hover:bg-accent border border-r-0 border-border',
              'transition-colors duration-150'
            )}
            style={{ right: isConfigPanelOpen ? configPanelWidth : 0 }}
            title={isConfigPanelOpen ? '收起配置面板' : '展开配置面板'}
          >
            {isConfigPanelOpen ? (
              <ChevronRight className="w-4 h-4 text-muted-foreground" />
            ) : (
              <ChevronLeft className="w-4 h-4 text-muted-foreground" />
            )}
          </Button>
        )}

        {/* ===== 执行进度面板（最右侧） ===== */}
        <ExecutionPanel />
      </div>

      {/* ===== 执行报告弹窗 ===== */}
      <ExecutionReport />

      {/* ===== 新建流程对话框 ===== */}
      <NewFlowDialog />

      {/* ===== 数据库连接对话框 ===== */}
      <ConnectionDialog />

      {/* ===== Toast 提示 ===== */}
      <Toaster />

      {/* ===== 通用确认对话框 ===== */}
      <ConfirmDialogHandler />
    </div>
  );
}
