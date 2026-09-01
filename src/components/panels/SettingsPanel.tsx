/**
 * SettingsPanel - 全局设置面板
 * 复用 NodeConfigPanel 的布局样式，提供配置文件存储位置提示
 */
import { useEffect, useState } from 'react';
import { Settings2, FolderOpen, FolderCog, FolderInput, Info, Sun, Moon, Monitor, Globe, Timer, Cpu, Layers } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useSettingsStore } from '@/stores/settings';
import type { ThemeMode, Locale } from '@/types/settings';
import { getLL } from '@/i18n/i18n-lazy';
import { AIProviderSettingsCard } from '@/components/ai/AIProviderSettingsCard';

import { isTauriEnv } from '@/lib/platform';

// 动态导入设置服务（根据环境）
async function getSettingsService() {
  if (isTauriEnv()) {
    return import('@/services/tauri/settings');
  }
  return import('@/services/tauri/settings.mock');
}

export function SettingsPanel() {
  const {
    reportDir,
    logDir,
    flowsDir,
    theme,
    locale,
    defaultExecutionTimeoutMs,
    pluginMaxConcurrency,
    pluginQueueCapacity,
    isLoading,
    loadSettings,
    setReportDir,
    setLogDir,
    setFlowsDir,
    setTheme,
    setLocale,
    setDefaultExecutionTimeoutMs,
    setPluginMaxConcurrency,
    setPluginQueueCapacity,
    openReportDir,
    openLogDir,
    openFlowsDir,
  } = useSettingsStore();

  // 获取翻译函数（模块级，locale 变化时由 I18nBridge 触发重渲染后重新计算）
  const ll = getLL();

  // 配置文件存储路径
  const [configFilePath, setConfigFilePath] = useState<string>('');
  const [timeoutInput, setTimeoutInput] = useState<string>(
    defaultExecutionTimeoutMs !== undefined ? String(defaultExecutionTimeoutMs) : '30000'
  );
  const [concurrencyInput, setConcurrencyInput] = useState<string>(
    pluginMaxConcurrency !== undefined ? String(pluginMaxConcurrency) : '256'
  );
  const [queueCapInput, setQueueCapInput] = useState<string>(
    pluginQueueCapacity !== undefined ? String(pluginQueueCapacity) : '2048'
  );

  // 当 store 中的值变化时同步到本地输入状态
  useEffect(() => {
    if (defaultExecutionTimeoutMs !== undefined) {
      setTimeoutInput(String(defaultExecutionTimeoutMs));
    }
  }, [defaultExecutionTimeoutMs]);
  useEffect(() => {
    if (pluginMaxConcurrency !== undefined) {
      setConcurrencyInput(String(pluginMaxConcurrency));
    }
  }, [pluginMaxConcurrency]);
  useEffect(() => {
    if (pluginQueueCapacity !== undefined) {
      setQueueCapInput(String(pluginQueueCapacity));
    }
  }, [pluginQueueCapacity]);
  // 面板打开时加载设置
  useEffect(() => {
    loadSettings();
    // 获取配置文件路径
    getConfigPath();
  }, [loadSettings]);

  // 获取配置文件路径
  const getConfigPath = async () => {
    try {
      const service = await getSettingsService();
      if (service.getConfigPath) {
        const path = await service.getConfigPath();
        setConfigFilePath(path);
      }
    } catch {
      // 非Tauri环境，使用默认值
      setConfigFilePath(ll.common.label_browser_storage());
    }
  };

  const handleSelectReportDir = async () => {
    const service = await getSettingsService();
    const dir = await service.selectDirectory(reportDir);
    if (dir) {
      await setReportDir(dir);
    }
  };

  const handleSelectLogDir = async () => {
    const service = await getSettingsService();
    const dir = await service.selectDirectory(logDir);
    if (dir) {
      await setLogDir(dir);
    }
  };

  const handleSelectFlowsDir = async () => {
    const service = await getSettingsService();
    const dir = await service.selectDirectory(flowsDir);
    if (dir) {
      await setFlowsDir(dir);
    }
  };

  return (
    <Card className="flex h-full flex-col overflow-hidden border-0 shadow-none w-full min-w-0">
      <CardHeader className="shrink-0 border-b border-muted bg-background pb-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 flex-1 min-w-0">
            <Settings2 className="h-4 w-4 text-muted-foreground shrink-0" />
            <CardTitle className="text-base font-semibold text-foreground truncate cursor-default">
              {ll?.settings?.panel_title() ?? 'Global Settings'}
            </CardTitle>
          </div>
          <Badge variant="outline" className="border-border text-muted-foreground shrink-0">
            {ll?.common?.settings_label() ?? 'Settings'}
          </Badge>
        </div>
      </CardHeader>

      <CardContent className="flex-1 overflow-hidden p-0">
        <ScrollArea className="h-full">
          <div className="w-full min-w-0 max-w-full overflow-hidden px-6 py-4 space-y-5">
            {/* 配置文件存储位置提示 */}
            <div className="text-xs text-muted-foreground py-2 px-3 rounded-lg bg-muted border border-border space-y-1">
              <div className="flex items-center gap-1.5">
                <Info className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                <span className="font-medium text-foreground">{ll?.settings?.config_file() ?? 'Config File'}</span>
              </div>
              <code className="block text-foreground break-all leading-relaxed">
                {isLoading ? (ll?.settings?.loading() ?? 'Loading...') : configFilePath || (ll?.settings?.unknown() ?? 'Unknown')}
              </code>
            </div>

            {/* 主题 */}
            <div className="space-y-2">
              <Label className="text-sm font-medium text-foreground flex items-center gap-2">
                {ll?.settings?.theme() ?? 'Theme'}
              </Label>
              <Select value={theme} onValueChange={(value) => setTheme(value as ThemeMode)}>
                <SelectTrigger className="w-full">
                  <SelectValue placeholder={ll?.settings?.select_theme() ?? 'Select Theme'} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="light">
                    <span className="flex items-center gap-2">
                      <Sun className="h-4 w-4" />
                      {ll?.settings?.theme_light() ?? 'Light'}
                    </span>
                  </SelectItem>
                  <SelectItem value="dark">
                    <span className="flex items-center gap-2">
                      <Moon className="h-4 w-4" />
                      {ll?.settings?.theme_dark() ?? 'Dark'}
                    </span>
                  </SelectItem>
                  <SelectItem value="system">
                    <span className="flex items-center gap-2">
                      <Monitor className="h-4 w-4" />
                      {ll?.settings?.theme_system() ?? 'System'}
                    </span>
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>

            {/* 语言 */}
            <div className="space-y-2">
              <Label className="text-sm font-medium text-foreground flex items-center gap-2">
                <Globe className="h-4 w-4 text-muted-foreground" />
                {ll?.settings?.language() ?? 'Language'}
              </Label>
              <Select value={locale} onValueChange={(value) => setLocale(value as Locale)}>
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
<SelectItem value="zh-CN">{ll?.settings?.zh_CN() ?? '简体中文'}</SelectItem>
<SelectItem value="en-US">{ll?.settings?.en_US() ?? 'English'}</SelectItem>
                </SelectContent>
              </Select>
            </div>
            {/* 默认节点执行超时 */}
            <div className="space-y-2">
              <Label className="text-sm font-medium text-foreground flex items-center gap-2">
                <Timer className="h-4 w-4 text-muted-foreground" />
                {ll?.settings?.default_timeout() ?? 'Default Node Execution Timeout (ms)'}
              </Label>
              <Input
                type="number"
                min={1000}
                max={3600000}
                step={1000}
                value={timeoutInput}
                onChange={(e) => {
                  const text = e.target.value;
                  setTimeoutInput(text);
                  if (text !== '') {
                    const val = Number(text);
                    if (!isNaN(val) && val >= 1000 && val <= 3600000) {
                      setDefaultExecutionTimeoutMs(val);
                    }
                  }
                }}
                onBlur={() => {
                  const val = Number(timeoutInput);
                  if (timeoutInput === '' || isNaN(val) || val < 1000) {
                    const fallback = 30000;
                    setTimeoutInput(String(fallback));
                    setDefaultExecutionTimeoutMs(fallback);
                  } else if (val > 3600000) {
                    const clamped = 3600000;
                    setTimeoutInput(String(clamped));
                    setDefaultExecutionTimeoutMs(clamped);
                  }
                }}
                placeholder="30000"
                className="w-full"
              />
              <p className="text-xs text-muted-foreground leading-relaxed">
                {ll?.settings?.default_timeout_tip() ?? 'Fallback timeout duration when a node, plugin, or datasource does not specify its own timeout (default 30000ms).'}
              </p>
            </div>

            {/* 插件最大并发数 */}
            <div className="space-y-2">
              <Label className="text-sm font-medium text-foreground flex items-center gap-2">
                <Cpu className="h-4 w-4 text-muted-foreground" />
                {ll?.settings?.plugin_max_concurrency() ?? 'Max Plugin Concurrency'}
              </Label>
              <Input
                type="number"
                min={1}
                max={8192}
                step={32}
                value={concurrencyInput}
                onChange={(e) => {
                  const text = e.target.value;
                  setConcurrencyInput(text);
                  if (text !== '') {
                    const val = Number(text);
                    if (!isNaN(val) && val >= 1 && val <= 8192) {
                      setPluginMaxConcurrency(val);
                    }
                  }
                }}
                onBlur={() => {
                  const val = Number(concurrencyInput);
                  if (concurrencyInput === '' || isNaN(val) || val < 1) {
                    const fallback = 256;
                    setConcurrencyInput(String(fallback));
                    setPluginMaxConcurrency(fallback);
                  } else if (val > 8192) {
                    const clamped = 8192;
                    setConcurrencyInput(String(clamped));
                    setPluginMaxConcurrency(clamped);
                  }
                }}
                placeholder="256"
                className="w-full"
              />
              <p className="text-xs text-muted-foreground leading-relaxed">
                {ll?.settings?.plugin_max_concurrency_tip() ?? 'Max in-flight concurrent requests per plugin process (default 256, increase to 1024+ for heavy stress testing).'}
              </p>
            </div>

            {/* 插件通信队列深度 */}
            <div className="space-y-2">
              <Label className="text-sm font-medium text-foreground flex items-center gap-2">
                <Layers className="h-4 w-4 text-muted-foreground" />
                {ll?.settings?.plugin_queue_capacity() ?? 'Plugin Queue Capacity'}
              </Label>
              <Input
                type="number"
                min={16}
                max={65536}
                step={256}
                value={queueCapInput}
                onChange={(e) => {
                  const text = e.target.value;
                  setQueueCapInput(text);
                  if (text !== '') {
                    const val = Number(text);
                    if (!isNaN(val) && val >= 16 && val <= 65536) {
                      setPluginQueueCapacity(val);
                    }
                  }
                }}
                onBlur={() => {
                  const val = Number(queueCapInput);
                  if (queueCapInput === '' || isNaN(val) || val < 16) {
                    const fallback = 2048;
                    setQueueCapInput(String(fallback));
                    setPluginQueueCapacity(fallback);
                  } else if (val > 65536) {
                    const clamped = 65536;
                    setQueueCapInput(String(clamped));
                    setPluginQueueCapacity(clamped);
                  }
                }}
                placeholder="2048"
                className="w-full"
              />
              <p className="text-xs text-muted-foreground leading-relaxed">
                {ll?.settings?.plugin_queue_capacity_tip() ?? 'Communication frame queue depth between host and plugin process (default 2048).'}
              </p>
            </div>

            {/* AI Copilot & LLM Provider 配置 */}
            <AIProviderSettingsCard />


            {/* 流程保存目录 */}
            <div className="space-y-2">
              <Label className="flex items-center gap-2 text-sm">
                <FolderInput className="h-4 w-4 text-muted-foreground" />
                {ll?.settings?.flows_dir() ?? 'Flows Directory'}
              </Label>
              <div className="text-sm text-foreground bg-muted border border-border rounded-md px-3 py-2 break-all leading-relaxed">
                {isLoading ? (ll?.settings?.loading() ?? 'Loading...') : flowsDir || (ll?.settings?.not_set() ?? 'Not Set')}
              </div>
              <div className="flex gap-2">
                <Button variant="outline" size="sm" onClick={handleSelectFlowsDir} className="flex-1">
                  {ll?.settings?.select_dir() ?? 'Select Directory'}
                </Button>
                <Button variant="outline" size="sm" onClick={openFlowsDir} className="flex-1">
                  {ll?.settings?.open_dir() ?? 'Open Directory'}
                </Button>
              </div>
            </div>

            {/* 报告目录 */}
            <div className="space-y-2">
              <Label className="flex items-center gap-2 text-sm">
                <FolderOpen className="h-4 w-4 text-muted-foreground" />
                {ll?.settings?.reports_dir() ?? 'Reports Directory'}
              </Label>
              {/* 路径显示 - 允许换行 */}
              <div className="text-sm text-foreground bg-muted border border-border rounded-md px-3 py-2 break-all leading-relaxed">
                {isLoading ? (ll?.settings?.loading() ?? 'Loading...') : reportDir || (ll?.settings?.not_set() ?? 'Not Set')}
              </div>
              {/* 按钮行 */}
              <div className="flex gap-2">
                <Button variant="outline" size="sm" onClick={handleSelectReportDir} className="flex-1">
                  {ll?.settings?.select_dir() ?? 'Select Directory'}
                </Button>
                <Button variant="outline" size="sm" onClick={openReportDir} className="flex-1">
                  {ll?.settings?.open_dir() ?? 'Open Directory'}
                </Button>
              </div>
            </div>

            {/* 日志目录 */}
            <div className="space-y-2">
              <Label className="flex items-center gap-2 text-sm">
                <FolderCog className="h-4 w-4 text-muted-foreground" />
                {ll?.settings?.logs_dir() ?? 'Logs Directory'}
              </Label>
              {/* 路径显示 - 允许换行 */}
              <div className="text-sm text-foreground bg-muted border border-border rounded-md px-3 py-2 break-all leading-relaxed">
                {isLoading ? (ll?.settings?.loading() ?? 'Loading...') : logDir || (ll?.settings?.not_set() ?? 'Not Set')}
              </div>
              {/* 按钮行 */}
              <div className="flex gap-2">
                <Button variant="outline" size="sm" onClick={handleSelectLogDir} className="flex-1">
                  {ll?.settings?.select_dir() ?? 'Select Directory'}
                </Button>
                <Button variant="outline" size="sm" onClick={openLogDir} className="flex-1">
                  {ll?.settings?.open_dir() ?? 'Open Directory'}
                </Button>
              </div>
            </div>
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
