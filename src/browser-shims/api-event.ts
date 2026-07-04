/**
 * Browser shim for @tauri-apps/api/event
 *
 * Provides no-op/stub implementations for browser-only environments.
 */

/**
 * Function type returned by listen() to unregister the event listener.
 */
export type UnlistenFn = () => void;

/**
 * Tauri event object matching the real @tauri-apps/api/event Event type.
 */
interface Event<T> {
  /** Event name */
  event: string;
  /** Event identifier used to unlisten */
  id: number;
  /** Event payload */
  payload: T;
}

/**
 * Listen to a Tauri global event.
 * In browser mode, Tauri events are not available — returns a no-op unlisten function.
 */
export async function listen<T>(
  _event: string,
  _handler: (event: Event<T>) => void
): Promise<UnlistenFn> {
  // Events from Tauri backend are not available in browser mode
  return () => {
    // no-op unlistener
  };
}
