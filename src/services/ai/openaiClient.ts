/**
 * OpenAI-compatible LLM Client & SSE Streaming Parser
 */

import { invoke, Channel } from '@tauri-apps/api/core';
import { isTauriEnv } from '@/lib/platform';
import type {
  AIProviderConfig,
  AIMessage,
  AIToolDefinition,
  AIToolCall,
} from '@/types/ai';

export interface ChatStreamCallbacks {
  /** 收到正文流式 chunk */
  onContentChunk?: (chunk: string) => void;
  /** 收到思考过程 (reasoning / thought) 流式 chunk */
  onThoughtChunk?: (chunk: string) => void;
  /** 工具调用更新 */
  onToolCallsUpdate?: (toolCalls: AIToolCall[]) => void;
}

export interface ChatCompletionResult {
  content: string;
  thought?: string;
  tool_calls?: AIToolCall[];
  finishReason?: string;
}

/**
 * 规范化 Base URL
 * 确保以 /v1 结尾或合适格式，处理尾部斜杠
 */
export function normalizeBaseUrl(url: string): string {
  let cleaned = url.trim().replace(/\/+$/, '');
  if (!cleaned) return '';
  // 如果用户直接写了完整路径到 /chat/completions，剥离它
  if (cleaned.endsWith('/chat/completions')) {
    cleaned = cleaned.substring(0, cleaned.length - '/chat/completions'.length);
  }
  try {
    const parsed = new URL(cleaned);
    if (parsed.pathname === '' || parsed.pathname === '/') {
      cleaned = `${cleaned}/v1`;
    }
  } catch {
    // 保持原文
  }
  return cleaned;
}

/**
 * 构造请求头
 */
function buildHeaders(
  config: AIProviderConfig,
  extraHeaders?: Record<string, string>,
): HeadersInit {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...config.customHeaders,
    ...extraHeaders,
  };

  if (config.apiKey) {
    headers['Authorization'] = `Bearer ${config.apiKey.trim()}`;
  }

  return headers;
}

/**
 * 辅助检测末尾是否为未闭合的部分起始标签（如 "<", "<t", "<th", "<think"）
 */
function findPartialOpenTagIndex(text: string): number {
  const lastLt = text.lastIndexOf('<');
  if (lastLt !== -1 && text.length - lastLt < 9) {
    const candidate = text.slice(lastLt).toLowerCase();
    if ('<think>'.startsWith(candidate) || '<thought>'.startsWith(candidate)) {
      return lastLt;
    }
  }
  return -1;
}

/**
 * 辅助检测末尾是否为未闭合的部分结束标签（如 "<", "</", "</t", "</th", "</think"）
 */
function findPartialCloseTagIndex(text: string): number {
  const lastLt = text.lastIndexOf('</');
  if (lastLt !== -1 && text.length - lastLt < 10) {
    const candidate = text.slice(lastLt).toLowerCase();
    if ('</think>'.startsWith(candidate) || '</thought>'.startsWith(candidate)) {
      return lastLt;
    }
  }
  if (text.endsWith('<')) {
    return text.length - 1;
  }
  return -1;
}

/**
 * SSE 流式数据处理器
 */
export class SseStreamParser {
  private buffer = '';
  private accumulatedContent = '';
  private accumulatedThought = '';
  private inThinkBlock = false;
  private pendingTagBuffer = '';
  private toolCallsMap = new Map<
    number,
    { id: string; name: string; arguments: string }
  >();
  private finishReason?: string;
  private callbacks?: ChatStreamCallbacks;

  constructor(callbacks?: ChatStreamCallbacks) {
    this.callbacks = callbacks;
  }

  feedChunk(chunkText: string) {
    this.buffer += chunkText;
    const lines = this.buffer.split('\n');
    this.buffer = lines.pop() || '';

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith(':')) continue;

