/**
 * Browser shim for @tauri-apps/api/window
 *
 * Provides no-op/stub implementations for browser-only environments.
 */

/**
 * A shim for the Window class.
 */
class Window {
  label = 'main';

  /** Show the window — no-op in browser mode */
  async show(): Promise<void> {
    // no-op in browser mode
  }
}

/**
 * Get an instance of `Window` for the current window.
 * In browser mode, returns a no-op stub.
 */
export function getCurrentWindow(): Window {
  return new Window();
}
