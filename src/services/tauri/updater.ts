/**
 * 客户端版本更新服务
 */
import { isTauriEnv } from '@/lib/platform';
import type { UpdateInfo, UpdateProgress } from '@/types/update';

interface RustUpdateCheckResult {
  current_version: string;
  latest_version: string;
  has_update: boolean;
  release_date?: string | null;
  release_notes?: string | null;
  is_prerelease: boolean;
  download_url?: string | null;
  platform: string;
}

interface RustDownloadProgress {
  received: number;
  total?: number | null;
  percent: number;
  bytes_per_sec: number;
}

/**
 * 获取当前客户端版本
 */
export async function getAppVersion(): Promise<string> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string>('get_app_version');
  }
  const { getAppVersion: mock } = await import('./updater.mock');
  return mock();
}

/**
 * 检查应用是否有新版本
 */
export async function checkAppUpdate(includePrerelease?: boolean): Promise<UpdateInfo> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    const res = await invoke<RustUpdateCheckResult>('check_app_update', {
      includePrerelease: includePrerelease ?? null,
    });

    return {
      currentVersion: res.current_version,
      latestVersion: res.latest_version,
      hasUpdate: res.has_update,
      releaseDate: res.release_date ?? null,
      releaseNotes: res.release_notes ?? null,
      isPrerelease: res.is_prerelease,
      downloadUrl: res.download_url ?? null,
      platform: res.platform,
    };
  }
  const { checkAppUpdate: mock } = await import('./updater.mock');
  return mock(includePrerelease);
}

/**
 * 下载指定版本的安装包，支持实时进度监听
 */
export async function downloadUpdatePackage(
  url: string,
  onProgress?: (p: UpdateProgress) => void
): Promise<string> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    const { listen } = await import('@tauri-apps/api/event');

    let unlisten: (() => void) | null = null;
    if (onProgress) {
      unlisten = await listen<RustDownloadProgress>('app-update-progress', (event) => {
        onProgress({
          received: event.payload.received,
          total: event.payload.total,
          percent: event.payload.percent,
          bytesPerSec: event.payload.bytes_per_sec,
        });
      });
    }

    try {
      return await invoke<string>('download_update_package', { url });
    } finally {
      if (unlisten) {
        unlisten();
      }
    }
  }
  const { downloadUpdatePackage: mock } = await import('./updater.mock');
  return mock(url, onProgress);
}

/**
 * 执行安装包并在 Windows 上自动退出当前程序
 */
export async function installUpdatePackage(filePath: string): Promise<void> {
  if (isTauriEnv()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<void>('install_update_package', { filePath });
  }
  const { installUpdatePackage: mock } = await import('./updater.mock');
  return mock(filePath);
}
