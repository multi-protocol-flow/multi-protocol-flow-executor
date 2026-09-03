/**
 * 前端统一 Logger 模块
 * 1. 在 Tauri 环境下调用 @tauri-apps/plugin-log，将日志持久化到本地日志文件 (logs/app.log)
 * 2. 在浏览器开发环境输出带命名空间与样式的 DevTools console
 * 3. 支持 createLogger(tag) 创建专属标签的子 Logger
 */
import { isTauriEnv } from '@/lib/platform';
import {
  error as tauriError,
  warn as tauriWarn,
  info as tauriInfo,
  debug as tauriDebug,
} from '@tauri-apps/plugin-log';

export interface Logger {
  debug: (message: string, ...args: unknown[]) => void;
  info: (message: string, ...args: unknown[]) => void;
  warn: (message: string, ...args: unknown[]) => void;
  error: (message: string, ...args: unknown[]) => void;
}

function formatArgs(args: unknown[]): string {
  if (args.length === 0) return '';
  return (
    ' ' +
    args
      .map((a) => {
        if (a instanceof Error) {
          return a.stack || a.message;
        }
        if (typeof a === 'object' && a !== null) {
          try {
            return JSON.stringify(a);
          } catch {
            return String(a);
          }
        }
        return String(a);
      })
      .join(' ')
  );
}

export function createLogger(tag: string): Logger {
  const prefix = `[${tag}]`;

  return {
    debug: (message: string, ...args: unknown[]) => {
      console.debug(`${prefix} ${message}`, ...args);
      if (isTauriEnv()) {
        tauriDebug(`${prefix} ${message}${formatArgs(args)}`).catch(() => {});
      }
    },
    info: (message: string, ...args: unknown[]) => {
      console.info(`${prefix} ${message}`, ...args);
      if (isTauriEnv()) {
        tauriInfo(`${prefix} ${message}${formatArgs(args)}`).catch(() => {});
      }
    },
    warn: (message: string, ...args: unknown[]) => {
      console.warn(`${prefix} ${message}`, ...args);
      if (isTauriEnv()) {
        tauriWarn(`${prefix} ${message}${formatArgs(args)}`).catch(() => {});
      }
    },
    error: (message: string, ...args: unknown[]) => {
      console.error(`${prefix} ${message}`, ...args);
      if (isTauriEnv()) {
        tauriError(`${prefix} ${message}${formatArgs(args)}`).catch(() => {});
      }
    },
  };
}

export const logger = createLogger('App');
