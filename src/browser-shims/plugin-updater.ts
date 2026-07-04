/**
 * Browser shim for @tauri-apps/plugin-updater
 *
 * Provides no-op/stub implementations for browser-only environments.
 */

/**
 * Browser shim for the Update class matching the real @tauri-apps/plugin-updater Update type.
 */
export class Update {
  available = false;
  currentVersion = '';
  version = '';
  date?: string;
  body?: string;
  rawJson: Record<string, unknown> = {};

  /** Download and install the update — throws in browser mode */
  async downloadAndInstall(_onEvent?: (progress: unknown) => void): Promise<void> {
    throw new Error('downloadAndInstall() not available in browser mode');
  }
}

/**
 * Check for application updates.
 * In browser mode, no updates are available.
 */
export async function check(): Promise<Update | null> {
  return null;
}
