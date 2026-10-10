/**
 * AI Copilot & ReAct Agent 状态管理
 */

import { create } from 'zustand';
import type {
  AIState,
  AIMessage,
  AISession,
  AIWorkspaceContext,
  AIToolCall,
  AIPendingAction,
} from '@/types/ai';
import { getLL } from '@/i18n/i18n-lazy';
import { toast } from 'sonner';
import { useSettingsStore } from './settings';
import { aiSessionStorage } from '@/services/storage/aiSessionStorage';
import { streamChatCompletion, type ChatCompletionResult } from '@/services/ai/openaiClient';
import {
  buildSystemPrompt,
  buildActiveStateSummary,
  buildCompactionSystemPrompt,
  buildCompactionUserPrompt,
  estimateTokens,
  pruneMessagesForSending,
} from '@/services/ai/contextBuilder';
import { executeSlashCommand } from '@/services/ai/slashCommands';
import {
  getAllAIToolDefinitions,
  isHighRiskTool,
  createPendingAction,
  dispatchToolCall,
} from '@/services/ai/toolDispatcher';

const PANEL_WIDTH_KEY = 'mpe-ai-panel-width';
const DEFAULT_PANEL_WIDTH = 380;
const PANEL_FLOATING_KEY = 'mpe-ai-panel-floating';
const PANEL_FLOATING_POS_KEY = 'mpe-ai-panel-floating-pos';
const ALWAYS_TRUST_HIGH_RISK_KEY = 'mpe-ai-always-trust-high-risk';

function getStoredPanelWidth(): number {
  try {
    const val = localStorage.getItem(PANEL_WIDTH_KEY);
    return val ? Math.max(300, Math.min(700, Number(val))) : DEFAULT_PANEL_WIDTH;
  } catch {
    return DEFAULT_PANEL_WIDTH;
  }
}

function getStoredFloating(): boolean {
  try {
    return localStorage.getItem(PANEL_FLOATING_KEY) === 'true';
  } catch {
    return false;
  }
}

function getStoredFloatingPos(): { x: number; y: number; width: number; height: number } {
  try {
    const val = localStorage.getItem(PANEL_FLOATING_POS_KEY);
    if (val) {
      const parsed = JSON.parse(val);
      if (
        typeof parsed.x === 'number' &&
        typeof parsed.y === 'number' &&
        typeof parsed.width === 'number' &&
        typeof parsed.height === 'number'
      ) {
        const safeY = Math.max(20, parsed.y);
        const safeWidth = Math.max(340, parsed.width);
        const safeHeight = Math.max(300, parsed.height);
        return { x: Math.max(0, parsed.x), y: safeY, width: safeWidth, height: safeHeight };
      }
    }
  } catch {
    // 忽略
  }
  const defaultWidth = 420;
  const defaultHeight = typeof window !== 'undefined' ? Math.max(450, window.innerHeight - 120) : 620;
  const defaultX = typeof window !== 'undefined' ? Math.max(50, window.innerWidth - defaultWidth - 30) : 300;
  const defaultY = 70;
  return { x: defaultX, y: defaultY, width: defaultWidth, height: defaultHeight };
}

function getStoredAlwaysTrustHighRisk(): boolean {
  try {
    return localStorage.getItem(ALWAYS_TRUST_HIGH_RISK_KEY) === 'true';
  } catch {
    return false;
  }
}


// 用于控制整个会话任务生命周期的 AbortController (用户显式点击"停止生成"时触发)
let sessionAbortController: AbortController | null = null;

// 用于控制当前单次 LLM streamChatCompletion 调用的 AbortController (中途插话纠偏时可中止单轮并进入下一轮)
let currentIterationAbortController: AbortController | null = null;
// 用于挂起等待用户确认高风险操作的 Promise 回调映射
const pendingResolvers = new Map<string, (approved: boolean) => void>();

export interface AIActions {
  /** 设置 Copilot 侧边栏开启状态 */
  setCopilotOpen: (open: boolean) => void;
  /** 切换 Copilot 侧边栏 */
  toggleCopilot: () => void;
  /** 设置 Spotlight 悬浮窗开启状态 */
  setSpotlightOpen: (open: boolean) => void;
  /** 切换 Spotlight 悬浮窗 */
  toggleSpotlight: () => void;
  /** 设置侧边栏宽度 */
  setPanelWidth: (width: number) => void;
  /** 设置面板浮动状态 */
  setFloating: (floating: boolean) => void;
  /** 切换面板浮动/吸附状态 */
  toggleFloating: () => void;
  /** 更新浮动面板位置与尺寸 */
  setFloatingPosition: (pos: Partial<{ x: number; y: number; width: number; height: number }>) => void;
  /** 设置是否开启完全信任模式 */
  setAlwaysTrustHighRisk: (trust: boolean) => void;
  /** 切换完全信任模式 */
  toggleAlwaysTrustHighRisk: () => void;
  // ===== 会话管理 =====
  /** 初始化加载所有历史会话列表 */
  initSessions: () => Promise<void>;
  /** 创建并切换到新会话 */
  createNewSession: (title?: string) => Promise<string>;
  /** 切换活跃会话 */
  switchSession: (sessionId: string) => Promise<void>;
  /** 重命名会话 */
  renameSession: (sessionId: string, newTitle: string) => Promise<void>;
  /** 删除指定会话 */
  deleteSession: (sessionId: string) => Promise<void>;
  // ===== 工作区感知 =====
  /** 设置当前活跃会话关联的本地项目工作区 */
  setSessionWorkspace: (ws: AIWorkspaceContext | null) => void;
  /** 打开原生文件选择器关联本地项目目录 */
  selectWorkspaceFolder: () => Promise<void>;


