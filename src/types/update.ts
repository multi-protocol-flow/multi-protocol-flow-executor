/**
 * 应用版本更新相关类型定义
 */

/**
 * 更新检查结果信息
 */
export interface UpdateInfo {
  /** 当前客户端版本 */
  currentVersion: string;
  /** 匹配到的最新版本 */
  latestVersion: string;
  /** 是否有可用更新 */
  hasUpdate: boolean;
  /** 发布日期（ISO 8601） */
  releaseDate?: string | null;
  /** 更新说明日志（Markdown 格式） */
  releaseNotes?: string | null;
  /** 是否为预发布版本 */
  isPrerelease: boolean;
  /** 当前平台的安装包直链 */
  downloadUrl?: string | null;
  /** 当前平台识别符 ("windows" | "macos" | "linux") */
  platform: string;
}

/**
 * 下载进度信息
 */
export interface UpdateProgress {
  /** 已接收字节数 */
  received: number;
  /** 文件总字节数 */
  total?: number | null;
  /** 进度百分比 (0.0 ~ 100.0) */
  percent: number;
  /** 当前下载速率（字节/秒） */
  bytesPerSec: number;
}

/**
 * 更新处理状态机状态
 */
export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'available'
  | 'up-to-date'
  | 'downloading'
  | 'downloaded'
  | 'error';

/**
 * 更新 Store 状态与操作
 */
export interface UpdateState {
  /** 当前状态 */
  status: UpdateStatus;
  /** 更新检测结果详情 */
  updateInfo: UpdateInfo | null;
  /** 当前运行时版本 */
  currentVersion: string;
  /** 实时下载进度 */
  progress: UpdateProgress | null;
  /** 已下载完成的文件本地绝对路径 */
  downloadedFilePath: string | null;
  /** 错误信息 */
  error: string | null;
  /** 上次检查时间戳（毫秒） */
  lastChecked: number | null;
  /** 是否弹出展示更新对话框 */
  showUpdateDialog: boolean;
}

export interface UpdateActions {
  /** 加载当前客户端版本号 */
  loadCurrentVersion: () => Promise<string>;
  /** 检查版本更新（manual = true 表示用户手动触发） */
  checkForUpdates: (manual?: boolean) => Promise<UpdateInfo | null>;
  /** 开始下载最新版本的安装包 */
  startDownload: () => Promise<string | null>;
  /** 执行安装程序并重启/打开镜像 */
  installAndRestart: () => Promise<void>;
  /** 用户选择忽略当前检测到的新版本 */
  ignoreCurrentVersion: () => Promise<void>;
  /** 打开更新详情对话框 */
  openUpdateDialog: () => void;
  /** 关闭更新详情对话框 */
  closeUpdateDialog: () => void;
  /** 重置状态（用于测试隔离） */
  reset: () => void;
}

export type UpdateStore = UpdateState & UpdateActions;
