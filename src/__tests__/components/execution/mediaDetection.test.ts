/**
 * mediaDetection 纯函数测试
 * - 后端媒体分流（V1）
 * - JSON 媒体 URL（V2）
 * - JSON base64 载荷（V3）
 * - 文件名推断与清理
 */
import { describe, it, expect } from 'vitest';
import {
  buildDefaultFilename,
  detectMedia,
  formatBytes,
  headerValue,
  sanitizeFilename,
} from '@/components/execution/viewers/mediaDetection';

/** 1x1 PNG */
const PNG_BASE64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';

describe('detectMedia', () => {
  describe('后端媒体分流（V1）', () => {
    it('内联 data_url 生成可预览候选', () => {
      const candidates = detectMedia({
        body: '[image/png 8 bytes]',
        media: {
          kind: 'image',
          mime: 'image/png',
          bytes: 8,
          data_url: 'data:image/png;base64,iVBORw0KGgo=',
        },
      });

      expect(candidates).toHaveLength(1);
      expect(candidates[0].kind).toBe('image');
      expect(candidates[0].mime).toBe('image/png');
      expect(candidates[0].source).toEqual({
        type: 'data_url',
        value: 'data:image/png;base64,iVBORw0KGgo=',
      });
    });

    it('超限时生成 omitted 候选且无 source', () => {
      const candidates = detectMedia({
        media: { kind: 'video', mime: 'video/mp4', bytes: 35_300_000, omitted: true },
      });

      expect(candidates).toHaveLength(1);
      expect(candidates[0].omitted).toBe(true);
      expect(candidates[0].source).toBeUndefined();
      expect(candidates[0].bytes).toBe(35_300_000);
    });
  });

  describe('JSON 媒体 URL（V2）', () => {
    it('识别 result_url 视频地址', () => {
      const candidates = detectMedia({
        body: { data: { result_url: 'https://cdn.example.com/a.mp4' } },
      });

      expect(candidates).toHaveLength(1);
      expect(candidates[0].kind).toBe('video');
      expect(candidates[0].source).toEqual({
        type: 'url',
        value: 'https://cdn.example.com/a.mp4',
      });
    });

    it('非白名单键与非 http 协议不产生候选', () => {
      const candidates = detectMedia({
        body: { message: 'https://example.com/logo.png is not media', url: 'ftp://x' },
      });

      expect(candidates).toEqual([]);
    });
  });

  describe('JSON base64 载荷（V3）', () => {
    it('识别 Gemini inlineData 并嗅探 MIME', () => {
      const candidates = detectMedia({
        body: {
          candidates: [
            {
              content: {
                parts: [{ inlineData: { mimeType: 'image/png', data: PNG_BASE64 } }],
              },
            },
          ],
        },
      });

      expect(candidates).toHaveLength(1);
      expect(candidates[0].mime).toBe('image/png');
      expect(candidates[0].kind).toBe('image');
      expect(candidates[0].source?.type).toBe('data_url');
      expect(candidates[0].source?.value).toBe(`data:image/png;base64,${PNG_BASE64}`);
    });

    it('非媒体 base64 载荷被丢弃', () => {
      const candidates = detectMedia({
        body: { inlineData: { mimeType: 'text/plain', data: 'aGVsbG8gd29ybGQh' } },
      });

      expect(candidates).toEqual([]);
    });

    it('URL-safe base64 归一化后可识别', () => {
      // 标准载荷内含 `+` 与 `/`，URL-safe 变体换成 `-` 与 `_`，仍应嗅探出 PNG
      const standard =
        'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggvu/+w==';
      const urlSafe = standard.replace(/\+/g, '-').replace(/\//g, '_');
      expect(urlSafe).not.toBe(standard);

      const candidates = detectMedia({ body: { image: urlSafe } });

      expect(candidates).toHaveLength(1);
      expect(candidates[0].mime).toBe('image/png');
      expect(candidates[0].source?.value).toBe(`data:image/png;base64,${standard}`);
    });

    it('长度非 4 的倍数（截断载荷）被丢弃', () => {
      const candidates = detectMedia({ body: { image: `${PNG_BASE64}A` } });

      expect(candidates).toEqual([]);
    });
  });

  describe('边界', () => {
    it('总数上限为 8', () => {
      const candidates = detectMedia({
        body: {
          urls: Array.from({ length: 20 }, (_, i) => `https://cdn.example.com/${i}.png`),
        },
      });
      expect(candidates.length).toBeLessThanOrEqual(8);
    });

    it('非对象输入返回空列表', () => {
      expect(detectMedia('plain text')).toEqual([]);
      expect(detectMedia(undefined)).toEqual([]);
    });
  });
});

describe('文件名工具', () => {
  it('sanitizeFilename 只保留 basename', () => {
    expect(sanitizeFilename('../../x.png', 'f.bin')).toBe('x.png');
  });

  it('sanitizeFilename 去掉非法字符，空则回退', () => {
    expect(sanitizeFilename('a<b>c:d.png', 'f.bin')).toBe('abcd.png');
    expect(sanitizeFilename('   ', 'f.bin')).toBe('f.bin');
  });

  it('buildDefaultFilename 优先 Content-Disposition', () => {
    const candidates = detectMedia({
      media: { kind: 'image', mime: 'image/png', bytes: 8, data_url: 'data:image/png;base64,x' },
    });
    const name = buildDefaultFilename(
      candidates[0],
      { 'Content-Disposition': 'attachment; filename="../../x.png"' },
      'node'
    );
    expect(name).toBe('x.png');
  });

  it('buildDefaultFilename 对 URL 候选使用 URL basename', () => {
    const candidates = detectMedia({ body: { image_url: 'https://cdn.example.com/pic.webp' } });
    const name = buildDefaultFilename(candidates[0], undefined, 'node');
    expect(name).toBe('pic.webp');
  });

  it('buildDefaultFilename 无信息时回退节点名', () => {
    const candidates = detectMedia({
      media: { kind: 'video', mime: 'video/mp4', bytes: 8, data_url: 'data:video/mp4;base64,x' },
    });
    const name = buildDefaultFilename(candidates[0], undefined, 'my-node');
    expect(name.startsWith('my-node-')).toBe(true);
    expect(name.endsWith('.mp4')).toBe(true);
  });

  it('headerValue 大小写不敏感并支持数组值', () => {
    expect(headerValue({ 'Content-Type': ['image/png', 'x'] }, 'content-type')).toBe('image/png');
    expect(headerValue(undefined, 'content-type')).toBeUndefined();
  });

  it('formatBytes 与后端文案一致', () => {
    expect(formatBytes(900)).toBe('900 bytes');
    expect(formatBytes(2048)).toBe('2.0 KB');
    expect(formatBytes(5 * 1024 * 1024)).toBe('5.0 MB');
  });
});
