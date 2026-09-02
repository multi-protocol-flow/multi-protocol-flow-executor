import { useEffect, useState } from 'react';
import { Settings2, FolderInput, FolderOpen, FolderCog } from 'lucide-react';
import { useSettingsStore } from '@/stores/settings';
import { getLL } from '@/i18n/i18n-lazy';
import { isTauriEnv } from '@/lib/platform';
import { PathSettingItem } from './PathSettingItem';

async function getSettingsService() {
  if (isTauriEnv()) {
    return import('@/services/tauri/settings');
  }
  return import('@/services/tauri/settings.mock');
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
    openReportDir,
    openLogDir,
    openFlowsDir,
  } = useSettingsStore();

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
        if (mounted) setConfigFilePath(ll.common.label_browser_storage?.() ?? 'Browser LocalStorage');
      }
    })();
    return () => {
      mounted = false;
    };
  }, [ll]);

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

  return (
    <div className="space-y-4">
      {/* 配置文件 */}
      <PathSettingItem
        label={ll.settings.config_file?.() ?? 'Config File'}
        description={ll.settings.config_file_desc?.() ?? 'Stores global preferences, credentials, and runtime parameters.'}
        path={configFilePath}
        icon={Settings2}
        isFile
        isLoading={isLoading}
        onOpenFile={handleOpenConfigFile}
        onOpenDirectory={handleOpenConfigFolder}
      />

      {/* 流程保存目录 */}
      <PathSettingItem
        label={ll.settings.flows_dir?.() ?? 'Flows Directory'}
        description={ll.settings.flows_dir_desc?.() ?? 'Default directory for saving local workflow (.mpf / JSON) files.'}
        path={flowsDir}
        icon={FolderInput}
        isLoading={isLoading}
        onSelectDirectory={handleSelectFlowsDir}
        onOpenDirectory={openFlowsDir}
      />

      {/* 报告目录 */}
      <PathSettingItem
        label={ll.settings.reports_dir?.() ?? 'Reports Directory'}
        description={ll.settings.reports_dir_desc?.() ?? 'Directory for flow execution history and stress testing reports.'}
        path={reportDir}
        icon={FolderOpen}
        isLoading={isLoading}
        onSelectDirectory={handleSelectReportDir}
        onOpenDirectory={openReportDir}
      />

      {/* 日志目录 */}
      <PathSettingItem
        label={ll.settings.logs_dir?.() ?? 'Logs Directory'}
        description={ll.settings.logs_dir_desc?.() ?? 'Directory for application logs and child process communication traces.'}
        path={logDir}
        icon={FolderCog}
        isLoading={isLoading}
        onSelectDirectory={handleSelectLogDir}
        onOpenDirectory={openLogDir}
      />
    </div>
  );
}
