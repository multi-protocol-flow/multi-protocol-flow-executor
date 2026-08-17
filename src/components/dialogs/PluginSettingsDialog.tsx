/**
 * PluginSettingsDialog - 插件设置对话框
 *
 * 两个 Tab：
 * - 已安装：查看/更改插件目录（运行时热重载立即生效）+ 已安装插件列表
 * - 市场：配置市场地址并拉取插件列表（下载按钮本轮占位，下一轮接线）
 */
import { useEffect, useState } from 'react';
import { FolderOpen, Puzzle, Loader2, Package, Globe, Download } from 'lucide-react';
import { Dialog, DialogContent } from '@/components/ui/dialog';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { toast } from 'sonner';
import { useUIStore, usePluginsStore } from '@/stores';
import { useSettingsStore } from '@/stores/settings';
import { getLL } from '@/i18n/i18n-lazy';
import { selectDirectory } from '@/services/tauri/settings';
import { resolveBackendIcon } from '@/lib/nodeIcons';
import { clearNodeMetadataCache } from '@/lib/NodeRegistry';
import type { PluginDirSource, MarketplacePlugin } from '@/types/plugins';

/** 将后端平台 key（windows-x64 等）与当前宿主平台对齐的展示名 */
function currentPlatformKey(): string {
  const platform = navigator.platform.toLowerCase();
  const os = platform.includes('win')
    ? 'windows'
    : platform.includes('mac')
      ? 'macos'
      : 'linux';
  const ua = navigator.userAgent;
  const arch = /arm64|aarch64/.test(ua) || platform.includes('arm') ? 'arm64' : 'x64';
  return `${os}-${arch}`;
}

/** 来源徽标文案 */
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

/** 市场插件条目（含当前平台徽标与下载安装按钮；已安装时按钮变「重新安装」） */
function MarketplaceItem({
  plugin,
  installed,
  installing,
  onInstall,
}: {
  plugin: MarketplacePlugin;
  installed: boolean;
  installing: boolean;
  onInstall: (plugin: MarketplacePlugin) => void;
}) {
  const ll = getLL().plugins;
  const platform = currentPlatformKey();
  const hasCurrentPlatform = Boolean(plugin.platforms[platform]);
  const Icon = plugin.icon ? resolveBackendIcon(plugin.icon) : undefined;

  return (
    <div className="flex items-start gap-3 rounded-md border border-border p-3">
      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
        {Icon ? <Icon className="h-5 w-5" /> : <Package className="h-5 w-5" />}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm font-medium text-foreground">{plugin.name}</span>
          <span className="text-xs text-muted-foreground">v{plugin.version}</span>
          {hasCurrentPlatform ? (
            <Badge variant="outline" className="text-xs">
              {ll.current_platform()} ({platform})
            </Badge>
          ) : (
            <Badge variant="secondary" className="text-xs">
              {platform}
            </Badge>
          )}
        </div>
        {plugin.description ? (
          <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">{plugin.description}</p>
        ) : null}
        {plugin.author ? (
          <p className="mt-1 text-xs text-muted-foreground">{plugin.author}</p>
        ) : null}
      </div>
      <Button
        variant="outline"
        size="sm"
        disabled={!hasCurrentPlatform || installing}
        title={hasCurrentPlatform ? undefined : ll.no_platform_asset()}
        className="shrink-0"
        onClick={() => onInstall(plugin)}
      >
        {installing ? (
          <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" />
        ) : (
          <Download className="mr-1 h-3.5 w-3.5" />
        )}
        {installing ? ll.installing() : installed ? ll.reinstall() : ll.download()}
      </Button>
    </div>
  );
}

