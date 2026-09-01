import { describe, it, expect, beforeEach } from 'vitest';
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
});