  /** 清空当前会话的消息历史 */
  clearMessages: () => void;
  /** 压缩总结当前会话历史，释放上下文窗口并节省 Token */
  compactHistory: () => Promise<boolean>;
  /** 中止当前 Agent 生成与工具执行 */
  stopGeneration: () => void;
  /** 发送用户消息并触发 ReAct Agent 循环 (支持附加图片) */
  sendMessage: (prompt: string, images?: string[]) => Promise<void>;
  /** 发送中途纠偏/补充指令 (正在生成时插话纠正，无需中断整个任务) */
  steerGeneration: (prompt: string, images?: string[]) => Promise<void>;
  /** 响应高风险操作确认或拒绝 (可附带完全信任后续所有操作) */
  resolvePendingAction: (actionId: string, approved: boolean, trustAll?: boolean) => Promise<void>;
  /** 重试上一条发送失败的消息 (清理错误状态并重新触发 ReAct 循环) */
  retryLastMessage: () => Promise<void>;
  /** 重置（测试隔离） */
  reset: () => void;
}

export type AIStore = AIState & AIActions;

const initialState: AIState = {
  currentSessionId: null,
  sessions: [],
  activeWorkspace: null,
  messages: [],
  agentStatus: 'idle',
  activeStreamingMessageId: null,
  pendingActions: [],
  isCopilotOpen: false,
  isSpotlightOpen: false,
  panelWidth: getStoredPanelWidth(),
  isFloating: getStoredFloating(),
  floatingPosition: getStoredFloatingPos(),
  alwaysTrustHighRisk: getStoredAlwaysTrustHighRisk(),
  error: null,
  isCompacting: false,
};
export const useAIStore = create<AIStore>((set, get) => ({
  ...initialState,

  setCopilotOpen: (open: boolean) => {
    set({ isCopilotOpen: open });
    if (open && !get().currentSessionId) {
      get().initSessions();
    }
  },
  toggleCopilot: () => {
    const next = !get().isCopilotOpen;
    set({ isCopilotOpen: next });
    if (next && !get().currentSessionId) {
      get().initSessions();
    }
  },

  setSpotlightOpen: (open: boolean) => set({ isSpotlightOpen: open }),
  toggleSpotlight: () => set((s) => ({ isSpotlightOpen: !s.isSpotlightOpen })),

  setPanelWidth: (width: number) => {
    const clamped = Math.max(300, Math.min(700, width));
    try {
      localStorage.setItem(PANEL_WIDTH_KEY, String(clamped));
    } catch {
      // 忽略
    }
    set({ panelWidth: clamped });
  },

  setFloating: (floating: boolean) => {
    try {
      localStorage.setItem(PANEL_FLOATING_KEY, String(floating));
    } catch {
      // 忽略
    }
    set({ isFloating: floating });
  },

  toggleFloating: () => {
    const next = !get().isFloating;
    try {
      localStorage.setItem(PANEL_FLOATING_KEY, String(next));
    } catch {
      // 忽略
    }
    set({ isFloating: next });
  },

  setFloatingPosition: (pos) => {
    const current = get().floatingPosition;
    const updated = {
      x: pos.x !== undefined ? Math.max(0, pos.x) : current.x,
      y: pos.y !== undefined ? Math.max(20, pos.y) : current.y,
      width: pos.width !== undefined ? Math.max(340, pos.width) : current.width,
      height: pos.height !== undefined ? Math.max(300, pos.height) : current.height,
    };
    try {
      localStorage.setItem(PANEL_FLOATING_POS_KEY, JSON.stringify(updated));
    } catch {
      // 忽略
    }
    set({ floatingPosition: updated });
  },

  setAlwaysTrustHighRisk: (trust: boolean) => {
    try {
      localStorage.setItem(ALWAYS_TRUST_HIGH_RISK_KEY, String(trust));
    } catch {
      // 忽略
    }
    set({ alwaysTrustHighRisk: trust });
    if (trust) {
      for (const [id, resolver] of pendingResolvers.entries()) {
        resolver(true);
        pendingResolvers.delete(id);
      }
      set((state) => ({
        pendingActions: state.pendingActions.map((act) =>
          act.status === 'pending' ? { ...act, status: 'approved' } : act,
        ),
      }));
    }
  },

  toggleAlwaysTrustHighRisk: () => {
    const next = !get().alwaysTrustHighRisk;
    get().setAlwaysTrustHighRisk(next);
  },


  setSessionWorkspace: (ws: AIWorkspaceContext | null) => {
    set({ activeWorkspace: ws });
    const currentId = get().currentSessionId;
    if (currentId) {
      const updatedSessions = get().sessions.map((s) =>
        s.id === currentId ? { ...s, workspace: ws ?? undefined } : s,
      );
      set({ sessions: updatedSessions });
      aiSessionStorage.loadSession(currentId).then((sess) => {
        if (sess) {
          sess.workspace = ws ?? undefined;
          sess.updatedAt = Date.now();
          aiSessionStorage.saveSession(sess);
        }
      });
    }
  },

  selectWorkspaceFolder: async () => {
    try {
      // 动态导入以支持 Web 浏览器模拟层与 Vitest 纯 node 测试环境
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: true,
        multiple: false,
      });
      if (!selected || typeof selected !== 'string') {
        return;
      }
      const cleanPath = selected.replace(/[/\\]+$/, '');
      const segments = cleanPath.split(/[/\\]/);
      const folderName = segments[segments.length - 1] || 'workspace';
      const ws: AIWorkspaceContext = {
        path: selected,
        name: folderName,
      };
      get().setSessionWorkspace(ws);
    } catch (err) {
      console.warn('Failed to open workspace folder dialog:', err);
    }
  },

  initSessions: async () => {
    try {
      const sessionMetas = await aiSessionStorage.listSessions();
      if (sessionMetas.length === 0) {
        // 如果没有历史会话，创建一个空白默认会话
        const newId = crypto.randomUUID();
        const initialSession: AISession = {
          id: newId,
          title: getLL().ai.copilot.new_session_title(),
          createdAt: Date.now(),
          updatedAt: Date.now(),
          messages: [],
        };
        await aiSessionStorage.saveSession(initialSession);
        const updatedList = await aiSessionStorage.listSessions();
        set({
          sessions: updatedList,
          currentSessionId: newId,
          activeWorkspace: null,
          messages: [],
        });
      } else {
        // 加载最新活跃的会话
        const activeId = get().currentSessionId || sessionMetas[0].id;
        const sessionData = await aiSessionStorage.loadSession(activeId);
        set({
          sessions: sessionMetas,
          currentSessionId: activeId,
          activeWorkspace: sessionData?.workspace ?? null,
          messages: sessionData ? sessionData.messages : [],
        });
      }
    } catch {
      // 失败时保底内存状态
    }
  },

  createNewSession: async (title?: string, initialWorkspace?: AIWorkspaceContext) => {
    get().stopGeneration();

    // 如果当前会话有消息，在创建新会话前保存当前会话进度
    const currentId = get().currentSessionId;
    if (currentId && get().messages.length > 0) {
      const currentMeta = get().sessions.find((s) => s.id === currentId);
      await aiSessionStorage.saveSession({
        id: currentId,
        title: currentMeta ? currentMeta.title : getLL().ai.copilot.new_session_title(),
        createdAt: currentMeta ? currentMeta.createdAt : Date.now(),
        updatedAt: Date.now(),
        messages: get().messages,
        workspace: get().activeWorkspace ?? undefined,
      });
    }

    const newId = crypto.randomUUID();
    const newSession: AISession = {
      id: newId,
      title: title || getLL().ai.copilot.new_session_title(),
      createdAt: Date.now(),
      updatedAt: Date.now(),
      messages: [],
      workspace: initialWorkspace,
    };

    await aiSessionStorage.saveSession(newSession);
    const sessionMetas = await aiSessionStorage.listSessions();

    set({
      sessions: sessionMetas,
      currentSessionId: newId,
      activeWorkspace: initialWorkspace ?? null,
      messages: [],
      error: null,
      pendingActions: [],
      agentStatus: 'idle',
    });

    return newId;
  },

  switchSession: async (sessionId: string) => {
    if (get().currentSessionId === sessionId) return;
    get().stopGeneration();

    // 1. 如果当前会话有消息，在切换离开前先行持久化当前会话
    const currentId = get().currentSessionId;
    if (currentId && get().messages.length > 0) {
      const currentMeta = get().sessions.find((s) => s.id === currentId);
      await aiSessionStorage.saveSession({
        id: currentId,
        title: currentMeta ? currentMeta.title : getLL().ai.copilot.new_session_title(),
        createdAt: currentMeta ? currentMeta.createdAt : Date.now(),
        updatedAt: Date.now(),
        messages: get().messages,
        workspace: get().activeWorkspace ?? undefined,
      });
    }

    // 2. 加载目标会话数据（若存储中缺失对应 sessionData，根据 metadata 兜底恢复）
    let sessionData = await aiSessionStorage.loadSession(sessionId);
    if (!sessionData) {
      const targetMeta = get().sessions.find((s) => s.id === sessionId);
      sessionData = {
        id: sessionId,
        title: targetMeta ? targetMeta.title : getLL().ai.copilot.new_session_title(),
        createdAt: targetMeta ? targetMeta.createdAt : Date.now(),
        updatedAt: targetMeta ? targetMeta.updatedAt : Date.now(),
        messages: [],
      };
      await aiSessionStorage.saveSession(sessionData);
    }

    const latestMetas = await aiSessionStorage.listSessions();

    // 3. 保证 currentSessionId 与 messages 一定切换成功
    set({
      sessions: latestMetas.length > 0 ? latestMetas : get().sessions,
      currentSessionId: sessionId,
      activeWorkspace: sessionData.workspace ?? null,
      messages: sessionData.messages || [],
      error: null,
      pendingActions: [],
      agentStatus: 'idle',
    });
  },

  renameSession: async (sessionId: string, newTitle: string) => {
    const sessionData = await aiSessionStorage.loadSession(sessionId);
    if (sessionData) {
      sessionData.title = newTitle.trim() || getLL().ai.copilot.unnamed_session();
      sessionData.updatedAt = Date.now();
      await aiSessionStorage.saveSession(sessionData);
      const sessionMetas = await aiSessionStorage.listSessions();
      set({ sessions: sessionMetas });
    }
  },

  deleteSession: async (sessionId: string) => {
    await aiSessionStorage.deleteSession(sessionId);
    const remaining = await aiSessionStorage.listSessions();

    if (get().currentSessionId === sessionId) {
      if (remaining.length > 0) {
        const nextSession = await aiSessionStorage.loadSession(remaining[0].id);
        set({
          sessions: remaining,
          currentSessionId: remaining[0].id,
          activeWorkspace: nextSession?.workspace ?? null,
          messages: nextSession ? nextSession.messages : [],
        });
      } else {
        await get().createNewSession();
      }
      set({ sessions: remaining });
    }
  },

  clearMessages: () => {
    get().stopGeneration();
    const currentId = get().currentSessionId;
    set({ messages: [], error: null, pendingActions: [], agentStatus: 'idle' });

    if (currentId) {
      aiSessionStorage.loadSession(currentId).then((sess) => {
        if (sess) {
          sess.messages = [];
          sess.updatedAt = Date.now();
          aiSessionStorage.saveSession(sess);
        }
      });
    }
  },

  compactHistory: async () => {
    if (get().isCompacting) return false;
    const isGenerating =
      get().agentStatus === 'thinking' ||
      get().agentStatus === 'executing_tools' ||
      get().agentStatus === 'waiting_confirmation';
    if (isGenerating) {
      toast.info(getLL().ai.copilot.compact_blocked_generating());
      return false;
    }

    const targetSessionId = get().currentSessionId;
    if (!targetSessionId) return false;

    const currentMessages = get().messages;
    // 提取所有未被归档的非系统消息
    const unarchivedMessages = currentMessages.filter(
      (m) => !m.isArchived && m.role !== 'system' && m.status !== 'error',
    );

    // 会话内容过少暂无需压缩
    if (unarchivedMessages.length <= 2) {
      toast.info(getLL().ai.copilot.compact_no_need());
      return false;
    }

    const settings = useSettingsStore.getState();
    const activeProvider = settings.aiProviders.find(
      (p) => p.id === settings.activeAiProviderId,
    );

    if (!activeProvider) {
      toast.error(getLL().ai.copilot.no_active_provider());
      return false;
    }

    set({ isCompacting: true });
    const toastId = toast.loading(getLL().ai.copilot.compact_running());

    try {
      const systemPrompt: AIMessage = {
        id: `sys_compact_${Date.now()}`,
        role: 'system',
        content: buildCompactionSystemPrompt(),
        timestamp: Date.now(),
      };

      const userPrompt: AIMessage = {
        id: `user_compact_${Date.now()}`,
        role: 'user',
        content: buildCompactionUserPrompt(unarchivedMessages),
        timestamp: Date.now(),
      };

      const result = await streamChatCompletion(
        activeProvider,
        [systemPrompt, userPrompt],
        undefined,
        undefined,
      );

      const summaryContent = result.content?.trim();
      if (!summaryContent) {
        throw new Error(getLL().ai.copilot.compact_empty_error());
      }

      const originalTokenEst = estimateTokens(unarchivedMessages);

      const summaryMessage: AIMessage = {
        id: `msg_${Date.now()}_summary`,
        role: 'assistant',
        content: summaryContent,
        isSummary: true,
        compactMeta: {
          compressedMessageCount: unarchivedMessages.length,
          originalTokenEstimate: originalTokenEst,
          timestamp: Date.now(),
        },
        timestamp: Date.now(),
        status: 'done',
      };

      const unarchivedIds = new Set(unarchivedMessages.map((m) => m.id));
      const isStillOnTargetSession = get().currentSessionId === targetSessionId;

      if (isStillOnTargetSession) {
        const updatedMessages: AIMessage[] = [];
        for (const m of get().messages) {
          if (unarchivedIds.has(m.id)) {
            updatedMessages.push({ ...m, isArchived: true });
          } else {
            updatedMessages.push(m);
          }
        }
        updatedMessages.push(summaryMessage);

        set({
          messages: updatedMessages,
          isCompacting: false,
        });

        // 持久化存储
        const currentMeta = get().sessions.find((s) => s.id === targetSessionId);
        const sessionObj: AISession = {
          id: targetSessionId,
          title: currentMeta ? currentMeta.title : getLL().ai.copilot.new_session_title(),
          createdAt: currentMeta ? currentMeta.createdAt : Date.now(),
          updatedAt: Date.now(),
          workspace: get().activeWorkspace ?? undefined,
          messages: updatedMessages,
        };
        await aiSessionStorage.saveSession(sessionObj);
        const updatedList = await aiSessionStorage.listSessions();
        set({ sessions: updatedList });
      } else {
        // 用户中途切换到了其他会话：仅持久化目标会话数据，不污染当前活跃会话状态
        const targetSessionData = await aiSessionStorage.loadSession(targetSessionId);
        if (targetSessionData) {
          const updatedMessages: AIMessage[] = [];
          for (const m of targetSessionData.messages || []) {
            if (unarchivedIds.has(m.id)) {
              updatedMessages.push({ ...m, isArchived: true });
            } else {
              updatedMessages.push(m);
            }
          }
          updatedMessages.push(summaryMessage);
          targetSessionData.messages = updatedMessages;
          targetSessionData.updatedAt = Date.now();
          await aiSessionStorage.saveSession(targetSessionData);
          const updatedList = await aiSessionStorage.listSessions();
          set({ sessions: updatedList, isCompacting: false });
        } else {
          set({ isCompacting: false });
        }
      }
      toast.success(getLL().ai.copilot.compact_success(), { id: toastId });
      return true;
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : String(err),
        { id: toastId },
      );
      set({ isCompacting: false });
      return false;
    }
  },

  stopGeneration: () => {
    if (sessionAbortController) {
      sessionAbortController.abort();
      sessionAbortController = null;
    }
    if (currentIterationAbortController) {
      currentIterationAbortController.abort();
      currentIterationAbortController = null;
    }
    for (const resolver of pendingResolvers.values()) {
      resolver(false);
    }
    pendingResolvers.clear();

    set((s) => ({
      agentStatus: 'idle',
      activeStreamingMessageId: null,
      messages: s.messages.filter(
        (m) =>
          !(
            m.status === 'streaming' &&
            !m.content?.trim() &&
            !m.thought?.trim() &&
            (!m.tool_calls || m.tool_calls.length === 0)
          ),
      ),
    }));
  },

  steerGeneration: async (prompt: string, images?: string[]) => {
    const trimmedPrompt = prompt.trim();
    if (!trimmedPrompt && (!images || images.length === 0)) return;

    // 挂起确认的高风险操作标记为被纠偏取消
    for (const resolver of pendingResolvers.values()) {
      resolver(false);
    }
    pendingResolvers.clear();

    const userMessage: AIMessage = {
      id: `msg_${Date.now()}_user_steer`,
      role: 'user',
      content: trimmedPrompt,
      isCorrection: true,
      images,
      timestamp: Date.now(),
    };
    set((s) => ({
      messages: [...s.messages, userMessage],
      pendingActions: s.pendingActions.map((act) =>
        act.status === 'pending' ? { ...act, status: 'rejected' } : act,
      ),
      agentStatus: 'thinking',
    }));

    const sessId = get().currentSessionId;
    if (sessId) {
      const currentMeta = get().sessions.find((s) => s.id === sessId);
      const sessionObj: AISession = {
        id: sessId,
        title: currentMeta ? currentMeta.title : getLL().ai.copilot.new_session_title(),
        createdAt: currentMeta ? currentMeta.createdAt : Date.now(),
        updatedAt: Date.now(),
        workspace: get().activeWorkspace ?? undefined,
        messages: get().messages.filter((m) => {
          if (m.role === 'assistant') {
            const hasContent = Boolean(m.content && m.content.trim().length > 0);
            const hasThought = Boolean(m.thought && m.thought.trim().length > 0);
            const hasToolCalls = Boolean(m.tool_calls && m.tool_calls.length > 0);
            return hasContent || hasThought || hasToolCalls;
          }
          return true;
        }),
      };
      await aiSessionStorage.saveSession(sessionObj);
      const updatedList = await aiSessionStorage.listSessions();
      set({ sessions: updatedList });
    }

    // 中断当前正在思考或流式输出的单轮，让 ReAct 循环带着新纠偏指令进入下一轮
    if (currentIterationAbortController) {
      currentIterationAbortController.abort();
      currentIterationAbortController = null;
    }
  },

  resolvePendingAction: async (actionId: string, approved: boolean, trustAll = false) => {
    if (trustAll) {
      try {
        localStorage.setItem(ALWAYS_TRUST_HIGH_RISK_KEY, 'true');
      } catch {
        // 忽略
      }
      set({ alwaysTrustHighRisk: true });
    }

    const resolver = pendingResolvers.get(actionId);
    if (resolver) {
      resolver(approved);
      pendingResolvers.delete(actionId);
    }

    if (trustAll && approved) {
      for (const [id, otherResolver] of pendingResolvers.entries()) {
        otherResolver(true);
        pendingResolvers.delete(id);
      }
      set((state) => ({
        pendingActions: state.pendingActions.map((act) =>
          act.status === 'pending' ? { ...act, status: 'approved' } : act,
        ),
      }));
    } else {
      set((state) => ({
        pendingActions: state.pendingActions.map((act) =>
          act.id === actionId
            ? { ...act, status: approved ? 'approved' : 'rejected' }
            : act,
        ),
      }));
    }
  },

  sendMessage: async (prompt: string, images?: string[]) => {
    const trimmedPrompt = prompt.trim();
    if (!trimmedPrompt && (!images || images.length === 0)) return;

    // 拦截并执行斜杠指令 (如 /compact, /clear, /diagnose, /review, /help)
    if (trimmedPrompt.startsWith('/') && (!images || images.length === 0)) {
      const handled = await executeSlashCommand(trimmedPrompt);
      if (handled) return;
    }

    // 正在压缩上下文时阻断新消息提交，防止并发状态冲突
    if (get().isCompacting) {
      toast.info(getLL().ai.copilot.compact_running());
      return;
    }

    const isGenerating =
      get().agentStatus === 'thinking' ||
      get().agentStatus === 'executing_tools' ||
      get().agentStatus === 'waiting_confirmation';
    // 若当前 Agent 正在生成中，直接转入中途插话纠偏逻辑，不重置对话与上下文！
    if (isGenerating && (!sessionAbortController || !sessionAbortController.signal.aborted)) {
      await get().steerGeneration(prompt, images);
      return;
    }
    const settings = useSettingsStore.getState();
    const activeProvider = settings.aiProviders.find(
      (p) => p.id === settings.activeAiProviderId,
    );

    let activeSessionId = get().currentSessionId;
    if (!activeSessionId) {
      activeSessionId = await get().createNewSession();
    }

    if (!activeProvider) {
      const errorMsg =
        'No active AI Provider configured. Please configure an OpenAI-compatible provider in Settings.';
      set((s) => ({
        messages: [
          ...s.messages,
          {
            id: `msg_${Date.now()}_user`,
            role: 'user',
            content: trimmedPrompt,
            images,
            timestamp: Date.now(),
          },
          {
            id: `msg_${Date.now()}_error`,
            role: 'assistant',
            content: errorMsg,
            status: 'error',
            error: errorMsg,
            timestamp: Date.now(),
          },
        ],
        error: errorMsg,
      }));
      return;
    }

    get().stopGeneration();

    sessionAbortController = new AbortController();
    const sessionSignal = sessionAbortController.signal;

    // 1. 添加 User 消息
    const userMessage: AIMessage = {
      id: `msg_${Date.now()}_user`,
      role: 'user',
      content: trimmedPrompt,
      images,
      timestamp: Date.now(),
    };

    const isFirstMessage = get().messages.length === 0;

    set((s) => ({
      messages: [...s.messages, userMessage],
      error: null,
      agentStatus: 'thinking',
    }));

    // 首条消息自动根据提示词重命名会话标题
    if (isFirstMessage && trimmedPrompt) {
      const autoTitle = trimmedPrompt.slice(0, 24).trim();
      get().renameSession(activeSessionId, autoTitle);
    }

    // 异步保存会话数据
    const persistCurrentSession = async () => {
      const sessId = get().currentSessionId;
      if (!sessId) return;
      const currentMeta = get().sessions.find((s) => s.id === sessId);
      const sessionObj: AISession = {
        id: sessId,
        title: currentMeta ? currentMeta.title : getLL().ai.copilot.new_session_title(),
        createdAt: currentMeta ? currentMeta.createdAt : Date.now(),
        updatedAt: Date.now(),
        workspace: get().activeWorkspace ?? undefined,
        messages: get().messages.filter((m) => {
          if (m.role === 'assistant') {
            const hasContent = Boolean(m.content && m.content.trim().length > 0);
            const hasThought = Boolean(m.thought && m.thought.trim().length > 0);
            const hasToolCalls = Boolean(m.tool_calls && m.tool_calls.length > 0);
            return hasContent || hasThought || hasToolCalls;
          }
          return true;
        }),
      };
      await aiSessionStorage.saveSession(sessionObj);
      const updatedList = await aiSessionStorage.listSessions();
      set({ sessions: updatedList });
    };

    await persistCurrentSession();

    const tools = getAllAIToolDefinitions();
    const MAX_REACT_ITERATIONS = 20;
    let iteration = 0;

    try {
      while (iteration < MAX_REACT_ITERATIONS) {
        if (sessionSignal.aborted) break;
        iteration++;

        currentIterationAbortController = new AbortController();
        const iterationSignal = currentIterationAbortController.signal;

        // 构建最新的全局 System Prompt + 运行时动态快照
        const systemPromptText = `${buildSystemPrompt(settings.aiCustomSystemPrompt)}\n\n${buildActiveStateSummary()}`;
        const systemMessage: AIMessage = {
          id: `sys_${Date.now()}`,
          role: 'system',
          content: systemPromptText,
          timestamp: Date.now(),
        };

        const currentHistory = get().messages;
        const steerPrefix = getLL().ai.copilot.steer_prefix();
        const formattedHistory: AIMessage[] = pruneMessagesForSending(
          currentHistory,
          steerPrefix,
        );
        const messagesToSend: AIMessage[] = [systemMessage, ...formattedHistory];

        // 占位 Assistant 消息
        const assistantMsgId = `msg_${Date.now()}_assistant_${iteration}`;
        const placeholderAssistantMsg: AIMessage = {
          id: assistantMsgId,
          role: 'assistant',
          content: '',
          status: 'streaming',
          timestamp: Date.now(),
        };

        set((s) => ({
          messages: [...s.messages, placeholderAssistantMsg],
          activeStreamingMessageId: assistantMsgId,
          agentStatus: 'thinking',
        }));

        let result: ChatCompletionResult | null = null;
        let iterationInterrupted = false;

        // 流式批处理缓冲器：聚合高频 chunk（尤其是上万字深度推理过程），避免主线程任务风暴与频繁同步重排
        let pendingContent = '';
        let pendingThought = '';
        let pendingToolCalls: AIToolCall[] | null = null;
        let streamFlushTimer: number | NodeJS.Timeout | null = null;

        const flushStreamBuffer = () => {
          if (streamFlushTimer) {
            clearTimeout(streamFlushTimer);
            streamFlushTimer = null;
          }
          if (!pendingContent && !pendingThought && !pendingToolCalls) {
            return;
          }
          const addContent = pendingContent;
          const addThought = pendingThought;
          const newToolCalls = pendingToolCalls;
          pendingContent = '';
          pendingThought = '';
          pendingToolCalls = null;

          set((s) => ({
            messages: s.messages.map((m) => {
              if (m.id !== assistantMsgId) return m;
              return {
                ...m,
                content: addContent ? m.content + addContent : m.content,
                thought: addThought ? (m.thought || '') + addThought : m.thought,
                tool_calls: newToolCalls ?? m.tool_calls,
              };
            }),
          }));
        };

        const scheduleStreamFlush = () => {
          if (!streamFlushTimer) {
            streamFlushTimer = setTimeout(() => {
              flushStreamBuffer();
            }, 50); // ~20fps 批处理频率，兼顾平滑视觉流出与主线程响应性
          }
        };

        try {
          result = await streamChatCompletion(
            activeProvider,
            messagesToSend,
            tools,
            {
              onContentChunk: (chunk) => {
                pendingContent += chunk;
                scheduleStreamFlush();
              },
              onThoughtChunk: (chunk) => {
                pendingThought += chunk;
                scheduleStreamFlush();
              },
              onToolCallsUpdate: (toolCalls) => {
                pendingToolCalls = toolCalls;
                scheduleStreamFlush();
              },
            },
            iterationSignal,
          );
        } catch (err: unknown) {
          if (sessionSignal.aborted) {
            break;
          }
          if (iterationSignal.aborted) {
            iterationInterrupted = true;
          } else {
            throw err;
          }
        } finally {
          flushStreamBuffer();
          currentIterationAbortController = null;
        }

        if (sessionSignal.aborted) break;

        // 如果单轮被用户中途插话打断，保留已有内容或清理空消息，并立即进入下一轮思考新指令
        if (iterationInterrupted || !result) {
          set((s) => {
            const existing = s.messages.find((m) => m.id === assistantMsgId);
            if (
              !existing ||
              (!existing.content?.trim() &&
                !existing.thought?.trim() &&
                (!existing.tool_calls || existing.tool_calls.length === 0))
            ) {
              return {
                messages: s.messages.filter((m) => m.id !== assistantMsgId),
                activeStreamingMessageId: null,
              };
            }
            return {
              messages: s.messages.map((m) =>
                m.id === assistantMsgId ? { ...m, status: 'done' as const } : m,
              ),
              activeStreamingMessageId: null,
            };
          });
          await persistCurrentSession();
          continue;
        }
        // 固化 Assistant 消息 (如果完全无内容无工具，则清除占位消息，避免多出空白头像)
        const hasContent = Boolean(result.content && result.content.trim().length > 0);
        const hasThought = Boolean(result.thought && result.thought.trim().length > 0);
        const hasToolCalls = Boolean(result.tool_calls && result.tool_calls.length > 0);

        if (!hasContent && !hasThought && !hasToolCalls) {
          set((s) => ({
            messages: s.messages.filter((m) => m.id !== assistantMsgId),
            activeStreamingMessageId: null,
          }));
        } else {
          set((s) => ({
            messages: s.messages.map((m) =>
              m.id === assistantMsgId
                ? {
                    ...m,
                    content: result.content,
                    thought: result.thought,
                    tool_calls: result.tool_calls,
                    status: 'done',
                  }
                : m,
            ),
            activeStreamingMessageId: null,
          }));
        }
        await persistCurrentSession();

        // 如果没有触发 Tool Calls，任务已自然完成或模型向用户做出说明/提问
        if (!result.tool_calls || result.tool_calls.length === 0) {
          set({ agentStatus: 'idle' });
          break;
        }

        // 存在 Tool Calls，开始执行工具
        set({ agentStatus: 'executing_tools' });

        for (const tc of result.tool_calls) {
          if (sessionSignal.aborted) break;
          let parsedArgs: Record<string, unknown> = {};
          try {
            parsedArgs = tc.function.arguments
              ? JSON.parse(tc.function.arguments)
              : {};
          } catch {
            parsedArgs = {};
          }

          let toolOutputStr = '';

          // 检查是否为高风险操作
          const isHighRisk = isHighRiskTool(tc.function.name, parsedArgs);
          const needsConfirmation = isHighRisk && !get().alwaysTrustHighRisk;

          if (needsConfirmation) {
            const pendingAction = createPendingAction(tc, parsedArgs);

            set((s) => ({
              pendingActions: [...s.pendingActions, pendingAction],
              agentStatus: 'waiting_confirmation',
            }));

            // 挂起等待用户点击确认
            const userApproved = await new Promise<boolean>((resolve) => {
              pendingResolvers.set(pendingAction.id, resolve);
            });

            if (userApproved) {
              set({ agentStatus: 'executing_tools' });
              try {
                const toolResult = await dispatchToolCall(
                  tc.function.name,
                  parsedArgs,
                );
                toolOutputStr = JSON.stringify(toolResult);
              } catch (err: unknown) {
                toolOutputStr = JSON.stringify({
                  success: false,
                  error: err instanceof Error ? err.message : String(err),
                });
              }
            } else {
              toolOutputStr = JSON.stringify({
                success: false,
                cancelled: true,
                message: 'User rejected the operation.',
              });
            }
          } else {
            // 普通安全工具，或开启完全信任模式下的高风险操作直接执行
            if (isHighRisk) {
              const pendingAction: AIPendingAction = {
                ...createPendingAction(tc, parsedArgs),
                status: 'approved',
              };
              set((s) => ({
                pendingActions: [...s.pendingActions, pendingAction],
              }));
            }
            try {
              const toolResult = await dispatchToolCall(
                tc.function.name,
                parsedArgs,
              );
              toolOutputStr = JSON.stringify(toolResult);
            } catch (err: unknown) {
              toolOutputStr = JSON.stringify({
                success: false,
                error: err instanceof Error ? err.message : String(err),
              });
            }
          }

          // 添加 Tool 结果消息
          const toolResultMessage: AIMessage = {
            id: `msg_${Date.now()}_tool_${tc.id}`,
            role: 'tool',
            tool_call_id: tc.id,
            name: tc.function.name,
            content: toolOutputStr,
            timestamp: Date.now(),
          };

          set((s) => ({
            messages: [
              ...s.messages.map((m) =>
                m.id === assistantMsgId && m.tool_calls
                  ? {
                      ...m,
                      tool_calls: m.tool_calls.map((c) =>
                        c.id === tc.id ? { ...c, output: toolOutputStr } : c,
                      ),
                    }
                  : m,
              ),
              toolResultMessage,
            ],
          }));

          await persistCurrentSession();
        }

        // 继续下一轮 ReAct 循环，让 LLM 观察工具执行结果
      }
    } catch (err: unknown) {
      if (!sessionSignal.aborted) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        set((s) => ({
          error: errorMsg,
          agentStatus: 'error',
          messages: [
            ...s.messages
              .filter(
                (m) =>
                  !(
                    m.status === 'streaming' &&
                    !m.content?.trim() &&
                    !m.thought?.trim() &&
                    (!m.tool_calls || m.tool_calls.length === 0)
                  ),
              )
              .map((m) => (m.status === 'streaming' ? { ...m, status: 'done' as const } : m)),
            {
              id: `msg_${Date.now()}_error`,
              role: 'assistant',
              content: errorMsg,
              status: 'error',
              error: errorMsg,
              timestamp: Date.now(),
            },
          ],
        }));
        await persistCurrentSession();
      }
    } finally {
      sessionAbortController = null;
      currentIterationAbortController = null;
      if (get().agentStatus !== 'waiting_confirmation') {
        set({ agentStatus: 'idle', activeStreamingMessageId: null });
      }
    }
  },
  retryLastMessage: async () => {
    const isGenerating =
      get().agentStatus === 'thinking' ||
      get().agentStatus === 'executing_tools' ||
      get().agentStatus === 'waiting_confirmation';
    if (isGenerating) return;

    const messages = get().messages;
    let lastUserIdx = -1;
    for (let i = messages.length - 1; i >= 0; i--) {
      if (messages[i].role === 'user') {
        lastUserIdx = i;
        break;
      }
    }
    if (lastUserIdx === -1) return;
    const lastUserMsg = messages[lastUserIdx];
    // 移除最后一条用户消息及其后续所有消息（包含报错消息）
    const cleaned = messages.slice(0, lastUserIdx);
    set({ messages: cleaned, error: null });

    // 重新触发发送
    await get().sendMessage(lastUserMsg.content, lastUserMsg.images);
  },

  reset: () => {
    get().stopGeneration();
    set({
      ...initialState,
      isFloating: getStoredFloating(),
      floatingPosition: getStoredFloatingPos(),
      alwaysTrustHighRisk: getStoredAlwaysTrustHighRisk(),
    });
  },
}));
