/**
 * 版本更新服务 Mock 实现（用于浏览器预览与前端测试）
 */
import type { UpdateInfo, UpdateProgress } from '@/types/update';

const MOCK_VERSION = '0.1.1-beta.2';

export async function getAppVersion(): Promise<string> {
  return MOCK_VERSION;
}

export async function checkAppUpdate(_includePrerelease?: boolean): Promise<UpdateInfo> {
  return {
    currentVersion: MOCK_VERSION,
    latestVersion: '0.1.2',
    hasUpdate: true,
    releaseDate: '2026-08-25T10:00:00Z',
    releaseNotes: '### What\'s New in v0.1.2\n- Support in-app updater and version checking\n- Performance improvements and bug fixes',
    isPrerelease: false,
    downloadUrl: 'https://download.mpe.run/releases/v0.1.2/mpe-0.1.2-x64-setup.exe',
    platform: 'windows',
  };
}

export async function downloadUpdatePackage(
  _url: string,
  onProgress?: (p: UpdateProgress) => void
): Promise<string> {
  if (onProgress) {
    onProgress({ received: 5000000, total: 20000000, percent: 25, bytesPerSec: 1024000 });
    await new Promise((r) => setTimeout(r, 50));
    onProgress({ received: 15000000, total: 20000000, percent: 75, bytesPerSec: 2048000 });
    await new Promise((r) => setTimeout(r, 50));
    onProgress({ received: 20000000, total: 20000000, percent: 100, bytesPerSec: 2500000 });
  }
  return '/tmp/mpe-update-mock.exe';
}

export async function installUpdatePackage(_filePath: string): Promise<void> {
  // No-op in mock mode
}
