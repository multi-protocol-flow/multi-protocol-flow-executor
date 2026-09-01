import { describe, it, expect, beforeEach } from 'vitest';
import { aiSessionStorage } from '@/services/storage/aiSessionStorage';
import type { AISession } from '@/types/ai';

describe('aiSessionStorage', () => {
  beforeEach(async () => {
    await aiSessionStorage.clearAllSessions();
  });

  it('保存并加载完整会话数据', async () => {
    const session: AISession = {
      id: 'session-1',
      title: '测试对话',
      createdAt: 1000,
      updatedAt: 2000,
      messages: [
        {
          id: 'm1',
          role: 'user',
          content: 'Hello with image',
          images: ['data:image/png;base64,iVBORw0KGgoAAAANSUhEUg=='],
          timestamp: 1000,
        },
      ],
    };

    await aiSessionStorage.saveSession(session);

    const loaded = await aiSessionStorage.loadSession('session-1');
    expect(loaded).toBeDefined();
    expect(loaded?.id).toBe('session-1');
    expect(loaded?.title).toBe('测试对话');
    expect(loaded?.messages).toHaveLength(1);
    expect(loaded?.messages[0].images).toEqual([
      'data:image/png;base64,iVBORw0KGgoAAAANSUhEUg==',
    ]);
  });

  it('获取轻量元数据列表并按更新时间排序', async () => {
    const s1: AISession = {
      id: 's1',
      title: '较早会话',
      createdAt: 1000,
      updatedAt: 1000,
      messages: [],
    };
    const s2: AISession = {
      id: 's2',
      title: '最新会话',
      createdAt: 2000,
      updatedAt: 3000,
      messages: [{ id: 'm1', role: 'user', content: 'hi', timestamp: 2000 }],
    };

    await aiSessionStorage.saveSession(s1);
    await aiSessionStorage.saveSession(s2);

    const list = await aiSessionStorage.listSessions();
    expect(list).toHaveLength(2);
    expect(list[0].id).toBe('s2');
    expect(list[0].messageCount).toBe(1);
    expect(list[1].id).toBe('s1');
  });

  it('删除指定会话', async () => {
    const s1: AISession = {
      id: 's1',
      title: '待删除',
      createdAt: 1000,
      updatedAt: 1000,
      messages: [],
    };
    await aiSessionStorage.saveSession(s1);

    await aiSessionStorage.deleteSession('s1');
    const loaded = await aiSessionStorage.loadSession('s1');
    expect(loaded).toBeNull();

    const list = await aiSessionStorage.listSessions();
    expect(list).toHaveLength(0);
  });
});
