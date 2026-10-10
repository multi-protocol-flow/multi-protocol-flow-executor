import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { AICopilotPanel } from '@/components/ai/AICopilotPanel';
import { useAIStore } from '@/stores/ai';
import { useSettingsStore } from '@/stores/settings';
import { getLL } from '@/i18n/i18n-lazy';
import { aiSessionStorage } from '@/services/storage/aiSessionStorage';

describe('AICopilotPanel UI & Execution Enhancements', () => {
  beforeEach(async () => {
    window.HTMLElement.prototype.scrollIntoView = vi.fn();
    await aiSessionStorage.clearAllSessions();
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

  it('6. 输入 / 弹出斜杠指令浮层菜单，支持点击与选择执行', async () => {
    await act(async () => {
      useAIStore.setState({ isCopilotOpen: true });
    });

    render(<AICopilotPanel />);
    const textarea = screen.getByPlaceholderText(getLL().ai.copilot.input_placeholder());

    // 用户在输入框中输入 '/'
    await act(async () => {
      fireEvent.change(textarea, { target: { value: '/' } });
    });

    // 快捷指令弹层显示
    expect(screen.getByText(getLL().ai.copilot.slash_commands_title())).toBeInTheDocument();
    expect(screen.getByText('/compact')).toBeInTheDocument();
    expect(screen.getByText('/clear')).toBeInTheDocument();
    expect(screen.getByText('/help')).toBeInTheDocument();

    // 点击 /help 指令项执行
    const helpBtn = screen.getByText('/help').closest('button');
    expect(helpBtn).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(helpBtn!);
    });

    // 消息列表中应注入 help 说明消息
    const messages = useAIStore.getState().messages;
    expect(messages.length).toBeGreaterThanOrEqual(1);
    expect(messages[0].content).toContain('/compact');
  });

  it('7. 渲染上下文压缩卡片并可切换查看已归档的原始记录', async () => {
    const testMessages = [
      {
        id: 'archived-1',
        role: 'user' as const,
        content: '第一轮历史提问',
        isArchived: true,
        timestamp: 1000,
      },
      {
        id: 'archived-2',
        role: 'assistant' as const,
        content: '第一轮历史回答',
        isArchived: true,
        timestamp: 2000,
      },
      {
        id: 'summary-1',
        role: 'assistant' as const,
        content: '### 会话状态摘要\n- 已完成步骤：已配置HTTP节点',
        isSummary: true,
        compactMeta: {
          compressedMessageCount: 2,
          originalTokenEstimate: 120,
          timestamp: 3000,
        },
        timestamp: 3000,
      },
      {
        id: 'recent-1',
        role: 'user' as const,
        content: '第二轮最新提问',
        timestamp: 4000,
      },
    ];

    await aiSessionStorage.saveSession({
      id: 'session-with-compact',
      title: '压缩测试',
      createdAt: Date.now(),
      updatedAt: Date.now(),
      messages: testMessages,
    });

    await act(async () => {
      useAIStore.setState({
        isCopilotOpen: true,
        currentSessionId: 'session-with-compact',
        messages: testMessages,
      });
    });

    render(<AICopilotPanel />);

    // 验证渲染了压缩卡片标题与 Token 统计
    expect(screen.getByText(getLL().ai.copilot.compact_card_title({ count: 2 }))).toBeInTheDocument();
    expect(screen.getByText(getLL().ai.copilot.compact_card_tokens({ tokens: 120 }))).toBeInTheDocument();
    expect(screen.getByText(/已配置HTTP节点/)).toBeInTheDocument();

    // 默认情况下，已归档的具体消息折叠未展开
    expect(screen.queryByText('第一轮历史提问')).not.toBeInTheDocument();

    // 点击展开查看归档记录
    const viewArchivedBtn = screen.getByText(new RegExp(getLL().ai.copilot.compact_card_view_archived()));
    await act(async () => {
      fireEvent.click(viewArchivedBtn);
    });

    // 展开后显示已归档的原始提问
    expect(screen.getByText('第一轮历史提问')).toBeInTheDocument();
    expect(screen.getByText('第一轮历史回答')).toBeInTheDocument();
  });

  it('8. 消息多于 2 条时头部工具栏显示压缩上下文按钮', async () => {
    const ll = getLL();
    await act(async () => {
      useAIStore.setState({
        isCopilotOpen: true,
        messages: [
          { id: '1', role: 'user', content: 'msg 1', timestamp: 1 },
          { id: '2', role: 'assistant', content: 'msg 2', timestamp: 2 },
          { id: '3', role: 'user', content: 'msg 3', timestamp: 3 },
        ],
      });
    });

    render(<AICopilotPanel />);

    const compactBtn = screen.getByTitle(ll.ai.copilot.compact_button_tip());
    expect(compactBtn).toBeInTheDocument();
  });

  it('8.1. 超窄宽度 (< 360px) 且有消息时，压缩与清空按钮收拢至更多操作菜单', async () => {
    const ll = getLL();
    const testMessages = [
      { id: '1', role: 'user' as const, content: 'msg 1', timestamp: 1 },
      { id: '2', role: 'assistant' as const, content: 'msg 2', timestamp: 2 },
      { id: '3', role: 'user' as const, content: 'msg 3', timestamp: 3 },
    ];
    await aiSessionStorage.saveSession({
      id: 'sess-narrow',
      title: '测试会话',
      createdAt: 1,
      updatedAt: 1,
      messages: testMessages,
    });
    await act(async () => {
      useAIStore.setState({
        isCopilotOpen: true,
        panelWidth: 320,
        isFloating: false,
        currentSessionId: 'sess-narrow',
        messages: testMessages,
      });
    });
    render(<AICopilotPanel />);

    // 平铺的压缩按钮被收拢，展示更多操作菜单按钮
    expect(screen.queryByTitle(ll.ai.copilot.compact_button_tip())).not.toBeInTheDocument();
    const moreBtn = screen.getByTitle(ll.ai.copilot.more_actions());
    expect(moreBtn).toBeInTheDocument();

    // 点击更多按钮展开下拉菜单
    await act(async () => {
      fireEvent.click(moreBtn);
    });

    expect(screen.getByText(ll.ai.copilot.command_compact())).toBeInTheDocument();
    expect(screen.getByText(ll.ai.copilot.clear_history())).toBeInTheDocument();
  });

  it('9. 侧边栏反复展开与收起，Hook 调用顺序与数量严格一致，无 Rendered more hooks 异常', async () => {
    // 初始关闭状态
    useAIStore.setState({ isCopilotOpen: false });
    const { rerender } = render(<AICopilotPanel />);

    // 展开面板
    await act(async () => {
      useAIStore.setState({ isCopilotOpen: true });
    });
    rerender(<AICopilotPanel />);
    expect(screen.getByRole('complementary')).toBeInTheDocument();

    // 收起面板
    await act(async () => {
      useAIStore.setState({ isCopilotOpen: false });
    });
    rerender(<AICopilotPanel />);
    expect(screen.queryByRole('complementary')).not.toBeInTheDocument();

    // 再次展开面板
    await act(async () => {
      useAIStore.setState({ isCopilotOpen: true });
    });
    rerender(<AICopilotPanel />);
    expect(screen.getByRole('complementary')).toBeInTheDocument();
  });
});
