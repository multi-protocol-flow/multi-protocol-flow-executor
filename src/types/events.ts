/**
 * 事件类型定义
 * 与后端 src-tauri/src/log_event.rs 中的 LogEvent 结构体对齐
 */

/**
 * 日志级别类型
 */
export type LogLevel = 'INFO' | 'DEBUG' | 'WARN' | 'ERROR';

/**
 * 日志事件结构
 */
export interface LogEvent {
  /** 日志级别 */
  level: LogLevel;
  /** 日志消息内容 */
  message: string;
  /** 时间戳（毫秒） */
  timestamp: number;
  /** 关联节点 UUID */
  nodeUuid?: string;
  /** 关联节点名称 */
  nodeName?: string;
  /** 关联流程 UUID */
  flowUuid: string;
}