      if (trimmed.startsWith('data: ')) {
        const dataStr = trimmed.slice(6).trim();
        if (dataStr === '[DONE]') continue;

        try {
          const parsed = JSON.parse(dataStr);
          const choice = parsed.choices?.[0];
          if (!choice) continue;

          if (choice.finish_reason) {
            this.finishReason = choice.finish_reason;
          }
          const delta = choice.delta;
          if (!delta) continue;

          // 1. 处理思考链 / reasoning_content (DeepSeek R1 / Reasoning API)
          const thoughtChunk =
            delta.reasoning_content || delta.reasoning || delta.thought;
          if (thoughtChunk) {
            this.appendThought(thoughtChunk);
          }

          // 2. 处理正文文本（包含对 <think>...</think> 标签的流式提取）
          const contentChunk = delta.content;
          if (contentChunk) {
            this.handleContentChunk(contentChunk);
          }
          // 3. 处理工具调用 delta
          if (delta.tool_calls && Array.isArray(delta.tool_calls)) {
            for (const tcDelta of delta.tool_calls) {
              const index = tcDelta.index ?? 0;
              let existing = this.toolCallsMap.get(index);
              if (!existing) {
                existing = {
                  id: tcDelta.id || `call_${Date.now()}_${index}`,
                  name: '',
                  arguments: '',
                };
                this.toolCallsMap.set(index, existing);
              }

              if (tcDelta.id) {
                existing.id = tcDelta.id;
              }
              if (tcDelta.function?.name) {
                existing.name += tcDelta.function.name;
              }
              if (tcDelta.function?.arguments) {
                existing.arguments += tcDelta.function.arguments;
              }
            }

            const currentToolCalls: AIToolCall[] = Array.from(
              this.toolCallsMap.values(),
            ).map((tc) => ({
              id: tc.id,
              type: 'function',
              function: {
                name: tc.name,
                arguments: tc.arguments,
              },
            }));

            this.callbacks?.onToolCallsUpdate?.(currentToolCalls);
          }
        } catch {
          // 忽略单行解析失败
        }
      }
    }
  }
  private appendContent(text: string) {
    if (!text) return;
    this.accumulatedContent += text;
    this.callbacks?.onContentChunk?.(text);
  }

  private appendThought(text: string) {
    if (!text) return;
    this.accumulatedThought += text;
    this.callbacks?.onThoughtChunk?.(text);
  }

  private handleContentChunk(chunk: string) {
    let text = this.pendingTagBuffer + chunk;
    this.pendingTagBuffer = '';

    while (text.length > 0) {
      if (!this.inThinkBlock) {
        // 处于正文区域：寻找起始标签 <think> 或 <thought>
        const openMatch = text.match(/<(think|thought)>/i);
        if (openMatch && openMatch.index !== undefined) {
          const before = text.slice(0, openMatch.index);
          this.appendContent(before);
          text = text.slice(openMatch.index + openMatch[0].length);
          this.inThinkBlock = true;
        } else {
          // 检查末尾是否含有半截起始标签
          const partialIndex = findPartialOpenTagIndex(text);
          if (partialIndex !== -1) {
            const before = text.slice(0, partialIndex);
            this.appendContent(before);
            this.pendingTagBuffer = text.slice(partialIndex);
            text = '';
          } else {
            this.appendContent(text);
            text = '';
          }
        }
      } else {
        // 处于思考区域：寻找闭合标签 </think> 或 </thought>
        const closeMatch = text.match(/<\/(think|thought)>/i);
        if (closeMatch && closeMatch.index !== undefined) {
          const before = text.slice(0, closeMatch.index);
          this.appendThought(before);
          text = text.slice(closeMatch.index + closeMatch[0].length);
          this.inThinkBlock = false;
        } else {
          // 检查末尾是否含有半截闭合标签
          const partialIndex = findPartialCloseTagIndex(text);
          if (partialIndex !== -1) {
            const before = text.slice(0, partialIndex);
            this.appendThought(before);
            this.pendingTagBuffer = text.slice(partialIndex);
            text = '';
          } else {
            this.appendThought(text);
            text = '';
          }
        }
      }
    }
  }

  getResult(): ChatCompletionResult {
    // 冲刷残留的标签缓冲区
    if (this.pendingTagBuffer) {
      if (this.inThinkBlock) {
        this.appendThought(this.pendingTagBuffer);
      } else {
        this.appendContent(this.pendingTagBuffer);
      }
      this.pendingTagBuffer = '';
    }

    const finalToolCalls: AIToolCall[] = Array.from(
      this.toolCallsMap.values(),
    ).map((tc) => ({
      id: tc.id,
      type: 'function',
      function: {
        name: tc.name,
        arguments: safeJsonArguments(tc.arguments),
      },
    }));

    return {
      content: this.accumulatedContent,
      thought: this.accumulatedThought || undefined,
      tool_calls: finalToolCalls.length > 0 ? finalToolCalls : undefined,
      finishReason: this.finishReason,
    };
  }
}

