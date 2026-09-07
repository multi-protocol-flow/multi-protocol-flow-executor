/**
 * AI 会话持久化存储服务
 * 基于 IndexedDB 异步存储与内存缓存，支持本地 Session 与大尺寸图片附件，零主线程阻塞
 */

import type { AISession, AISessionMetadata } from '@/types/ai';

const DB_NAME = 'mpe_ai_db';
const DB_VERSION = 1;
const STORE_INDEX = 'sessions_index';
const STORE_DATA = 'sessions_data';

// 内存镜像缓存（加速读取并在 JSDOM / Node 环境下平滑回退）
const memoryIndex = new Map<string, AISessionMetadata>();
const memoryData = new Map<string, AISession>();

let dbPromise: Promise<IDBDatabase | null> | null = null;

function getDB(): Promise<IDBDatabase | null> {
  if (typeof window === 'undefined' || !window.indexedDB) {
    return Promise.resolve(null);
  }

  if (!dbPromise) {
    dbPromise = new Promise<IDBDatabase | null>((resolve) => {
      try {
        const request = indexedDB.open(DB_NAME, DB_VERSION);

        request.onupgradeneeded = (event) => {
          const db = (event.target as IDBOpenDBRequest).result;
          if (!db.objectStoreNames.contains(STORE_INDEX)) {
            const indexStore = db.createObjectStore(STORE_INDEX, { keyPath: 'id' });
            indexStore.createIndex('updatedAt', 'updatedAt', { unique: false });
          }
          if (!db.objectStoreNames.contains(STORE_DATA)) {
            db.createObjectStore(STORE_DATA, { keyPath: 'id' });
          }
        };

        request.onsuccess = () => {
          resolve(request.result);
        };

        request.onerror = () => {
          resolve(null);
        };
      } catch {
        resolve(null);
      }
    });
  }

  return dbPromise;
}

export const aiSessionStorage = {
  /**
   * 获取所有会话轻量元数据列表（按最近更新时间倒序排列）
   */
  async listSessions(): Promise<AISessionMetadata[]> {
    const db = await getDB();
    if (db) {
      try {
        const fromDb = await new Promise<AISessionMetadata[]>((resolve, reject) => {
          const tx = db.transaction(STORE_INDEX, 'readonly');
          const store = tx.objectStore(STORE_INDEX);
          const index = store.index('updatedAt');
          const request = index.openCursor(null, 'prev');

          const results: AISessionMetadata[] = [];
          request.onsuccess = () => {
            const cursor = request.result;
            if (cursor) {
              results.push(cursor.value);
              cursor.continue();
            } else {
              resolve(results);
            }
          };

          request.onerror = () => reject(request.error);
        });

        if (fromDb.length > 0 || memoryIndex.size === 0) {
          for (const item of fromDb) {
            memoryIndex.set(item.id, item);
          }
          return fromDb;
        }
      } catch {
        // 回退内存
      }
    }

    return Array.from(memoryIndex.values()).sort(
      (a, b) => b.updatedAt - a.updatedAt,
    );
  },

  /**
   * 加载指定会话的完整消息数据
   */
  async loadSession(id: string): Promise<AISession | null> {
    if (memoryData.has(id)) {
      return memoryData.get(id) || null;
    }

    const db = await getDB();
    if (db) {
      try {
        const fromDb = await new Promise<AISession | null>((resolve, reject) => {
          const tx = db.transaction(STORE_DATA, 'readonly');
          const store = tx.objectStore(STORE_DATA);
          const request = store.get(id);

          request.onsuccess = () => {
            resolve(request.result || null);
          };

          request.onerror = () => reject(request.error);
        });

        if (fromDb) {
          memoryData.set(id, fromDb);
          return fromDb;
        }
      } catch {
        // 回退内存
      }
    }

    return memoryData.get(id) || null;
  },

  /**
   * 保存或更新会话
   */
  async saveSession(session: AISession): Promise<void> {
    const metadata: AISessionMetadata = {
      id: session.id,
      title: session.title,
      createdAt: session.createdAt,
      updatedAt: session.updatedAt,
      messageCount: session.messages.length,
      workspace: session.workspace,
    };

    memoryIndex.set(session.id, metadata);
    memoryData.set(session.id, { ...session });

    const db = await getDB();
    if (db) {
      try {
        await new Promise<void>((resolve, reject) => {
          const tx = db.transaction([STORE_INDEX, STORE_DATA], 'readwrite');
          const indexStore = tx.objectStore(STORE_INDEX);
          const dataStore = tx.objectStore(STORE_DATA);

          indexStore.put(metadata);
          dataStore.put(session);

          tx.oncomplete = () => resolve();
          tx.onerror = () => reject(tx.error);
        });
      } catch {
        // 忽略
      }
    }
  },

  /**
   * 删除指定会话
   */
  async deleteSession(id: string): Promise<void> {
    memoryIndex.delete(id);
    memoryData.delete(id);

    const db = await getDB();
    if (db) {
      try {
        await new Promise<void>((resolve, reject) => {
          const tx = db.transaction([STORE_INDEX, STORE_DATA], 'readwrite');
          const indexStore = tx.objectStore(STORE_INDEX);
          const dataStore = tx.objectStore(STORE_DATA);

          indexStore.delete(id);
          dataStore.delete(id);

          tx.oncomplete = () => resolve();
          tx.onerror = () => reject(tx.error);
        });
      } catch {
        // 忽略
      }
    }
  },

  /**
   * 清空所有历史会话
   */
  async clearAllSessions(): Promise<void> {
    memoryIndex.clear();
    memoryData.clear();

    const db = await getDB();
    if (db) {
      try {
        await new Promise<void>((resolve, reject) => {
          const tx = db.transaction([STORE_INDEX, STORE_DATA], 'readwrite');
          const indexStore = tx.objectStore(STORE_INDEX);
          const dataStore = tx.objectStore(STORE_DATA);

          indexStore.clear();
          dataStore.clear();

          tx.oncomplete = () => resolve();
          tx.onerror = () => reject(tx.error);
        });
      } catch {
        // 忽略
      }
    }
  },
};
