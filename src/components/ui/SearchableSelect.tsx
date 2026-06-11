/**
 * 可搜索的通用下拉选择组件
 *
 * 替代 shadcn Select，支持搜索过滤和固定高度滚动。
 * 适用于选项较多（10+）的场景。
 *
 * 用法与 Select 一致，仅 options 属性不同：
 *   <SearchableSelect
 *     value={value}
 *     onValueChange={setValue}
 *     options={[
 *       { value: "foo", label: "Foo" },
 *       { value: "bar", label: "Bar" },
 *     ]}
 *     placeholder="选择..."
 *   />
 *
 * 功能：
 * - 输入文字即时搜索过滤（大小写不敏感）
 * - 滚动限制高度 (max-h-60)，溢出可滚
 * - 键盘导航 (Up/Down/Enter/Escape)
 * - 点击外部关闭
 * - 风格与 shadcn/ui 一致
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { ChevronDown, Search, X } from "lucide-react";
import { cn } from "@/lib/utils";

export interface SearchableSelectOption {
  value: string;
  label: string;
}

interface SearchableSelectProps {
  /** 当前选中的值 */
  value: string;
  /** 选择变更回调 */
  onValueChange: (value: string) => void;
  /** 选项列表 */
  options: SearchableSelectOption[];
  /** 占位文本 */
  placeholder?: string;
  /** 搜索框占位文本 */
  searchPlaceholder?: string;
  /** 空状态文本 */
  emptyText?: string;
  /** 搜索无结果显示文本 */
  noResultsText?: string;
  /** 是否禁用 */
  disabled?: boolean;
  /** 自定义 trigger 类名 */
  className?: string;
}

export function SearchableSelect({
  value,
  onValueChange,
  options,
  placeholder = "Select...",
  searchPlaceholder = "Search...",
  emptyText = "No options available",
  noResultsText = "No matching results",
  disabled = false,
  className,
}: SearchableSelectProps) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [highlightedIndex, setHighlightedIndex] = useState(-1);

  const containerRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  /** 根据搜索词过滤选项 */
  const filteredOptions = search.trim()
    ? options.filter(
        (opt) =>
          opt.label.toLowerCase().includes(search.toLowerCase()) ||
          opt.value.toLowerCase().includes(search.toLowerCase())
      )
    : options;

  /** 当前选中的选项 */
  const selectedOption = options.find((opt) => opt.value === value);

  /** 打开时聚焦搜索框 */
  useEffect(() => {
    if (open) {
      const timer = setTimeout(() => {
        searchInputRef.current?.focus();
      }, 50);
      return () => clearTimeout(timer);
    } else {
      setSearch("");
      setHighlightedIndex(-1);
    }
  }, [open]);

  /** 点击外部关闭 */
  useEffect(() => {
    if (!open) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [open]);

  /** 选择选项 */
  const selectItem = useCallback(
    (val: string) => {
      onValueChange(val);
      setOpen(false);
    },
    [onValueChange]
  );

  /** 键盘导航 */
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (!open) {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          setOpen(true);
        }
        return;
      }

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setHighlightedIndex((prev) =>
            prev < filteredOptions.length - 1 ? prev + 1 : 0
          );
          break;
        case "ArrowUp":
          e.preventDefault();
          setHighlightedIndex((prev) =>
            prev > 0 ? prev - 1 : filteredOptions.length - 1
          );
          break;
        case "Enter":
          e.preventDefault();
          if (highlightedIndex >= 0 && highlightedIndex < filteredOptions.length) {
            selectItem(filteredOptions[highlightedIndex].value);
          }
          break;
        case "Escape":
          e.preventDefault();
          setOpen(false);
          break;
      }
    },
    [open, filteredOptions, highlightedIndex, selectItem]
  );

  /** 高亮变化时滚动到可见区域 */
  useEffect(() => {
    if (highlightedIndex < 0 || !listRef.current) return;
    const items = listRef.current.querySelectorAll<HTMLElement>("[data-index]");
    const target = items[highlightedIndex];
    if (target) {
      target.scrollIntoView({ block: "nearest" });
    }
  }, [highlightedIndex]);

  return (
    <div ref={containerRef} className="relative" onKeyDown={handleKeyDown}>
      {/* 触发按钮 */}
      <button
        type="button"
        disabled={disabled}
        onClick={() => setOpen((prev) => !prev)}
        className={cn(
          "flex h-8 w-full min-w-0 items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-xs text-left focus:outline-none focus:border-[3px] focus:border-ring",
          "disabled:cursor-not-allowed disabled:opacity-50",
          !selectedOption && "text-muted-foreground",
          className
        )}
      >
        <span className="truncate min-w-0" title={selectedOption?.label || placeholder}>
          {selectedOption ? selectedOption.label : placeholder}
        </span>
        <ChevronDown
          className={cn(
            "h-4 w-4 shrink-0 opacity-50 transition-transform",
            open && "rotate-180"
          )}
        />
      </button>

      {/* 下拉菜单 */}
      {open && (
        <div
          className={cn(
            "absolute z-50 mt-1 w-full min-w-[10rem] rounded-md border border-border bg-popover text-popover-foreground shadow-md",
            "animate-in fade-in-0 zoom-in-95"
          )}
        >
          {/* 搜索框 */}
          <div className="flex items-center border-b border-border px-3">
            <Search className="mr-2 h-4 w-4 shrink-0 opacity-50" />
            <input
              ref={searchInputRef}
              type="text"
              value={search}
              onChange={(e) => {
                setSearch(e.target.value);
                setHighlightedIndex(-1);
              }}
              placeholder={searchPlaceholder}
              className="flex h-9 w-full bg-transparent py-2 text-sm outline-none placeholder:text-muted-foreground"
            />
            {search && (
              <button
                type="button"
                onClick={() => setSearch("")}
                className="shrink-0 opacity-50 hover:opacity-100"
              >
                <X className="h-4 w-4" />
              </button>
            )}
          </div>

          {/* 列表 */}
          {options.length === 0 ? (
            <div className="px-3 py-6 text-center text-sm text-muted-foreground">
              {emptyText}
            </div>
          ) : filteredOptions.length === 0 ? (
            <div className="px-3 py-6 text-center text-sm text-muted-foreground">
              {noResultsText}
            </div>
          ) : (
            <div
              ref={listRef}
              className="max-h-60 overflow-y-auto p-1"
            >
              {filteredOptions.map((opt, index) => {
                const isSelected = opt.value === value;
                const isHighlighted = index === highlightedIndex;
                return (
                  <button
                    key={opt.value}
                    type="button"
                    title={opt.label}
                    data-index={index}
                    onClick={() => selectItem(opt.value)}
                    onMouseEnter={() => setHighlightedIndex(index)}
                    className={cn(
                      "relative flex w-full cursor-default select-none items-center rounded-sm px-2 py-1.5 text-sm text-left outline-none transition-colors",
                      isHighlighted && "bg-accent text-accent-foreground",
                      isSelected && "font-medium"
                    )}
                  >
                    <span className="flex-1 truncate">{opt.label}</span>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
