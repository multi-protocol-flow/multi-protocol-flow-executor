/**
 * 变量选择器组件
 * 下拉框显示所有可用变量，支持搜索过滤和按节点分组
 */
import * as React from "react";
import * as PopoverPrimitive from "@radix-ui/react-popover";
import { Variable } from "@/types/variable";
import { cn } from "@/lib/utils";
import { Input } from "./input";
import { ScrollArea } from "./scroll-area";
import { Braces, Search, X, Sparkles, BookOpen, ChevronDown } from "lucide-react";
import {
  DYNAMIC_VARIABLES,
  type DynamicVariableItem,
} from "@/types/dynamicVariable";
import { useUIStore } from '@/stores/ui';
import { getLL } from '@/i18n/i18n-lazy';

const LL = getLL();

// ============================================================================
// Popover 基础组件 (shadcn/ui 风格)
// ============================================================================

const Popover = PopoverPrimitive.Root;
const PopoverTrigger = PopoverPrimitive.Trigger;

const PopoverContent = React.forwardRef<
  React.ComponentRef<typeof PopoverPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof PopoverPrimitive.Content>
>(({ className, align = "center", sideOffset = 4, ...props }, ref) => (
  <PopoverPrimitive.Portal>
    <PopoverPrimitive.Content
      ref={ref}
      align={align}
      sideOffset={sideOffset}
      className={cn(
        "z-50 w-72 rounded-md border border-input bg-background p-0 text-popover-foreground shadow-md",
        "data-[state=open]:animate-in data-[state=closed]:animate-out",
        "data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
        "data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95",
        "data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2",
        "data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2",
        className
      )}
      {...props}
    />
  </PopoverPrimitive.Portal>
));
PopoverContent.displayName = PopoverPrimitive.Content.displayName;

// ============================================================================
// VariableSelector 组件
// ============================================================================

export interface VariableSelectorProps {
  /** 所有可用变量 */
  variables: Variable[];
  /** 选择回调，返回变量名 */
  onSelect: (variableName: string) => void;
  /** 受控打开状态 */
  open?: boolean;
  /** 状态变更回调 */
  onOpenChange?: (open: boolean) => void;
  /** 节点 ID -> 显示名称的映射 */
  nodeLabels?: Record<string, string>;
  /** 触发按钮的 className */
  triggerClassName?: string;
  /** 触发按钮的子元素（自定义触发器） */
  triggerChildren?: React.ReactNode;
  /** 占位文本 */
  placeholder?: string;
  /** 空状态文本 */
  emptyText?: string;
}

/**
 * 变量选择器
 * 
 * @example
 * ```tsx
 * <VariableSelector
 *   variables={variables}
 *   nodeLabels={{ "node-1": "Entry 节点", "node-2": "HTTP 请求" }}
 *   onSelect={(name) => insertVariable(name)}
 * />
 * ```
 */
