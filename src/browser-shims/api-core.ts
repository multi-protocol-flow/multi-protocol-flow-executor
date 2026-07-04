/**
 * Browser shim for @tauri-apps/api/core
 *
 * Provides no-op/stub implementations for browser-only environments
 * where Tauri APIs are not available.
 */

/**
 * Invoke a Tauri backend command.
 * In browser mode, this is not available and will throw an error.
 */
export async function invoke<T>(command: string, _args?: Record<string, unknown>): Promise<T> {
  throw new Error(`invoke() not available in browser mode: ${command}`);
}

/**
 * Channel for receiving streaming data from Tauri commands.
 * In browser mode, this is a no-op stub.
 */
export class Channel<T> {
  /** The channel id */
  id: number = 0;

  /** Callback invoked when a message is received on this channel */
  onmessage: (response: T) => void = () => {
    // no-op in browser mode
  };

  /** Serialize for IPC — no-op in browser mode */
  toJSON(): Record<string, unknown> {
    return { id: this.id };
  }
}
