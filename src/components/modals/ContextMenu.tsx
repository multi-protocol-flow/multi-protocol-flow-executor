import React from 'react';
import { Settings, Code, Copy, Trash2, Info } from 'lucide-react';

interface ContextMenuProps {
  x: number;
  y: number;
  onClose: () => void;
  onRename: () => void;
  onCopy: () => void;
  onDelete: () => void;
  onInfo: () => void;
  onChangeType?: () => void;
  nodeType: string;
}

export const ContextMenu = ({
  x,
  y,
  onClose,
  onRename,
  onCopy,
  onDelete,
  onInfo,
  onChangeType,
  nodeType
}: ContextMenuProps) => {
  return (
    <div
      className="fixed z-50 bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded-lg shadow-xl py-1 min-w-[140px]"
      style={{ left: x, top: y }}
      onClick={(e) => e.stopPropagation()}
    >
      <button
        onClick={() => { onRename(); onClose(); }}
        className="w-full text-left px-4 py-2 text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800 flex items-center gap-2"
      >
        <Settings size={14} /> 重命名
      </button>
      {nodeType === "task" && onChangeType && (
        <button
          onClick={() => { onChangeType(); onClose(); }}
          className="w-full text-left px-4 py-2 text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800 flex items-center gap-2"
        >
          <Code size={14} /> 更改类型
        </button>
      )}
      <button
        onClick={() => { onCopy(); onClose(); }}
        className="w-full text-left px-4 py-2 text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800 flex items-center gap-2"
      >
        <Copy size={14} /> 复制
      </button>
      <button
        onClick={() => { onDelete(); onClose(); }}
        className="w-full text-left px-4 py-2 text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800 flex items-center gap-2 text-red-500"
      >
        <Trash2 size={14} /> 删除
      </button>
      <button
        onClick={() => { onInfo(); onClose(); }}
        className="w-full text-left px-4 py-2 text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800 flex items-center gap-2 border-t border-zinc-100 dark:border-zinc-800"
      >
        <Info size={14} /> 查看信息
      </button>
    </div>
  );
};
