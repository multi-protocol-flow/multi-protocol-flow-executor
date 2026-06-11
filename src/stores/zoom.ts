/**
 * 缩放状态管理
 * 管理页面级缩放，通过 Tauri 原生 WebView setZoom API 实现
 */
import { create } from 'zustand';

/** 缩放范围常量 */
const ZOOM_MIN = 0.5;
const ZOOM_MAX = 2.0;
const ZOOM_STEP = 0.1;
const ZOOM_DEFAULT = 1.0;
const STORAGE_KEY = 'app-zoom-level';

/** 检测是否在 Tauri 环境中 */
function isTauriEnv(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

/** 从 localStorage 恢复缩放值 */
function loadZoomFromStorage(): number {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      const value = parseFloat(saved);
      if (!isNaN(value) && value >= ZOOM_MIN && value <= ZOOM_MAX) {
        return value;
      }
    }
  } catch {
    // localStorage 不可用时忽略
  }
  return ZOOM_DEFAULT;
}

/** 保存缩放值到 localStorage */
function saveZoomToStorage(zoom: number): void {
  try {
    localStorage.setItem(STORAGE_KEY, zoom.toString());
  } catch {
    // localStorage 不可用时忽略
  }
}

/** 调用 Tauri 原生 setZoom API */
async function applyNativeZoom(zoom: number): Promise<void> {
  if (!isTauriEnv()) return;
  try {
    const { getCurrentWebview } = await import('@tauri-apps/api/webview');
    await getCurrentWebview().setZoom(zoom);
  } catch (error) {
    console.debug('[Zoom] Failed to set native zoom:', error);
  }
}

interface ZoomState {
  /** 当前缩放比例 (0.5 - 2.0) */
  zoom: number;

  // ===== Actions =====
  /** 设置缩放比例（自动钳位到有效范围） */
  setZoom: (zoom: number) => void;
  /** 放大一步 */
  zoomIn: () => void;
  /** 缩小一步 */
  zoomOut: () => void;
  /** 重置为默认缩放 */
  resetZoom: () => void;
  /** 应用当前缩放（启动时恢复用） */
  applyZoom: () => void;
}

export const useZoomStore = create<ZoomState>((set, get) => ({
  zoom: loadZoomFromStorage(),

  setZoom: (zoom: number) => {
    const clamped = Math.round(Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, zoom)) * 100) / 100;
    saveZoomToStorage(clamped);
    applyNativeZoom(clamped);
    set({ zoom: clamped });
  },

  zoomIn: () => {
    const { zoom, setZoom } = get();
    setZoom(zoom + ZOOM_STEP);
  },

  zoomOut: () => {
    const { zoom, setZoom } = get();
    setZoom(zoom - ZOOM_STEP);
  },

  resetZoom: () => {
    get().setZoom(ZOOM_DEFAULT);
  },

  applyZoom: () => {
    const { zoom } = get();
    applyNativeZoom(zoom);
  },
}));

// ===== 常量导出（供 UI 组件使用） =====
export { ZOOM_MIN, ZOOM_MAX, ZOOM_STEP, ZOOM_DEFAULT };

// ===== Selectors =====
/** 选择当前缩放比例 */
export const selectZoom = (state: ZoomState) => state.zoom;
