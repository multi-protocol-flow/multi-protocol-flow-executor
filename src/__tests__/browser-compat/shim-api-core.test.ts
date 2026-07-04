/**
 * @tauri-apps/api/core 浏览器垫片测试
 * 测试 invoke() 和 Channel<T> 在浏览器模式下的行为
 */

import { describe, it, expect } from 'vitest';
import { invoke, Channel } from '@/browser-shims/api-core';

describe('invoke (browser shim)', () => {
  it('调用 invoke 时抛出 "not available in browser mode" 错误', async () => {
    await expect(invoke('test_command')).rejects.toThrow(
      'invoke() not available in browser mode: test_command',
    );
  });

  it('错误消息包含命令名称', async () => {
    await expect(invoke('create_flow')).rejects.toThrow('create_flow');
  });

  it('传入参数时仍然抛出错误', async () => {
    await expect(invoke('create_flow', { name: 'test' })).rejects.toThrow(
      'create_flow',
    );
  });

  it('不同命令返回不同的错误消息', async () => {
    await expect(invoke('save_flow')).rejects.toThrow('save_flow');
    await expect(invoke('load_flow')).rejects.toThrow('load_flow');
    await expect(invoke('delete_flow')).rejects.toThrow('delete_flow');
  });
});

describe('Channel (browser shim)', () => {
  it('可以创建 Channel 实例', () => {
    const channel = new Channel<string>();
    expect(channel).toBeInstanceOf(Channel);
  });

  it('Channel id 默认值为 0', () => {
    const channel = new Channel<string>();
    expect(channel.id).toBe(0);
  });

  it('Channel onmessage 默认是 no-op 函数', () => {
    const channel = new Channel<string>();
    expect(typeof channel.onmessage).toBe('function');
    // 调用不应抛出
    expect(() => channel.onmessage('test')).not.toThrow();
  });

  it('Channel onmessage 可以被覆盖', () => {
    const channel = new Channel<string>();
    const received: string[] = [];
    channel.onmessage = (msg) => {
      received.push(msg);
    };

    channel.onmessage('hello');
    channel.onmessage('world');

    expect(received).toEqual(['hello', 'world']);
  });

  it('toJSON 返回包含 id 的对象', () => {
    const channel = new Channel<number>();
    const json = channel.toJSON();
    expect(json).toHaveProperty('id', 0);
  });

  it('支持泛型类型参数', () => {
    const numberChannel = new Channel<number>();
    const objectChannel = new Channel<{ key: string }>();
    const arrayChannel = new Channel<number[]>();

    expect(numberChannel).toBeInstanceOf(Channel);
    expect(objectChannel).toBeInstanceOf(Channel);
    expect(arrayChannel).toBeInstanceOf(Channel);
  });

  it('id 可以被赋值', () => {
    const channel = new Channel<string>();
    channel.id = 42;
    expect(channel.id).toBe(42);
  });
});
