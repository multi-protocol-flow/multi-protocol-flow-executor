import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { AICopilotPanel } from '@/components/ai/AICopilotPanel';
import { useAIStore } from '@/stores/ai';
import { useSettingsStore } from '@/stores/settings';
import { getLL } from '@/i18n/i18n-lazy';

describe('AICopilotPanel UI & Execution Enhancements', () => {
  beforeEach(() => {
    window.HTMLElement.prototype.scrollIntoView = vi.fn();
    useAIStore.getState().reset();
    useSettingsStore.setState({
      aiProviders: [
        {
          id: 'p1',
          name: 'Test Provider',
          baseUrl: 'https://api.test.com/v1',
          apiKey: 'key',
          model: 'gpt-4o',
        },
      ],
      activeAiProviderId: 'p1',
      aiEnabled: true,
    });
  });

  it('1. 面板背景完全不透明 (bg-background，且无 bg-background/95 与 backdrop-blur)', async () => {
    await act(async () => {
      useAIStore.setState({ isCopilotOpen: true, isFloating: false });
    });
    const { container, rerender } = render(<AICopilotPanel />);

    // 停靠模式下的 aside 面板
    const dockedAside = container.querySelector('aside');
    expect(dockedAside).toBeInTheDocument();
    expect(dockedAside?.className).toContain('bg-background');
    expect(dockedAside?.className).not.toContain('bg-background/95');
    expect(dockedAside?.className).not.toContain('backdrop-blur');

    // 切换到浮动模式
    await act(async () => {
      useAIStore.setState({ isFloating: true });
    });
    rerender(<AICopilotPanel />);

    const floatingDiv = screen.getByLabelText('AI Copilot Floating Panel');
    expect(floatingDiv).toBeInTheDocument();
    expect(floatingDiv.className).toContain('bg-background');
    expect(floatingDiv.className).not.toContain('bg-background/95');
    expect(floatingDiv.className).not.toContain('backdrop-blur');
  });

  it('2. 停靠与浮动形态切换按钮及尺寸定位正确工作', async () => {
    const ll = getLL();
    await act(async () => {
      useAIStore.setState({
        isCopilotOpen: true,
        isFloating: false,
        panelWidth: 420,
      });
    });

    const { rerender } = render(<AICopilotPanel />);

    // 停靠状态下：显示脱离吸附 (浮动) 按钮和拉伸边条
    const undockBtn = screen.getByTitle(ll.ai.copilot.undock_panel());
    expect(undockBtn).toBeInTheDocument();
    expect(screen.getByTitle(ll.ai.copilot.drag_resize())).toBeInTheDocument();

    // 点击脱离吸附按钮
    await act(async () => {
      fireEvent.click(undockBtn);
    });
    expect(useAIStore.getState().isFloating).toBe(true);

    // 重新渲染为浮动状态
    rerender(<AICopilotPanel />);

    // 浮动状态下：显示吸附至右侧按钮
    const dockBtn = screen.getByTitle(ll.ai.copilot.dock_to_right());
    expect(dockBtn).toBeInTheDocument();

    // 浮动面板具有绝对定位样式
    const floatingPanel = screen.getByLabelText('AI Copilot Floating Panel');
    expect(floatingPanel.style.left).toBeDefined();
    expect(floatingPanel.style.top).toBeDefined();

    // 点击吸附至右侧按钮
    await act(async () => {
      fireEvent.click(dockBtn);
    });
    expect(useAIStore.getState().isFloating).toBe(false);
  });

  it('3. 执行到底 (Auto-Execute) 开关正确切换状态', async () => {
    const ll = getLL();
    await act(async () => {
      useAIStore.setState({ isCopilotOpen: true, autoExecuteToGoal: true });
    });

    render(<AICopilotPanel />);

    const autoBtn = screen.getByTitle(ll.ai.copilot.auto_execute_tip());
    expect(autoBtn).toBeInTheDocument();
    expect(screen.getByText(ll.ai.copilot.auto_execute())).toBeInTheDocument();

    // 点击切换关闭
    await act(async () => {
      fireEvent.click(autoBtn);
    });
    expect(useAIStore.getState().autoExecuteToGoal).toBe(false);

    // 再次点击切换打开
    await act(async () => {
      fireEvent.click(autoBtn);
    });
    expect(useAIStore.getState().autoExecuteToGoal).toBe(true);
  });

  it('4. 执行中支持中途二次输入内容并展示中途纠偏按钮', async () => {
    const ll = getLL();
    await act(async () => {
      useAIStore.setState({
        isCopilotOpen: true,
        agentStatus: 'thinking',
        currentSessionId: 'sess-1',
        sessions: [
          {
            id: 'sess-1',
            title: '会话 1',
            createdAt: Date.now(),
            updatedAt: Date.now(),
            messageCount: 0,
          },
        ],
      });
    });

    render(<AICopilotPanel />);

    // 正在生成中，文本框提示为中途纠偏占位符
    const textarea = screen.getByPlaceholderText(ll.ai.copilot.input_midway_placeholder());
    expect(textarea).toBeInTheDocument();

    // 输入框尚未输入时，显示停止按钮，未显示纠偏按钮
    const stopBtn = screen.getByTitle(ll.ai.copilot.stop());
    expect(stopBtn).toBeInTheDocument();
    expect(screen.queryByTitle(ll.ai.copilot.send_correction_tip())).not.toBeInTheDocument();

    // 用户在执行中途输入矫正指令
    await act(async () => {
      fireEvent.change(textarea, { target: { value: '改用 PostgreSQL 数据库' } });
    });

    // 中途纠偏按钮立即呈现
    const steerBtn = screen.getByTitle(ll.ai.copilot.send_correction_tip());
    expect(steerBtn).toBeInTheDocument();
    expect(screen.getByText(ll.ai.copilot.send_correction())).toBeInTheDocument();

    // 点击纠偏发送
    await act(async () => {
      fireEvent.click(steerBtn);
    });

    // 验证纠偏消息已插入 store，且带有 isCorrection 标识
    const messages = useAIStore.getState().messages;
    const steerMsg = messages.find((m) => m.content === '改用 PostgreSQL 数据库');
    expect(steerMsg).toBeDefined();
    expect(steerMsg?.isCorrection).toBe(true);

    // 验证消息列表上方渲染了补充要求徽章
    expect(screen.getByText(ll.ai.copilot.steer_badge())).toBeInTheDocument();
  });

  it('5. 完全信任模式 (Trust Mode) 开关切换与提示横幅', async () => {
    const ll = getLL();
    await act(async () => {
      useAIStore.setState({ isCopilotOpen: true, alwaysTrustHighRisk: false });
    });

    const { rerender } = render(<AICopilotPanel />);

    const trustBtn = screen.getByTitle(ll.ai.copilot.trust_mode_tip());
    expect(trustBtn).toBeInTheDocument();
    expect(screen.queryByText(ll.ai.copilot.trust_mode_active_hint())).not.toBeInTheDocument();

    // 点击开启完全信任模式
    await act(async () => {
      fireEvent.click(trustBtn);
    });
    expect(useAIStore.getState().alwaysTrustHighRisk).toBe(true);

    rerender(<AICopilotPanel />);
    // 出现生效横幅与禁用按钮
    expect(screen.getByText(ll.ai.copilot.trust_mode_active_hint())).toBeInTheDocument();
    const disableBtn = screen.getByText(ll.ai.copilot.disable_trust_mode());
    expect(disableBtn).toBeInTheDocument();

    // 点击禁用按钮
    await act(async () => {
      fireEvent.click(disableBtn);
    });
    expect(useAIStore.getState().alwaysTrustHighRisk).toBe(false);
  });
});
