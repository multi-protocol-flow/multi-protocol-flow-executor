/**
 * DebugPanel - 调试面板
 * 显示当前节点信息、变量列表、执行堆栈和节点输出
 */
import { useState } from 'react';
import { ChevronDown, ChevronRight, Copy, Edit2, Check, X, Pause, CheckCircle, Settings2 } from 'lucide-react';
import { useDebugStore, selectIsPaused, selectCurrentNodeUuid } from '@/stores';
import { useFlowStore } from '@/stores';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
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
                onEdit={(_, newVal) => {
                  // 更新嵌套对象的值
                  const updated = { ...(value as Record<string, unknown>) };
                  updated[name] = newVal;
                  onEdit?.(name, updated);
                }}
              />
            ))}
          {isArray &&
            (value as unknown[]).map((item, idx) => (
              <VariableItem
                key={idx}
                name={`[${idx}]`}
                value={item}
                depth={depth + 1}
                onEdit={(_, newVal) => {
                  // 更新数组项的值
                  const updated = [...(value as unknown[])];
                  updated[idx] = newVal;
                  onEdit?.(name, updated);
                }}
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
  const variables = useDebugStore((state) => state.variables);
  const executionStack = useDebugStore((state) => state.executionStack);
  const currentNodeOutput = useDebugStore((state) => state.currentNodeOutput);
  const tempNodeConfigs = useDebugStore((state) => state.tempNodeConfigs);
  const modifyNodeConfig = useDebugStore((state) => state.modifyNodeConfig);

  const flow = useFlowStore((state) => state.flow);
  const focusOnNode = useFlowStore((state) => state.focusOnNode);

  // 获取当前节点信息
  const currentNode = flow?.nodes.find((n) => n.uuid === currentNodeUuid);

  // 编辑配置状态
  const [isEditingConfig, setIsEditingConfig] = useState(false);
  const [editConfigValue, setEditConfigValue] = useState('');

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

  // 开始编辑配置
  const handleStartEditConfig = () => {
    if (!currentNode) return;
    // 使用当前节点配置，如果有临时配置则使用临时配置
    const currentConfig = tempNodeConfigs[currentNode.uuid] || currentNode.data;
    setEditConfigValue(JSON.stringify(currentConfig, null, 2));
    setIsEditingConfig(true);
  };

  // 保存配置修改
  const handleSaveConfig = async () => {
    if (!currentNode) return;
    try {
      const parsedConfig = JSON.parse(editConfigValue) as NodeData;
      await modifyNodeConfig(currentNode.uuid, parsedConfig);
      setIsEditingConfig(false);
      toast.success('节点配置已临时修改');
    } catch (error) {
      if (error instanceof SyntaxError) {
        toast.error('JSON 格式错误');
      } else {
        toast.error(`修改配置失败: ${(error as Error).message}`);
      }
    }
  };

  // 取消编辑配置
  const handleCancelEditConfig = () => {
    setIsEditingConfig(false);
    setEditConfigValue('');
  };

  // 检查当前节点是否有临时配置
  const hasTempConfig = currentNode && tempNodeConfigs[currentNode.uuid] !== undefined;

  // 复制输出到剪贴板
  const handleCopyOutput = async () => {
    if (!currentNodeOutput) return;
    try {
      await navigator.clipboard.writeText(JSON.stringify(currentNodeOutput, null, 2));
      toast.success('已复制到剪贴板');
    } catch (error) {
      toast.error('复制失败');
    }
  };

  // 跳转到指定节点
  const handleNavigateToNode = (uuid: string) => {
    focusOnNode(uuid);
  };

  return (
    <div className="h-full flex flex-col bg-white overflow-hidden">
      {/* ===== 当前节点区域 ===== */}
      <div className="border-b border-slate-200 p-3">
        <h3 className="text-xs font-semibold text-slate-600 mb-2 flex items-center gap-1.5">
          <div className="w-1.5 h-1.5 rounded-full bg-blue-500" />
          当前节点
        </h3>
        {currentNode ? (
          <div className="space-y-2">
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
                {isPaused && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleStartEditConfig}
                    className="h-7 px-2 text-xs"
                    title="编辑节点配置（临时）"
                  >
                    <Settings2 className="w-3 h-3 mr-1" />
                    编辑配置
                  </Button>
                )}
              </div>
            </div>

            {/* 配置编辑区域 */}
            {isEditingConfig && (
              <div className="bg-slate-50 rounded p-2 space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-medium text-slate-600">临时配置</span>
                  <div className="flex items-center gap-1">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={handleSaveConfig}
                      className="h-6 px-2 text-xs text-green-600 hover:text-green-700 hover:bg-green-50"
                    >
                      <Check className="w-3 h-3 mr-1" />
                      保存
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={handleCancelEditConfig}
                      className="h-6 px-2 text-xs text-red-600 hover:text-red-700 hover:bg-red-50"
                    >
                      <X className="w-3 h-3 mr-1" />
                      取消
                    </Button>
                  </div>
                </div>
                <textarea
                  value={editConfigValue}
                  onChange={(e) => setEditConfigValue(e.target.value)}
                  className="w-full h-40 px-2 py-1 text-xs font-mono border border-slate-300 rounded resize-none focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder="输入 JSON 格式的节点配置"
                />
                <p className="text-xs text-slate-400">
                  注意：修改仅调试期间生效，调试结束后自动丢弃
                </p>
              </div>
            )}
          </div>
        ) : (
          <div className="text-sm text-slate-400 italic">无当前节点</div>
        )}
      </div>

      {/* ===== 变量区域 ===== */}
      <div className="flex-1 overflow-hidden flex flex-col border-b border-slate-200">
        <div className="p-3 border-b border-slate-100">
          <h3 className="text-xs font-semibold text-slate-600 flex items-center gap-1.5">
            <div className="w-1.5 h-1.5 rounded-full bg-green-500" />
            上下文变量
            <span className="text-slate-400 font-normal ml-auto">
              {Object.keys(variables).length}
            </span>
          </h3>
        </div>

        <div className="flex-1 overflow-y-auto">
          {Object.keys(variables).length > 0 ? (
            Object.entries(variables).map(([name, value]) => (
              <VariableItem
                key={name}
                name={name}
                value={value}
                onEdit={isPaused ? handleModifyVariable : undefined}
              />
            ))
          ) : (
            <div className="p-4 text-sm text-slate-400 text-center">暂无变量</div>
          )}
        </div>
      </div>

      {/* ===== 执行堆栈区域 ===== */}
      <div className="h-48 border-b border-slate-200 flex flex-col">
        <div className="p-3 border-b border-slate-100">
          <h3 className="text-xs font-semibold text-slate-600 flex items-center gap-1.5">
            <div className="w-1.5 h-1.5 rounded-full bg-purple-500" />
            执行堆栈
            <span className="text-slate-400 font-normal ml-auto">
              {executionStack.length}
            </span>
          </h3>
        </div>

        <div className="flex-1 overflow-y-auto">
          {executionStack.length > 0 ? (
            executionStack.map((entry) => (
              <StackEntryItem
                key={entry.node_uuid}
                entry={entry}
                isCurrentNode={entry.node_uuid === currentNodeUuid}
                onNavigateToNode={handleNavigateToNode}
              />
            ))
          ) : (
            <div className="p-4 text-sm text-slate-400 text-center">暂无执行记录</div>
          )}
        </div>
      </div>

      {/* ===== 最近节点输出区域 ===== */}
      <div className="h-48 flex flex-col">
        <div className="p-3 border-b border-slate-100 flex items-center justify-between">
          <h3 className="text-xs font-semibold text-slate-600 flex items-center gap-1.5">
            <div className="w-1.5 h-1.5 rounded-full bg-amber-500" />
            最近节点输出
          </h3>
          {currentNodeOutput !== null && currentNodeOutput !== undefined && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCopyOutput}
              className="h-6 px-2 text-xs"
            >
              <Copy className="w-3 h-3 mr-1" />
              复制
            </Button>
          )}
        </div>

        <div className="flex-1 overflow-y-auto p-3">
          {currentNodeOutput ? (
            <pre className="text-xs font-mono text-slate-700 whitespace-pre-wrap break-words">
              {JSON.stringify(currentNodeOutput, null, 2)}
            </pre>
          ) : (
            <div className="text-sm text-slate-400 text-center italic">暂无输出</div>
          )}
        </div>
      </div>
    </div>
  );
}

// 导入需要的函数
import { sendDebugCommand } from '@/services/tauri';
