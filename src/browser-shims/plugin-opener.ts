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

/**
 * Reveal a path in the system default file manager.
 * No-op fallback in browser mode.
 */
export async function revealItemInDir(_path: string | string[]): Promise<void> {
  // Browser fallback - no native file manager access
}

/**
 * Open a path with the system default application.
 * No-op fallback in browser mode.
 */
export async function openPath(_path: string, _openWith?: string): Promise<void> {
  // Browser fallback
}
