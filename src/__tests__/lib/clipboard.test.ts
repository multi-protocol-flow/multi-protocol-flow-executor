/**
 * clipboard & safeStringify 测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { safeStringify, copyToClipboard } from '@/lib/clipboard';

describe('safeStringify', () => {
  it('序列化普通对象与基本类型', () => {
    expect(safeStringify(null)).toBe('');
    expect(safeStringify(undefined)).toBe('');
    expect(safeStringify('hello')).toBe('hello');
    expect(safeStringify(123)).toBe('123');
    expect(safeStringify({ a: 1 })).toBe('{"a":1}');
  });

  it('安全处理循环引用对象，不崩溃', () => {
    const obj: Record<string, unknown> = { name: 'circular' };
    obj.self = obj;

    const result = safeStringify(obj);
    expect(result).toContain('"self":"[Circular]"');
    expect(result).toContain('"name":"circular"');
  });

  it('正确序列化 Error 对象（包含 message 和 stack）', () => {
    const err = new Error('test error');
    const result = safeStringify(err);
    expect(result).toContain('"message":"test error"');
    expect(result).toContain('"name":"Error"');
  });
});

describe('copyToClipboard', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('优先调用 navigator.clipboard.writeText', async () => {
    const writeTextMock = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: {
        writeText: writeTextMock,
      },
    });

    const success = await copyToClipboard('test copy');
    expect(success).toBe(true);
    expect(writeTextMock).toHaveBeenCalledWith('test copy');
  });

  it('当 writeText 超时或失败时降级到 document.execCommand', async () => {
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockRejectedValue(new Error('Permission denied')),
      },
    });

    const execCommandMock = vi.fn().mockReturnValue(true);
    document.execCommand = execCommandMock;

    const success = await copyToClipboard('fallback copy');
    expect(success).toBe(true);
    expect(execCommandMock).toHaveBeenCalledWith('copy');
  });
});
