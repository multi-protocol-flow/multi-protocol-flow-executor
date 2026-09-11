import { useEffect, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import {
  RefreshCw,
  Sparkles,
  CheckCircle2,
  AlertCircle,
  ExternalLink,
  Globe,
  FileText,
  Github,
  BookOpen,
  Cpu,
  Download,
  Loader2,
  Calendar,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { Progress } from '@/components/ui/progress';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useUpdateStore } from '@/stores/update';
import { useSettingsStore } from '@/stores/settings';
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

export function AboutSettingsTab() {
  const ll = getLL();
  const currentVersion = useUpdateStore((s) => s.currentVersion);
  const status = useUpdateStore((s) => s.status);
  const updateInfo = useUpdateStore((s) => s.updateInfo);
  const progress = useUpdateStore((s) => s.progress);
  const error = useUpdateStore((s) => s.error);
  const loadCurrentVersion = useUpdateStore((s) => s.loadCurrentVersion);
  const checkForUpdates = useUpdateStore((s) => s.checkForUpdates);
  const startDownload = useUpdateStore((s) => s.startDownload);
  const installAndRestart = useUpdateStore((s) => s.installAndRestart);
  const ignoreCurrentVersion = useUpdateStore((s) => s.ignoreCurrentVersion);

  const [showChangelog, setShowChangelog] = useState(false);

  const autoCheckUpdate = useSettingsStore((s) => s.autoCheckUpdate);
  const includePrerelease = useSettingsStore((s) => s.includePrerelease);
  const setAutoCheckUpdate = useSettingsStore((s) => s.setAutoCheckUpdate);
  const setIncludePrerelease = useSettingsStore((s) => s.setIncludePrerelease);

  useEffect(() => {
    void loadCurrentVersion();
  }, [loadCurrentVersion]);

  const isChecking = status === 'checking';
  const isDownloading = status === 'downloading';
  const isDownloaded = status === 'downloaded';
  const isError = status === 'error';
  const hasUpdate = updateInfo?.hasUpdate;

  return (
    <div className="space-y-6 select-none">
      {/* 软件标识与版本头图卡片 */}
      <div className="rounded-xl border border-border/70 bg-muted/20 p-5">
        <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
          <div className="flex items-center gap-4">
            <div className="h-14 w-14 rounded-xl bg-background border border-border/80 flex items-center justify-center p-2 shadow-2xs">
              <img src="/mpe-logo.svg" alt="MPE Flow Logo" className="h-full w-full object-contain" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-base font-bold text-foreground">{ll.settings.app_name()}</h3>
                <Badge variant="outline" className="text-xs font-mono font-normal">
                  v{currentVersion || '0.1.1-beta.2'}
                </Badge>
              </div>
              <p className="text-xs text-muted-foreground mt-0.5">
                {ll.settings.app_subtitle()}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2.5 self-end sm:self-center">
            <Button
              variant="outline"
              size="sm"
              className="text-xs h-8 gap-1.5 cursor-pointer"
              onClick={() => checkForUpdates(true)}
              disabled={isChecking}
            >
              <RefreshCw className={`h-3.5 w-3.5 ${isChecking ? 'animate-spin text-primary' : ''}`} />
              <span>{isChecking ? ll.settings.checking_updates() : ll.settings.check_updates()}</span>
            </Button>
          </div>
        </div>

        {/* 更新状态反馈条 */}
        {/* 更新状态与操作区域 */}
        {hasUpdate && updateInfo ? (
          <div className="mt-4 pt-3.5 border-t border-border/50 space-y-3">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs">
              <div className="flex items-center gap-2 text-amber-600 dark:text-amber-400">
                <Sparkles className="h-4 w-4 shrink-0" />
                <div className="flex flex-wrap items-center gap-2">
                  <span>
                    {ll.settings.update_available()}: <strong>v{updateInfo.latestVersion}</strong>
                  </span>
                  {updateInfo.isPrerelease && (
                    <Badge variant="secondary" className="text-[10px] px-1.5 py-0 text-amber-600 bg-amber-500/10 border-amber-500/20">
                      Pre-release
                    </Badge>
                  )}
                  {updateInfo.releaseDate && (
                    <span className="text-[11px] text-muted-foreground flex items-center gap-1 font-normal">
                      <Calendar className="h-3 w-3" />
                      {new Date(updateInfo.releaseDate).toLocaleDateString()}
                    </span>
                  )}
                </div>
              </div>

              {/* 操作按钮组 */}
              <div className="flex items-center gap-2 flex-wrap self-start sm:self-auto">
                {updateInfo.releaseNotes && (
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-7 text-xs px-2.5 gap-1 cursor-pointer"
                    onClick={() => setShowChangelog((prev) => !prev)}
                  >
                    {showChangelog ? (
                      <>
                        <ChevronUp className="h-3.5 w-3.5" />
                        <span>{ll.settings.hide_changelog()}</span>
                      </>
                    ) : (
                      <>
                        <ChevronDown className="h-3.5 w-3.5" />
                        <span>{ll.settings.view_changelog()}</span>
                      </>
                    )}
                  </Button>
                )}

                {isDownloaded ? (
                  <Button
                    variant="default"
                    size="sm"
                    className="h-7 text-xs px-3 bg-green-600 hover:bg-green-700 text-white gap-1.5 cursor-pointer"
                    onClick={installAndRestart}
                  >
                    <CheckCircle2 className="h-3.5 w-3.5" />
                    <span>{ll.settings.install_and_restart()}</span>
                  </Button>
                ) : isDownloading ? (
                  <Button variant="default" size="sm" className="h-7 text-xs px-3 gap-1.5" disabled>
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    <span>{ll.settings.downloading_update()}</span>
                  </Button>
                ) : (
                  <Button
                    variant="default"
                    size="sm"
                    className="h-7 text-xs px-3 bg-amber-600 hover:bg-amber-700 text-white gap-1.5 cursor-pointer"
                    onClick={startDownload}
                  >
                    <Download className="h-3.5 w-3.5" />
                    <span>{ll.settings.download_update()}</span>
                  </Button>
                )}

                {updateInfo.downloadUrl && !isDownloading && !isDownloaded && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-7 text-xs px-2 text-muted-foreground hover:text-foreground gap-1 cursor-pointer"
                    onClick={() => openExternalUrl(updateInfo.downloadUrl!)}
                    title={ll.settings.open_in_browser()}
                  >
                    <ExternalLink className="h-3 w-3" />
                    <span className="hidden md:inline">{ll.settings.open_in_browser()}</span>
                  </Button>
                )}

                {!isDownloading && !isDownloaded && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-7 text-xs px-2 text-muted-foreground hover:text-foreground cursor-pointer"
                    onClick={ignoreCurrentVersion}
                  >
                    {ll.settings.skip_version()}
                  </Button>
                )}
              </div>
            </div>

            {/* 下载进度展示 */}
            {isDownloading && (
              <div className="rounded-lg border border-primary/20 bg-primary/5 p-3.5 space-y-2">
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
              <div className="flex items-center gap-2 rounded-lg border border-green-500/20 bg-green-500/10 p-3 text-xs text-green-700 dark:text-green-300">
                <CheckCircle2 className="h-4 w-4 shrink-0 text-green-600" />
                <span>{ll.settings.download_completed()}</span>
              </div>
            )}

            {/* 错误提示 */}
            {isError && (
              <div className="flex items-start gap-2.5 rounded-lg border border-destructive/20 bg-destructive/10 p-3 text-xs text-destructive">
                <AlertCircle className="h-4 w-4 shrink-0 mt-0.5" />
                <div className="space-y-1 flex-1">
                  <div className="font-medium">{ll.settings.update_check_failed()}</div>
                  {error && <div className="text-[11px] opacity-90">{error}</div>}
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  className="h-6 text-xs px-2"
                  onClick={startDownload}
                >
                  {ll.settings.retry_update()}
                </Button>
              </div>
            )}

            {/* 折叠/展开更新日志 */}
            {showChangelog && updateInfo.releaseNotes && (
              <div className="pt-2">
                <ScrollArea className="h-48 w-full rounded-lg border border-border/60 bg-muted/20 p-4">
                  <div className="prose prose-sm dark:prose-invert max-w-none text-xs leading-relaxed space-y-2 select-text">
                    <ReactMarkdown remarkPlugins={[remarkGfm]}>
                      {updateInfo.releaseNotes}
                    </ReactMarkdown>
                  </div>
                </ScrollArea>
              </div>
            )}
          </div>
        ) : (
          <div className="mt-4 pt-3.5 border-t border-border/50 flex items-center justify-between text-xs">
            {status === 'up-to-date' ? (
              <div className="flex items-center gap-1.5 text-green-600 dark:text-green-400">
                <CheckCircle2 className="h-4 w-4" />
                <span>{ll.settings.update_already_latest()}</span>
              </div>
            ) : status === 'error' ? (
              <div className="flex items-center gap-1.5 text-destructive">
                <AlertCircle className="h-4 w-4" />
                <span>{ll.settings.update_check_failed()}{error ? `: ${error}` : ''}</span>
              </div>
            ) : (
              <div className="text-muted-foreground">
                {ll.settings.version_current()}: v{currentVersion || '0.1.1-beta.2'}
              </div>
            )}
          </div>
        )}
      </div>

      {/* 更新策略偏好卡片 */}
      <div className="space-y-4">
        <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          {ll.settings.appearance()}
        </h4>

        <div className="rounded-xl border border-border/70 divide-y divide-border/50 bg-muted/10">
          <div className="p-4 flex items-center justify-between">
            <div className="space-y-0.5 pr-4">
              <div className="text-xs font-medium text-foreground">
                {ll.settings.auto_check_updates()}
              </div>
              <div className="text-[11px] text-muted-foreground">
                {ll.settings.auto_check_updates_desc()}
              </div>
            </div>
            <Switch
              checked={autoCheckUpdate}
              onCheckedChange={(checked) => void setAutoCheckUpdate?.(checked)}
            />
          </div>

          <div className="p-4 flex items-center justify-between">
            <div className="space-y-0.5 pr-4">
              <div className="text-xs font-medium text-foreground">
                {ll.settings.include_prerelease()}
              </div>
              <div className="text-[11px] text-muted-foreground">
                {ll.settings.include_prerelease_desc()}
              </div>
            </div>
            <Switch
              checked={includePrerelease}
              onCheckedChange={(checked) => void setIncludePrerelease?.(checked)}
            />
          </div>
        </div>
      </div>

      {/* 运行时环境信息 */}
      <div className="space-y-3">
        <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5">
          <Cpu className="h-3.5 w-3.5" />
          <span>{ll.settings.system_platform()}</span>
        </h4>

        <div className="rounded-xl border border-border/70 p-4 bg-muted/10 grid grid-cols-2 sm:grid-cols-3 gap-3 text-xs">
          <div>
            <div className="text-[11px] text-muted-foreground">Architecture</div>
            <div className="font-medium font-mono text-foreground mt-0.5">Tauri 2.x + Rust</div>
          </div>
          <div>
            <div className="text-[11px] text-muted-foreground">Frontend</div>
            <div className="font-medium font-mono text-foreground mt-0.5">React 19 + TypeScript</div>
          </div>
          <div>
            <div className="text-[11px] text-muted-foreground">Protocol Crates</div>
            <div className="font-medium font-mono text-foreground mt-0.5">7 Native + Sidecar Plugins</div>
          </div>
        </div>
      </div>

      {/* 社区与官方资源卡片 */}
      <div className="space-y-3">
        <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          {ll.settings.official_website()}
        </h4>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
          <button
            type="button"
            onClick={() => openExternalUrl('https://mpe.run')}
            className="flex items-center justify-between p-3 rounded-lg border border-border/60 bg-muted/20 hover:bg-muted/50 transition-colors text-left group cursor-pointer"
          >
            <div className="flex items-center gap-2.5">
              <Globe className="h-4 w-4 text-primary" />
              <div>
                <div className="text-xs font-medium text-foreground">{ll.settings.official_website()}</div>
                <div className="text-[11px] text-muted-foreground font-mono">https://mpe.run</div>
              </div>
            </div>
            <ExternalLink className="h-3.5 w-3.5 text-muted-foreground group-hover:text-foreground transition-colors" />
          </button>

          <button
            type="button"
            onClick={() => openExternalUrl('https://mpe.run/changelog')}
            className="flex items-center justify-between p-3 rounded-lg border border-border/60 bg-muted/20 hover:bg-muted/50 transition-colors text-left group cursor-pointer"
          >
            <div className="flex items-center gap-2.5">
              <FileText className="h-4 w-4 text-primary" />
              <div>
                <div className="text-xs font-medium text-foreground">{ll.settings.view_changelog()}</div>
                <div className="text-[11px] text-muted-foreground font-mono">https://mpe.run/changelog</div>
              </div>
            </div>
            <ExternalLink className="h-3.5 w-3.5 text-muted-foreground group-hover:text-foreground transition-colors" />
          </button>

          <button
            type="button"
            onClick={() => openExternalUrl('https://mpe.run/docs')}
            className="flex items-center justify-between p-3 rounded-lg border border-border/60 bg-muted/20 hover:bg-muted/50 transition-colors text-left group cursor-pointer"
          >
            <div className="flex items-center gap-2.5">
              <BookOpen className="h-4 w-4 text-primary" />
              <div>
                <div className="text-xs font-medium text-foreground">Documentation</div>
                <div className="text-[11px] text-muted-foreground font-mono">https://mpe.run/docs</div>
              </div>
            </div>
            <ExternalLink className="h-3.5 w-3.5 text-muted-foreground group-hover:text-foreground transition-colors" />
          </button>

          <button
            type="button"
            onClick={() => openExternalUrl('https://github.com/multi-protocol-flow-executor')}
            className="flex items-center justify-between p-3 rounded-lg border border-border/60 bg-muted/20 hover:bg-muted/50 transition-colors text-left group cursor-pointer"
          >
            <div className="flex items-center gap-2.5">
              <Github className="h-4 w-4 text-primary" />
              <div>
                <div className="text-xs font-medium text-foreground">{ll.settings.open_source_repo()}</div>
                <div className="text-[11px] text-muted-foreground font-mono">GitHub / MPE Flow</div>
              </div>
            </div>
            <ExternalLink className="h-3.5 w-3.5 text-muted-foreground group-hover:text-foreground transition-colors" />
          </button>
        </div>
      </div>
    </div>
  );
}
