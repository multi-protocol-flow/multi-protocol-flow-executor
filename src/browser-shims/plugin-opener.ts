/**
 * Browser shim for @tauri-apps/plugin-opener
 *
 * Provides fallback implementation using the browser's window.open.
 */

/**
 * Open a URL in the default browser.
 * Falls back to window.open in browser mode.
 */
export async function openUrl(url: string): Promise<void> {
  window.open(url, '_blank');
}
