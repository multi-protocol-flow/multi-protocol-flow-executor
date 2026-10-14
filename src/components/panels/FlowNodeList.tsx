/**
 * FlowNodeList - 流程节点大纲列表
 * 按加入时间排序，支持选择高亮、跳转定位、全选和批量删除
 */
import { useState, useMemo, useCallback } from 'react';
import {
  useFlowStore,
  useUIStore,
  selectAllStageNodes,
  type FlowStage,
} from '@/stores';
import { NODE_META, getNodeDisplayName } from '@/lib/nodeMeta';
import { getCachedNodeMeta } from '@/lib/NodeRegistry';
import { resolveBackendIcon } from '@/lib/nodeIcons';
import { cn } from '@/lib/utils';
import type { NodeType } from '@/types';
import { isEntryNodeType } from '@/types';
import { getLL } from '@/i18n/i18n-lazy';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { Trash2, List, Check, Minus } from 'lucide-react';

// ============================================================================
// 类型定义
// ============================================================================

interface FlowNodeListProps {
  /** 只读模式（调试时隐藏 checkbox 和删除按钮） */
  readOnly?: boolean;
  /** 是否显示独立卡片标题栏（嵌入侧边栏 CollapsiblePanel 时设为 false） */
  showHeader?: boolean;
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

export function FlowNodeList({ readOnly = false, showHeader = true }: FlowNodeListProps) {
  const ll = getLL().panels.flowNodeList;
  const stageLl = getLL().stages;
  // 三阶段节点按 准备 → 核心 → 收尾 顺序聚合展示（各阶段内保持加入顺序）
  const stageNodes = useFlowStore(selectAllStageNodes);
  const nodes = useMemo(() => stageNodes.map((entry) => entry.node), [stageNodes]);
  const stageLabel = useCallback(
    (stage: FlowStage) =>
      stage === 'setup'
        ? stageLl.short_setup()
        : stage === 'teardown'
          ? stageLl.short_teardown()
          : stageLl.short_main(),
    [stageLl],
  );
  const stageByUuid = useMemo(
    () => new Map(stageNodes.map((entry) => [entry.node.uuid, entry.stage])),
    [stageNodes],
  );
  const activeStage = useFlowStore((s) => s.activeStage);
  const setActiveStage = useFlowStore((s) => s.setActiveStage);
  const selectedNodeId = useFlowStore((s) => s.selectedNodeId);
  const [checkedIds, setCheckedIds] = useState<Set<string>>(new Set());

  // 计算各阶段内的入口节点数量（按阶段隔离）
  const entryCountByStage = useMemo(() => {
    const counts: Record<FlowStage, number> = { setup: 0, main: 0, teardown: 0 };
    for (const entry of stageNodes) {
      if (isEntryNodeType(entry.node.type)) {
        counts[entry.stage] = (counts[entry.stage] ?? 0) + 1;
      }
    }
    return counts;
  }, [stageNodes]);

  // 判断节点是否为受保护的孤立入口（所属阶段仅有该单一入口时禁止勾选删除）
  const isSolitaryEntryNode = useCallback(
    (uuid: string, type: string) => {
      if (!isEntryNodeType(type)) return false;
      const stage = stageByUuid.get(uuid);
      if (!stage) return true;
      return (entryCountByStage[stage] ?? 0) <= 1;
    },
    [stageByUuid, entryCountByStage]
  );

  // 可选节点数量（排除各阶段孤立入口节点）
  const selectableCount = useMemo(
    () => nodes.filter((n) => !isSolitaryEntryNode(n.uuid, n.type)).length,
    [nodes, isSolitaryEntryNode]
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
        if (!isSolitaryEntryNode(node.uuid, node.type)) {
          all.add(node.uuid);
        }
      }
      setCheckedIds(all);
    }
  }, [allChecked, nodes, isSolitaryEntryNode]);

  // 点击节点 → 先切到其所属阶段画布，再选中并居中定位
  const handleClickNode = useCallback(
    (uuid: string) => {
      const stage = stageByUuid.get(uuid);
      if (stage && stage !== activeStage) setActiveStage(stage);
      useFlowStore.getState().selectNode(uuid);
      useFlowStore.getState().focusOnNode(uuid);
    },
    [activeStage, setActiveStage, stageByUuid],
  );

  // 批量删除
  const handleBatchDelete = useCallback(() => {
    if (checkedIds.size === 0) return;
    const count = checkedIds.size;

    useUIStore.getState().showConfirm({
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
    if (!showHeader) {
      return (
        <div className="flex h-full items-center justify-center p-4">
          <p className="text-xs text-muted-foreground">{ll.empty()}</p>
        </div>
      );
    }
    return (
      <Card className="flex h-full flex-col overflow-hidden border-0 shadow-none">
        <CardHeader className="shrink-0 border-b border-muted bg-background px-2 pt-4 pb-3">
          <CardTitle className="flex items-center gap-2 text-sm font-medium">
            <List className="h-4 w-4 text-muted-foreground" />
            {ll.title()}
          </CardTitle>
        </CardHeader>
        <CardContent className="flex flex-1 items-center justify-center p-4">
          <p className="text-xs text-muted-foreground">{ll.empty()}</p>
        </CardContent>
      </Card>
    );
  }

  // 工具条内容（全选 + 批量删除）
  const toolbarContent = !readOnly && nodes.length > 1 && (
    <div className="flex items-center justify-between px-2 py-1.5 border-b border-border/40 bg-muted/20 text-xs shrink-0">
      <div className="flex items-center gap-1.5">
        <div className="w-5 shrink-0 flex items-center justify-center">
          <InlineCheckbox
            checked={allChecked ? true : someChecked ? 'indeterminate' : false}
            onChange={handleToggleAll}
          />
        </div>
        <span className="text-[11px] text-muted-foreground">
          {allChecked ? ll.deselectAll() : ll.selectAll()}
        </span>
      </div>

      {checkedIds.size > 0 && (
        <Button
          variant="ghost"
          size="sm"
          className="h-6 gap-1 px-1.5 text-xs text-destructive hover:text-destructive hover:bg-destructive/10"
          onClick={handleBatchDelete}
        >
          <Trash2 className="h-3 w-3" />
          {ll.deleteSelected({ count: checkedIds.size || '' })}
        </Button>
      )}
    </div>
  );

  const listContent = (
    <ScrollArea className="h-full">
      <div className="px-1.5 py-1 space-y-0.5">
        {nodes.map((node) => {
          const isEntry = isSolitaryEntryNode(node.uuid, node.type);
          const meta = NODE_META[node.type];
          // 插件节点 badge：内置节点用 NODE_META，插件用后端 display_name
          const badgeName = meta?.name ?? getNodeDisplayName(node.type);
          const hasBadge = Boolean(meta) || Boolean(getCachedNodeMeta(node.type));
          const isSelected = selectedNodeId === node.uuid;

          return (
            <div
              key={node.uuid}
              className={cn(
                'group flex items-center gap-1.5 rounded-md px-1.5 py-1 cursor-pointer transition-colors duration-100',
                isSelected
                  ? 'bg-primary/10 text-primary font-medium'
                  : 'hover:bg-accent text-foreground/90'
              )}
              onClick={() => handleClickNode(node.uuid)}
            >
              {/* Checkbox 列 - 固定宽度对齐 */}
              {!readOnly && (
                <div
                  className="w-5 shrink-0 flex items-center justify-center"
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
              <span className="truncate text-xs flex-1">{node.name}</span>

              {/* 所属阶段标记（Setup/Teardown 节点需与画布阶段区分） */}
              {stageByUuid.get(node.uuid) !== 'main' && (
                <span
                  className={cn(
                    'shrink-0 rounded px-1 py-0.5 text-[10px] font-medium leading-none',
                    stageByUuid.get(node.uuid) === 'setup'
                      ? 'bg-emerald-500/10 text-emerald-600'
                      : 'bg-amber-500/10 text-amber-600',
                  )}
                >
                  {stageLabel(stageByUuid.get(node.uuid) as FlowStage)}
                </span>
              )}

              {/* 节点类型标签 */}
              {hasBadge && (
                <span
                  className={cn(
                    'ml-auto shrink-0 rounded px-1 py-0.5 text-[10px] font-medium leading-none',
                    meta?.badge ?? 'bg-muted text-muted-foreground'
                  )}
                >
                  {badgeName}
                </span>
              )}
            </div>
          );
        })}
      </div>
    </ScrollArea>
  );

  if (!showHeader) {
    return (
      <div className="flex h-full flex-col overflow-hidden bg-background">
        {toolbarContent}
        <div className="flex-1 overflow-hidden min-h-0">
          {listContent}
        </div>
      </div>
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

          {/* 批量操作区 */}
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
        {listContent}
      </CardContent>
    </Card>
  );
}
