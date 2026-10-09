import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { AIConfirmCard } from '@/components/ai/AIConfirmCard';
import { useAIStore } from '@/stores/ai';
import { getLL } from '@/i18n/i18n-lazy';
import type { AIPendingAction } from '@/types/ai';

describe('AIConfirmCard', () => {
  beforeEach(() => {
    useAIStore.getState().reset();
  });

  const mockAction: AIPendingAction = {
    id: 'test-action-1',
    toolCallId: 'tc-123',
    toolName: 'clear_canvas',
    args: {},
    description: '清空当前流程画布中的所有节点与连线',
    diffSummary: '当前画布共有 5 个节点和 4 条连线将被全部移除。',
    status: 'pending',
    createdAt: Date.now(),
  };

  it('1. 正确渲染待确认高风险操作及取消、确认、完全信任并执行三个按钮', () => {
    const ll = getLL();
    render(<AIConfirmCard action={mockAction} />);

    expect(screen.getByText(ll.ai.confirm_card.title())).toBeInTheDocument();
    expect(screen.getByText(mockAction.description)).toBeInTheDocument();
    expect(screen.getByText(mockAction.diffSummary!)).toBeInTheDocument();

    expect(screen.getByText(ll.ai.confirm_card.reject())).toBeInTheDocument();
    expect(screen.getByText(ll.ai.confirm_card.approve())).toBeInTheDocument();
    expect(screen.getByText(ll.ai.confirm_card.trust_all())).toBeInTheDocument();
  });

  it('2. 点击确认执行按钮触发 resolvePendingAction(id, true)', async () => {
    const spy = vi.spyOn(useAIStore.getState(), 'resolvePendingAction').mockResolvedValue();
    const ll = getLL();
    render(<AIConfirmCard action={mockAction} />);
    const approveBtn = screen.getByText(ll.ai.confirm_card.approve());
    await act(async () => {
      fireEvent.click(approveBtn);
    });

    expect(spy).toHaveBeenCalledWith('test-action-1', true);
    spy.mockRestore();
  });

  it('3. 点击取消执行按钮触发 resolvePendingAction(id, false)', async () => {
    const spy = vi.spyOn(useAIStore.getState(), 'resolvePendingAction').mockResolvedValue();
    const ll = getLL();

    render(<AIConfirmCard action={mockAction} />);
    const rejectBtn = screen.getByText(ll.ai.confirm_card.reject());
    await act(async () => {
      fireEvent.click(rejectBtn);
    });

    expect(spy).toHaveBeenCalledWith('test-action-1', false);
    spy.mockRestore();
  });

  it('4. 点击完全信任并执行按钮触发 resolvePendingAction(id, true, true)', async () => {
    const spy = vi.spyOn(useAIStore.getState(), 'resolvePendingAction').mockResolvedValue();
    const ll = getLL();

    render(<AIConfirmCard action={mockAction} />);
    const trustAllBtn = screen.getByText(ll.ai.confirm_card.trust_all());
    await act(async () => {
      fireEvent.click(trustAllBtn);
    });

    expect(spy).toHaveBeenCalledWith('test-action-1', true, true);
    spy.mockRestore();
  });

  it('5. 处于 approved 或 rejected 状态时展示对应状态徽章，不展示操作按钮', () => {
    const ll = getLL();
    const { rerender } = render(
      <AIConfirmCard action={{ ...mockAction, status: 'approved' }} />
    );

    expect(screen.getByText(ll.ai.confirm_card.status_approved())).toBeInTheDocument();
    expect(screen.queryByText(ll.ai.confirm_card.approve())).not.toBeInTheDocument();
    expect(screen.queryByText(ll.ai.confirm_card.trust_all())).not.toBeInTheDocument();

    rerender(<AIConfirmCard action={{ ...mockAction, status: 'rejected' }} />);
    expect(screen.getByText(ll.ai.confirm_card.status_rejected())).toBeInTheDocument();
  });
});
