/**
 * 通知与系统日志分发测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { notifyError, notifySuccess, notifyWarning, notifyInfo, formatErrorDetail } from '@/lib/notify';
import { useConsoleStore } from '@/stores/console';
import { toast } from 'sonner';

vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
    warning: vi.fn(),
    info: vi.fn(),
  },
}));

describe('notify', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useConsoleStore.getState().reset();
  });

  describe('formatErrorDetail', () => {
    it('处理 string 错误', () => {
      const res = formatErrorDetail('网络超时');
      expect(res.message).toBe('网络超时');
    });

    it('处理 Error 实例', () => {
      const err = new Error('IO 失败');
      const res = formatErrorDetail(err);
      expect(res.message).toBe('IO 失败');
      expect(res.stack).toBeDefined();
    });

    it('处理对象结构错误', () => {
      const res = formatErrorDetail({ message: '自定义错误', code: 500 });
      expect(res.message).toBe('自定义错误');
    });

    it('处理空/未知错误', () => {
      const res = formatErrorDetail(null);
      expect(res.message).toBe('Unknown error');
    });
  });

  describe('notifyError', () => {
    it('弹出 toast.error 并向控制台写入 ERROR 级别系统日志', () => {
      notifyError('保存流程失败', new Error('磁盘空间不足'), { category: 'FlowStorage' });

      expect(toast.error).toHaveBeenCalledWith(
        '保存流程失败',
        expect.objectContaining({
          description: '磁盘空间不足',
          duration: 5000,
        })
      );

      const logs = useConsoleStore.getState().logs;
      expect(logs.length).toBe(1);
      expect(logs[0].level).toBe('ERROR');
      expect(logs[0].message).toBe('保存流程失败: 磁盘空间不足');
      expect(logs[0].nodeName).toBe('[FlowStorage]');
      expect(logs[0].source).toBe('system');
    });

    it('当 silentToast 为 true 时，不弹窗但仍写入系统日志', () => {
      notifyError('后台自动保存失败', '连接断开', { silentToast: true, category: 'FlowStorage' });

      expect(toast.error).not.toHaveBeenCalled();

      const logs = useConsoleStore.getState().logs;
      expect(logs.length).toBe(1);
      expect(logs[0].level).toBe('ERROR');
      expect(logs[0].message).toBe('后台自动保存失败: 连接断开');
    });
  });

  describe('notifySuccess', () => {
    it('触发 toast.success 并写入 INFO 系统日志', () => {
      notifySuccess('保存成功', '流程已持久化到磁盘', { category: 'FlowStorage' });

      expect(toast.success).toHaveBeenCalledWith('保存成功', expect.objectContaining({
        description: '流程已持久化到磁盘',
      }));

      const logs = useConsoleStore.getState().logs;
      expect(logs.length).toBe(1);
      expect(logs[0].level).toBe('INFO');
      expect(logs[0].message).toBe('保存成功 - 流程已持久化到磁盘');
    });
  });

  describe('notifyWarning and notifyInfo', () => {
    it('notifyWarning 触发 toast.warning 并写入 WARN 日志', () => {
      notifyWarning('警告', '可能存在性能风险', { category: 'Plugin' });
      expect(toast.warning).toHaveBeenCalled();
      const logs = useConsoleStore.getState().logs;
      expect(logs[0].level).toBe('WARN');
    });

    it('notifyInfo 触发 toast.info 并写入 INFO 日志', () => {
      notifyInfo('提示', '插件已重载', { category: 'Plugin' });
      expect(toast.info).toHaveBeenCalled();
      const logs = useConsoleStore.getState().logs;
      expect(logs[0].level).toBe('INFO');
    });
  });
});
