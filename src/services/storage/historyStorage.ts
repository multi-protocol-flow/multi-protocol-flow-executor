/**
 * 执行历史存储服务
 * 使用 localStorage 存储执行历史
 */

/** 执行历史项 */
export interface ExecutionHistoryItem {
  uuid: string;
  flow_uuid: string;
  flow_name: string;
  status: 'success' | 'failed';
  duration_ms: number;
  timestamp: number;
}

const STORAGE_KEY = 'mpe_history';

/** 最大历史记录数量 */
const MAX_HISTORY_ITEMS = 100;

/**
 * 保存执行历史
 * @param item 执行历史项
 */
export function saveExecutionHistory(item: ExecutionHistoryItem): void {
  const history = listExecutionHistory();
  history.unshift(item);
  // 限制历史记录数量
  if (history.length > MAX_HISTORY_ITEMS) {
    history.length = MAX_HISTORY_ITEMS;
  }
  localStorage.setItem(STORAGE_KEY, JSON.stringify(history));
}

/**
 * 列出执行历史
 * @returns 执行历史列表（按时间倒序）
 */
export function listExecutionHistory(): ExecutionHistoryItem[] {
  const json = localStorage.getItem(STORAGE_KEY);
  if (json) {
    try {
      return JSON.parse(json) as ExecutionHistoryItem[];
    } catch {
      return [];
    }
  }
  return [];
}

/**
 * 清空执行历史
 */
export function clearExecutionHistory(): void {
  localStorage.removeItem(STORAGE_KEY);
}

/**
 * 删除指定流程的执行历史
 * @param flowUuid 流程 UUID
 */
export function deleteExecutionHistoryByFlow(flowUuid: string): void {
  const history = listExecutionHistory().filter((h) => h.flow_uuid !== flowUuid);
  localStorage.setItem(STORAGE_KEY, JSON.stringify(history));
}
