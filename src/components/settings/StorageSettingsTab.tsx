import { useEffect, useState } from 'react';
import { Settings2, FolderInput, FolderOpen, FolderCog, Puzzle, ChevronRight } from 'lucide-react';
import { useSettingsStore } from '@/stores/settings';
import { usePluginsStore } from '@/stores/plugins';
import { useUIStore } from '@/stores/ui';
import { getLL } from '@/i18n/i18n-lazy';
import { isTauriEnv } from '@/lib/platform';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { clearNodeMetadataCache } from '@/lib/NodeRegistry';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';
import type { PluginDirSource } from '@/types/plugins';
import { PathSettingItem } from './PathSettingItem';
async function getSettingsService() {
  if (isTauriEnv()) {
    return import('@/services/tauri/settings');
  }
  return import('@/services/tauri/settings.mock');
}

function sourceLabel(source: PluginDirSource): string {
  const ll = getLL().plugins;
  switch (source) {
    case 'env':
      return ll.source_env();
    case 'config':
      return ll.source_config();
    default:
      return ll.source_default();
  }
}

export function StorageSettingsTab() {
  const {
    reportDir,
    logDir,
    flowsDir,
    isLoading,
    setReportDir,
    setLogDir,
    setFlowsDir,
    setPluginDir,
    openReportDir,
    openLogDir,
    openFlowsDir,
  } = useSettingsStore();

  const { pluginDirInfo, loadPluginDirInfo, reloadPlugins } = usePluginsStore();
  const openPluginSettingsDialog = useUIStore((s) => s.openPluginSettingsDialog);
  const closeSettingsDialog = useUIStore((s) => s.closeSettingsDialog);

  const ll = getLL();
  const [configFilePath, setConfigFilePath] = useState<string>('');

  useEffect(() => {
    let mounted = true;
    (async () => {
      try {
        const service = await getSettingsService();
        if (service.getConfigPath) {
          const p = await service.getConfigPath();
          if (mounted) setConfigFilePath(p);
        }
      } catch {
        if (mounted) setConfigFilePath(ll.common.label_browser_storage());
      }
    })();
    return () => {
      mounted = false;
    };
  }, [ll]);

  useEffect(() => {
    if (!pluginDirInfo) {
      void loadPluginDirInfo();
    }
  }, [pluginDirInfo, loadPluginDirInfo]);
  const handleOpenConfigFile = async () => {
    if (!configFilePath) return;
    const service = await getSettingsService();
    await service.openDirectory(configFilePath);
  };

  const handleOpenConfigFolder = async () => {
    if (!configFilePath) return;
    const lastSlash = Math.max(configFilePath.lastIndexOf('/'), configFilePath.lastIndexOf('\\'));
    const folder = lastSlash > 0 ? configFilePath.substring(0, lastSlash) : configFilePath;
    const service = await getSettingsService();
    await service.openDirectory(folder);
  };

  const handleSelectFlowsDir = async () => {
    const service = await getSettingsService();
    const dir = await service.selectDirectory(flowsDir);
    if (dir) await setFlowsDir(dir);
  };

  const handleSelectReportDir = async () => {
    const service = await getSettingsService();
    const dir = await service.selectDirectory(reportDir);
    if (dir) await setReportDir(dir);
  };

  const handleSelectLogDir = async () => {
    const service = await getSettingsService();
    const dir = await service.selectDirectory(logDir);
    if (dir) await setLogDir(dir);
  };

  const envLocked = pluginDirInfo?.source === 'env';

  const handleSelectPluginDir = async () => {
    const service = await getSettingsService();
    const dir = await service.selectDirectory(pluginDirInfo?.effectiveDir ?? '');
    if (!dir) return;
    try {
      await setPluginDir(dir);
      await reloadPlugins(dir);
      clearNodeMetadataCache();
      toast.success(ll.plugins.reload_success());
    } catch (e) {
      toast.error(ll.plugins.reload_failed(), {
        description: e instanceof Error ? e.message : String(e),
      });
    }
  };

  const handleOpenPluginDir = async () => {
    if (!pluginDirInfo?.effectiveDir) return;
    const service = await getSettingsService();
    await service.openDirectory(pluginDirInfo.effectiveDir);
  };
  return (
    <div className="space-y-4">
      {/* 配置文件 */}
      <PathSettingItem
        label={ll.settings.config_file()}
        description={ll.settings.config_file_desc()}
        path={configFilePath}
        icon={Settings2}
        isFile
        isLoading={isLoading}
        onOpenFile={handleOpenConfigFile}
        onOpenDirectory={handleOpenConfigFolder}
      />

      {/* 流程保存目录 */}
      <PathSettingItem
        label={ll.settings.flows_dir()}
        description={ll.settings.flows_dir_desc()}
        path={flowsDir}
        icon={FolderInput}
        isLoading={isLoading}
        onSelectDirectory={handleSelectFlowsDir}
        onOpenDirectory={openFlowsDir}
      />

      {/* 插件存储目录 */}
      <PathSettingItem
        label={ll.settings.plugins_dir()}
        description={ll.settings.plugins_dir_desc()}
        path={pluginDirInfo?.effectiveDir ?? ''}
        icon={Puzzle}
        badge={
          pluginDirInfo ? (
            <Badge
              variant="outline"
              className={cn(
                'text-[10px] px-1.5 py-0 h-4 font-normal',
                envLocked && 'border-amber-500/40 text-amber-600 dark:text-amber-400'
              )}
            >
              {sourceLabel(pluginDirInfo.source)}
            </Badge>
          ) : null
        }
        extraHeaderAction={
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => {
              closeSettingsDialog();
              openPluginSettingsDialog();
            }}
            className="h-6 px-2 text-xs text-muted-foreground hover:text-foreground gap-1 cursor-pointer shrink-0"
          >
            <span>{ll.settings.plugins_manager_link()}</span>
            <ChevronRight className="h-3 w-3" />
          </Button>
        }
        isLoading={isLoading}
        onSelectDirectory={envLocked ? undefined : handleSelectPluginDir}
        onOpenDirectory={handleOpenPluginDir}
      />
      <PathSettingItem
        label={ll.settings.reports_dir()}
        description={ll.settings.reports_dir_desc()}
        path={reportDir}
        icon={FolderOpen}
        isLoading={isLoading}
        onSelectDirectory={handleSelectReportDir}
        onOpenDirectory={openReportDir}
      />

      {/* 日志目录 */}
      <PathSettingItem
        label={ll.settings.logs_dir()}
        description={ll.settings.logs_dir_desc()}
        path={logDir}
        icon={FolderCog}
        isLoading={isLoading}
        onSelectDirectory={handleSelectLogDir}
        onOpenDirectory={openLogDir}
      />
    </div>
  );
}
