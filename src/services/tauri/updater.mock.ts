import type { UpdateInfo } from '@/types/updater'

export async function checkForUpdates(): Promise<UpdateInfo | null> {
  return { available: false }
}

export async function downloadAndInstallUpdate(): Promise<void> {
  throw new Error('Cannot download updates in development mode')
}
