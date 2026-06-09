/**
 * UpdateDialog - 应用更新对话框组件
 * 支持 4 种状态: 检查中、更新可用(含进度)、已是最新、错误
 */
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { useUIStore } from '@/stores/ui';
import { useUpdaterStore, selectUpdateAvailable } from '@/stores/updater';
import { getLL } from '@/i18n/i18n-lazy';
import { useEffect } from 'react';

export function UpdateDialog() {
  const showUpdateDialog = useUIStore((state) => state.showUpdateDialog);
  const closeUpdateDialog = useUIStore((state) => state.closeUpdateDialog);
  const checking = useUpdaterStore((state) => state.checking);
  const updateInfo = useUpdaterStore((state) => state.updateInfo);
  const downloading = useUpdaterStore((state) => state.downloading);
  const downloadProgress = useUpdaterStore((state) => state.downloadProgress);
  const error = useUpdaterStore((state) => state.error);
  const checkForUpdates = useUpdaterStore((state) => state.checkForUpdates);
  const downloadAndInstall = useUpdaterStore((state) => state.downloadAndInstall);
  const dismissUpdate = useUpdaterStore((state) => state.dismissUpdate);
  const hasUpdate = useUpdaterStore(selectUpdateAvailable);
  const LL = getLL();

  useEffect(() => {
    if (showUpdateDialog && !updateInfo && !checking) {
      checkForUpdates();
    }
  }, [showUpdateDialog]);

  if (!showUpdateDialog) return null;

  return (
    <Dialog open={showUpdateDialog} onOpenChange={(open) => { if (!open) closeUpdateDialog(); }}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{LL.settings.update.check_updates()}</DialogTitle>
        </DialogHeader>

        {checking && (
          <p className="text-sm text-muted-foreground">{LL.settings.update.checking()}</p>
        )}

        {error && (
          <p className="text-sm text-red-500">{LL.settings.update.update_error({ error })}</p>
        )}

        {hasUpdate && updateInfo && (
          <div className="space-y-4">
            <p>{LL.settings.update.update_available({ version: updateInfo.version || '' })}</p>
            {updateInfo.date && (
              <p className="text-sm text-muted-foreground">
                {LL.settings.update.released_at({ date: updateInfo.date })}
              </p>
            )}
            {updateInfo.body && (
              <div className="max-h-40 overflow-y-auto rounded-md border p-2 text-sm">
                <p className="font-medium mb-1">{LL.settings.update.release_notes()}</p>
                <p className="whitespace-pre-wrap text-muted-foreground">{updateInfo.body}</p>
              </div>
            )}

            {downloading && (
              <div className="space-y-2">
                <Progress value={downloadProgress} className="w-full" />
                <p className="text-xs text-muted-foreground text-right">
                  {LL.settings.update.downloading({ progress: Math.round(downloadProgress) })}
                </p>
              </div>
            )}

            <DialogFooter className="flex gap-2">
              {!downloading && (
                <>
                  <Button onClick={downloadAndInstall}>
                    {LL.settings.update.download_and_install()}
                  </Button>
                  <Button variant="outline" onClick={dismissUpdate}>
                    {LL.settings.update.skip_version()}
                  </Button>
                </>
              )}
              <Button variant="ghost" onClick={closeUpdateDialog}>
                {LL.settings.update.later()}
              </Button>
            </DialogFooter>
          </div>
        )}

        {!checking && !hasUpdate && !error && (
          <p className="text-sm text-muted-foreground">{LL.settings.update.up_to_date()}</p>
        )}
      </DialogContent>
    </Dialog>
  );
}
