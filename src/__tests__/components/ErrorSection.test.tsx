import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ErrorSection } from '@/components/execution/viewers/ErrorSection';
import { useAIStore } from '@/stores/ai';
import type { ErrorDetail } from '@/types';

describe('ErrorSection', () => {
  const mockError: ErrorDetail = {
    error_kind: 'connection',
    error_code: 'E_CONN_REFUSED',
    message: 'Connection refused on port 8080',
    stack_trace: 'Error: Connection refused\n  at TCP.connect',
  };

  it('点击 AI 诊断修复按钮时发送国际化诊断提示词 (zh-CN)', async () => {
    localStorage.setItem('locale', 'zh-CN');
    const sendSpy = vi.spyOn(useAIStore.getState(), 'sendMessage').mockResolvedValue();

    render(
      <ErrorSection
        error={mockError}
        nodeName="API 请求"
        nodeType="http"
      />
    );

    const btn = screen.getByRole('button', { name: /AI 诊断修复/i });
    expect(btn).toBeInTheDocument();
    fireEvent.click(btn);

    expect(sendSpy).toHaveBeenCalledTimes(1);
    const prompt = sendSpy.mock.calls[0][0];
    expect(prompt).toContain('API 请求');
    expect(prompt).toContain('http');
    expect(prompt).toContain('connection');
    expect(prompt).toContain('分析具体报错原因');

    sendSpy.mockRestore();
  });

  it('在 en-US 语言环境下发送英文提示词', async () => {
    localStorage.setItem('locale', 'en-US');
    const sendSpy = vi.spyOn(useAIStore.getState(), 'sendMessage').mockResolvedValue();

    render(
      <ErrorSection
        error={mockError}
        nodeName="Fetch User"
        nodeType="http"
      />
    );

    const btn = screen.getByRole('button', { name: /AI Diagnose & Fix/i });
    expect(btn).toBeInTheDocument();
    fireEvent.click(btn);

    expect(sendSpy).toHaveBeenCalledTimes(1);
    const prompt = sendSpy.mock.calls[0][0];
    expect(prompt).toContain('Fetch User');
    expect(prompt).toContain('http');
    expect(prompt).toContain('connection');
    expect(prompt).toContain('execution failed');
    expect(prompt).toContain('analyze the root cause');

    // 恢复默认语言
    localStorage.setItem('locale', 'zh-CN');
    sendSpy.mockRestore();
  });

  it('无 nodeName 时发送通用的当前节点诊断提示词', async () => {
    localStorage.setItem('locale', 'zh-CN');
    const sendSpy = vi.spyOn(useAIStore.getState(), 'sendMessage').mockResolvedValue();

    render(<ErrorSection error={mockError} />);

    const btn = screen.getByRole('button', { name: /AI 诊断修复/i });
    fireEvent.click(btn);

    expect(sendSpy).toHaveBeenCalledTimes(1);
    const prompt = sendSpy.mock.calls[0][0];
    expect(prompt).toContain('当前节点执行报错了');

    sendSpy.mockRestore();
  });
});
