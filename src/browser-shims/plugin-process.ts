/**
 * Browser shim for @tauri-apps/plugin-process
 *
 * Provides no-op/stub implementations for browser-only environments.
 */

/**
 * Relaunch the application.
 * In browser mode, this is not available and will throw an error.
 */
export async function relaunch(): Promise<void> {
  throw new Error('relaunch() not available in browser mode');
}
