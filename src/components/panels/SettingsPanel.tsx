/**
 * SettingsPanel - 全局设置面板
 * 复用 NodeConfigPanel 的布局样式，提供配置文件存储位置提示
 */
import { useEffect, useState } from 'react';
import { Settings2, FolderOpen, FolderCog, FolderInput, Info, Sun, Moon, Monitor } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useSettingsStore } from '@/stores/settings';
import type { ThemeMode } from '@/types/settings';

// 动态导入设置服务（根据环境）
async function getSettingsService() {
  if (typeof window !== 'undefined' && '__TAURI__' in window) {
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
    isLoading,
    loadSettings,
    setReportDir,
    setLogDir,
    setFlowsDir,
    setTheme,
    openReportDir,
    openLogDir,
    openFlowsDir,
  } = useSettingsStore();

  // 配置文件存储路径
  const [configFilePath, setConfigFilePath] = useState<string>('');

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
      setConfigFilePath('浏览器本地存储 (localStorage)');
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
              全局设置
            </CardTitle>
          </div>
          <Badge variant="outline" className="border-border text-muted-foreground shrink-0">
            设置
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
                <span className="font-medium text-foreground">配置文件</span>
              </div>
              <code className="block text-foreground break-all leading-relaxed">
                {isLoading ? '加载中...' : configFilePath || '未知'}
              </code>
            </div>

            {/* 主题 */}
            <div className="space-y-2">
              <Label className="text-sm font-medium text-foreground flex items-center gap-2">
                主题
              </Label>
              <Select value={theme} onValueChange={(value) => setTheme(value as ThemeMode)}>
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="选择主题" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="light">
                    <span className="flex items-center gap-2">
                      <Sun className="h-4 w-4" />
                      浅色
                    </span>
                  </SelectItem>
                  <SelectItem value="dark">
                    <span className="flex items-center gap-2">
                      <Moon className="h-4 w-4" />
                      暗黑
                    </span>
                  </SelectItem>
                  <SelectItem value="system">
                    <span className="flex items-center gap-2">
                      <Monitor className="h-4 w-4" />
                      跟随系统
                    </span>
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>

            {/* 流程保存目录 */}
            <div className="space-y-2">
              <Label className="flex items-center gap-2 text-sm">
                <FolderInput className="h-4 w-4 text-muted-foreground" />
                流程保存目录
              </Label>
              <div className="text-sm text-foreground bg-muted border border-border rounded-md px-3 py-2 break-all leading-relaxed">
                {isLoading ? '加载中...' : flowsDir || '未设置'}
              </div>
              <div className="flex gap-2">
                <Button variant="outline" size="sm" onClick={handleSelectFlowsDir} className="flex-1">
                  选择目录
                </Button>
                <Button variant="outline" size="sm" onClick={openFlowsDir} className="flex-1">
                  打开目录
                </Button>
              </div>
            </div>

            {/* 报告目录 */}
            <div className="space-y-2">
              <Label className="flex items-center gap-2 text-sm">
                <FolderOpen className="h-4 w-4 text-muted-foreground" />
                报告目录
              </Label>
              {/* 路径显示 - 允许换行 */}
              <div className="text-sm text-foreground bg-muted border border-border rounded-md px-3 py-2 break-all leading-relaxed">
                {isLoading ? '加载中...' : reportDir || '未设置'}
              </div>
              {/* 按钮行 */}
              <div className="flex gap-2">
                <Button variant="outline" size="sm" onClick={handleSelectReportDir} className="flex-1">
                  选择目录
                </Button>
                <Button variant="outline" size="sm" onClick={openReportDir} className="flex-1">
                  打开目录
                </Button>
              </div>
            </div>

            {/* 日志目录 */}
            <div className="space-y-2">
              <Label className="flex items-center gap-2 text-sm">
                <FolderCog className="h-4 w-4 text-muted-foreground" />
                日志目录
              </Label>
              {/* 路径显示 - 允许换行 */}
              <div className="text-sm text-foreground bg-muted border border-border rounded-md px-3 py-2 break-all leading-relaxed">
                {isLoading ? '加载中...' : logDir || '未设置'}
              </div>
              {/* 按钮行 */}
              <div className="flex gap-2">
                <Button variant="outline" size="sm" onClick={handleSelectLogDir} className="flex-1">
                  选择目录
                </Button>
                <Button variant="outline" size="sm" onClick={openLogDir} className="flex-1">
                  打开目录
                </Button>
              </div>
            </div>
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
