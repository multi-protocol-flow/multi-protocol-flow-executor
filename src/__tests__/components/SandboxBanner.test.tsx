/**
 * SandboxBanner 组件测试
 */
import { render, screen } from '@testing-library/react';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { SandboxBanner } from '@/components/common/SandboxBanner';

describe('SandboxBanner', () => {
  beforeEach(() => {
    delete (globalThis as Record<string, unknown>).__TAURI__;
  });

  afterEach(() => {
    delete (globalThis as Record<string, unknown>).__TAURI__;
  });

  it('在非 Tauri（浏览器沙箱）环境下正常渲染警告与官网跳转按钮', () => {
    render(<SandboxBanner />);

    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText('Sandbox Mode')).toBeInTheDocument();

    const officialSiteLink = screen.getByRole('link', { name: /Official Website/i });
    expect(officialSiteLink).toBeInTheDocument();
    expect(officialSiteLink).toHaveAttribute('href', '../');
  });

  it('在 Tauri 环境下不渲染', () => {
    (globalThis as Record<string, unknown>).__TAURI__ = {};
    const { container } = render(<SandboxBanner />);
    expect(container).toBeEmptyDOMElement();
  });
});
