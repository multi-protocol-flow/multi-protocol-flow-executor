import { useState, type ComponentType } from 'react';
import { Copy, Check, FolderOpen, FolderEdit, FileText, ExternalLink } from 'lucide-react';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';
import { isTauriEnv } from '@/lib/platform';
import { cn } from '@/lib/utils';

export interface PathSettingItemProps {
  label: string;
  description: string;
  path: string;
  icon: ComponentType<{ className?: string }>;
  isFile?: boolean;
  isLoading?: boolean;
  onSelectDirectory?: () => Promise<void> | void;
  onOpenDirectory?: () => Promise<void> | void;
  onOpenFile?: () => Promise<void> | void;
}

export function PathSettingItem({
  label,
  description,
  path,
  icon: Icon,
  isFile = false,
  isLoading = false,
  onSelectDirectory,
  onOpenDirectory,
  onOpenFile,
}: PathSettingItemProps) {
  const ll = getLL();
  const [copied, setCopied] = useState(false);

  const displayPath = isLoading
    ? (ll.settings.loading?.() ?? 'Loading...')
    : path || (ll.settings.not_set?.() ?? 'Not Set');

  const handleCopy = async () => {
    if (!path) return;
    try {
      await navigator.clipboard.writeText(path);
      setCopied(true);
      toast.success(ll.settings.path_copied?.() ?? 'Path copied to clipboard');
      setTimeout(() => setCopied(false), 1500);
    } catch {
      toast.error(ll.settings.copy_failed?.() ?? 'Failed to copy path');
    }
  };

  const handleOpenDirectory = async () => {
    if (!isTauriEnv()) {
      toast.info(ll.settings.cannot_open_in_browser?.() ?? 'Running in browser environment');
      return;
    }
    if (onOpenDirectory) {
      await onOpenDirectory();
    }
  };

  const handleOpenFile = async () => {
    if (!isTauriEnv()) {
      toast.info(ll.settings.cannot_open_in_browser?.() ?? 'Running in browser environment');
      return;
    }
    if (onOpenFile) {
      await onOpenFile();
    }
  };

  return (
    <div className="rounded-xl border border-border/60 bg-card p-3.5 space-y-2 transition-all hover:border-border/90">
      {/* 标题与描述 */}
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2.5">
          <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
            <Icon className="h-4 w-4 text-foreground/80" />
          </div>
          <div>
            <div className="text-sm font-medium text-foreground leading-snug">{label}</div>
            <p className="text-xs text-muted-foreground">{description}</p>
          </div>
        </div>
      </div>

      {/* 一体化路径栏：路径 + 内嵌微动作图标组 */}
      <div className="group relative flex items-center h-9 w-full rounded-lg border border-border/60 bg-muted/40 hover:bg-muted/60 transition-colors pl-3 pr-1.5 focus-within:border-primary/60 focus-within:ring-1 focus-within:ring-primary/20">
        {/* 路径文本（可单行截断并悬浮显示完整 Tooltip） */}
        <div
          title={path}
          className="flex-1 min-w-0 font-mono text-xs text-foreground/90 truncate select-all"
        >
          {displayPath}
        </div>

        {/* 内嵌微动作按钮组 */}
        <div className="flex items-center gap-0.5 shrink-0 ml-2">
          {/* 复制路径按钮 */}
          <button
            type="button"
            onClick={handleCopy}
            disabled={!path || isLoading}
            className={cn(
              'flex h-7 w-7 items-center justify-center rounded-md transition-colors cursor-pointer',
              'text-muted-foreground/70 hover:text-foreground hover:bg-background/80',
              'disabled:opacity-40 disabled:cursor-not-allowed',
              copied && 'text-green-600 bg-green-500/10'
            )}
            title={ll.settings.copy_path?.() ?? 'Copy Path'}
          >
            {copied ? (
              <Check className="h-3.5 w-3.5 text-green-600 stroke-[2.5]" />
            ) : (
              <Copy className="h-3.5 w-3.5" />
            )}
          </button>

          {/* 细分隔线 */}
          <div className="h-3.5 w-px bg-border/50 mx-0.5 shrink-0" />

          {isFile ? (
            <>
              {onOpenFile && (
                <button
                  type="button"
                  onClick={handleOpenFile}
                  disabled={!path || isLoading}
                  className="flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground/70 hover:text-foreground hover:bg-background/80 transition-colors disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
                  title={ll.settings.open_file?.() ?? 'Open File'}
                >
                  <FileText className="h-3.5 w-3.5" />
                </button>
              )}
              {onOpenDirectory && (
                <button
                  type="button"
                  onClick={handleOpenDirectory}
                  disabled={!path || isLoading}
                  className="flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground/70 hover:text-foreground hover:bg-background/80 transition-colors disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
                  title={ll.settings.open_in_folder?.() ?? 'Open Enclosing Folder'}
                >
                  <FolderOpen className="h-3.5 w-3.5" />
                </button>
              )}
            </>
          ) : (
            <>
              {onSelectDirectory && (
                <button
                  type="button"
                  onClick={onSelectDirectory}
                  disabled={isLoading}
                  className="flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground/70 hover:text-foreground hover:bg-background/80 transition-colors disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
                  title={ll.settings.select_dir?.() ?? 'Select Directory'}
                >
                  <FolderEdit className="h-3.5 w-3.5" />
                </button>
              )}
              {onOpenDirectory && (
                <button
                  type="button"
                  onClick={handleOpenDirectory}
                  disabled={!path || isLoading}
                  className="flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground/70 hover:text-foreground hover:bg-background/80 transition-colors disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
                  title={ll.settings.open_dir?.() ?? 'Open Directory'}
                >
                  <ExternalLink className="h-3.5 w-3.5" />
                </button>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
