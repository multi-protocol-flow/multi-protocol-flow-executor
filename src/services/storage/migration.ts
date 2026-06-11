/**
 * 数据迁移服务
 * 将 localStorage 中的流程数据一次性迁移到 .mpf 文件存储
 */

import { flowStorage } from './flowStorage';
import type { Flow } from '@/types/flow';

/** 迁移完成标记 key */
const MIGRATION_FLAG_KEY = 'mpf_migrated_v1';

/** localStorage 中旧数据的 key */
const LEGACY_KEYS = {
  FLOWS: 'mpe_flows',
  FLOW_LIST: 'mpe_flow_list',
} as const;

/** 旧版 localStorage 存储结构 */
interface LegacyFlowStorage {
  version: number;
  flows: Record<string, Flow>;
}

/**
 * 将 localStorage 中的流程数据迁移到 .mpf 文件存储
 *
 * 幂等操作：已迁移时直接返回，可安全多次调用。
 * 部分失败时保留 localStorage 数据并记录错误，下次启动可重试。
 */
export async function migrateFlowsToFiles(): Promise<void> {
  // 1. 检查迁移标记
  const migrated = localStorage.getItem(MIGRATION_FLAG_KEY);
  if (migrated === 'true') {
    return; // 已迁移，直接返回
  }

  // 2. 读取旧数据
  const stored = localStorage.getItem(LEGACY_KEYS.FLOWS);
  if (!stored) {
    // 无数据需要迁移，直接设置标记
    localStorage.setItem(MIGRATION_FLAG_KEY, 'true');
    return;
  }

  // 3. 解析旧数据
  let storage: LegacyFlowStorage;
  try {
    storage = JSON.parse(stored) as LegacyFlowStorage;
  } catch {
    console.error('Migration failed: unable to parse flow data in localStorage');
    return;
  }

  // 4. 逐个迁移流程
  const flows = Object.values(storage.flows);
  const errors: string[] = [];

  for (const flow of flows) {
    try {
      await flowStorage.saveFlow(flow);
      // 验证：读回并校验 UUID
      const loaded = await flowStorage.loadFlow(flow.uuid);
      if (!loaded || loaded.uuid !== flow.uuid) {
        throw new Error(`Validation failed: ${flow.name}`);
      }
    } catch (e) {
      const msg = `Migration of flow "${flow.name}" failed: ${e}`;
      errors.push(msg);
      console.error(msg, e);
    }
  }

  // 5. 处理迁移结果
  if (errors.length === 0) {
    // 全部成功 — 设置标记并清理旧数据
    localStorage.setItem(MIGRATION_FLAG_KEY, 'true');
    localStorage.removeItem(LEGACY_KEYS.FLOWS);
    localStorage.removeItem(LEGACY_KEYS.FLOW_LIST);
    console.log(`Successfully migrated ${flows.length} flows to file storage`);
  } else {
    // 部分失败 — 保留 localStorage 数据，不设置标记，下次可重试
    console.error(
      `Migration completed with ${errors.length} failure(s):`,
      errors,
    );
  }
}
