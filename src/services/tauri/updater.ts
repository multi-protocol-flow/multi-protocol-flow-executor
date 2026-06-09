import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import type { DownloadEvent, UpdateInfo } from '@/types/updater'

export async function checkForUpdates(): Promise<UpdateInfo | null> {
  try {
    const update = await check()
    if (!update) return null
    return {
      available: true,
      currentVersion: update.currentVersion,
      version: update.version,
      date: update.date,
      body: update.body,
    }
  } catch (e) {
    return { available: false, error: String(e) }
  }
}

export async function downloadAndInstallUpdate(
  onEvent?: (event: DownloadEvent) => void
): Promise<void> {
  const update = await check()
  if (!update) throw new Error('No update available')

  await update.downloadAndInstall((progress) => {
    if (onEvent) {
      onEvent({
        event: progress.event,
        data: 'data' in progress ? progress.data : undefined,
      })
    }
  })

  await relaunch()
}
