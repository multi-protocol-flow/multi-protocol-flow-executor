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
import { streamChatCompletion, type ChatCompletionResult } from '@/services/ai/openaiClient';
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
const PANEL_FLOATING_KEY = 'mpe-ai-panel-floating';
const PANEL_FLOATING_POS_KEY = 'mpe-ai-panel-floating-pos';
const AUTO_EXECUTE_KEY = 'mpe-ai-auto-execute-to-goal';

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
        return parsed;
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

function getStoredAutoExecute(): boolean {
  try {
    const val = localStorage.getItem(AUTO_EXECUTE_KEY);
    return val === null ? true : val === 'true';
  } catch {
    return true;
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
  /** 设置是否开启自主执行到底模式 */
  setAutoExecuteToGoal: (auto: boolean) => void;
  /** 切换自主执行到底模式 */
  toggleAutoExecuteToGoal: () => void;
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
  /** 发送中途纠偏/补充指令 (正在生成时插话纠正，无需中断整个任务) */
  steerGeneration: (prompt: string, images?: string[]) => Promise<void>;
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
  isFloating: getStoredFloating(),
  floatingPosition: getStoredFloatingPos(),
  autoExecuteToGoal: getStoredAutoExecute(),
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
      x: pos.x !== undefined ? pos.x : current.x,
      y: pos.y !== undefined ? pos.y : current.y,
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

  setAutoExecuteToGoal: (auto: boolean) => {
    try {
      localStorage.setItem(AUTO_EXECUTE_KEY, String(auto));
    } catch {
      // 忽略
    }
    set({ autoExecuteToGoal: auto });
  },

  toggleAutoExecuteToGoal: () => {
    const next = !get().autoExecuteToGoal;
    try {
      localStorage.setItem(AUTO_EXECUTE_KEY, String(next));
    } catch {
      // 忽略
    }
    set({ autoExecuteToGoal: next });
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

    const autoExecute = get().autoExecuteToGoal;
    const tools = getAllAIToolDefinitions();
    const MAX_REACT_ITERATIONS = autoExecute ? 30 : 12;
    let iteration = 0;

    const ACTION_KEYWORDS_REGEX = /(创建|新建|添加|生成|构建|排版|布局|执行|运行|测试|调试|修复|修改|更新|删除|配置|连线|连接|清理|清空|编排|流程|节点|create|build|add|generate|layout|run|execute|test|debug|fix|update|modify|delete|clear|connect|pipeline|flow)/i;
    const isActionPrompt = ACTION_KEYWORDS_REGEX.test(trimmedPrompt);

    try {
      while (iteration < MAX_REACT_ITERATIONS) {
        if (sessionSignal.aborted) break;
        iteration++;

        currentIterationAbortController = new AbortController();
        const iterationSignal = currentIterationAbortController.signal;

        // 构建最新的全局 System Prompt + 运行时动态快照
        const systemPromptText = `${buildSystemPrompt(settings.aiCustomSystemPrompt, autoExecute)}\n\n${buildActiveStateSummary()}`;
        const systemMessage: AIMessage = {
          id: `sys_${Date.now()}`,
          role: 'system',
          content: systemPromptText,
          timestamp: Date.now(),
        };

        const currentHistory = get().messages;
        const steerPrefix = getLL().ai.copilot.steer_prefix();
        const formattedHistory: AIMessage[] = currentHistory.map((m) => {
          if (m.role === 'user' && m.isCorrection) {
            return {
              ...m,
              content: `${steerPrefix} ${m.content}`,
            };
          }
          return m;
        });
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

        try {
          result = await streamChatCompletion(
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

        // 如果没有触发 Tool Calls
        if (!result.tool_calls || result.tool_calls.length === 0) {
          // 如果开启了"执行到底"模式，且首轮用户明确要求执行操作，而模型仅返回了文本规划而未调用工具：
          if (autoExecute && isActionPrompt && iteration === 1) {
            const nudgeMessage: AIMessage = {
              id: `msg_${Date.now()}_system_nudge`,
              role: 'user',
              content:
                '[System Directive: "执行到底"模式已开启。检测到你仅输出了方案说明，尚未实际调用工具。请直接调用相关工具（如 batch_create_pipeline, add_node, connect_nodes, auto_layout, execute_flow 等）将该任务执行到底，无需等待二次确认。]',
              timestamp: Date.now(),
            };
            set((s) => ({
              messages: [...s.messages, nudgeMessage],
            }));
            await persistCurrentSession();
            continue;
          }

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
      if (!sessionSignal.aborted) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        set((s) => ({
          error: errorMsg,
          agentStatus: 'error',
          messages: [
            ...s.messages.filter(
              (m) =>
                !(
                  m.status === 'streaming' &&
                  !m.content?.trim() &&
                  !m.thought?.trim() &&
                  (!m.tool_calls || m.tool_calls.length === 0)
                ),
            ),
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
      sessionAbortController = null;
      currentIterationAbortController = null;
      if (get().agentStatus !== 'waiting_confirmation') {
        set({ agentStatus: 'idle', activeStreamingMessageId: null });
      }
    }
  },

  reset: () => {
    get().stopGeneration();
    set({
      ...initialState,
      isFloating: getStoredFloating(),
      floatingPosition: getStoredFloatingPos(),
      autoExecuteToGoal: getStoredAutoExecute(),
    });
  },
}));
