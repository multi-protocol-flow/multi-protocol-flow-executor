import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import {
  Download,
  ExternalLink,
  Sparkles,
  CheckCircle2,
  AlertCircle,
  Loader2,
  Calendar,
  Layers,
} from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useUpdateStore } from '@/stores/update';
import { getLL } from '@/i18n/i18n-lazy';

async function openExternalUrl(url: string) {
  try {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  } catch {
    window.open(url, '_blank');
  }
}

function formatBytes(bytes?: number | null): string {
  if (bytes == null || isNaN(bytes)) return '-- MB';
  const mb = bytes / (1024 * 1024);
  return `${mb.toFixed(1)} MB`;
}

function formatSpeed(bytesPerSec?: number): string {
  if (!bytesPerSec || bytesPerSec <= 0) return '-- MB/s';
  const mbps = bytesPerSec / (1024 * 1024);
  return `${mbps.toFixed(2)} MB/s`;
}

export function UpdateDialog() {
  const ll = getLL();
  const showUpdateDialog = useUpdateStore((s) => s.showUpdateDialog);
  const closeUpdateDialog = useUpdateStore((s) => s.closeUpdateDialog);
  const updateInfo = useUpdateStore((s) => s.updateInfo);
  const status = useUpdateStore((s) => s.status);
  const progress = useUpdateStore((s) => s.progress);
  const error = useUpdateStore((s) => s.error);
  const startDownload = useUpdateStore((s) => s.startDownload);
  const installAndRestart = useUpdateStore((s) => s.installAndRestart);
  const ignoreCurrentVersion = useUpdateStore((s) => s.ignoreCurrentVersion);

  if (!updateInfo) {
    return null;
  }

  const isDownloading = status === 'downloading';
  const isDownloaded = status === 'downloaded';
  const isError = status === 'error';

  const formattedDate = updateInfo.releaseDate
    ? new Date(updateInfo.releaseDate).toLocaleDateString()
    : null;

  return (
    <Dialog open={showUpdateDialog} onOpenChange={(open) => !open && closeUpdateDialog()}>
      <DialogContent className="max-w-xl w-[90vw] p-0 gap-0 overflow-hidden flex flex-col bg-background sm:rounded-xl shadow-2xl border-border">
        {/* 对话框头部 */}
        <DialogHeader className="px-6 pt-5 pb-4 border-b border-border/70 text-left bg-muted/10 shrink-0">
          <div className="flex items-center justify-between pr-6">
            <div className="flex items-center gap-2.5">
              <div className="p-2 rounded-lg bg-primary/10 text-primary">
                <Sparkles className="h-5 w-5" />
              </div>
              <div>
                <DialogTitle className="text-base font-semibold text-foreground flex items-center gap-2">
                  <span>{ll.settings.update_available()}</span>
                  <Badge variant="default" className="text-xs px-2 py-0.5 font-mono">
                    v{updateInfo.latestVersion}
                  </Badge>
                  {updateInfo.isPrerelease && (
                    <Badge variant="secondary" className="text-[11px] px-1.5 py-0 text-amber-600 bg-amber-500/10 border-amber-500/20">
                      Pre-release
                    </Badge>
                  )}
                </DialogTitle>
                <DialogDescription className="text-xs text-muted-foreground mt-0.5 flex items-center gap-3">
                  {formattedDate && (
                    <span className="flex items-center gap-1">
                      <Calendar className="h-3 w-3" />
                      {formattedDate}
                    </span>
                  )}
                  <span className="flex items-center gap-1 font-mono text-[11px]">
                    <Layers className="h-3 w-3" />
                    v{updateInfo.currentVersion} → v{updateInfo.latestVersion}
                  </span>
                </DialogDescription>
              </div>
            </div>
          </div>
        </DialogHeader>

        {/* 更新日志正文 */}
        <div className="p-6 flex-1 min-h-0 space-y-4">
          <div className="text-xs font-medium text-muted-foreground">
            {ll.settings.view_changelog()}:
          </div>

          <ScrollArea className="h-52 w-full rounded-lg border border-border/60 bg-muted/20 p-4">
            {updateInfo.releaseNotes ? (
              <div className="prose prose-sm dark:prose-invert max-w-none text-xs leading-relaxed space-y-2 select-text">
                <ReactMarkdown remarkPlugins={[remarkGfm]}>
                  {updateInfo.releaseNotes}
                </ReactMarkdown>
              </div>
            ) : (
              <p className="text-xs text-muted-foreground italic">
                No detailed release notes provided for this version.
              </p>
            )}
          </ScrollArea>

          {/* 下载进度展示 */}
          {isDownloading && (
            <div className="rounded-lg border border-primary/20 bg-primary/5 p-4 space-y-2.5">
              <div className="flex items-center justify-between text-xs">
                <span className="font-medium text-primary flex items-center gap-1.5">
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  {ll.settings.downloading_update()}
                </span>
                <span className="font-mono text-muted-foreground">
                  {progress ? `${Math.round(progress.percent)}%` : '0%'}
                </span>
              </div>

              <Progress value={progress?.percent ?? 0} className="h-2" />

              <div className="flex items-center justify-between text-[11px] text-muted-foreground font-mono">
                <span>
                  {formatBytes(progress?.received)} / {formatBytes(progress?.total)}
                </span>
                <span>{formatSpeed(progress?.bytesPerSec)}</span>
              </div>
            </div>
          )}

          {/* 下载完成提示 */}
          {isDownloaded && (
            <div className="flex items-center gap-2.5 rounded-lg border border-green-500/20 bg-green-500/10 p-3.5 text-xs text-green-700 dark:text-green-300">
              <CheckCircle2 className="h-4 w-4 shrink-0 text-green-600" />
              <span>{ll.settings.download_completed()}</span>
            </div>
          )}

          {/* 错误提示 */}
          {isError && (
            <div className="flex items-start gap-2.5 rounded-lg border border-destructive/20 bg-destructive/10 p-3.5 text-xs text-destructive">
              <AlertCircle className="h-4 w-4 shrink-0 mt-0.5" />
              <div className="space-y-1">
                <div className="font-medium">{ll.settings.update_check_failed()}</div>
                <div className="text-[11px] opacity-90">{error}</div>
              </div>
            </div>
          )}
        </div>

        {/* 底部操作按钮 */}
        <DialogFooter className="px-6 py-4 border-t border-border/60 bg-muted/10 shrink-0 flex items-center justify-between sm:justify-between">
          <div className="flex items-center gap-2">
            {!isDownloading && !isDownloaded && (
              <Button
                variant="ghost"
                size="sm"
                className="text-xs text-muted-foreground hover:text-foreground h-8"
                onClick={ignoreCurrentVersion}
              >
                {ll.settings.skip_version()}
              </Button>
            )}

            {updateInfo.downloadUrl && (
              <Button
                variant="ghost"
                size="sm"
                className="text-xs text-muted-foreground hover:text-foreground h-8 gap-1"
                onClick={() => openExternalUrl(updateInfo.downloadUrl!)}
              >
                <ExternalLink className="h-3 w-3" />
                {ll.settings.open_in_browser()}
              </Button>
            )}
          </div>

          <div className="flex items-center gap-2">
            {!isDownloaded && (
              <Button
                variant="outline"
                size="sm"
                className="text-xs h-8"
                onClick={closeUpdateDialog}
                disabled={isDownloading}
              >
                {ll.settings.remind_later()}
              </Button>
            )}

            {isDownloaded ? (
              <Button
                variant="default"
                size="sm"
                className="text-xs h-8 gap-1.5 bg-green-600 hover:bg-green-700 text-white"
                onClick={installAndRestart}
              >
                <CheckCircle2 className="h-3.5 w-3.5" />
                {ll.settings.install_and_restart()}
              </Button>
            ) : isDownloading ? (
              <Button variant="default" size="sm" className="text-xs h-8 gap-1.5" disabled>
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                {ll.settings.downloading_update()}
              </Button>
            ) : (
              <Button
                variant="default"
                size="sm"
                className="text-xs h-8 gap-1.5"
                onClick={startDownload}
              >
                <Download className="h-3.5 w-3.5" />
                {ll.settings.download_update()}
              </Button>
            )}
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
