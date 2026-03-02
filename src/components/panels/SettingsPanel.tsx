/**
 * SettingsPanel - 全局设置面板
 * 复用 NodeConfigPanel 的布局样式，提供配置文件存储位置提示
 */
import { useEffect, useState } from 'react';
import { Settings2, FolderOpen, FolderCog, Info } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { useSettingsStore } from '@/stores/settings';

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
    isLoading,
    loadSettings,
    setReportDir,
    setLogDir,
    openReportDir,
    openLogDir,
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

  return (
    <Card className="flex h-full flex-col overflow-hidden border-0 shadow-none w-full min-w-0">
      <CardHeader className="shrink-0 border-b border-muted bg-white pb-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 flex-1 min-w-0">
            <Settings2 className="h-4 w-4 text-slate-400 shrink-0" />
            <CardTitle className="text-base font-semibold text-black truncate cursor-default">
              全局设置
            </CardTitle>
          </div>
          <Badge variant="outline" className="border-slate-200 text-slate-600 shrink-0">
            设置
          </Badge>
        </div>
      </CardHeader>

      <CardContent className="flex-1 overflow-hidden p-0">
        <ScrollArea className="h-full">
          <div className="w-full min-w-0 max-w-full overflow-hidden px-6 py-4 space-y-5">
            {/* 配置文件存储位置提示 */}
            <div className="text-xs text-slate-500 py-2 px-3 rounded-lg bg-slate-50 border border-slate-100 space-y-1">
              <div className="flex items-center gap-1.5">
                <Info className="h-3.5 w-3.5 text-slate-400 shrink-0" />
                <span className="font-medium text-slate-600">配置文件</span>
              </div>
              <code className="block text-slate-600 break-all leading-relaxed">
                {isLoading ? '加载中...' : configFilePath || '未知'}
              </code>
            </div>

            {/* 报告目录 */}
            <div className="space-y-2">
              <Label className="flex items-center gap-2 text-sm">
                <FolderOpen className="h-4 w-4 text-slate-400" />
                报告目录
              </Label>
              {/* 路径显示 - 允许换行 */}
              <div className="text-sm text-slate-600 bg-slate-50 border border-slate-200 rounded-md px-3 py-2 break-all leading-relaxed">
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
                <FolderCog className="h-4 w-4 text-slate-400" />
                日志目录
              </Label>
              {/* 路径显示 - 允许换行 */}
              <div className="text-sm text-slate-600 bg-slate-50 border border-slate-200 rounded-md px-3 py-2 break-all leading-relaxed">
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