export function VariableSelector({
  variables,
  onSelect,
  open: controlledOpen,
  onOpenChange,
  nodeLabels = {},
  triggerClassName,
  triggerChildren,
  placeholder = LL.flow.variable_selector_search(),
  emptyText = LL.flow.variable_selector_empty(),
}: VariableSelectorProps) {
  const LL = getLL();

  // 搜索状态
  const [searchQuery, setSearchQuery] = React.useState("");
  const [uncontrolledOpen, setUncontrolledOpen] = React.useState(false);

  
  // 支持受控和非受控模式
  const open = controlledOpen ?? uncontrolledOpen;
  const setOpen = onOpenChange ?? setUncontrolledOpen;

  // 过滤变量
  const filteredVariables = React.useMemo(() => {
    if (!searchQuery.trim()) return variables;
    const query = searchQuery.toLowerCase();
    return variables.filter(
      (v) =>
        v.name.toLowerCase().includes(query) ||
        v.description?.toLowerCase().includes(query)
    );
  }, [variables, searchQuery]);

  // 按节点分组
  const groupedVariables = React.useMemo(() => {
    const groups = new Map<string, Variable[]>();
    
    for (const variable of filteredVariables) {
      const nodeId = variable.sourceNodeUuid;
      if (!groups.has(nodeId)) {
        groups.set(nodeId, []);
      }
      groups.get(nodeId)!.push(variable);
    }
    
    return groups;
  }, [filteredVariables]);

  // 过滤动态变量
  const filteredDynamicVariables = React.useMemo(() => {
    if (!searchQuery.trim()) return DYNAMIC_VARIABLES;
    const query = searchQuery.toLowerCase();
    return DYNAMIC_VARIABLES.filter(
      (v) =>
        v.name.toLowerCase().includes(query) ||
        v.category.toLowerCase().includes(query)
    );
  }, [searchQuery]);

  // 处理变量选择
  const handleSelect = React.useCallback(
    (variableName: string) => {
      onSelect(variableName);
      setOpen(false);
      setSearchQuery("");
    },
    [onSelect, setOpen]
  );

  // 清空搜索
  const clearSearch = React.useCallback(() => {
    setSearchQuery("");
  }, []);

  return (
    <>
      <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        {triggerChildren ? (
          <button
            type="button"
            className={cn(
              "inline-flex items-center justify-center",
              triggerClassName
            )}
          >
            {triggerChildren}
          </button>
        ) : (
          <button
            type="button"
            className={cn(
              "inline-flex items-center gap-1.5 px-2.5 py-1.5",
              "text-xs font-medium rounded-md",
              "border border-border bg-background",
              "hover:bg-accent hover:border-muted-foreground",
              "focus:outline-none focus:ring-2 focus:ring-muted-foreground focus:ring-offset-1",
              "transition-colors duration-150",
              "disabled:opacity-50 disabled:cursor-not-allowed",
              triggerClassName
            )}
          >
            <Braces className="h-3.5 w-3.5 text-muted-foreground" />
            <span className="text-foreground">{LL.flow.variable_selector_label()}</span>
          </button>
        )}
      </PopoverTrigger>

      <PopoverContent className="w-80" align="start">
        {/* 搜索框 */}
        <div className="p-2 border-b border-border">
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
            <Input
              type="text"
              placeholder={placeholder}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-8 pr-8 h-8 text-xs"
              autoFocus
            />
            {searchQuery && (
              <button
                type="button"
                onClick={clearSearch}
                className="absolute right-2 top-1/2 -translate-y-1/2 p-0.5 rounded hover:bg-accent"
              >
                <X className="h-3 w-3 text-muted-foreground" />
              </button>
            )}
          </div>
        </div>

        {/* 变量列表 */}
        <ScrollArea className="h-64">
          {groupedVariables.size === 0 && !searchQuery.trim() ? (
            <>
              <div className="flex items-center justify-center h-16 text-xs text-muted-foreground">
                {emptyText}
              </div>
              <DynamicVariableGroup
                variables={filteredDynamicVariables}
                onSelect={handleSelect}
              />
            </>
          ) : groupedVariables.size === 0 && filteredDynamicVariables.length === 0 ? (
            <div className="flex items-center justify-center h-20 text-xs text-muted-foreground">
              {emptyText}
            </div>
          ) : (
            <div className="py-1">
              {Array.from(groupedVariables.entries()).map(([nodeId, vars]) => (
                <VariableGroup
                  key={nodeId}
                  nodeId={nodeId}
                  nodeLabel={nodeLabels[nodeId]}
                  variables={vars}
                  onSelect={handleSelect}
                />
              ))}
              <DynamicVariableGroup
                variables={filteredDynamicVariables}
                onSelect={handleSelect}
              />
            </div>
          )}
        </ScrollArea>

        {/* 动态变量参考手册入口 */}
        <div className="px-3 py-2 border-t border-border bg-emerald-50/40 dark:bg-emerald-950/20 flex items-center justify-between">
          <span className="text-[11px] text-muted-foreground truncate mr-2">
            {LL.flow.dynamic_variables_selector_hint()}
          </span>
          <button
            type="button"
            onClick={() => {
              setOpen(false);
              useUIStore.getState().openDynamicVariablesHelpDialog();
            }}
            className="text-[11px] text-emerald-600 dark:text-emerald-400 hover:text-emerald-700 dark:hover:text-emerald-300 font-medium hover:underline flex items-center gap-1 shrink-0"
          >
            <BookOpen className="h-3 w-3" />
            <span>{LL.flow.dynamic_variables_open_handbook()}</span>
          </button>
        </div>

        {/* 底部提示 */}
        <div className="px-3 py-2 border-t border-border bg-muted/50">
          <p className="text-[10px] text-muted-foreground">
            {LL.flow.variable_selector_hint()} <code className="px-1 py-0.5 bg-muted rounded text-muted-foreground">{"{{" + LL.common.variable_name_example() + "}}"}</code>
          </p>
        </div>
      </PopoverContent>
      </Popover>

    </>
  );
}

