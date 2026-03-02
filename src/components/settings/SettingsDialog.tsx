/**
 * SettingsDialog - 全局设置对话框组件
 * 配置报告目录和日志目录
 */
import { useEffect } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { useUIStore } from '@/stores/ui';
import { useSettingsStore } from '@/stores/settings';

// 动态导入设置服务（根据环境）
async function getSettingsService() {
  if (typeof window !== 'undefined' && '__TAURI__' in window) {
    return import('@/services/tauri/settings');
  }
  return import('@/services/tauri/settings.mock');
}

export function SettingsDialog() {
  const showSettingsDialog = useUIStore((state) => state.showSettingsDialog);
  const closeSettingsDialog = useUIStore((state) => state.closeSettingsDialog);

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

  // 对话框打开时加载设置
  useEffect(() => {
    if (showSettingsDialog) {
      loadSettings();
    }
  }, [showSettingsDialog, loadSettings]);

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

  if (!showSettingsDialog) return null;

  return (
    <Dialog open={showSettingsDialog} onOpenChange={(open) => !open && closeSettingsDialog()}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>全局设置</DialogTitle>
        </DialogHeader>
        <div className="space-y-6 py-4">
          {/* 报告目录 */}
          <div className="space-y-2">
            <Label htmlFor="report-dir">报告目录</Label>
            <div className="flex gap-2">
              <Input
                id="report-dir"
                value={isLoading ? '加载中...' : reportDir}
                readOnly
                className="flex-1"
              />
              <Button variant="outline" onClick={handleSelectReportDir}>
                选择
              </Button>
              <Button variant="outline" onClick={openReportDir}>
                打开
              </Button>
            </div>
          </div>

          {/* 日志目录 */}
          <div className="space-y-2">
            <Label htmlFor="log-dir">日志目录</Label>
            <div className="flex gap-2">
              <Input
                id="log-dir"
                value={isLoading ? '加载中...' : logDir}
                readOnly
                className="flex-1"
              />
              <Button variant="outline" onClick={handleSelectLogDir}>
                选择
              </Button>
              <Button variant="outline" onClick={openLogDir}>
                打开
              </Button>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
