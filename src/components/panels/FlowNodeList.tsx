/**
 * FlowNodeList - 流程节点列表面板
 * 按加入时间排序，批量删除，点击定位
 */
import { useState, useMemo, useCallback } from 'react';
import { useFlowStore, useUIStore, selectNodes } from '@/stores';
import { NODE_META, getNodeDisplayName } from '@/lib/nodeMeta';
import { getCachedNodeMeta } from '@/lib/NodeRegistry';
import { resolveBackendIcon } from '@/lib/nodeIcons';
import { cn } from '@/lib/utils';
import type { NodeType } from '@/types';
import { getLL } from '@/i18n/i18n-lazy';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Trash2, List, Check, Minus, ChevronDown, ChevronRight, Eye, Pencil, Info } from 'lucide-react';
import { MarkdownViewer } from '@/components/ui/MarkdownViewer';

// ============================================================================
// 类型定义
// ============================================================================

interface FlowNodeListProps {
  /** 只读模式（调试时隐藏 checkbox 和删除按钮） */
  readOnly?: boolean;
}

// ============================================================================
// 内联 Checkbox 组件
// ============================================================================

interface InlineCheckboxProps {
  checked: boolean | 'indeterminate';
  disabled?: boolean;
  onChange: () => void;
}

function InlineCheckbox({ checked, disabled, onChange }: InlineCheckboxProps) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={checked === 'indeterminate' ? 'mixed' : checked}
      disabled={disabled}
      onClick={onChange}
      className={cn(
        'h-3.5 w-3.5 shrink-0 rounded-[3px] border flex items-center justify-center',
        'transition-colors duration-100 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
        disabled
          ? 'border-muted-foreground/20 bg-muted/50 opacity-50 cursor-not-allowed'
          : checked === true
            ? 'border-primary bg-primary text-primary-foreground cursor-pointer'
            : checked === 'indeterminate'
              ? 'border-primary bg-primary text-primary-foreground cursor-pointer'
              : 'border-muted-foreground/40 bg-background cursor-pointer hover:border-primary/50'
      )}
    >
      {checked === true && <Check className="h-2.5 w-2.5" strokeWidth={3} />}
      {checked === 'indeterminate' && <Minus className="h-2.5 w-2.5" strokeWidth={3} />}
    </button>
  );
}

// ============================================================================
// 节点类型图标
// ============================================================================

function NodeTypeIcon({ type }: { type: NodeType }) {
  const meta = NODE_META[type];
  // 插件节点：静态 NODE_META 查不到时，从后端 metadata 兜底解析 icon/color
  const pluginMeta = getCachedNodeMeta(type);
  const Icon = meta?.icon ?? (pluginMeta ? resolveBackendIcon(pluginMeta.icon) : undefined);

  if (!Icon) {
    return (
      <div className="flex h-5 w-5 shrink-0 items-center justify-center rounded-[4px] bg-gray-400">
        <span className="text-[8px] font-bold text-white">?</span>
      </div>
    );
  }

  const bgStyle = !meta && pluginMeta?.color ? { backgroundColor: pluginMeta.color } : undefined;
  return (
    <div
      className={cn('flex h-5 w-5 shrink-0 items-center justify-center rounded-[4px]', !bgStyle && meta?.iconBg)}
      style={bgStyle}
    >
      <Icon className="h-3 w-3 text-white" />
    </div>
  );
}

// ============================================================================
// 主组件
// ============================================================================

