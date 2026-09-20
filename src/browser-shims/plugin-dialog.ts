/**
 * Browser shim for @tauri-apps/plugin-dialog
 *
 * Provides no-op/stub implementations for browser-only environments.
 */

/**
 * Open a native file/directory selection dialog.
 * In browser mode, this is not available and returns null (user cancelled).
 */
export async function open(_options?: Record<string, unknown>): Promise<string | null> {
  // File dialogs not available in browser — return "cancelled"
  return null;
}

/**
 * Open a native save file dialog.
 * In browser mode, returns null.
 */
export async function save(_options?: Record<string, unknown>): Promise<string | null> {
  return null;
}
