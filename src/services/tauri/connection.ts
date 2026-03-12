/**
 * Tauri 数据库连接持久化服务
 * 使用 tauri-plugin-store 进行持久化
 */

import { LazyStore } from '@tauri-apps/plugin-store';
import { invoke } from '@tauri-apps/api/core';
import type { DbConnection } from '@/types/database';

const CONNECTIONS_FILE = 'connections.bin';
const CONNECTIONS_KEY = 'connections';

let store: LazyStore | null = null;

/**
 * 获取 Store 实例（延迟加载）
 */
async function getStore(): Promise<LazyStore> {
  if (!store) {
    const appDataDir = await invoke<string>('get_app_data_dir');
    store = new LazyStore(`${appDataDir}/store/${CONNECTIONS_FILE}`);
  }
  return store;
}

/**
 * 加载所有数据库连接
 */
export async function loadConnections(): Promise<DbConnection[]> {
  const s = await getStore();
  const connections = await s.get<DbConnection[]>(CONNECTIONS_KEY);
  return connections ?? [];
}

/**
 * 保存数据库连接（新增或更新）
 */
export async function saveConnection(connection: DbConnection): Promise<void> {
  const s = await getStore();
  const connections = (await s.get<DbConnection[]>(CONNECTIONS_KEY)) ?? [];

  // 查找是否存在
  const existingIndex = connections.findIndex((c) => c.id === connection.id);
  if (existingIndex >= 0) {
    // 更新现有连接
    connections[existingIndex] = connection;
  } else {
    // 新增连接
    connections.push(connection);
  }

  await s.set(CONNECTIONS_KEY, connections);
  await s.save();
}

/**
 * 删除数据库连接
 */
export async function deleteConnection(id: string): Promise<void> {
  const s = await getStore();
  const connections = (await s.get<DbConnection[]>(CONNECTIONS_KEY)) ?? [];
  const filtered = connections.filter((c) => c.id !== id);

  await s.set(CONNECTIONS_KEY, filtered);
  await s.save();
}

/**
 * 测试数据库连接
 * 调用后端 Tauri 命令测试连接
 */
export async function testConnection(id: string): Promise<boolean> {
  const s = await getStore();
  const connections = (await s.get<DbConnection[]>(CONNECTIONS_KEY)) ?? [];
  const connection = connections.find((c) => c.id === id);

  if (!connection) {
    throw new Error(`连接 ${id} 不存在`);
  }

  try {
    const result = await invoke<boolean>('test_db_connection', { connection });
    return result;
  } catch {
    return false;
  }
}

/**
 * 获取连接存储文件路径
 */
export async function getConnectionsPath(): Promise<string> {
  const appDataDir = await invoke<string>('get_app_data_dir');
  return `${appDataDir}/store/${CONNECTIONS_FILE}`;
}

/**
 * 获取应用数据目录
 */
export async function getAppDataDir(): Promise<string> {
  return invoke<string>('get_app_data_dir');
}