export function FlowNodeList({ readOnly = false }: FlowNodeListProps) {
  const ll = getLL().panels.flowNodeList;
  const flowLl = getLL().flow;
  // 按加入时间排序（flow.nodes 的顺序即为加入顺序）
  const nodes = useFlowStore(selectNodes);
  const flow = useFlowStore((s) => s.flow);
  const updateFlowDescription = useFlowStore((s) => s.updateFlowDescription);
  const [checkedIds, setCheckedIds] = useState<Set<string>>(new Set());
  const [flowInfoExpanded, setFlowInfoExpanded] = useState(true);
  const [descMode, setDescMode] = useState<'edit' | 'preview'>('edit');

  // 可选节点数量（排除 entry）
  const selectableCount = useMemo(
    () => nodes.filter((n) => n.type !== 'entry').length,
    [nodes]
  );

  // 全选状态
  const allChecked = selectableCount > 0 && checkedIds.size === selectableCount;
  const someChecked = checkedIds.size > 0 && !allChecked;

  // 切换单个节点选中
  const handleToggleCheck = useCallback((uuid: string) => {
    setCheckedIds((prev) => {
      const next = new Set(prev);
      if (next.has(uuid)) {
        next.delete(uuid);
      } else {
        next.add(uuid);
      }
      return next;
    });
  }, []);

  // 全选/取消全选
  const handleToggleAll = useCallback(() => {
    if (allChecked) {
      setCheckedIds(new Set());
    } else {
      const all = new Set<string>();
      for (const node of nodes) {
        if (node.type !== 'entry') {
          all.add(node.uuid);
        }
      }
      setCheckedIds(all);
    }
  }, [allChecked, nodes]);

  // 点击节点 → 选中并居中定位，切换到节点配置面板
  const handleClickNode = useCallback((uuid: string) => {
    useFlowStore.getState().selectNode(uuid);
    useFlowStore.getState().focusOnNode(uuid);
  }, []);

// 批量删除
  const handleBatchDelete = useCallback(() => {
    if (checkedIds.size === 0) return;
    const count = checkedIds.size;

    useUIStore.getState().showConfirm({
      // 使用函数延迟求值，对话框渲染时调用 getLL() 获取当前语言文本
      title: () => getLL().panels.confirmDelete(),
      description: () => getLL().panels.deleteConfirmDesc({ count }),
      confirmText: () => getLL().panels.delete(),
      onConfirm: () => {
        useFlowStore.getState().removeNodes([...checkedIds]);
        setCheckedIds(new Set());
      },
    });
  }, [checkedIds]);

  // 空状态
  if (nodes.length === 0) {
    return (
      <Card className="flex h-full flex-col overflow-hidden border-0 shadow-none">
        <CardHeader className="shrink-0 border-b border-muted bg-background px-2 pt-4 pb-3">
          <CardTitle className="flex items-center gap-2 text-sm font-medium">
            <List className="h-4 w-4 text-muted-foreground" />
            {ll.title()}
          </CardTitle>
        </CardHeader>
        <CardContent className="flex flex-1 items-center justify-center p-4">
          <p className="text-sm text-muted-foreground">{ll.empty()}</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="flex h-full flex-col overflow-hidden border-0 shadow-none">
      <CardHeader className="shrink-0 border-b border-muted bg-background px-2 pt-4 pb-3">
        <div className="flex items-center justify-between px-1">
          <CardTitle className="flex items-center gap-2 text-sm font-medium">
            <List className="h-4 w-4 text-muted-foreground" />
            {ll.title()}
            <span className="text-xs text-muted-foreground font-normal">
              ({nodes.length})
            </span>
          </CardTitle>

          {/* 批量操作区 - 固定高度避免出现/消失时列表跳动 */}
          {!readOnly && (
            <div className="flex items-center gap-2 h-7">
              <Button
                variant="ghost"
                size="sm"
                className={cn(
                  'h-7 gap-1.5 px-2 text-destructive hover:text-destructive hover:bg-destructive/10 transition-opacity',
                  checkedIds.size > 0 ? 'opacity-100' : 'opacity-0 pointer-events-none'
                )}
                onClick={handleBatchDelete}
              >
                <Trash2 className="h-3.5 w-3.5" />
                {ll.deleteSelected({ count: checkedIds.size || '' })}
              </Button>
            </div>
          )}
        </div>

        {/* 全选行 */}
        {!readOnly && nodes.length > 1 && (
          <div className="flex items-center gap-1.5 mt-1.5 px-1">
            <div className="w-6 shrink-0 flex items-center justify-center">
              <InlineCheckbox
                checked={allChecked ? true : someChecked ? 'indeterminate' : false}
                onChange={handleToggleAll}
              />
            </div>
            <span className="text-xs text-muted-foreground">
              {allChecked ? ll.deselectAll() : ll.selectAll()}
            </span>
          </div>
        )}
      </CardHeader>

      <CardContent className="flex-1 overflow-hidden p-0">
        <ScrollArea className="h-full">
          {/* 流程信息区（可折叠） */}
          {flow && (
            <div className="border-b border-muted">
              <button
                type="button"
                onClick={() => setFlowInfoExpanded(!flowInfoExpanded)}
                className="flex w-full items-center gap-2 px-3 py-2 text-sm font-medium hover:bg-accent transition-colors"
              >
                {flowInfoExpanded ? (
                  <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                ) : (
                  <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                )}
                <Info className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                <span className="truncate">{flowLl.flow_info_title()}</span>
              </button>

              {flowInfoExpanded && (
                <div className="px-3 pb-3 space-y-3">
                  {/* 流程名称（只读显示） */}
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-muted-foreground shrink-0">{flowLl.flow_info_title()}:</span>
                    <span className="text-sm text-foreground truncate font-medium">{flow.name}</span>
                  </div>

                  {/* 流程描述（编辑/预览切换） */}
                  <div>
                    <div className="flex items-center justify-between mb-1">
                      <label className="text-xs font-medium text-muted-foreground">
                        {flowLl.flow_info_description()}
                      </label>
                      <button
                        type="button"
                        onClick={() => setDescMode(descMode === 'edit' ? 'preview' : 'edit')}
                        className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
                      >
                        {descMode === 'edit' ? (
                          <>
                            <Eye className="h-3 w-3" />
                            {flowLl.flow_info_preview()}
                          </>
                        ) : (
                          <>
                            <Pencil className="h-3 w-3" />
                            {flowLl.flow_info_edit()}
                          </>
                        )}
                      </button>
                    </div>

                    {descMode === 'edit' ? (
                      <Textarea
                        value={flow.description ?? ''}
                        onChange={(e) => updateFlowDescription(e.target.value)}
                        placeholder={flowLl.flow_info_description_placeholder()}
                        className="min-h-[72px] text-xs resize-y"
                      />
                    ) : (
                      <div className="min-h-[72px] rounded-md border border-input bg-background px-3 py-2">
                        {flow.description ? (
                          <MarkdownViewer content={flow.description} compact />
                        ) : (
                          <p className="text-xs text-muted-foreground italic">
                            {flowLl.flow_info_description_placeholder()}
                          </p>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          )}

          <div className="px-2 py-1.5 space-y-0.5">
            {nodes.map((node) => {
              const isEntry = node.type === 'entry';
              const meta = NODE_META[node.type];
              // 插件节点 badge：内置节点用 NODE_META，插件用后端 display_name
              const badgeName = meta?.name ?? getNodeDisplayName(node.type);
              const hasBadge = Boolean(meta) || Boolean(getCachedNodeMeta(node.type));

              return (
                <div
                  key={node.uuid}
                  className={cn(
                    'group flex items-center gap-1.5 rounded-md px-1 py-1.5 cursor-pointer',
                    'hover:bg-accent transition-colors duration-100'
                  )}
                  onClick={() => handleClickNode(node.uuid)}
                >
                  {/* Checkbox 列 - 固定宽度对齐 */}
                  {!readOnly && (
                    <div
                      className="w-6 shrink-0 flex items-center justify-center"
                      onClick={(e) => e.stopPropagation()}
                    >
                      <InlineCheckbox
                        checked={checkedIds.has(node.uuid)}
                        disabled={isEntry}
                        onChange={() => handleToggleCheck(node.uuid)}
                      />
                    </div>
                  )}

                  {/* 节点类型图标 */}
                  <NodeTypeIcon type={node.type} />

                  {/* 节点名称 */}
                  <span className="truncate text-sm text-foreground/90">{node.name}</span>

                  {/* 节点类型标签 */}
                  {hasBadge && (
                    <span className={cn('ml-auto shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium leading-none', meta?.badge ?? 'bg-muted text-muted-foreground')}>
                      {badgeName}
                    </span>
                  )}
                </div>
              );
            })}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
