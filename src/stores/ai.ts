/**
 * AI Copilot & ReAct Agent 状态管理
 */

import { create } from 'zustand';
import type {
  AIState,
  AIMessage,
  AISession,
} from '@/types/ai';
import { getLL } from '@/i18n/i18n-lazy';
import { useSettingsStore } from './settings';
import { aiSessionStorage } from '@/services/storage/aiSessionStorage';
import { streamChatCompletion } from '@/services/ai/openaiClient';
import {
  buildSystemPrompt,
  buildActiveStateSummary,
} from '@/services/ai/contextBuilder';
import {
  getAllAIToolDefinitions,
  isHighRiskTool,
  createPendingAction,
  dispatchToolCall,
} from '@/services/ai/toolDispatcher';

const PANEL_WIDTH_KEY = 'mpe-ai-panel-width';
const DEFAULT_PANEL_WIDTH = 380;

function getStoredPanelWidth(): number {
  try {
    const val = localStorage.getItem(PANEL_WIDTH_KEY);
    return val ? Math.max(300, Math.min(700, Number(val))) : DEFAULT_PANEL_WIDTH;
  } catch {
    return DEFAULT_PANEL_WIDTH;
  }
}

// 用于控制中止请求的 AbortController
let currentAbortController: AbortController | null = null;

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

  /** 清空当前会话的消息历史 */
  clearMessages: () => void;
  /** 中止当前 Agent 生成与工具执行 */
  stopGeneration: () => void;
  /** 发送用户消息并触发 ReAct Agent 循环 (支持附加图片) */
  sendMessage: (prompt: string, images?: string[]) => Promise<void>;
  /** 响应高风险操作确认或拒绝 */
  resolvePendingAction: (actionId: string, approved: boolean) => Promise<void>;
  /** 重置（测试隔离） */
  reset: () => void;
}

export type AIStore = AIState & AIActions;

