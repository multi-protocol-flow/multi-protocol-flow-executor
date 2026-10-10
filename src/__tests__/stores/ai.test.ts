import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useAIStore } from '@/stores/ai';
import { useSettingsStore } from '@/stores/settings';
import { useFlowStore } from '@/stores/flow';
import { aiSessionStorage } from '@/services/storage/aiSessionStorage';
import * as openaiClientModule from '@/services/ai/openaiClient';

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

  it('完全信任模式 (alwaysTrustHighRisk) 状态切换与自动批准挂起操作', async () => {
    const store = useAIStore.getState();
    expect(store.alwaysTrustHighRisk).toBe(false);

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
        {
          id: 'action-2',
          toolCallId: 'tc-2',
          toolName: 'delete_nodes',
          args: { node_uuids: ['n1'] },
          description: '删除节点',
          status: 'pending',
          createdAt: Date.now(),
        },
      ],
    });

    store.setAlwaysTrustHighRisk(true);
    expect(useAIStore.getState().alwaysTrustHighRisk).toBe(true);
    expect(useAIStore.getState().pendingActions.every((a) => a.status === 'approved')).toBe(true);

    store.toggleAlwaysTrustHighRisk();
    expect(useAIStore.getState().alwaysTrustHighRisk).toBe(false);
  });

  it('resolvePendingAction 传入 trustAll=true 时同时批准当前操作并开启完全信任模式', async () => {
    useAIStore.setState({
      alwaysTrustHighRisk: false,
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
        {
          id: 'action-2',
          toolCallId: 'tc-2',
          toolName: 'delete_nodes',
          args: { node_uuids: ['n1'] },
          description: '删除节点',
          status: 'pending',
          createdAt: Date.now(),
        },
      ],
    });

    await useAIStore.getState().resolvePendingAction('action-1', true, true);
    expect(useAIStore.getState().alwaysTrustHighRisk).toBe(true);
    expect(useAIStore.getState().pendingActions[0].status).toBe('approved');
    expect(useAIStore.getState().pendingActions[1].status).toBe('approved');
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

  it('setSessionWorkspace 正确设置活跃工作区并持久化至当前会话', async () => {
    await useAIStore.getState().initSessions();
    const ws = {
      path: '/projects/demo-service',
      name: 'demo-service',
      techStack: 'Node.js (Express)',
      apiSpecPath: 'openapi.yaml',
    };

    useAIStore.getState().setSessionWorkspace(ws);
    expect(useAIStore.getState().activeWorkspace).toEqual(ws);

    // 验证切换会话或新建会话的行为
    const currentId = useAIStore.getState().currentSessionId;
    expect(currentId).toBeDefined();

    // 创建一个新会话，新会话默认无工作区
    const newSessionId = await useAIStore.getState().createNewSession('新会话');
    expect(useAIStore.getState().currentSessionId).toBe(newSessionId);
    expect(useAIStore.getState().activeWorkspace).toBeNull();

    // 切回原会话，工作区自动恢复
    await useAIStore.getState().switchSession(currentId!);
    expect(useAIStore.getState().activeWorkspace).toEqual(ws);
  });

  it('流式生成时高频推送大量 thinking chunk 能被平滑缓冲并最终完整落库', async () => {
    const streamSpy = vi
      .spyOn(openaiClientModule, 'streamChatCompletion')
      .mockImplementation(async (_cfg, _msgs, _tools, callbacks) => {
        // 模拟数千甚至上万字的大量高频 thinking chunk
        for (let i = 0; i < 20; i++) {
          callbacks?.onThoughtChunk?.(`thinking chunk ${i}; `);
        }
        callbacks?.onContentChunk?.('Final content answer.');

        return {
          content: 'Final content answer.',
          thought: Array.from({ length: 20 }, (_, i) => `thinking chunk ${i}; `).join(''),
        };
      });

    await useAIStore.getState().sendMessage('请深度推理');

    const messages = useAIStore.getState().messages;
    expect(messages.length).toBeGreaterThanOrEqual(2);
    const assistantMsg = messages.find((m) => m.role === 'assistant');
    expect(assistantMsg).toBeDefined();
    expect(assistantMsg?.content).toBe('Final content answer.');
    expect(assistantMsg?.thought).toContain('thinking chunk 0;');
    expect(assistantMsg?.thought).toContain('thinking chunk 19;');
    expect(assistantMsg?.status).toBe('done');

    streamSpy.mockRestore();
  });

  describe('compactHistory & slash command interception', () => {
    it('compactHistory 会话消息少于等于2条时不执行并返回 false', async () => {
      useAIStore.setState({
        messages: [
          { id: '1', role: 'user', content: 'hello', timestamp: 1 },
          { id: '2', role: 'assistant', content: 'hi', timestamp: 2 },
        ],
      });

      const res = await useAIStore.getState().compactHistory();
      expect(res).toBe(false);
      expect(useAIStore.getState().messages).toHaveLength(2);
    });

    it('compactHistory 正常总结多轮会话、标记已归档并追加 isSummary 摘要卡片', async () => {
      const sessionId = await useAIStore.getState().createNewSession('测试压缩会话');
      useAIStore.setState({
        currentSessionId: sessionId,
        messages: [
          { id: '1', role: 'user', content: '步骤1：创建HTTP请求节点', timestamp: 1 },
          { id: '2', role: 'assistant', content: '已为您创建HTTP节点', timestamp: 2 },
          { id: '3', role: 'user', content: '步骤2：添加状态断言', timestamp: 3 },
          { id: '4', role: 'assistant', content: '已为您添加断言', timestamp: 4 },
        ],
      });

      const streamSpy = vi
        .spyOn(openaiClientModule, 'streamChatCompletion')
        .mockResolvedValue({
          content: '### Objective\n- User created HTTP request and assertions.',
        });

      const res = await useAIStore.getState().compactHistory();
      expect(res).toBe(true);

      const msgs = useAIStore.getState().messages;
      expect(msgs).toHaveLength(5);
      // 前4条历史消息应全部被标记为 isArchived: true
      expect(msgs.slice(0, 4).every((m) => m.isArchived === true)).toBe(true);

      // 第5条为新注入的 isSummary 消息
      const summaryMsg = msgs[4];
      expect(summaryMsg.isSummary).toBe(true);
      expect(summaryMsg.compactMeta?.compressedMessageCount).toBe(4);
      expect(summaryMsg.compactMeta?.originalTokenEstimate).toBeGreaterThan(0);
      expect(summaryMsg.content).toContain('User created HTTP request and assertions');

      // 验证已持久化至 aiSessionStorage
      const loaded = await aiSessionStorage.loadSession(sessionId);
      expect(loaded?.messages).toHaveLength(5);
      expect(loaded?.messages[4].isSummary).toBe(true);

      streamSpy.mockRestore();
    });

    it('sendMessage 遇到 /clear 指令自动拦截执行，不触发大模型请求', async () => {
      const streamSpy = vi.spyOn(openaiClientModule, 'streamChatCompletion');
      useAIStore.setState({
        messages: [{ id: '1', role: 'user', content: 'hello', timestamp: 1 }],
      });

      await useAIStore.getState().sendMessage('/clear');

      // messages 已被清空
      expect(useAIStore.getState().messages).toHaveLength(0);
      // streamChatCompletion 未被调用
      expect(streamSpy).not.toHaveBeenCalled();

      streamSpy.mockRestore();
    });

    it('sendMessage 遇到 /help 指令自动拦截并追加指令说明', async () => {
      const streamSpy = vi.spyOn(openaiClientModule, 'streamChatCompletion');

      await useAIStore.getState().sendMessage('/help');

      const msgs = useAIStore.getState().messages;
      expect(msgs).toHaveLength(1);
      expect(msgs[0].role).toBe('assistant');
      expect(msgs[0].content).toContain('/compact');
      expect(streamSpy).not.toHaveBeenCalled();

      streamSpy.mockRestore();
    });

    it('compactHistory 处于生成状态中阻断执行并返回 false', async () => {
      useAIStore.setState({
        agentStatus: 'thinking',
        messages: [
          { id: '1', role: 'user', content: 'msg1', timestamp: 1 },
          { id: '2', role: 'assistant', content: 'msg2', timestamp: 2 },
          { id: '3', role: 'user', content: 'msg3', timestamp: 3 },
        ],
      });

      const res = await useAIStore.getState().compactHistory();
      expect(res).toBe(false);
      // messages 保持不变
      expect(useAIStore.getState().messages).toHaveLength(3);
    });

    it('compactHistory 执行期间用户切换会话，不污染新会话状态，安全落盘原会话', async () => {
      const sess1 = await useAIStore.getState().createNewSession('会话1');
      const sess2 = await useAIStore.getState().createNewSession('会话2');

      // 切换到会话1
      await useAIStore.getState().switchSession(sess1);
      const sess1Messages = [
        { id: 'm1', role: 'user' as const, content: '会话1步骤1', timestamp: 1 },
        { id: 'm2', role: 'assistant' as const, content: '会话1步骤2', timestamp: 2 },
        { id: 'm3', role: 'user' as const, content: '会话1步骤3', timestamp: 3 },
      ];
      useAIStore.setState({ messages: sess1Messages });

      // 模拟 streamChatCompletion 执行中途用户切换到了会话2
      const streamSpy = vi
        .spyOn(openaiClientModule, 'streamChatCompletion')
        .mockImplementation(async () => {
          // 模拟异步返回前，用户点击了切换到会话2
          useAIStore.setState({
            currentSessionId: sess2,
            messages: [{ id: 's2_m1', role: 'user' as const, content: '会话2内容', timestamp: 10 }],
          });
          return { content: '会话1结构化摘要' };
        });

      const res = await useAIStore.getState().compactHistory();
      expect(res).toBe(true);

      // 当前活跃会话依然是会话2，messages 不被会话1污染
      expect(useAIStore.getState().currentSessionId).toBe(sess2);
      expect(useAIStore.getState().messages).toEqual([
        { id: 's2_m1', role: 'user', content: '会话2内容', timestamp: 10 },
      ]);

      // 验证后台持久化中的会话1已被安全更新并写入摘要
      const savedSess1 = await aiSessionStorage.loadSession(sess1);
      expect(savedSess1?.messages).toBeDefined();
      expect(savedSess1?.messages.some((m) => m.isSummary)).toBe(true);

      streamSpy.mockRestore();
    });

    it('sendMessage 在正在执行压缩时阻断新消息提交', async () => {
      const streamSpy = vi.spyOn(openaiClientModule, 'streamChatCompletion');
      useAIStore.setState({ isCompacting: true });

      await useAIStore.getState().sendMessage('新提问');

      // 没有触发大模型生成，新提问未加入 messages
      expect(streamSpy).not.toHaveBeenCalled();
      expect(useAIStore.getState().messages).toHaveLength(0);

      streamSpy.mockRestore();
    });
  });
});
