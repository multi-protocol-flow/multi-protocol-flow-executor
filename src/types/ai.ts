/**
 * AI 助手核心类型定义
 */

// ============================================================================
// Provider 配置
// ============================================================================

/**
 * AI Provider 配置 (OpenAI 兼容协议)
 */
export interface AIProviderConfig {
  /** 唯一标识 UUID */
  id: string;
  /** 显示名称 (如: DeepSeek Official, Local Ollama, OpenAI) */
  name: string;
  /** API 基础端点 (如: https://api.deepseek.com/v1 或 http://localhost:11434/v1) */
  baseUrl: string;
  /** API 密钥 */
  apiKey: string;
  /** 模型名称 (如: deepseek-chat, gpt-4o, qwen2.5) */
  model: string;
  /** 生成温度 (0.0 ~ 2.0，默认 0.7) */
  temperature?: number;
  /** 最大生成 Token 数 */
  maxTokens?: number;
  /** 自定义请求头 (JSON 键值对) */
  customHeaders?: Record<string, string>;
}

// ============================================================================
// 消息与 Tool Call 协议 (OpenAI 兼容)
// ============================================================================

/**
 * Function Call 定义
 */
export interface AIFunctionCall {
  name: string;
  arguments: string;
}

/**
 * Tool Call 结构
 */
export interface AIToolCall {
  id: string;
  type: 'function';
  function: AIFunctionCall;
  /** 工具执行返回结果 (可选，执行完成后由 Agent 填充) */
  output?: string;
}

/**
 * 消息角色
 */
export type AIMessageRole = 'user' | 'assistant' | 'system' | 'tool';

/**
 * 消息状态
 */
export type AIMessageStatus = 'sending' | 'streaming' | 'done' | 'error';

/**
 * 对话消息
 */
export interface AIMessage {
  /** 消息唯一 ID */
  id: string;
  /** 角色 */
  role: AIMessageRole;
  /** 消息正文文本 */
  content: string;
  /** 附加图片列表 (Base64 data URLs) */
  images?: string[];
  /** 关联的 Tool Calls (当 role 为 assistant 且触发工具调用时) */
  tool_calls?: AIToolCall[];
  /** 关联的 Tool Call ID (当 role 为 tool 时) */
  tool_call_id?: string;
  /** 工具名称 (当 role 为 tool 时) */
  name?: string;
  /** 思考链内容 (如 DeepSeek-R1 的 reasoning_content) */
  thought?: string;
  /** 时间戳 */
  timestamp: number;
  /** 消息状态 */
  status?: AIMessageStatus;
  /** 是否为中途补充要求/调整指令 */
  isCorrection?: boolean;
  /** 错误信息 */
  error?: string;
}

// ============================================================================
// Tool 定义协议 (JSON Schema)
// ============================================================================

/**
 * 工具定义 (OpenAI Function Calling 规范)
 */
export interface AIToolDefinition {
  type: 'function';
  function: {
    name: string;
    description: string;
    parameters: {
      type: 'object';
      properties: Record<string, unknown>;
      required?: string[];
      [key: string]: unknown;
    };
  };
}

// ============================================================================
// 高风险操作拦截确认卡片
// ============================================================================

/**
 * 待确认操作状态
 */
export type AIPendingActionStatus = 'pending' | 'approved' | 'rejected';

/**
 * 高风险操作数据包
 */
export interface AIPendingAction {
  /** 操作唯一 ID */
  id: string;
  /** 工具调用 ID (关联对应的 tool_calls[].id) */
  toolCallId: string;
  /** 工具名称 */
  toolName: string;
  /** 调用参数 */
  args: Record<string, unknown>;
  /** 人类可读的操作描述 */
  description: string;
  /** 变更 Diff 概览或提示文案 */
  diffSummary?: string;
  /** 确认状态 */
  status: AIPendingActionStatus;
  /** 创建时间戳 */
  createdAt: number;
}


// ============================================================================
// 项目工作区感知 (Workspace Context)
// ============================================================================

/**
 * 本地项目工作区上下文
 */
export interface AIWorkspaceContext {
  /** 本地项目绝对根路径 */
  path: string;
  /** 项目目录名称或包名 */
  name: string;
  /** 探测到的技术栈（如 "Node.js (Express)", "Go (Gin)", "Python (FastAPI)"） */
  techStack?: string;
  /** 检测到的 OpenAPI/Swagger/Proto 文件相对路径 */
  apiSpecPath?: string;
}
// ============================================================================
// AI 会话管理 (Session Management)
// ============================================================================

/**
 * 会话轻量元数据 (用于历史列表索引)
 */
export interface AISessionMetadata {
  /** 会话唯一 ID */
  id: string;
  /** 会话标题 */
  title: string;
  /** 创建时间戳 */
  createdAt: number;
  /** 最后更新时间戳 */
  updatedAt: number;
  /** 消息数量 */
  messageCount: number;
  /** 关联的本地项目工作区 */
  workspace?: AIWorkspaceContext;
}

/**
 * 完整会话数据 (单文件存储)
 */
export interface AISession {
  /** 会话唯一 ID */
  id: string;
  /** 会话标题 */
  title: string;
  /** 创建时间戳 */
  createdAt: number;
  /** 最后更新时间戳 */
  updatedAt: number;
  /** 该会话的所有消息 */
  messages: AIMessage[];
  /** 关联的本地项目工作区 */
  workspace?: AIWorkspaceContext;
}

// ============================================================================
// Agent 状态机
// ============================================================================

/**
 * Agent 执行状态
 */
export type AIAgentStatus =
  | 'idle'
  | 'thinking'
  | 'executing_tools'
  | 'waiting_confirmation'
  | 'error';

/**
 * AI Store 状态定义
 */
export interface AIState {
  /** 当前活跃会话 ID */
  currentSessionId: string | null;
  sessions: AISessionMetadata[];
  /** 当前关联的本地项目工作区 */
  activeWorkspace: AIWorkspaceContext | null;
  /** 当前活跃会话的消息记录 */
  messages: AIMessage[];
  agentStatus: AIAgentStatus;
  /** 当前正在流式输出的消息 ID */
  activeStreamingMessageId: string | null;
  /** 待确认的高风险操作列表 */
  pendingActions: AIPendingAction[];
  /** Copilot 侧边栏是否展开 */
  isCopilotOpen: boolean;
  /** Spotlight 悬浮窗是否展开 */
  isSpotlightOpen: boolean;
  /** 侧边栏自定义宽度 (像素，默认 380) */
  panelWidth: number;
  /** 面板是否处于浮动/脱离吸附状态 */
  isFloating: boolean;
  /** 浮动窗口坐标与尺寸 { x, y, width, height } */
  floatingPosition: { x: number; y: number; width: number; height: number };
  /** 是否开启自主执行到底模式 (目标达成或中断前持续推进) */
  autoExecuteToGoal: boolean;
  /** 是否开启完全信任模式 (高风险操作无需每次手动确认直接推进执行) */
  alwaysTrustHighRisk: boolean;
  /** 错误信息 */
  error: string | null;
}
