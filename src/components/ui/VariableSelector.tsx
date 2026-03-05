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
import { Braces, Search, X } from "lucide-react";

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
        "z-50 w-72 rounded-md border border-black bg-white p-0 text-popover-foreground shadow-md",
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
  placeholder = "搜索变量...",
  emptyText = "暂无变量",
}: VariableSelectorProps) {
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
              "border border-slate-300 bg-white",
              "hover:bg-slate-50 hover:border-slate-400",
              "focus:outline-none focus:ring-2 focus:ring-slate-400 focus:ring-offset-1",
              "transition-colors duration-150",
              "disabled:opacity-50 disabled:cursor-not-allowed",
              triggerClassName
            )}
          >
            <Braces className="h-3.5 w-3.5 text-slate-600" />
            <span className="text-slate-700">变量</span>
          </button>
        )}
      </PopoverTrigger>

      <PopoverContent className="w-80" align="start">
        {/* 搜索框 */}
        <div className="p-2 border-b border-slate-200">
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-slate-400" />
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
                className="absolute right-2 top-1/2 -translate-y-1/2 p-0.5 rounded hover:bg-slate-100"
              >
                <X className="h-3 w-3 text-slate-400" />
              </button>
            )}
          </div>
        </div>

        {/* 变量列表 */}
        <ScrollArea className="h-64">
          {groupedVariables.size === 0 ? (
            <div className="flex items-center justify-center h-20 text-xs text-slate-400">
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
            </div>
          )}
        </ScrollArea>

        {/* 底部提示 */}
        <div className="px-3 py-2 border-t border-slate-100 bg-slate-50/50">
          <p className="text-[10px] text-slate-400">
            点击变量名插入 <code className="px-1 py-0.5 bg-slate-100 rounded text-slate-500">{"{{变量名}}"}</code>
          </p>
        </div>
      </PopoverContent>
    </Popover>
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
  // 生成显示名称：优先使用 nodeLabel，否则截取 UUID
  const displayName = nodeLabel ?? `节点 ${nodeId.slice(0, 8)}...`;

  return (
    <div className="px-1">
      {/* 分组标题 */}
      <div className="px-2 py-1.5 text-[10px] font-semibold text-slate-500 uppercase tracking-wider bg-slate-50/80 rounded-sm mb-0.5">
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
          "hover:bg-slate-100 focus:bg-slate-100 focus:outline-none",
          "transition-colors duration-100",
          "group"
        )}
      >
        <div className="flex items-center justify-between gap-2">
          <span className="text-slate-700 group-hover:text-slate-900">
            {variable.name}
          </span>
          <span className="text-slate-400 group-hover:text-slate-500 text-[10px]">
            {"{{" + variable.name + "}}"}
          </span>
        </div>
        {variable.description && (
          <p className="mt-0.5 text-[10px] text-slate-400 truncate">
            {variable.description}
          </p>
        )}
      </button>
    </li>
  );
}

export default VariableSelector;
