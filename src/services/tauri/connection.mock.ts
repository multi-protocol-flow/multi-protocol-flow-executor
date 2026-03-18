/**
 * Mock 数据库连接服务 - 用于非 Tauri 环境测试
 * 使用 localStorage 模拟持久化存储
 */

import type { DbConnection } from '@/types/database';
import type { TestConnectionResult } from './connection';

const STORAGE_KEY = 'app-db-connections';

/**
 * 从 localStorage 获取存储的连接
 */
function getStoredConnections(): DbConnection[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored ? JSON.parse(stored) : [];
  } catch {
    return [];
  }
}

/**
 * 保存连接到 localStorage
 */
function saveToStorage(connections: DbConnection[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(connections));
}

/**
 * 加载所有数据库连接
 */
export async function loadConnections(): Promise<DbConnection[]> {
  // 模拟网络延迟
  await new Promise((resolve) => setTimeout(resolve, 100));
  return getStoredConnections();
}

/**
 * 保存数据库连接（新增或更新）
 */
export async function saveConnection(connection: DbConnection): Promise<void> {
  // 模拟网络延迟
  await new Promise((resolve) => setTimeout(resolve, 100));

  const connections = getStoredConnections();
  const existingIndex = connections.findIndex((c) => c.id === connection.id);

  if (existingIndex >= 0) {
    connections[existingIndex] = connection;
  } else {
    connections.push(connection);
  }

  saveToStorage(connections);
}

/**
 * 删除数据库连接
 */
export async function deleteConnection(id: string): Promise<void> {
  // 模拟网络延迟
  await new Promise((resolve) => setTimeout(resolve, 100));

  const connections = getStoredConnections();
  const filtered = connections.filter((c) => c.id !== id);
  saveToStorage(filtered);
}

/**
 * 测试数据库连接（Mock: 模拟成功/失败）
 * 根据连接配置模拟测试结果
 */
export async function testConnection(id: string): Promise<TestConnectionResult> {
  // 模拟网络延迟
  await new Promise((resolve) => setTimeout(resolve, 500));

  const connections = getStoredConnections();
  const connection = connections.find((c) => c.id === id);

  if (!connection) {
    return { success: false, error: `连接 ${id} 不存在` };
  }

  // Mock: 如果主机名包含 "fail" 或 "error" 则返回失败
  if (
    connection.host?.toLowerCase().includes('fail') ||
    connection.host?.toLowerCase().includes('error')
  ) {
    return { success: false, error: '模拟连接失败：主机名包含 fail 或 error' };
  }

  // Mock: 默认返回成功
  return { success: true };
}

/**
 * 获取连接存储路径（Mock）
 */
export async function getConnectionsPath(): Promise<string> {
  return '浏览器本地存储 (localStorage)';
}

/**
 * 获取应用数据目录（Mock）
 */
export async function getAppDataDir(): Promise<string> {
  return '浏览器环境';
}
