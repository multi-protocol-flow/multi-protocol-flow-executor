/**
 * EntryConfig 组件测试
 *
 * 注意：初始变量已统一到流程级 initial_variables + 环境管理，
 * Entry 节点不再独立管理变量。当前测试仅验证剩余功能（说明卡片等）。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { EntryConfig } from '@/components/panels/EntryConfig';
import type { EntryConfig as EntryConfigType } from '@/types/node';

describe('EntryConfig', () => {
  const mockOnSave = vi.fn();
  const defaultData: EntryConfigType = { type: 'entry' };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('应该渲染说明卡片标题', () => {
    render(<EntryConfig data={defaultData} onSave={mockOnSave} />);
    expect(screen.getByText('入口节点')).toBeDefined();
  });

  it('说明卡片内容应该左对齐', () => {
    render(<EntryConfig data={defaultData} onSave={mockOnSave} />);
    const card = screen.getByText('入口节点').closest('div');
    expect(card).toBeDefined();
    const titleElement = screen.getByText('入口节点');
    expect(titleElement.className).toContain('flex items-center gap-2');
  });
});