/**
 * 校验并确保函数调用参数为合法 JSON 字符串，防止因空字符串或残缺 JSON 导致 Provider 报 invalid tool call arguments
 */
export function safeJsonArguments(raw?: string | null): string {
  if (!raw || typeof raw !== 'string' || !raw.trim()) return '{}';
  try {
    JSON.parse(raw);
    return raw.trim();
  } catch {
    return '{}';
  }
}

/**
 * 严格校验并修复对话消息序列，确保完全符合 OpenAI / Gemini / Claude / WorkBuddy AI 的 Wire 契约：
 * 1. 过滤掉 UI 专用报错卡片 (status === 'error') 和纯空消息；
 * 2. 规范化每个 tool_call 的 arguments 为合法 JSON 字符串（至少为 "{}"），防止报 invalid tool call arguments；
 * 3. 严格遵循函数调用协议：assistant (tool_calls) 必须紧跟对应 tool 结果，不得被 user 消息插队；
 * 4. 自动为遗漏或被中途打断的 tool_call 补全 cancelled 的 tool 响应；
 * 5. 丢弃未匹配到 assistant.tool_calls 的孤立 tool 消息。
 */
export function sanitizeChatMessages(messages: AIMessage[]): AIMessage[] {
  // 1. 基础过滤：排除纯报错消息和空占位消息
  const validMessages = messages.filter((m) => {
    if (m.status === 'error') return false;
    if (m.role === 'assistant') {
      const hasContent = Boolean(m.content && m.content.trim().length > 0);
      const hasThought = Boolean(m.thought && m.thought.trim().length > 0);
      const hasToolCalls = Boolean(m.tool_calls && m.tool_calls.length > 0);
      return hasContent || hasThought || hasToolCalls;
    }
    return true;
  });

  const result: AIMessage[] = [];
  let i = 0;

  while (i < validMessages.length) {
    const current = validMessages[i];

    if (current.role !== 'assistant' || !current.tool_calls || current.tool_calls.length === 0) {
      if (current.role === 'tool') {
        // 孤立的 tool 消息（前面没有匹配包含该 tool_call_id 的 assistant），丢弃防止报错
        i++;
        continue;
      }
      result.push(current);
      i++;
      continue;
    }

    // 处理带有 tool_calls 的 Assistant 消息
    const sanitizedToolCalls: AIToolCall[] = current.tool_calls.map((tc) => ({
      id: tc.id || `call_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      type: 'function',
      function: {
        name: tc.function?.name || 'unknown_tool',
        arguments: safeJsonArguments(tc.function?.arguments),
      },
      output: tc.output,
    }));

    const sanitizedAssistantMsg: AIMessage = {
      ...current,
      tool_calls: sanitizedToolCalls,
    };
    result.push(sanitizedAssistantMsg);

    // 收集预期的 tool_call_id
    const expectedIds = new Set(sanitizedToolCalls.map((tc) => tc.id));
    const matchedToolMessagesMap = new Map<string, AIMessage>();
    const deferredNonToolMessages: AIMessage[] = [];

    // 向后扫描收集紧随的 tool 消息及误插入其间的 user/assistant 消息
    let j = i + 1;
    while (j < validMessages.length) {
      const nextMsg = validMessages[j];
      if (nextMsg.role === 'assistant' && nextMsg.tool_calls && nextMsg.tool_calls.length > 0) {
        // 遇到了下一个包含 tool_calls 的 assistant 轮次，停止当前轮 lookahead
        break;
      }
      if (nextMsg.role === 'tool' && nextMsg.tool_call_id && expectedIds.has(nextMsg.tool_call_id)) {
        if (!matchedToolMessagesMap.has(nextMsg.tool_call_id)) {
          matchedToolMessagesMap.set(nextMsg.tool_call_id, nextMsg);
        }
      } else {
        deferredNonToolMessages.push(nextMsg);
      }
      j++;
    }

    // 按照 tool_calls 的原始顺序依次输出对应的 tool 响应
    for (const tc of sanitizedToolCalls) {
      const matched = matchedToolMessagesMap.get(tc.id);
      if (matched) {
        result.push({
          ...matched,
          content:
            typeof matched.content === 'string' && matched.content.trim() ? matched.content : '{}',
          name: tc.function.name,
        });
      } else {
        // 缺失响应：补全 cancelled tool 响应，保证大模型 Wire 协议完整性
        result.push({
          id: `synth_tool_${tc.id}`,
          role: 'tool',
          tool_call_id: tc.id,
          name: tc.function.name,
          content: JSON.stringify({
            success: false,
            cancelled: true,
            message: 'Tool call was cancelled or interrupted.',
          }),
          timestamp: current.timestamp + 1,
        });
      }
    }

    // 将原本被误放在 tool 之前的 user 消息等延迟放到 tool 响应之后
    for (const defMsg of deferredNonToolMessages) {
      if (defMsg.role !== 'tool') {
        result.push(defMsg);
      }
    }

    i = j;
  }

  return result;
}

/**
 * 发送流式对话请求
 */
export async function streamChatCompletion(
  config: AIProviderConfig,
  messages: AIMessage[],
  tools?: AIToolDefinition[],
  callbacks?: ChatStreamCallbacks,
  signal?: AbortSignal,
): Promise<ChatCompletionResult> {
  const sanitizedMessages = sanitizeChatMessages(messages);
  const formattedMessages = sanitizedMessages.map((msg) => {
    let content: unknown = msg.content || '';
    if (msg.role === 'user' && msg.images && msg.images.length > 0) {
      const parts: Array<{ type: string; text?: string; image_url?: { url: string } }> = [];
      if (msg.content) {
        parts.push({ type: 'text', text: msg.content });
      }
      for (const imgUrl of msg.images) {
        parts.push({
          type: 'image_url',
          image_url: { url: imgUrl },
        });
      }
      content = parts;
    }

    const formatted: Record<string, unknown> = {
      role: msg.role,
      content,
    };
    if (msg.role === 'assistant' && msg.tool_calls && msg.tool_calls.length > 0) {
      formatted.tool_calls = msg.tool_calls.map((tc) => ({
        id: tc.id,
        type: tc.type || 'function',
        function: {
          name: tc.function.name,
          arguments: safeJsonArguments(tc.function.arguments),
        },
      }));
    }
    if (msg.role === 'tool' && msg.tool_call_id) {
      formatted.tool_call_id = msg.tool_call_id;
      if (msg.name) formatted.name = msg.name;
      formatted.content =
        typeof msg.content === 'string' && msg.content.trim() ? msg.content : '{}';
    }
    return formatted;
  });

  const requestBody: Record<string, unknown> = {
    model: config.model,
    messages: formattedMessages,
    stream: true,
    temperature: config.temperature ?? 0.1,
  };

  if (config.maxTokens) {
    requestBody.max_tokens = config.maxTokens;
  }

  if (tools && tools.length > 0) {
    requestBody.tools = tools;
    requestBody.tool_choice = 'auto';
  }

  const parser = new SseStreamParser(callbacks);

  // 优先在 Tauri 原生环境下使用后端 IPC 流式通道（彻底规避 WebView CORS / CSP 限制）
  if (isTauriEnv()) {
    const channel = new Channel<string>();
    channel.onmessage = (chunk: string) => {
      parser.feedChunk(chunk);
    };

    await invoke<void>('ai_chat_stream', {
      baseUrl: config.baseUrl,
      apiKey: config.apiKey,
      requestBody,
      customHeaders: config.customHeaders,
      onChunk: channel,
    });

    return parser.getResult();
  }

  // 纯浏览器 Mock 环境：使用 fetch 直连
  const baseUrl = normalizeBaseUrl(config.baseUrl);
  const endpoint = `${baseUrl}/chat/completions`;

  const response = await fetch(endpoint, {
    method: 'POST',
    headers: buildHeaders(config),
    body: JSON.stringify(requestBody),
    signal,
  });

  if (!response.ok) {
    let errorDetails = '';
    try {
      const errorJson = await response.json();
      errorDetails =
        errorJson.error?.message ||
        errorJson.message ||
        JSON.stringify(errorJson);
    } catch {
      errorDetails = await response.text();
    }
    throw new Error(
      `API Request Failed (${response.status} ${response.statusText}): ${errorDetails}`,
    );
  }

  if (!response.body) {
    throw new Error('Response body is null, cannot stream');
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder('utf-8');

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      const chunkText = decoder.decode(value, { stream: true });
      parser.feedChunk(chunkText);
    }
  } finally {
    reader.releaseLock();
  }

  return parser.getResult();
}

/**
 * 测试 Provider 连通性
 */
export async function testConnection(
  config: AIProviderConfig,
): Promise<{ success: boolean; latencyMs: number; error?: string }> {
  // 优先在 Tauri 环境使用原生 Rust 请求（无 CORS/CSP 限制）
  if (isTauriEnv()) {
    try {
      const res = await invoke<{
        success: boolean;
        latency_ms: number;
        error?: string;
      }>('ai_test_connection', {
        baseUrl: config.baseUrl,
        apiKey: config.apiKey,
        model: config.model,
        customHeaders: config.customHeaders,
      });

      return {
        success: res.success,
        latencyMs: res.latency_ms,
        error: res.error,
      };
    } catch (err: unknown) {
      return {
        success: false,
        latencyMs: 0,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  // 纯浏览器 Mock 环境
  const startTime = Date.now();
  try {
    const baseUrl = normalizeBaseUrl(config.baseUrl);
    const endpoint = `${baseUrl}/chat/completions`;

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 15000);

    const response = await fetch(endpoint, {
      method: 'POST',
      headers: buildHeaders(config),
      body: JSON.stringify({
        model: config.model,
        messages: [{ role: 'user', content: 'hi' }],
        max_tokens: 5,
      }),
      signal: controller.signal,
    });

    clearTimeout(timeoutId);
    const latencyMs = Date.now() - startTime;

    if (!response.ok) {
      let errorText = '';
      try {
        const errorJson = await response.json();
        errorText =
          errorJson.error?.message ||
          errorJson.message ||
          JSON.stringify(errorJson);
      } catch {
        errorText = await response.text();
      }
      return {
        success: false,
        latencyMs,
        error: `HTTP ${response.status}: ${errorText || response.statusText}`,
      };
    }

    return {
      success: true,
      latencyMs,
    };
  } catch (err: unknown) {
    const latencyMs = Date.now() - startTime;
    return {
      success: false,
      latencyMs,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

/**
 * 拉取模型列表 (GET /v1/models)
 */
export async function fetchModels(
  config: AIProviderConfig,
): Promise<string[]> {
  if (isTauriEnv()) {
    try {
      return await invoke<string[]>('ai_fetch_models', {
        baseUrl: config.baseUrl,
        apiKey: config.apiKey,
        customHeaders: config.customHeaders,
      });
    } catch {
      return [];
    }
  }

  try {
    const baseUrl = normalizeBaseUrl(config.baseUrl);
    const endpoint = `${baseUrl}/models`;

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 10000);

    const response = await fetch(endpoint, {
      method: 'GET',
      headers: buildHeaders(config),
      signal: controller.signal,
    });

    clearTimeout(timeoutId);

    if (!response.ok) {
      return [];
    }

    const data = await response.json();
    if (Array.isArray(data?.data)) {
      return data.data
        .map((m: { id?: string }) => m.id)
        .filter((id: unknown): id is string => typeof id === 'string');
    }
    return [];
  } catch {
    return [];
  }
}