const initialState: AIState = {
  currentSessionId: null,
  sessions: [],
  messages: [],
  agentStatus: 'idle',
  activeStreamingMessageId: null,
  pendingActions: [],
  isCopilotOpen: false,
  isSpotlightOpen: false,
  panelWidth: getStoredPanelWidth(),
  error: null,
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
          messages: [],
        });
      } else {
        // 加载最新活跃的会话
        const activeId = get().currentSessionId || sessionMetas[0].id;
        const sessionData = await aiSessionStorage.loadSession(activeId);
        set({
          sessions: sessionMetas,
          currentSessionId: activeId,
          messages: sessionData ? sessionData.messages : [],
        });
      }
    } catch {
      // 失败时保底内存状态
    }
  },

  createNewSession: async (title?: string) => {
    get().stopGeneration();

    const newId = crypto.randomUUID();
    const newSession: AISession = {
      id: newId,
      title: title || getLL().ai.copilot.new_session_title(),
      createdAt: Date.now(),
      updatedAt: Date.now(),
      messages: [],
    };

    await aiSessionStorage.saveSession(newSession);
    const sessionMetas = await aiSessionStorage.listSessions();

    set({
      sessions: sessionMetas,
      currentSessionId: newId,
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

    const sessionData = await aiSessionStorage.loadSession(sessionId);
    if (sessionData) {
      set({
        currentSessionId: sessionId,
        messages: sessionData.messages,
        error: null,
        pendingActions: [],
        agentStatus: 'idle',
      });
    }
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
          messages: nextSession ? nextSession.messages : [],
        });
      } else {
        await get().createNewSession();
      }
    } else {
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

  stopGeneration: () => {
    if (currentAbortController) {
      currentAbortController.abort();
      currentAbortController = null;
    }
    for (const resolver of pendingResolvers.values()) {
      resolver(false);
    }
    pendingResolvers.clear();

    set({
      agentStatus: 'idle',
      activeStreamingMessageId: null,
    });
  },

  resolvePendingAction: async (actionId: string, approved: boolean) => {
    const resolver = pendingResolvers.get(actionId);
    if (resolver) {
      resolver(approved);
      pendingResolvers.delete(actionId);
    }

    set((state) => ({
      pendingActions: state.pendingActions.map((act) =>
        act.id === actionId
          ? { ...act, status: approved ? 'approved' : 'rejected' }
          : act,
      ),
    }));
  },

  sendMessage: async (prompt: string, images?: string[]) => {
    const trimmedPrompt = prompt.trim();
    if (!trimmedPrompt && (!images || images.length === 0)) return;

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

    currentAbortController = new AbortController();
    const abortSignal = currentAbortController.signal;

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
        messages: get().messages,
      };
      await aiSessionStorage.saveSession(sessionObj);
      const updatedList = await aiSessionStorage.listSessions();
      set({ sessions: updatedList });
    };

    await persistCurrentSession();

    const tools = getAllAIToolDefinitions();
    const MAX_REACT_ITERATIONS = 10;
    let iteration = 0;

    try {
      while (iteration < MAX_REACT_ITERATIONS) {
        if (abortSignal.aborted) break;
        iteration++;

        // 构建最新的全局 System Prompt + 运行时动态快照
        const systemPromptText = `${buildSystemPrompt(settings.aiCustomSystemPrompt)}\n\n${buildActiveStateSummary()}`;
        const systemMessage: AIMessage = {
          id: `sys_${Date.now()}`,
          role: 'system',
          content: systemPromptText,
          timestamp: Date.now(),
        };

        const currentHistory = get().messages;
        const messagesToSend: AIMessage[] = [systemMessage, ...currentHistory];

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

        const result = await streamChatCompletion(
          activeProvider,
          messagesToSend,
          tools,
          {
            onContentChunk: (chunk) => {
              set((s) => ({
                messages: s.messages.map((m) =>
                  m.id === assistantMsgId
                    ? { ...m, content: m.content + chunk }
                    : m,
                ),
              }));
            },
            onThoughtChunk: (chunk) => {
              set((s) => ({
                messages: s.messages.map((m) =>
                  m.id === assistantMsgId
                    ? { ...m, thought: (m.thought || '') + chunk }
                    : m,
                ),
              }));
            },
            onToolCallsUpdate: (toolCalls) => {
              set((s) => ({
                messages: s.messages.map((m) =>
                  m.id === assistantMsgId ? { ...m, tool_calls: toolCalls } : m,
                ),
              }));
            },
          },
          abortSignal,
        );

        // 固化 Assistant 消息
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

        await persistCurrentSession();

        // 如果没有触发 Tool Calls，则 ReAct 循环结束
        if (!result.tool_calls || result.tool_calls.length === 0) {
          set({ agentStatus: 'idle' });
          break;
        }

        // 存在 Tool Calls，开始执行工具
        set({ agentStatus: 'executing_tools' });

        for (const tc of result.tool_calls) {
          if (abortSignal.aborted) break;

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
          if (isHighRiskTool(tc.function.name)) {
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
            // 普通安全工具，直接执行
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
            messages: [...s.messages, toolResultMessage],
          }));

          await persistCurrentSession();
        }

        // 继续下一轮 ReAct 循环，让 LLM 观察工具执行结果
      }
    } catch (err: unknown) {
      if (!abortSignal.aborted) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        set((s) => ({
          error: errorMsg,
          agentStatus: 'error',
          messages: [
            ...s.messages,
            {
              id: `msg_${Date.now()}_error`,
              role: 'assistant',
              content: `Error: ${errorMsg}`,
              status: 'error',
              error: errorMsg,
              timestamp: Date.now(),
            },
          ],
        }));
        await persistCurrentSession();
      }
    } finally {
      currentAbortController = null;
      if (get().agentStatus !== 'waiting_confirmation') {
        set({ agentStatus: 'idle', activeStreamingMessageId: null });
      }
    }
  },

  reset: () => {
    get().stopGeneration();
    set(initialState);
  },
}));
