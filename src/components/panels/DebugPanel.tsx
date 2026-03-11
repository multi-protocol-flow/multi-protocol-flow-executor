/**
 * DebugPanel - 调试面板
 * 显示当前节点信息、变量列表、执行堆栈和节点输出
 */
import { useState, useRef, useEffect, useCallback } from 'react';
import { ChevronDown, ChevronRight, Copy, Edit2, Check, X, Pause, CheckCircle, Settings2, Lock } from 'lucide-react';
import { useDebugStore, selectIsPaused, selectCurrentNodeUuid, selectIsDebugDataReadOnly } from '@/stores';
import { useFlowStore, useUIStore } from '@/stores';
import { Button } from '@/components/ui/button';
import { CollapsiblePanel } from '@/components/ui/CollapsiblePanel';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { DebugNodeConfigDialog } from '@/components/dialogs/DebugNodeConfigDialog';
import { WsMessageStreamPanel } from '@/components/debug/WsMessageStreamPanel';
import type { ExecutionStackEntry } from '@/types/debug';
import type { NodeData } from '@/types/node';

/**
 * 变量项组件 - 显示单个变量
 */
function VariableItem({
  name,
  value,
  depth = 0,
  onEdit,
}: {
  name: string;
  value: unknown;
  depth?: number;
  onEdit?: (name: string, value: unknown) => void;
}) {
  const [isExpanded, setIsExpanded] = useState(depth < 1);
  const [isEditing, setIsEditing] = useState(false);
  const [editValue, setEditValue] = useState('');

  const isObject = value !== null && typeof value === 'object' && !Array.isArray(value);
  const isArray = Array.isArray(value);
  const isComplex = isObject || isArray;

  // 复制变量到剪贴板
  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(JSON.stringify(value, null, 2));
      toast.success('已复制到剪贴板');
    } catch (error) {
      toast.error('复制失败');
    }
  };

  // 开始编辑
  const handleStartEdit = () => {
    setEditValue(JSON.stringify(value, null, 2));
    setIsEditing(true);
  };

  // 保存编辑
  const handleSaveEdit = () => {
    try {
      const parsed = JSON.parse(editValue);
      onEdit?.(name, parsed);
      setIsEditing(false);
      toast.success('变量已修改');
    } catch (error) {
      toast.error('JSON 格式错误');
    }
  };

  // 取消编辑
  const handleCancelEdit = () => {
    setIsEditing(false);
    setEditValue('');
  };

  // 获取简单类型的显示值
  const getSimpleDisplay = () => {
    if (value === null) return 'null';
    if (value === undefined) return 'undefined';
    if (typeof value === 'string') return `"${value}"`;
    if (typeof value === 'boolean') return value ? 'true' : 'false';
    if (typeof value === 'number') return String(value);
    return String(value);
  };

  return (
    <div className="border-b border-slate-100 last:border-b-0">
      {/* 变量头部 */}
      <div
        className={cn(
          'flex items-center gap-2 px-2 py-1.5 group',
          'hover:bg-slate-50 transition-colors'
        )}
        style={{ paddingLeft: `${depth * 16 + 8}px` }}
      >
        {/* 展开/折叠按钮（仅复杂类型） */}
        {isComplex ? (
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="w-4 h-4 flex items-center justify-center text-slate-400 hover:text-slate-600"
          >
            {isExpanded ? (
              <ChevronDown className="w-3 h-3" />
            ) : (
              <ChevronRight className="w-3 h-3" />
            )}
          </button>
        ) : (
          <div className="w-4 h-4" />
        )}

        {/* 变量名 */}
        <span className="text-sm font-medium text-slate-700 min-w-0 flex-shrink-0">
          {name}
        </span>

        {/* 简单类型值 */}
        {!isComplex && !isEditing && (
          <span className="text-sm text-slate-500 font-mono flex-1 truncate">
            {getSimpleDisplay()}
          </span>
        )}

        {/* 复杂类型预览 */}
        {isComplex && !isExpanded && (
          <span className="text-sm text-slate-400 flex-1 truncate">
            {isObject ? '{...}' : `[${(value as unknown[]).length}]`}
          </span>
        )}

        {/* 操作按钮（仅在暂停时显示） */}
        {!isEditing && (
          <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <button
              onClick={handleCopy}
              className="w-6 h-6 flex items-center justify-center text-slate-400 hover:text-slate-600 hover:bg-slate-100 rounded"
              title="复制"
            >
              <Copy className="w-3 h-3" />
            </button>
            {onEdit && !isComplex && (
              <button
                onClick={handleStartEdit}
                className="w-6 h-6 flex items-center justify-center text-slate-400 hover:text-slate-600 hover:bg-slate-100 rounded"
                title="编辑"
              >
                <Edit2 className="w-3 h-3" />
              </button>
            )}
          </div>
        )}

        {/* 编辑模式 */}
        {isEditing && (
          <>
            <button
              onClick={handleSaveEdit}
              className="w-6 h-6 flex items-center justify-center text-green-600 hover:bg-green-50 rounded"
              title="保存"
            >
              <Check className="w-3 h-3" />
            </button>
            <button
              onClick={handleCancelEdit}
              className="w-6 h-6 flex items-center justify-center text-red-600 hover:bg-red-50 rounded"
              title="取消"
            >
              <X className="w-3 h-3" />
            </button>
          </>
        )}
      </div>

      {/* 编辑文本框 */}
      {isEditing && (
        <div className="px-2 pb-2" style={{ paddingLeft: `${depth * 16 + 8}px` }}>
          <textarea
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            className="w-full h-24 px-2 py-1 text-xs font-mono border border-slate-300 rounded resize-none focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder="输入 JSON 格式的值"
          />
        </div>
      )}

      {/* 复杂类型展开内容 */}
      {isComplex && isExpanded && !isEditing && (
        <div className="bg-slate-50/50">
          {isObject &&
            Object.entries(value as Record<string, unknown>).map(([key, val]) => (
              <VariableItem
                key={key}
                name={key}
                value={val}
                depth={depth + 1}
                onEdit={onEdit ? (_, newVal) => {
                  // 更新嵌套对象的值
                  const updated = { ...(value as Record<string, unknown>) };
                  updated[name] = newVal;
                  onEdit(name, updated);
                } : undefined}
              />
            ))}
          {isArray &&
            (value as unknown[]).map((item, idx) => (
              <VariableItem
                key={idx}
                name={`[${idx}]`}
                value={item}
                depth={depth + 1}
                onEdit={onEdit ? (_, newVal) => {
                  // 更新数组项的值
                  const updated = [...(value as unknown[])];
                  updated[idx] = newVal;
                  onEdit(name, updated);
                } : undefined}
              />
            ))}
        </div>
      )}
    </div>
  );
}

