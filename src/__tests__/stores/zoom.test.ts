/**
 * Zoom Store 单元测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { useZoomStore, selectZoom, ZOOM_MIN, ZOOM_MAX, ZOOM_STEP, ZOOM_DEFAULT } from '@/stores/zoom';

describe('ZoomStore', () => {
  beforeEach(() => {
    localStorage.clear();
    useZoomStore.setState({ zoom: ZOOM_DEFAULT });
  });

  describe('初始状态', () => {
    it('默认缩放为 1.0', () => {
      const state = useZoomStore.getState();
      expect(state.zoom).toBe(ZOOM_DEFAULT);
      expect(selectZoom(state)).toBe(1);
    });
  });

  describe('setZoom', () => {
    it('更新缩放值并持久化到 localStorage', () => {
      useZoomStore.getState().setZoom(1.5);
      expect(useZoomStore.getState().zoom).toBe(1.5);
      expect(localStorage.getItem('app-zoom-level')).toBe('1.5');
    });

    it('超出上限 2.0 时钳位到 2.0', () => {
      useZoomStore.getState().setZoom(5);
      expect(useZoomStore.getState().zoom).toBe(ZOOM_MAX);
      expect(localStorage.getItem('app-zoom-level')).toBe('2');
    });

    it('低于下限 0.5 时钳位到 0.5', () => {
      useZoomStore.getState().setZoom(0.1);
      expect(useZoomStore.getState().zoom).toBe(ZOOM_MIN);
      expect(localStorage.getItem('app-zoom-level')).toBe('0.5');
    });
  });

  describe('zoomIn / zoomOut', () => {
    it('zoomIn 每次增加一个步长', () => {
      useZoomStore.getState().setZoom(1);
      useZoomStore.getState().zoomIn();
      expect(useZoomStore.getState().zoom).toBe(1 + ZOOM_STEP);
    });

    it('zoomOut 每次减少一个步长', () => {
      useZoomStore.getState().setZoom(1);
      useZoomStore.getState().zoomOut();
      expect(useZoomStore.getState().zoom).toBe(1 - ZOOM_STEP);
    });
  });

  describe('resetZoom / applyZoom', () => {
    it('resetZoom 还原为默认缩放', () => {
      useZoomStore.getState().setZoom(1.7);
      useZoomStore.getState().resetZoom();
      expect(useZoomStore.getState().zoom).toBe(ZOOM_DEFAULT);
    });

    it('applyZoom 不改变缩放状态', () => {
      useZoomStore.getState().setZoom(1.2);
      useZoomStore.getState().applyZoom();
      expect(useZoomStore.getState().zoom).toBe(1.2);
    });
  });
});
