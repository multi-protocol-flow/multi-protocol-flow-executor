import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useAIStore } from '@/stores/ai';
import { useSettingsStore } from '@/stores/settings';
import { useFlowStore } from '@/stores/flow';
import { aiSessionStorage } from '@/services/storage/aiSessionStorage';

describe('aiStore', () => {
  beforeEach(async () => {
    await aiSessionStorage.clearAllSessions();
    useAIStore.getState().reset();
    useSettingsStore.setState({
      aiProviders: [
        {
          id: 'test-prov-1',
          name: 'Test DeepSeek',
          baseUrl: 'https://api.deepseek.com/v1',
          apiKey: 'sk-test',
          model: 'deepseek-chat',
        },
      ],
      activeAiProviderId: 'test-prov-1',
      aiEnabled: true,
    });
    useFlowStore.getState().reset();
  });

  it('初始状态正确', () => {
    const state = useAIStore.getState();
    expect(state.messages).toEqual([]);
    expect(state.agentStatus).toBe('idle');
    expect(state.isCopilotOpen).toBe(false);
    expect(state.isSpotlightOpen).toBe(false);
  });

  it('切换侧边栏与 Spotlight 窗口状态', () => {
    const store = useAIStore.getState();
    store.toggleCopilot();
    expect(useAIStore.getState().isCopilotOpen).toBe(true);

    store.toggleSpotlight();
    expect(useAIStore.getState().isSpotlightOpen).toBe(true);
  });

  it('没有配置 Provider 时发送消息记录错误', async () => {
    useSettingsStore.setState({ aiProviders: [], activeAiProviderId: null });

    await useAIStore.getState().sendMessage('Hello AI');

    const messages = useAIStore.getState().messages;
    expect(messages).toHaveLength(2);
    expect(messages[0].role).toBe('user');
    expect(messages[1].role).toBe('assistant');
    expect(messages[1].status).toBe('error');
    expect(messages[1].content).toContain('No active AI Provider configured');
  });

  it('resolvePendingAction 正确更新待确认操作状态', async () => {
    useAIStore.setState({
      pendingActions: [
        {
          id: 'action-1',
          toolCallId: 'tc-1',
          toolName: 'clear_canvas',
          args: {},
          description: '清空画布',
          status: 'pending',
          createdAt: Date.now(),
        },
      ],
    });

    await useAIStore.getState().resolvePendingAction('action-1', true);
    expect(useAIStore.getState().pendingActions[0].status).toBe('approved');

    await useAIStore.getState().resolvePendingAction('action-1', false);
    expect(useAIStore.getState().pendingActions[0].status).toBe('rejected');
  });

  it('创建、重命名、切换与删除会话', async () => {
    const store = useAIStore.getState();
    const s1Id = await store.createNewSession('会话 1');
    expect(useAIStore.getState().currentSessionId).toBe(s1Id);

    const s2Id = await store.createNewSession('会话 2');
    expect(useAIStore.getState().currentSessionId).toBe(s2Id);
    expect(useAIStore.getState().sessions).toHaveLength(2);

    await store.renameSession(s1Id, '重命名会话 1');
    const s1Meta = useAIStore.getState().sessions.find((s) => s.id === s1Id);
    expect(s1Meta?.title).toBe('重命名会话 1');

    await store.switchSession(s1Id);
    expect(useAIStore.getState().currentSessionId).toBe(s1Id);

    await store.deleteSession(s1Id);
    expect(useAIStore.getState().sessions).toHaveLength(1);
    expect(useAIStore.getState().currentSessionId).toBe(s2Id);
  });

  it('切换会话时自动持久化当前会话并在目标会话缺失时安全兜底', async () => {
    const store = useAIStore.getState();
    const s1Id = await store.createNewSession('会话 A');
    useAIStore.setState({
      messages: [
        { id: 'm1', role: 'user', content: 'Session A message', timestamp: Date.now() },
      ],
    });

    const s2Id = await store.createNewSession('会话 B');
    expect(useAIStore.getState().currentSessionId).toBe(s2Id);
    expect(useAIStore.getState().messages).toEqual([]);

    // 切换回会话 A：自动持久化会话 B 并恢复会话 A 的消息
    await store.switchSession(s1Id);
    expect(useAIStore.getState().currentSessionId).toBe(s1Id);
    expect(useAIStore.getState().messages).toHaveLength(1);
    expect(useAIStore.getState().messages[0].content).toBe('Session A message');

    // 模拟目标会话在存储中被删除，switchSession 依然能正确切换到该 ID 并安全兜底为空消息
    await aiSessionStorage.deleteSession('non-existent-session-id');
    await store.switchSession('non-existent-session-id');
    expect(useAIStore.getState().currentSessionId).toBe('non-existent-session-id');
    expect(useAIStore.getState().messages).toEqual([]);
  });

  it('修改侧边栏宽度', () => {
    const store = useAIStore.getState();
    store.setPanelWidth(450);
    expect(useAIStore.getState().panelWidth).toBe(450);

    // 验证边界值夹取 (300 ~ 700)
    store.setPanelWidth(100);
    expect(useAIStore.getState().panelWidth).toBe(300);

    store.setPanelWidth(1000);
    expect(useAIStore.getState().panelWidth).toBe(700);
  });

  it('浮动窗口状态切换与位置更新', () => {
    const store = useAIStore.getState();
    expect(store.isFloating).toBe(false);

    store.setFloating(true);
    expect(useAIStore.getState().isFloating).toBe(true);

    store.toggleFloating();
    expect(useAIStore.getState().isFloating).toBe(false);

    store.setFloatingPosition({ x: 200, y: 150, width: 500, height: 600 });
    const pos = useAIStore.getState().floatingPosition;
    expect(pos.x).toBe(200);
    expect(pos.y).toBe(150);
    expect(pos.width).toBe(500);
    expect(pos.height).toBe(600);

    // 验证最小宽度和高度限制 (>= 340, >= 300)
    store.setFloatingPosition({ width: 100, height: 100 });
    const clampedPos = useAIStore.getState().floatingPosition;
    expect(clampedPos.width).toBe(340);
    expect(clampedPos.height).toBe(300);
  });

  it('自主执行到底模式状态切换', () => {
    const store = useAIStore.getState();
    // 默认开启自主执行到底模式
    expect(store.autoExecuteToGoal).toBe(true);

    store.setAutoExecuteToGoal(false);
    expect(useAIStore.getState().autoExecuteToGoal).toBe(false);

    store.toggleAutoExecuteToGoal();
    expect(useAIStore.getState().autoExecuteToGoal).toBe(true);
  });

  it('中途纠偏 steerGeneration 正常插入纠偏消息并重置待确认高风险状态', async () => {
    useAIStore.setState({
      agentStatus: 'thinking',
      pendingActions: [
        {
          id: 'act-1',
          toolCallId: 'call-1',
          toolName: 'clear_canvas',
          args: {},
          description: '清空画布',
          status: 'pending',
          createdAt: Date.now(),
        },
      ],
    });

    await useAIStore.getState().steerGeneration('请不要清空，改用重命名');

    const state = useAIStore.getState();
    const lastMsg = state.messages[state.messages.length - 1];
    expect(lastMsg.role).toBe('user');
    expect(lastMsg.content).toBe('请不要清空，改用重命名');
    expect(lastMsg.isCorrection).toBe(true);
    expect(state.pendingActions[0].status).toBe('rejected');
  });

  it('正在生成中调用 sendMessage 自动委托给 steerGeneration', async () => {
    useAIStore.setState({
      agentStatus: 'executing_tools',
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

    await useAIStore.getState().sendMessage('中途追加要求：超时设为 5000ms');

    const messages = useAIStore.getState().messages;
    const steerMsg = messages.find((m) => m.content.includes('超时设为 5000ms'));
    expect(steerMsg).toBeDefined();
    expect(steerMsg?.role).toBe('user');
    expect(steerMsg?.isCorrection).toBe(true);
  });
  it('retryLastMessage 移除错误消息并重新触发最后一条用户消息发送', async () => {
    const sendSpy = vi.spyOn(useAIStore.getState(), 'sendMessage').mockResolvedValue();

    useAIStore.setState({
      messages: [
        { id: '1', role: 'user', content: '创建工作流', timestamp: 1 },
        { id: '2', role: 'assistant', content: '网络错误', status: 'error', timestamp: 2 },
      ],
      error: '网络错误',
      agentStatus: 'idle',
    });

    await useAIStore.getState().retryLastMessage();

    // 验证旧错误与原用户消息已被清出，并以原内容调用 sendMessage
    expect(useAIStore.getState().error).toBeNull();
    expect(sendSpy).toHaveBeenCalledWith('创建工作流', undefined);

    sendSpy.mockRestore();
  });
});
