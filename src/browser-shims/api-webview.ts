/**
 * Browser shim for @tauri-apps/api/webview
 *
 * Provides no-op/stub implementations for browser-only environments.
 */

/**
 * A shim for the Webview class.
 */
class Webview {
  label = 'main';

  /** Set webview zoom level — no-op in browser mode */
  async setZoom(_scaleFactor: number): Promise<void> {
    // no-op in browser mode
  }

  /** Show the webview — no-op in browser mode */
  async show(): Promise<void> {
    // no-op in browser mode
  }
}

/**
 * Get an instance of `Webview` for the current webview.
 * In browser mode, returns a no-op stub.
 */
export function getCurrentWebview(): Webview {
  return new Webview();
}
