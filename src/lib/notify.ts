/**
 * 全局统一操作通知与日志分发中心
 * 1. 在右上角弹出带详情与复制按钮的 Toast 通知
 * 2. 同步将系统/操作日志写入控制台 (useConsoleStore.addSystemLog)
 * 3. 记录到前端日志与浏览器控制台
 */
import { toast } from 'sonner';
import { useConsoleStore } from '@/stores/console';
import { createLogger } from '@/lib/logger';
import { copyToClipboard } from '@/lib/clipboard';
import { getLL } from '@/i18n/i18n-lazy';
export interface NotifyErrorOptions {
  /** 模块/分类名称，例如 'FlowStorage', 'Plugin', 'Settings', 'IPC', 'Execution' */
  category?: string;
  /** 关联的流程 UUID */
  flowUuid?: string;
  /** 是否静默 Toast（仅记录到控制台系统日志，不弹窗） */
  silentToast?: boolean;
  /** 自定义 Toast 显示时长 (ms) */
  duration?: number;
}

export interface NotifySuccessOptions {
  category?: string;
  flowUuid?: string;
  duration?: number;
}

export interface NotifyInfoOptions {
  category?: string;
  flowUuid?: string;
  duration?: number;
}

export interface NotifyWarningOptions {
  category?: string;
  flowUuid?: string;
  duration?: number;
}

/**
 * 格式化提取错误详情文本
 */
export function formatErrorDetail(error: unknown): { message: string; stack?: string; raw: unknown } {
  if (!error) {
    return { message: 'Unknown error', raw: error };
  }
  if (typeof error === 'string') {
    return { message: error, raw: error };
  }
  if (error instanceof Error) {
    return {
      message: error.message,
      stack: error.stack,
      raw: error,
    };
  }
  if (typeof error === 'object') {
    const errObj = error as Record<string, unknown>;
    const message =
      typeof errObj.message === 'string'
        ? errObj.message
        : typeof errObj.error === 'string'
          ? errObj.error
          : JSON.stringify(error);
    return {
      message,
      stack: typeof errObj.stack === 'string' ? errObj.stack : undefined,
      raw: error,
    };
  }
  return { message: String(error), raw: error };
}

/**
 * 统一操作失败通知与系统日志记录
 */
export function notifyError(title: string, error: unknown, options?: NotifyErrorOptions): void {
  const { message, stack, raw } = formatErrorDetail(error);
  const category = options?.category ?? 'System';

  // 1. 同步写入底部 Console 面板
  try {
    useConsoleStore.getState().addSystemLog({
      level: 'ERROR',
      message: `${title}: ${message}`,
      category,
      flowUuid: options?.flowUuid,
      details: stack ? { message, stack, raw } : raw,
    });
  } catch (logErr) {
    console.error('[notifyError] Failed to add system log to console store:', logErr);
  }

  // 2. 本地控制台输出与持久化日志 (Tauri app.log)
  createLogger(category).error(`${title}: ${message}`, stack ? { stack } : undefined);
  // 3. 右上角弹窗 Toast (带详细描述和复制按钮)
  if (!options?.silentToast) {
    const copyText = stack
      ? `${title}\nError: ${message}\nStack: ${stack}`
      : `${title}\nError: ${message}`;
    const ll = getLL();
    const copyLabel = ll.flow?.copy_details?.() ?? 'Copy Details';
    const copiedLabel = ll.flow?.details_copied?.() ?? 'Copied to clipboard';

    toast.error(title, {
      description: message,
      duration: options?.duration ?? 5000,
      action: {
        label: copyLabel,
        onClick: () => {
          copyToClipboard(copyText).then((ok) => {
            if (ok) {
              toast.success(copiedLabel, { duration: 1500 });
            }
          }).catch(() => {});
        },
      },
    });
  }
}

/**
 * 统一操作成功通知与系统日志记录
 */
export function notifySuccess(
  title: string,
  description?: string,
  options?: NotifySuccessOptions
): void {
  const category = options?.category ?? 'System';
  try {
    useConsoleStore.getState().addSystemLog({
      level: 'INFO',
      message: description ? `${title} - ${description}` : title,
      category,
      flowUuid: options?.flowUuid,
    });
  } catch {
    // 忽略记录错误
  }
  createLogger(category).info(description ? `${title} - ${description}` : title);

  toast.success(title, {
    description,
    duration: options?.duration ?? 3000,
  });
}

/**
 * 统一警告通知与系统日志记录
 */
export function notifyWarning(
  title: string,
  description?: string,
  options?: NotifyWarningOptions
): void {
  const category = options?.category ?? 'System';
  try {
    useConsoleStore.getState().addSystemLog({
      level: 'WARN',
      message: description ? `${title} - ${description}` : title,
      category,
      flowUuid: options?.flowUuid,
    });
  } catch {
    // 忽略记录错误
  }

  createLogger(category).warn(description ? `${title} - ${description}` : title);
  toast.warning(title, {
    description,
    duration: options?.duration ?? 4000,
  });
}

/**
 * 统一信息通知与系统日志记录
 */
export function notifyInfo(
  title: string,
  description?: string,
  options?: NotifyInfoOptions
): void {
  const category = options?.category ?? 'System';
  try {
    useConsoleStore.getState().addSystemLog({
      level: 'INFO',
      message: description ? `${title} - ${description}` : title,
      category,
      flowUuid: options?.flowUuid,
    });
  } catch {
    // 忽略记录错误
  }
  createLogger(category).info(description ? `${title} - ${description}` : title);

  toast.info(title, {
    description,
    duration: options?.duration ?? 3000,
  });
}