export function PluginSettingsDialog() {
  const show = useUIStore((s) => s.showPluginSettingsDialog);
  const closePluginSettingsDialog = useUIStore((s) => s.closePluginSettingsDialog);
  const {
    pluginDirInfo,
    installed,
    marketplace,
    loading,
    error,
    loadPluginDirInfo,
    loadInstalled,
    reloadPlugins,
    loadMarketplace,
    installMarketplacePlugin,
  } = usePluginsStore();
  const pluginRegistryUrl = useSettingsStore((s) => s.pluginRegistryUrl);
  const setPluginRegistryUrl = useSettingsStore((s) => s.setPluginRegistryUrl);
  const [marketUrl, setMarketUrl] = useState('');
  const [changingDir, setChangingDir] = useState(false);
  const [installingName, setInstallingName] = useState<string | null>(null);
  const ll = getLL().plugins;

  // 打开时加载目录信息与已安装列表
  useEffect(() => {
    if (show) {
      loadPluginDirInfo();
      loadInstalled();
    }
  }, [show, loadPluginDirInfo, loadInstalled]);

  // 市场地址初值跟随设置
  useEffect(() => {
    setMarketUrl(pluginRegistryUrl ?? '');
  }, [pluginRegistryUrl, show]);

  const handleOpenChange = (open: boolean) => {
    if (!open) {
      closePluginSettingsDialog();
    }
  };

  /** 选择新目录 → 持久化 → 热重载 → 刷新已安装列表与节点面板 */
  const handleChangeDir = async () => {
    const dir = await selectDirectory(pluginDirInfo?.effective_dir ?? '');
    if (!dir) return;
    setChangingDir(true);
    try {
      const { setPluginDir } = useSettingsStore.getState();
      await setPluginDir(dir);
      await reloadPlugins(dir);
      // 插件节点面板元数据缓存失效，重新拉取（画布下次渲染即生效）
      clearNodeMetadataCache();
      toast.success(ll.reload_success());
    } catch (e) {
      toast.error(ll.reload_failed(), {
        description: e instanceof Error ? e.message : String(e),
      });
    } finally {
      setChangingDir(false);
    }
  };

  /** 保存市场地址并拉取列表 */
  const handleSaveAndLoadMarket = async () => {
    const url = marketUrl.trim();
    if (!url) return;
    try {
      await setPluginRegistryUrl(url);
      await loadMarketplace(url);
      toast.success(ll.market_save_and_load());
    } catch (e) {
      toast.error(ll.market_error(), {
        description: e instanceof Error ? e.message : String(e),
      });
    }
  };

  /** 安装市场插件（store 内完成安装 + 热重载 + 列表刷新；已安装时 force 重装） */
  const handleInstallPlugin = async (plugin: MarketplacePlugin) => {
    const url = pluginRegistryUrl ?? marketUrl.trim();
    if (!url) return;
    const alreadyInstalled = installed.some((p) => p.name === plugin.name);
    setInstallingName(plugin.name);
    try {
      await installMarketplacePlugin(url, plugin.name, undefined, alreadyInstalled);
      await loadInstalled();
      toast.success(ll.install_success({ name: plugin.name }));
    } catch (e) {
      toast.error(ll.install_failed({ name: plugin.name }), {
        description: e instanceof Error ? e.message : String(e),
      });
    } finally {
      setInstallingName(null);
    }
  };

  const envLocked = pluginDirInfo?.source === 'env';

  return (
    <Dialog open={show} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-[560px] h-[70vh] !gap-0 p-0 flex flex-col">
        {/* 标题 */}
        <div className="flex items-center gap-2 px-6 pt-5 pb-3 border-b border-border">
          <Puzzle className="w-5 h-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold text-foreground">{ll.settings_title()}</h2>
        </div>

        <Tabs defaultValue="installed" className="flex min-h-0 flex-1 flex-col px-6">
          <TabsList className="mt-4 w-fit">
            <TabsTrigger value="installed">{ll.tab_installed()}</TabsTrigger>
            <TabsTrigger value="marketplace">{ll.tab_marketplace()}</TabsTrigger>
          </TabsList>

          {/* ===== 已安装 Tab ===== */}
          <TabsContent value="installed" className="min-h-0 flex-1 overflow-y-auto pb-4">
            {/* 插件目录行 */}
            <div className="rounded-md border border-border p-3">
              <div className="flex items-center gap-2">
                <FolderOpen className="h-4 w-4 text-muted-foreground shrink-0" />
                <span className="text-sm font-medium text-foreground">{ll.effective_dir()}</span>
                {pluginDirInfo ? (
                  <Badge variant="outline" className="text-xs">
                    {sourceLabel(pluginDirInfo.source)}
                  </Badge>
                ) : null}
                <Button
                  variant="outline"
                  size="sm"
                  className="ml-auto shrink-0"
                  disabled={envLocked || changingDir}
                  onClick={handleChangeDir}
                >
                  {changingDir ? (
                    <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <FolderOpen className="mr-1 h-3.5 w-3.5" />
                  )}
                  {ll.change_dir()}
                </Button>
              </div>
              <p className="mt-2 break-all text-xs text-muted-foreground">
                {pluginDirInfo?.effective_dir ?? '-'}
              </p>
              {envLocked ? (
                <p className="mt-2 text-xs text-amber-600">{ll.source_env()}</p>
              ) : null}
            </div>

            {/* 已安装列表 */}
            <div className="mt-4 space-y-2">
              {installed.length === 0 ? (
                <p className="py-8 text-center text-sm text-muted-foreground">
                  {ll.installed_empty()}
                </p>
              ) : (
                installed.map((plugin) => (
                  <div
                    key={plugin.path}
                    className="flex items-start gap-3 rounded-md border border-border p-3"
                  >
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <span className="truncate text-sm font-medium text-foreground">
                          {plugin.name}
                        </span>
                        <span className="text-xs text-muted-foreground">v{plugin.version}</span>
                        <Badge
                          variant={plugin.status === 'loaded' ? 'default' : 'destructive'}
                          className="text-xs"
                        >
                          {plugin.status === 'loaded' ? ll.status_loaded() : ll.status_error()}
                        </Badge>
                      </div>
                      {plugin.description ? (
                        <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">
                          {plugin.description}
                        </p>
                      ) : null}
                      {plugin.error ? (
                        <p className="mt-1 text-xs text-destructive">{plugin.error}</p>
                      ) : null}
                    </div>
                  </div>
                ))
              )}
            </div>
          </TabsContent>

          {/* ===== 市场 Tab ===== */}
          <TabsContent value="marketplace" className="min-h-0 flex-1 overflow-y-auto pb-4">
            <div className="flex items-center gap-2">
              <Globe className="h-4 w-4 text-muted-foreground shrink-0" />
              <Input
                type="text"
                placeholder="https://market.example.com"
                value={marketUrl}
                onChange={(e) => setMarketUrl(e.target.value)}
                className="h-9 text-sm"
              />
              <Button
                variant="default"
                size="sm"
                className="shrink-0"
                disabled={!marketUrl.trim() || loading}
                onClick={handleSaveAndLoadMarket}
              >
                {loading ? <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" /> : null}
                {ll.market_save_and_load()}
              </Button>
            </div>

            {error && (
              <p className="mt-3 text-xs text-destructive">{error}</p>
            )}

            {marketplace.length === 0 ? (
              <p className="py-8 text-center text-sm text-muted-foreground">
                {pluginRegistryUrl ? ll.market_empty() : ll.market_url_hint()}
              </p>
            ) : (
              <div className="mt-4 space-y-2">
                {marketplace.map((plugin) => (
                  <MarketplaceItem
                    key={`${plugin.name}@${plugin.version}`}
                    plugin={plugin}
                    installed={installed.some((p) => p.name === plugin.name)}
                    installing={installingName === plugin.name}
                    onInstall={handleInstallPlugin}
                  />
                ))}
              </div>
            )}
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}

export default PluginSettingsDialog;
