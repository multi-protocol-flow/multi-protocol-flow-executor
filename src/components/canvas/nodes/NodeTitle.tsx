import { memo, useState, useRef, useEffect } from 'react';
import { useUIStore, useFlowStore } from '@/stores';
import { cn } from '@/lib/utils';

export interface NodeTitleProps {
  nodeId: string;
  label?: string;
  defaultLabel: string;
  className?: string;
}

export const NodeTitle = memo(function NodeTitle({
  nodeId,
  label,
  defaultLabel,
  className,
}: NodeTitleProps) {
  const isRenaming = useUIStore((s) => s.renamingNodeId === nodeId);
  const setRenamingNodeId = useUIStore((s) => s.setRenamingNodeId);
  const updateNode = useFlowStore((s) => s.updateNode);
  const currentName = label || defaultLabel;
  const [value, setValue] = useState(currentName);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isRenaming) {
      setValue(label || defaultLabel);
      requestAnimationFrame(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      });
    }
  }, [isRenaming, label, defaultLabel]);

  if (isRenaming) {
    const handleCommit = () => {
      const trimmed = value.trim();
      if (trimmed && trimmed !== currentName) {
        updateNode(nodeId, { name: trimmed });
      }
      setRenamingNodeId(null);
    };

    const handleCancel = () => {
      setValue(currentName);
      setRenamingNodeId(null);
    };

    return (
      <input
        ref={inputRef}
        type="text"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            e.stopPropagation();
            handleCommit();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            e.stopPropagation();
            handleCancel();
          }
        }}
        onBlur={handleCommit}
        onClick={(e) => e.stopPropagation()}
        onDoubleClick={(e) => e.stopPropagation()}
        onMouseDown={(e) => e.stopPropagation()}
        onPointerDown={(e) => e.stopPropagation()}
        className={cn(
          'nodrag nopan w-full text-sm font-semibold text-foreground bg-background',
          'border border-primary rounded px-1 py-0.5 -mx-1 -my-0.5',
          'outline-none ring-1 ring-primary shadow-xs'
        )}
      />
    );
  }

  return (
    <span
      className={cn('text-sm font-semibold text-foreground break-words cursor-text select-text', className)}
      onDoubleClick={(e) => {
        e.stopPropagation();
        setRenamingNodeId(nodeId);
      }}
      title={currentName}
    >
      {currentName}
    </span>
  );
});