/**
 * 堆栈条目组件 - 显示单个已执行节点
 */
function StackEntryItem({
  entry,
  isCurrentNode,
  onNavigateToNode,
}: {
  entry: ExecutionStackEntry;
  isCurrentNode: boolean;
  onNavigateToNode: (uuid: string) => void;
}) {
  const [isExpanded, setIsExpanded] = useState(false);

  // 复制数据到剪贴板
  const handleCopyData = async (data: unknown, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(JSON.stringify(data, null, 2));
      toast.success('已复制到剪贴板');
    } catch {
      toast.error('复制失败');
    }
  };

  // 格式化执行时间
  const formatDuration = () => {
    if (!entry.ended_at || !entry.started_at) return null;
    const duration = entry.ended_at - entry.started_at;
    if (duration < 1000) return `${duration}ms`;
    return `${(duration / 1000).toFixed(2)}s`;
  };

  const hasInput = entry.input !== undefined && entry.input !== null;
  const hasOutput = entry.output !== undefined && entry.output !== null;

  return (
    <div className="border-b border-slate-100 last:border-b-0">
      {/* 条目头部 */}
      <div
        className={cn(
          'flex items-center gap-2 px-3 py-2 group',
          'hover:bg-slate-50 transition-colors',
          isCurrentNode && 'bg-blue-50 hover:bg-blue-50'
        )}
      >
        {/* 状态图标 */}
        <div
          className={cn(
            'w-5 h-5 rounded-full flex items-center justify-center text-xs',
            isCurrentNode
              ? 'bg-amber-100 text-amber-600'
              : entry.success
                ? 'bg-green-100 text-green-600'
                : 'bg-red-100 text-red-600'
          )}
          title={isCurrentNode ? '当前暂停' : entry.success ? '执行成功' : '执行失败'}
        >
          {isCurrentNode ? (
            <Pause className="w-3 h-3" />
          ) : entry.success ? (
            <CheckCircle className="w-3 h-3" />
          ) : (
            '✗'
          )}
        </div>

        {/* 展开/折叠按钮（有输入/输出数据时显示） */}
        {(hasInput || hasOutput) ? (
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="w-4 h-4 flex items-center justify-center text-slate-400 hover:text-slate-600"
          >
            {isExpanded ? (
              <ChevronDown className="w-3 h-3" />
            ) : (
              <ChevronRight className="w-3 h-3" />
            )}
          </button>
        ) : (
          <div className="w-4 h-4" />
        )}

        {/* 节点名称（可点击跳转） */}
        <button
          onClick={() => onNavigateToNode(entry.node_uuid)}
          className="flex-1 min-w-0 text-left"
          title="点击跳转到该节点"
        >
          <div
            className={cn(
              'text-sm font-medium truncate',
              isCurrentNode ? 'text-blue-700' : 'text-slate-900 hover:text-blue-600'
            )}
          >
            {entry.node_name}
          </div>
          <div className="text-xs text-slate-500 flex items-center gap-2">
            <span>{entry.node_type}</span>
            {formatDuration() && (
              <>
                <span>·</span>
                <span>{formatDuration()}</span>
              </>
            )}
            {entry.error && (
              <>
                <span>·</span>
                <span className="text-red-500 truncate">{entry.error}</span>
              </>
            )}
          </div>
        </button>
      </div>

      {/* 展开内容：输入/输出数据 */}
      {isExpanded && (hasInput || hasOutput) && (
        <div className="bg-slate-50 px-3 pb-2 space-y-2">
          {/* 输入数据 */}
          {hasInput && (
            <div>
              <div className="flex items-center justify-between mb-1">
                <span className="text-xs font-medium text-slate-600">输入</span>
                <button
                  onClick={(e) => handleCopyData(entry.input, e)}
                  className="opacity-0 group-hover:opacity-100 p-1 hover:bg-slate-200 rounded"
                  title="复制输入"
                >
                  <Copy className="w-3 h-3 text-slate-400" />
                </button>
              </div>
              <pre className="text-xs font-mono text-slate-700 bg-white p-2 rounded border border-slate-200 overflow-x-auto max-h-32 overflow-y-auto">
                {JSON.stringify(entry.input, null, 2)}
              </pre>
            </div>
          )}

          {/* 输出数据 */}
          {hasOutput && (
            <div>
              <div className="flex items-center justify-between mb-1">
                <span className="text-xs font-medium text-slate-600">输出</span>
                <button
                  onClick={(e) => handleCopyData(entry.output, e)}
                  className="opacity-0 group-hover:opacity-100 p-1 hover:bg-slate-200 rounded"
                  title="复制输出"
                >
                  <Copy className="w-3 h-3 text-slate-400" />
                </button>
              </div>
              <pre className="text-xs font-mono text-slate-700 bg-white p-2 rounded border border-slate-200 overflow-x-auto max-h-32 overflow-y-auto">
                {JSON.stringify(entry.output, null, 2)}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * 调试面板组件
 */
export function DebugPanel() {
  const isPaused = useDebugStore(selectIsPaused);
  const currentNodeUuid = useDebugStore(selectCurrentNodeUuid);
  const isDebugDataReadOnly = useDebugStore(selectIsDebugDataReadOnly);
  const variables = useDebugStore((state) => state.variables);
  const executionStack = useDebugStore((state) => state.executionStack);
  const tempNodeConfigs = useDebugStore((state) => state.tempNodeConfigs);
  const modifyNodeConfig = useDebugStore((state) => state.modifyNodeConfig);

  const flow = useFlowStore((state) => state.flow);
  const focusOnNode = useFlowStore((state) => state.focusOnNode);

  // 调试面板折叠状态
  const isContextVariablesCollapsed = useUIStore((state) => state.isContextVariablesCollapsed);
  const isExecutionStackCollapsed = useUIStore((state) => state.isExecutionStackCollapsed);
  const isWsStreamCollapsed = useUIStore((state) => state.isWsStreamCollapsed);
  const executionStackHeight = useUIStore((state) => state.executionStackHeight);
  const wsStreamHeight = useUIStore((state) => state.wsStreamHeight);
  const toggleContextVariablesCollapsed = useUIStore((state) => state.toggleContextVariablesCollapsed);
  const toggleExecutionStackCollapsed = useUIStore((state) => state.toggleExecutionStackCollapsed);
  const toggleWsStreamCollapsed = useUIStore((state) => state.toggleWsStreamCollapsed);
  const setExecutionStackHeight = useUIStore((state) => state.setExecutionStackHeight);
  const setWsStreamHeight = useUIStore((state) => state.setWsStreamHeight);

  // 获取当前节点信息
  const currentNode = flow?.nodes.find((n) => n.uuid === currentNodeUuid);

  // 弹窗状态
  const [isConfigDialogOpen, setIsConfigDialogOpen] = useState(false);

  // 容器引用和可用高度计算
  const containerRef = useRef<HTMLDivElement>(null);
  const [maxBottomPanelHeight, setMaxBottomPanelHeight] = useState(400);

  // 是否允许编辑（暂停时且非只读模式）
  const canEdit = isPaused && !isDebugDataReadOnly;

  // 计算底部面板区的最大可用高度
  const updateMaxHeight = useCallback(() => {
    if (!containerRef.current) return;
    const containerHeight = containerRef.current.clientHeight;
    // 固定区域高度
    const HEADER_HEIGHT = 32;
    const MIN_CONTENT_HEIGHT = 100;
    const CURRENT_NODE_HEIGHT = 80; // 当前节点区域大约高度
    const READONLY_BANNER_HEIGHT = isDebugDataReadOnly ? 50 : 0; // 只读横幅高度

    // 上下文变量至少保留标题栏高度（折叠时）或最小高度（展开时）
    const contextVariablesReserved = isContextVariablesCollapsed
      ? HEADER_HEIGHT
      : MIN_CONTENT_HEIGHT + HEADER_HEIGHT;

    // 底部面板区可用高度 = 总高度 - 只读横幅 - 当前节点 - 上下文变量
    const available = containerHeight - READONLY_BANNER_HEIGHT - CURRENT_NODE_HEIGHT - contextVariablesReserved;
    setMaxBottomPanelHeight(Math.max(MIN_CONTENT_HEIGHT, available));
  }, [isContextVariablesCollapsed, isDebugDataReadOnly]);

  useEffect(() => {
    updateMaxHeight();

    // 监听窗口大小变化
    window.addEventListener('resize', updateMaxHeight);
    return () => window.removeEventListener('resize', updateMaxHeight);
  }, [updateMaxHeight]);

  // 折叠状态变化时重新计算
  useEffect(() => {
    requestAnimationFrame(updateMaxHeight);
  }, [updateMaxHeight]);

  // 处理变量修改
  const handleModifyVariable = async (name: string, value: unknown) => {
    try {
      await sendDebugCommand({
        ModifyVariable: { name, value },
      });
    } catch (error) {
      toast.error(`修改变量失败: ${(error as Error).message}`);
    }
  };

  // 打开配置弹窗
  const handleOpenConfigDialog = () => {
    setIsConfigDialogOpen(true);
  };

  // 保存配置修改
  const handleSaveConfig = async (data: NodeData) => {
    if (!currentNode) return;
    await modifyNodeConfig(currentNode.uuid, data);
  };

  // 检查当前节点是否有临时配置
  const hasTempConfig = currentNode && tempNodeConfigs[currentNode.uuid] !== undefined;

  // 跳转到指定节点
  const handleNavigateToNode = (uuid: string) => {
    focusOnNode(uuid);
  };

  // 渲染上下文变量内容
  const renderVariablesContent = () => (
    <>
      {Object.keys(variables).length > 0 ? (
        Object.entries(variables).map(([name, value]) => (
          <VariableItem
            key={name}
            name={name}
            value={value}
            onEdit={canEdit ? handleModifyVariable : undefined}
          />
        ))
      ) : (
        <div className="p-4 text-sm text-slate-400 text-center">暂无变量</div>
      )}
    </>
  );

  // 渲染执行堆栈内容
  const renderExecutionStackContent = () => (
    <>
      {executionStack.length > 0 ? (
        executionStack.map((entry) => (
          <StackEntryItem
            key={entry.node_uuid}
            entry={entry}
            isCurrentNode={!isDebugDataReadOnly && entry.node_uuid === currentNodeUuid}
            onNavigateToNode={handleNavigateToNode}
          />
        ))
      ) : (
        <div className="p-4 text-sm text-slate-400 text-center">暂无执行记录</div>
      )}
    </>
  );

  return (
    <div ref={containerRef} className="h-full flex flex-col bg-white overflow-hidden">
      {/* 只读模式横幅 */}
      {isDebugDataReadOnly && (
        <div className="flex flex-col gap-0.5 px-3 py-2 bg-amber-50 border-b border-amber-200 text-amber-700 text-xs shrink-0">
          <div className="flex items-center gap-1.5">
            <Lock className="w-3.5 h-3.5" />
            <span className="font-medium">调试已结束</span>
          </div>
          <span className="text-amber-600">只读模式（点击节点恢复正常）</span>
        </div>
      )}

      {/* ===== 当前节点区域 ===== */}
      <div className="border-b border-slate-200 p-3 shrink-0">
        <h3 className="text-xs font-semibold text-slate-600 mb-2 flex items-center gap-1.5">
          <div className="w-1.5 h-1.5 rounded-full bg-blue-500" />
          当前节点
        </h3>
        {currentNode ? (
          <div className="bg-slate-50 rounded px-2 py-1.5">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm font-medium text-slate-900">{currentNode.name}</div>
                <div className="text-xs text-slate-500 mt-0.5 flex items-center gap-2">
                  <span>{currentNode.type}</span>
                  {hasTempConfig && (
                    <span className="text-amber-600 font-medium">(已修改)</span>
                  )}
                </div>
              </div>
              {canEdit && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleOpenConfigDialog}
                  className="h-7 px-2 text-xs"
                  title="编辑节点配置（临时）"
                >
                  <Settings2 className="w-3 h-3 mr-1" />
                  编辑配置
                </Button>
              )}
            </div>
          </div>
        ) : (
          <div className="text-sm text-slate-400 italic">无当前节点</div>
        )}
      </div>

      {/* ===== 上下文变量区域（顶部，自动扩展填充剩余空间） ===== */}
      <CollapsiblePanel
        title="上下文变量"
        isCollapsed={isContextVariablesCollapsed}
        onToggle={toggleContextVariablesCollapsed}
        className={isContextVariablesCollapsed ? "shrink-0" : "flex-1 min-h-0 border-b border-slate-200"}
        style={!isContextVariablesCollapsed ? { minHeight: 132 } : undefined}
        headerRight={
          <span className="text-xs text-slate-400 font-normal">
            {Object.keys(variables).length}
          </span>
        }
      >
        {renderVariablesContent()}
      </CollapsiblePanel>

      {/* ===== 底部面板区 ===== */}
      <div
        className="flex flex-col mt-auto shrink-0 overflow-hidden"
        style={{ maxHeight: maxBottomPanelHeight }}
      >
        {/* 可滚动区域：执行堆栈 + WebSocket 消息流 */}
        <div className="flex-1 min-h-0 overflow-y-auto">
          {/* 执行堆栈 */}
          <CollapsiblePanel
            title="执行堆栈"
            isCollapsed={isExecutionStackCollapsed}
            onToggle={toggleExecutionStackCollapsed}
            height={executionStackHeight}
            onHeightChange={setExecutionStackHeight}
            resizable={!isExecutionStackCollapsed}
            maxHeight={maxBottomPanelHeight - 32}
            className="border-t border-border"
            headerRight={
              <span className="text-xs text-slate-400 font-normal">
                {executionStack.length}
              </span>
            }
          >
            {renderExecutionStackContent()}
          </CollapsiblePanel>

          {/* WebSocket 消息流 */}
          <CollapsiblePanel
            title="WebSocket 消息流"
            isCollapsed={isWsStreamCollapsed}
            onToggle={toggleWsStreamCollapsed}
            height={wsStreamHeight}
            onHeightChange={setWsStreamHeight}
            resizable={!isWsStreamCollapsed}
            maxHeight={maxBottomPanelHeight - 32}
            className="border-t border-border"
          >
            <WsMessageStreamPanel />
          </CollapsiblePanel>
        </div>
      </div>

      {/* 节点配置编辑弹窗 */}
      <DebugNodeConfigDialog
        open={isConfigDialogOpen}
        nodeData={currentNode ? (tempNodeConfigs[currentNode.uuid] as NodeData || currentNode.data) : null}
        nodeName={currentNode?.name || ''}
        onSave={handleSaveConfig}
        onCancel={() => setIsConfigDialogOpen(false)}
      />
    </div>
  );
}

// 导入需要的函数
import { sendDebugCommand } from '@/services/tauri';
