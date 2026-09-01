import { describe, it, expect } from 'vitest';
import {
  extractImagesFromClipboard,
  extractImagesFromDrop,
} from '@/lib/imageUtils';

describe('imageUtils', () => {
  it('处理空剪贴板数据', async () => {
    const res = await extractImagesFromClipboard(null);
    expect(res).toEqual([]);
  });

  it('处理空拖拽数据', async () => {
    const res = await extractImagesFromDrop(null);
    expect(res).toEqual([]);
  });
});
