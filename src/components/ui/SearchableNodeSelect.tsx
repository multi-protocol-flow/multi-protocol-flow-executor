/**
 * 可搜索的节点选择下拉组件
 *
 * 替代原生 select / shadcn Select，支持：
 * - 输入搜索过滤节点名称和类型
 * - 滚动限制高度
 * - 键盘导航 (Up/Down/Enter/Escape)
 * - 点击外部关闭
 * - 风格与 shadcn/ui 一致
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { ChevronDown, Search, X } from "lucide-react";
import { cn } from "@/lib/utils";

export interface SearchableNodeSelectItem {
  uuid: string;
  name: string;
  type: string;
}

interface SearchableNodeSelectProps {
  /** 当前选中的节点 UUID */
  value: string;
  /** 选择变更回调 */
  onValueChange: (value: string) => void;
  /** 可选节点列表 */
  items: SearchableNodeSelectItem[];
  /** 占位文本 */
  placeholder?: string;
  /** 空状态文本 */
  emptyText?: string;
  /** 搜索无结果显示文本 */
  noResultsText?: string;
  /** 搜索框占位文本 */
  searchPlaceholder?: string;
  /** 是否禁用 */
  disabled?: boolean;
}

export function SearchableNodeSelect({
  value,
  onValueChange,
  items,
  placeholder = "选择节点...",
  emptyText = "无可用节点",
  noResultsText = "未找到匹配节点",
  searchPlaceholder = "搜索节点名称或类型...",
  disabled = false,
}: SearchableNodeSelectProps) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [highlightedIndex, setHighlightedIndex] = useState(-1);

  const containerRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  /** 根据搜索词过滤节点 */
  const filteredItems = search.trim()
    ? items.filter(
        (item) =>
          item.name.toLowerCase().includes(search.toLowerCase()) ||
          item.type.toLowerCase().includes(search.toLowerCase())
      )
    : items;

  /** 当前选中的节点 */
  const selectedItem = items.find((item) => item.uuid === value);

  /** 打开时聚焦搜索框 */
  useEffect(() => {
    if (open) {
      // 延迟聚焦确保动画完成
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

  /** 选择节点 */
  const selectItem = useCallback(
    (uuid: string) => {
      onValueChange(uuid);
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
            prev < filteredItems.length - 1 ? prev + 1 : 0
          );
          break;
        case "ArrowUp":
          e.preventDefault();
          setHighlightedIndex((prev) =>
            prev > 0 ? prev - 1 : filteredItems.length - 1
          );
          break;
        case "Enter":
          e.preventDefault();
          if (highlightedIndex >= 0 && highlightedIndex < filteredItems.length) {
            selectItem(filteredItems[highlightedIndex].uuid);
          }
          break;
        case "Escape":
          e.preventDefault();
          setOpen(false);
          break;
      }
    },
    [open, filteredItems, highlightedIndex, selectItem]
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
          "flex h-9 w-full min-w-0 items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm text-left focus:outline-none focus:border-[3px] focus:border-ring",
          "disabled:cursor-not-allowed disabled:opacity-50",
          !selectedItem && "text-muted-foreground"
        )}
      >
        <span className="truncate min-w-0">
          {selectedItem
            ? `${selectedItem.name} (${selectedItem.type})`
            : placeholder}
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
            "absolute z-50 mt-1 w-full min-w-[14rem] rounded-md border border-border bg-popover text-popover-foreground shadow-md",
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
          {items.length === 0 ? (
            <div className="px-3 py-6 text-center text-sm text-muted-foreground">
              {emptyText}
            </div>
          ) : filteredItems.length === 0 ? (
            <div className="px-3 py-6 text-center text-sm text-muted-foreground">
              {noResultsText}
            </div>
          ) : (
            <div
              ref={listRef}
              className="max-h-60 overflow-y-auto p-1"
            >
              {filteredItems.map((item, index) => {
                const isSelected = item.uuid === value;
                const isHighlighted = index === highlightedIndex;
                return (
                  <button
                    key={item.uuid}
                    type="button"
                    data-index={index}
                    onClick={() => selectItem(item.uuid)}
                    onMouseEnter={() => setHighlightedIndex(index)}
                    className={cn(
                      "relative flex w-full cursor-default select-none items-center rounded-sm px-2 py-1.5 text-sm text-left outline-none transition-colors",
                      isHighlighted && "bg-accent text-accent-foreground",
                      isSelected && "font-medium"
                    )}
                  >
                    <span className="flex-1 truncate">{item.name}</span>
                    <span className="ml-2 shrink-0 rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground uppercase">
                      {item.type}
                    </span>
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
