/**
 * LocalExecViewer - 本地命令节点执行详情查看器
 *
 * 展示：命令行（等宽 + 复制）、退出码徽章、耗时、stdout/stderr 折叠区
 * （各自带截断提示）、产物 `media` 预览（复用前端 `MediaPreview`）。
 */
import { useMemo, useState } from 'react';
import { ChevronDown, ChevronRight, Clock, Film, Terminal, ArrowDownToLine } from 'lucide-react';
import type { NodeExecutionReport } from '@/types';
import { Badge } from '@/components/ui/badge';
import { CopyButton } from '@/components/common';
import { getLL } from '@/i18n/i18n-lazy';
import { detectMedia } from './mediaDetection';
import { MediaPreview } from './MediaPreview';

interface LocalExecViewerProps {
  /** 节点执行报告 */
  report: NodeExecutionReport;
}

interface CollapsibleSectionProps {
  title: string;
  icon: React.ReactNode;
  defaultOpen?: boolean;
  extra?: React.ReactNode;
  children: React.ReactNode;
}

/** 折叠面板（与其它 viewer 保持一致的交互与视觉） */
function CollapsibleSection({ title, icon, defaultOpen = false, extra, children }: CollapsibleSectionProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      <div className="flex flex-wrap items-center justify-between w-full bg-muted/30 hover:bg-muted/50 transition-colors gap-2 pr-3">
        <button
          type="button"
          onClick={() => setIsOpen(!isOpen)}
          className="flex-1 min-w-0 flex items-center gap-2 p-3 text-left"
          aria-expanded={isOpen}
        >
          <div className="text-muted-foreground shrink-0">
            {isOpen ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
          </div>
          <div className="text-muted-foreground shrink-0">{icon}</div>
          <span className="font-medium text-foreground text-sm truncate min-w-0">{title}</span>
        </button>
        {extra && <div className="shrink-0 pl-3 sm:pl-0 py-1">{extra}</div>}
      </div>
      {isOpen && <div className="p-3 border-t border-border bg-background">{children}</div>}
    </div>
  );
}

/** 退出码徽章配色 */
function exitCodeClass(exitCode: number | null): string {
  if (exitCode === 0) return 'bg-emerald-100 text-emerald-700';
  if (exitCode === null) return 'bg-amber-100 text-amber-700';
  return 'bg-red-100 text-red-700';
}

/**
 * 本地命令节点查看器
 */
export function LocalExecViewer({ report }: LocalExecViewerProps) {
  const ll = getLL().execution.viewers;
  const output = (report.output_data ?? {}) as Record<string, unknown>;

  const command = (output.command ?? {}) as { mode?: string; display?: string };
  const display = command.display ?? '';
  const exitCode = typeof output.exit_code === 'number' ? output.exit_code : null;
  const durationMs = typeof output.duration_ms === 'number' ? output.duration_ms : null;
  const stdout = typeof output.stdout === 'string' ? output.stdout : '';
  const stderr = typeof output.stderr === 'string' ? output.stderr : '';
  const stdoutTruncated = output.stdout_truncated === true;
  const stderrTruncated = output.stderr_truncated === true;
  const cwd = typeof output.cwd === 'string' ? output.cwd : '';
  const error = typeof output.error === 'string' ? output.error : '';

  const mediaCandidates = useMemo(() => detectMedia(output), [output]);

  return (
    <div className="space-y-3 p-4">
      {/* 命令行 */}
      <div className="rounded-lg border border-border bg-muted/20 p-3 space-y-1.5">
        <div className="flex items-center justify-between gap-2">
          <span className="text-xs font-medium text-muted-foreground">
            {command.mode === 'shell' ? 'shell' : 'argv'}
          </span>
          <div className="flex items-center gap-2">
            {exitCode !== null && (
              <Badge className={exitCodeClass(exitCode)}>exit {exitCode}</Badge>
            )}
            {durationMs !== null && (
              <span className="flex items-center gap-1 text-xs text-muted-foreground">
                <Clock className="w-3 h-3" />
                {durationMs}ms
              </span>
            )}
            {display && <CopyButton value={display} format="raw" />}
          </div>
        </div>
        <pre className="font-mono text-xs whitespace-pre-wrap break-all text-foreground">
          {display}
        </pre>
        {cwd && (
          <div className="text-xs text-muted-foreground">
            cwd: <span className="font-mono">{cwd}</span>
          </div>
        )}
      </div>

      {error && (
        <div className="rounded-lg border border-red-200 bg-red-50 p-3 text-xs text-red-700 whitespace-pre-wrap">
          {error}
        </div>
      )}

      {/* stdout */}
      <CollapsibleSection
        title={ll.stdout()}
        icon={<Terminal className="w-4 h-4" />}
        defaultOpen={stdout !== ''}
        extra={
          stdoutTruncated ? <Badge className="bg-amber-100 text-amber-700">{ll.truncated()}</Badge> : null
        }
      >
        <pre className="font-mono text-xs whitespace-pre-wrap break-all max-h-72 overflow-auto">
          {stdout === '' ? ll.empty_output() : stdout}
        </pre>
      </CollapsibleSection>

      {/* stderr */}
      <CollapsibleSection
        title={ll.stderr()}
        icon={<ArrowDownToLine className="w-4 h-4" />}
        defaultOpen={stderr !== ''}
        extra={
          stderrTruncated ? <Badge className="bg-amber-100 text-amber-700">{ll.truncated()}</Badge> : null
        }
      >
        <pre className="font-mono text-xs whitespace-pre-wrap break-all max-h-72 overflow-auto">
          {stderr === '' ? ll.empty_output() : stderr}
        </pre>
      </CollapsibleSection>

      {/* 产物 */}
      {mediaCandidates.length > 0 && (
        <CollapsibleSection
          title={ll.artifact()}
          icon={<Film className="w-4 h-4" />}
          defaultOpen={true}
        >
          <MediaPreview candidates={mediaCandidates} nodeName={report.node_name} />
        </CollapsibleSection>
      )}
    </div>
  );
}
