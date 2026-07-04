/**
 * Browser shim for @tauri-apps/plugin-store
 *
 * Provides an in-memory Map-based implementation for browser-only environments.
 * Data is NOT persisted across page reloads.
 */

/**
 * LazyStore provides a simple key-value store.
 * In browser mode, data is stored in memory and will be lost on page reload.
 */
export class LazyStore {
  private data = new Map<string, string>();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  constructor(_path: string) {
    // Store path is ignored in browser mode
  }

  /**
   * Get a value by key.
   * @returns the stored value, or undefined if not found
   */
  async get<T = string>(key: string): Promise<T | undefined> {
    return this.data.get(key) as T | undefined;
  }

  /**
   * Set a value by key.
   */
  async set(key: string, value: unknown): Promise<void> {
    this.data.set(key, typeof value === 'string' ? value : JSON.stringify(value));
  }

  /**
   * Persist data to disk.
   * In browser mode, this is a no-op.
   */
  async save(): Promise<void> {
    // In-memory only — no persistence in browser mode
  }
}
