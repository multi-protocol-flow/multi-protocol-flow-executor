import { describe, it, expect, vi } from 'vitest';
import {
  formatBytes,
  estimateDataSize,
  getAdaptiveExpandStrategy,
  downloadContentAsFile,
  JSON_VIEWER_THRESHOLDS,
} from '@/components/execution/viewers/jsonViewerUtils';

describe('jsonViewerUtils', () => {
  describe('formatBytes', () => {
    it('处理 0 或负数或无效值', () => {
      expect(formatBytes(0)).toBe('0 B');
      expect(formatBytes(-10)).toBe('0 B');
      expect(formatBytes(NaN)).toBe('0 B');
      expect(formatBytes(Infinity)).toBe('0 B');
    });

    it('格式化字节', () => {
      expect(formatBytes(100)).toBe('100 B');
      expect(formatBytes(1023)).toBe('1023 B');
    });

    it('格式化 KB', () => {
      expect(formatBytes(1024)).toBe('1.0 KB');
      expect(formatBytes(1536)).toBe('1.5 KB');
      expect(formatBytes(50 * 1024)).toBe('50.0 KB');
    });

    it('格式化 MB', () => {
      expect(formatBytes(1024 * 1024)).toBe('1.0 MB');
      expect(formatBytes(2.5 * 1024 * 1024)).toBe('2.5 MB');
    });

    it('格式化 GB', () => {
      expect(formatBytes(1024 * 1024 * 1024)).toBe('1.0 GB');
    });
  });

  describe('estimateDataSize', () => {
    it('处理 rawString', () => {
      expect(estimateDataSize(null, 'hello')).toBe(5);
    });

    it('处理 string 类型', () => {
      expect(estimateDataSize('123456')).toBe(6);
    });

    it('处理 null/undefined', () => {
      expect(estimateDataSize(null)).toBe(0);
      expect(estimateDataSize(undefined)).toBe(0);
    });

    it('处理原始值', () => {
      expect(estimateDataSize(12345)).toBe(5);
      expect(estimateDataSize(true)).toBe(4);
    });

    it('处理对象', () => {
      const obj = { key: 'value' };
      expect(estimateDataSize(obj)).toBe(JSON.stringify(obj).length);
    });
  });

  describe('getAdaptiveExpandStrategy', () => {
    it('用户强制全部展开', () => {
      const strategy = getAdaptiveExpandStrategy(10 * 1024 * 1024, 'all');
      expect(strategy(0)).toBe(true);
      expect(strategy(5)).toBe(true);
      expect(strategy(10)).toBe(true);
    });

    it('用户强制全部折叠', () => {
      const strategy = getAdaptiveExpandStrategy(10, 'collapse');
      expect(strategy(0)).toBe(false);
      expect(strategy(1)).toBe(false);
    });

    it('小体积默认全部展开', () => {
      const strategy = getAdaptiveExpandStrategy(JSON_VIEWER_THRESHOLDS.SMALL - 1, null);
      expect(strategy(0)).toBe(true);
      expect(strategy(5)).toBe(true);
    });

    it('中体积默认展开前 2 层', () => {
      const strategy = getAdaptiveExpandStrategy(JSON_VIEWER_THRESHOLDS.SMALL + 1, null);
      expect(strategy(0)).toBe(true);
      expect(strategy(1)).toBe(true);
      expect(strategy(2)).toBe(false);
      expect(strategy(3)).toBe(false);
    });

    it('大体积默认只展开第 1 层', () => {
      const strategy = getAdaptiveExpandStrategy(JSON_VIEWER_THRESHOLDS.MEDIUM + 1, null);
      expect(strategy(0)).toBe(true);
      expect(strategy(1)).toBe(false);
      expect(strategy(2)).toBe(false);
    });
  });

  describe('downloadContentAsFile', () => {
    it('正确创建并释放对象 URL 进行模拟下载', () => {
      const createObjectURLMock = vi.fn(() => 'blob:test-url');
      const revokeObjectURLMock = vi.fn();
      window.URL.createObjectURL = createObjectURLMock;
      window.URL.revokeObjectURL = revokeObjectURLMock;

      const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});

      downloadContentAsFile('{"test":123}', 'test.json', 'application/json');

      expect(createObjectURLMock).toHaveBeenCalled();
      expect(clickSpy).toHaveBeenCalled();
      expect(revokeObjectURLMock).toHaveBeenCalledWith('blob:test-url');

      clickSpy.mockRestore();
    });
  });
});