// ============================================================================
// 子组件: 变量分组
// ============================================================================

interface VariableGroupProps {
  nodeId: string;
  nodeLabel?: string;
  variables: Variable[];
  onSelect: (variableName: string) => void;
}

function VariableGroup({ nodeId, nodeLabel, variables, onSelect }: VariableGroupProps) {
  const LL = getLL();
  // 生成显示名称：优先使用 nodeLabel，否则截取 UUID
  const displayName = nodeLabel ?? LL.flow.variable_node_prefix({ id: nodeId.slice(0, 8) + '...' });

  return (
    <div className="px-1">
      {/* 分组标题 */}
      <div className="px-2 py-1.5 text-[10px] font-semibold text-muted-foreground uppercase tracking-wider bg-muted/80 rounded-sm mb-0.5">
        {displayName}
      </div>

      {/* 变量列表 */}
      <ul className="space-y-0.5">
        {variables.map((variable) => (
          <VariableItem
            key={variable.name}
            variable={variable}
            onSelect={onSelect}
          />
        ))}
      </ul>
    </div>
  );
}

// ============================================================================
// 子组件: 变量项
// ============================================================================

interface VariableItemProps {
  variable: Variable;
  onSelect: (variableName: string) => void;
}

function VariableItem({ variable, onSelect }: VariableItemProps) {
  return (
    <li>
      <button
        type="button"
        onClick={() => onSelect(variable.name)}
        className={cn(
          "w-full text-left px-2 py-1.5 rounded-sm",
          "text-xs font-mono",
          "hover:bg-accent focus:bg-accent focus:outline-none",
          "transition-colors duration-100",
          "group"
        )}
      >
        <div className="flex items-center justify-between gap-2">
          <span className="text-foreground group-hover:text-foreground">
            {variable.name}
          </span>
          <span className="text-muted-foreground group-hover:text-muted-foreground text-[10px]">
            {"{{" + variable.name + "}}"}
          </span>
        </div>
        {variable.description && (
          <p className="mt-0.5 text-[10px] text-muted-foreground truncate">
            {variable.description}
          </p>
        )}
      </button>
    </li>
  );
}


// ============================================================================
// 子组件: 动态变量分组
// ============================================================================

interface DynamicVariableGroupProps {
  variables: DynamicVariableItem[];
  onSelect: (variableName: string) => void;
}

function DynamicVariableGroup({ variables, onSelect }: DynamicVariableGroupProps) {
  const LL = getLL();
  const [collapsed, setCollapsed] = React.useState(false);

  if (variables.length === 0) return null;

  return (
    <div className="px-1 mt-1 pt-1 border-t border-border">
      <button
        type="button"
        onClick={() => setCollapsed(!collapsed)}
        className={cn(
          "w-full flex items-center justify-between px-2 py-1",
          "text-[11px] font-semibold text-emerald-700 dark:text-emerald-400",
          "hover:bg-accent rounded-sm transition-colors",
          "focus:outline-none"
        )}
      >
        <span className="flex items-center gap-1.5">
          <Sparkles className="h-3 w-3 text-emerald-600 dark:text-emerald-400 shrink-0" />
          <span>{LL.flow.dynamic_variables_group()}</span>
          <span className="text-[10px] text-muted-foreground font-normal">
            ({variables.length})
          </span>
        </span>
        <ChevronDown
          className={cn(
            "h-3 w-3 text-muted-foreground transition-transform duration-150",
            collapsed && "-rotate-90"
          )}
        />
      </button>

      {!collapsed && (
        <ul className="mt-0.5 space-y-0.5">
          {variables.map((item) => (
            <li key={item.name}>
              <button
                type="button"
                onClick={() => onSelect(item.name)}
                className={cn(
                  "w-full text-left px-2 py-1.5 rounded-sm",
                  "text-xs font-mono",
                  "hover:bg-accent focus:bg-accent focus:outline-none",
                  "transition-colors duration-100",
                  "group"
                )}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="text-emerald-600 dark:text-emerald-400 font-medium">
                    {item.name}
                  </span>
                  <span className="text-muted-foreground text-[10px] truncate max-w-[120px]">
                    {item.example}
                  </span>
                </div>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
export default VariableSelector;
