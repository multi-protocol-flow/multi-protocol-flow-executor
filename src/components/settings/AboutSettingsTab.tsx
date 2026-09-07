import { useEffect } from 'react';
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
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
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

export function AboutSettingsTab() {
  const ll = getLL();
  const currentVersion = useUpdateStore((s) => s.currentVersion);
  const status = useUpdateStore((s) => s.status);
  const updateInfo = useUpdateStore((s) => s.updateInfo);
  const loadCurrentVersion = useUpdateStore((s) => s.loadCurrentVersion);
  const checkForUpdates = useUpdateStore((s) => s.checkForUpdates);
  const openUpdateDialog = useUpdateStore((s) => s.openUpdateDialog);

  const autoCheckUpdate = useSettingsStore((s) => s.autoCheckUpdate);
  const includePrerelease = useSettingsStore((s) => s.includePrerelease);
  const setAutoCheckUpdate = useSettingsStore((s) => s.setAutoCheckUpdate);
  const setIncludePrerelease = useSettingsStore((s) => s.setIncludePrerelease);

  useEffect(() => {
    void loadCurrentVersion();
  }, [loadCurrentVersion]);

  const isChecking = status === 'checking';
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
        <div className="mt-4 pt-3.5 border-t border-border/50 flex items-center justify-between text-xs">
          {hasUpdate ? (
            <div className="flex items-center justify-between w-full">
              <div className="flex items-center gap-2 text-amber-600 dark:text-amber-400">
                <Sparkles className="h-4 w-4" />
                <span>
                  {ll.settings.update_available()}: <strong>v{updateInfo.latestVersion}</strong>
                </span>
              </div>
              <Button
                variant="default"
                size="sm"
                className="h-7 text-xs px-2.5 bg-amber-600 hover:bg-amber-700 text-white"
                onClick={openUpdateDialog}
              >
                {ll.settings.view_changelog()}
              </Button>
            </div>
          ) : status === 'up-to-date' ? (
            <div className="flex items-center gap-1.5 text-green-600 dark:text-green-400">
              <CheckCircle2 className="h-4 w-4" />
              <span>{ll.settings.update_already_latest()}</span>
            </div>
          ) : status === 'error' ? (
            <div className="flex items-center gap-1.5 text-destructive">
              <AlertCircle className="h-4 w-4" />
              <span>{ll.settings.update_check_failed()}</span>
            </div>
          ) : (
            <div className="text-muted-foreground">
              {ll.settings.version_current()}: v{currentVersion || '0.1.1-beta.2'}
            </div>
          )}
        </div>
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
