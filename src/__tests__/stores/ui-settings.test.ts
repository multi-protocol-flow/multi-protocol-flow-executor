/**
 * UI Store - Settings Dialog 测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { useUIStore, selectShowSettingsDialog } from '@/stores/ui';

describe('UIStore - Settings Dialog', () => {
  beforeEach(() => {
    // 重置 store 到初始状态
    useUIStore.setState({
      showSettingsDialog: false,
    });
  });

  describe('初始状态', () => {
    it('showSettingsDialog 默认为 false', () => {
      expect(useUIStore.getState().showSettingsDialog).toBe(false);
    });
  });

  describe('openSettingsDialog', () => {
    it('将 showSettingsDialog 设置为 true', () => {
      useUIStore.getState().openSettingsDialog();
      expect(useUIStore.getState().showSettingsDialog).toBe(true);
    });

    it('重复调用保持 true', () => {
      useUIStore.getState().openSettingsDialog();
      useUIStore.getState().openSettingsDialog();
      expect(useUIStore.getState().showSettingsDialog).toBe(true);
    });
  });

  describe('closeSettingsDialog', () => {
    it('将 showSettingsDialog 设置为 false', () => {
      useUIStore.getState().openSettingsDialog();
      expect(useUIStore.getState().showSettingsDialog).toBe(true);

      useUIStore.getState().closeSettingsDialog();
      expect(useUIStore.getState().showSettingsDialog).toBe(false);
    });

    it('重复调用保持 false', () => {
      useUIStore.getState().closeSettingsDialog();
      useUIStore.getState().closeSettingsDialog();
      expect(useUIStore.getState().showSettingsDialog).toBe(false);
    });
  });

  describe('openSettingsDialog 和 closeSettingsDialog 交替使用', () => {
    it('可以多次打开和关闭', () => {
      // 第一次打开
      useUIStore.getState().openSettingsDialog();
      expect(useUIStore.getState().showSettingsDialog).toBe(true);

      // 第一次关闭
      useUIStore.getState().closeSettingsDialog();
      expect(useUIStore.getState().showSettingsDialog).toBe(false);

      // 第二次打开
      useUIStore.getState().openSettingsDialog();
      expect(useUIStore.getState().showSettingsDialog).toBe(true);

      // 第二次关闭
      useUIStore.getState().closeSettingsDialog();
      expect(useUIStore.getState().showSettingsDialog).toBe(false);
    });
  });

  describe('Selector', () => {
    it('selectShowSettingsDialog 返回正确值', () => {
      const state = useUIStore.getState();
      expect(selectShowSettingsDialog(state)).toBe(false);

      useUIStore.getState().openSettingsDialog();
      const updatedState = useUIStore.getState();
      expect(selectShowSettingsDialog(updatedState)).toBe(true);
    });
  });

  describe('不影响其他模态框状态', () => {
    it('操作设置对话框不影响 showNewFlowDialog', () => {
      useUIStore.setState({ showNewFlowDialog: false });
      useUIStore.getState().openSettingsDialog();

      expect(useUIStore.getState().showSettingsDialog).toBe(true);
      expect(useUIStore.getState().showNewFlowDialog).toBe(false);
    });

    it('操作设置对话框不影响 showDeleteConfirmDialog', () => {
      useUIStore.setState({
        showDeleteConfirmDialog: false,
        pendingDeleteFlowUuid: null,
      });
      useUIStore.getState().openSettingsDialog();

      expect(useUIStore.getState().showSettingsDialog).toBe(true);
      expect(useUIStore.getState().showDeleteConfirmDialog).toBe(false);
      expect(useUIStore.getState().pendingDeleteFlowUuid).toBe(null);
    });
  });
});
