import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  normalizeBaseUrl,
  streamChatCompletion,
  testConnection,
  fetchModels,
  SseStreamParser,
  safeJsonArguments,
  sanitizeChatMessages,
} from '@/services/ai/openaiClient';
import type { AIProviderConfig, AIMessage } from '@/types/ai';

describe('openaiClient', () => {
  const mockConfig: AIProviderConfig = {
    id: 'test-provider',
    name: 'Test Provider',
    baseUrl: 'https://api.deepseek.com/v1',
    apiKey: 'sk-test-key',
    model: 'deepseek-chat',
    temperature: 0.7,
  };

  beforeEach(() => {
    vi.restoreAllMocks();
  });

  describe('normalizeBaseUrl', () => {
    it('规范化尾部斜杠', () => {
      expect(normalizeBaseUrl('https://api.openai.com/v1/')).toBe('https://api.openai.com/v1');
      expect(normalizeBaseUrl('https://api.openai.com/v1///')).toBe('https://api.openai.com/v1');
    });

    it('对无路径的主机自动补全 /v1', () => {
      expect(normalizeBaseUrl('https://api.deepseek.com')).toBe('https://api.deepseek.com/v1');
      expect(normalizeBaseUrl('http://localhost:11434')).toBe('http://localhost:11434/v1');
    });

    it('去除用户误填的 /chat/completions 路径', () => {
      expect(normalizeBaseUrl('https://api.deepseek.com/v1/chat/completions')).toBe(
        'https://api.deepseek.com/v1',
      );
    });

    it('处理空字符串', () => {
      expect(normalizeBaseUrl('')).toBe('');
      expect(normalizeBaseUrl('   ')).toBe('');
    });
  });

  describe('streamChatCompletion', () => {
    it('正确解析 SSE 文本流并触发回调', async () => {
      const ssePayload = [
        'data: {"choices":[{"delta":{"content":"Hello"}}]}\n\n',
        'data: {"choices":[{"delta":{"content":" World"}}]}\n\n',
        'data: [DONE]\n\n',
      ].join('');

      const stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode(ssePayload));
          controller.close();
        },
      });

      const fetchMock = vi.fn().mockResolvedValue({
        ok: true,
        body: stream,
      });
      global.fetch = fetchMock;

      const chunks: string[] = [];
      const messages: AIMessage[] = [{ id: '1', role: 'user', content: 'hi', timestamp: Date.now() }];

      const result = await streamChatCompletion(mockConfig, messages, undefined, {
        onContentChunk: (chunk) => chunks.push(chunk),
      });

      expect(result.content).toBe('Hello World');
      expect(chunks).toEqual(['Hello', ' World']);
      expect(fetchMock).toHaveBeenCalledWith(
        'https://api.deepseek.com/v1/chat/completions',
        expect.objectContaining({
          method: 'POST',
          headers: expect.objectContaining({
            Authorization: 'Bearer sk-test-key',
            'Content-Type': 'application/json',
          }),
        }),
      );
    });

    it('正确解析 Tool Calls delta 流式分块', async () => {
      const ssePayload = [
        'data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_123","function":{"name":"add_node","arguments":"{\\"type\\":\\""}}]}}]}\n\n',
        'data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"http:request\\"}"}}]}}]}\n\n',
        'data: [DONE]\n\n',
      ].join('');

      const stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode(ssePayload));
          controller.close();
        },
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        body: stream,
      });

      const messages: AIMessage[] = [{ id: '1', role: 'user', content: 'add node', timestamp: Date.now() }];
      const result = await streamChatCompletion(mockConfig, messages);

      expect(result.tool_calls).toHaveLength(1);
      expect(result.tool_calls?.[0]).toEqual({
        id: 'call_123',
        type: 'function',
        function: {
          name: 'add_node',
          arguments: '{"type":"http:request"}',
        },
      });
    });

    it('正确解析思考链 reasoning_content', async () => {
      const ssePayload = [
        'data: {"choices":[{"delta":{"reasoning_content":"Let me think."}}]}\n\n',
        'data: {"choices":[{"delta":{"reasoning_content":" Done thinking."}}]}\n\n',
        'data: {"choices":[{"delta":{"content":"Here is the answer."}}]}\n\n',
        'data: [DONE]\n\n',
      ].join('');

      const stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode(ssePayload));
          controller.close();
        },
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        body: stream,
      });

      const thoughtChunks: string[] = [];
      const messages: AIMessage[] = [{ id: '1', role: 'user', content: 'think', timestamp: Date.now() }];
      const result = await streamChatCompletion(mockConfig, messages, undefined, {
        onThoughtChunk: (chunk) => thoughtChunks.push(chunk),
      });

      expect(result.thought).toBe('Let me think. Done thinking.');
      expect(result.content).toBe('Here is the answer.');
      expect(thoughtChunks).toEqual(['Let me think.', ' Done thinking.']);
    });

    it('正确流式提取 delta.content 中的 <think>...</think> 标签', async () => {
      const ssePayload = [
        'data: {"choices":[{"delta":{"content":"<think>Let me plan "}}]}\n\n',
        'data: {"choices":[{"delta":{"content":"the pipeline.</think>I will create "}}]}\n\n',
        'data: {"choices":[{"delta":{"content":"the nodes."}}]}\n\n',
        'data: [DONE]\n\n',
      ].join('');

      const stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode(ssePayload));
          controller.close();
        },
      });

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        body: stream,
      });

      const thoughtChunks: string[] = [];
      const contentChunks: string[] = [];
      const messages: AIMessage[] = [{ id: '1', role: 'user', content: 'create pipeline', timestamp: Date.now() }];
      const result = await streamChatCompletion(mockConfig, messages, undefined, {
        onThoughtChunk: (chunk) => thoughtChunks.push(chunk),
        onContentChunk: (chunk) => contentChunks.push(chunk),
      });

      expect(result.thought).toBe('Let me plan the pipeline.');
      expect(result.content).toBe('I will create the nodes.');
      expect(thoughtChunks.join('')).toBe('Let me plan the pipeline.');
      expect(contentChunks.join('')).toBe('I will create the nodes.');
    });

    it('SseStreamParser 能正确处理跨 chunk 拆分的 <think> 和 </think> 标签', () => {
      const thoughtChunks: string[] = [];
      const contentChunks: string[] = [];
      const parser = new SseStreamParser({
        onThoughtChunk: (c) => thoughtChunks.push(c),
        onContentChunk: (c) => contentChunks.push(c),
      });

      // <think> 拆分在两个 chunk
      parser.feedChunk('data: {"choices":[{"delta":{"content":"<th"}}]}\n\n');
      parser.feedChunk('data: {"choices":[{"delta":{"content":"ink>Thinking about "}}]}\n\n');
      parser.feedChunk('data: {"choices":[{"delta":{"content":"architecture."}}]}\n\n');
      // </think> 拆分在两个 chunk
      parser.feedChunk('data: {"choices":[{"delta":{"content":"</th"}}]}\n\n');
      parser.feedChunk('data: {"choices":[{"delta":{"content":"ink>Here is the plan."}}]}\n\n');
      parser.feedChunk('data: [DONE]\n\n');

      const result = parser.getResult();
      expect(result.thought).toBe('Thinking about architecture.');
      expect(result.content).toBe('Here is the plan.');
      expect(thoughtChunks.join('')).toBe('Thinking about architecture.');
      expect(contentChunks.join('')).toBe('Here is the plan.');
    });

    it('正确将用户图片组装为 OpenAI Vision 多模态格式', async () => {
      const ssePayload = 'data: {"choices":[{"delta":{"content":"I see an image"}}]}\n\ndata: [DONE]\n\n';
      const stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode(ssePayload));
          controller.close();
        },
      });

      const fetchMock = vi.fn().mockResolvedValue({
        ok: true,
        body: stream,
      });
      global.fetch = fetchMock;

      const messages: AIMessage[] = [
        {
          id: '1',
          role: 'user',
          content: 'What is this?',
          images: ['data:image/png;base64,12345'],
          timestamp: Date.now(),
        },
      ];

      const result = await streamChatCompletion(mockConfig, messages);
      expect(result.content).toBe('I see an image');
      expect(fetchMock).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          body: expect.stringContaining('"type":"image_url"'),
        }),
      );
    });
    it('未指定 temperature 时默认回退到 0.1', async () => {
      const stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode('data: {"choices":[{"delta":{"content":"ok"}}]}\n\ndata: [DONE]\n\n'));
          controller.close();
        },
      });
      const fetchMock = vi.fn().mockResolvedValue({
        ok: true,
        body: stream,
      });
      global.fetch = fetchMock;

      const configWithoutTemp: AIProviderConfig = {
        id: 'p1',
        name: 'P1',
        baseUrl: 'https://api.deepseek.com/v1',
        apiKey: 'sk',
        model: 'deepseek-chat',
      };

      await streamChatCompletion(configWithoutTemp, [{ id: '1', role: 'user', content: 'hi', timestamp: Date.now() }]);
      expect(fetchMock).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          body: expect.stringContaining('"temperature":0.1'),
        }),
      );
    });
  });

  describe('testConnection', () => {
    it('成功时返回 success=true 及 latencyMs', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ choices: [{ message: { content: 'hello' } }] }),
      });

      const res = await testConnection(mockConfig);
      expect(res.success).toBe(true);
      expect(typeof res.latencyMs).toBe('number');
    });

    it('失败时返回 success=false 及 error 字符串', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 401,
        statusText: 'Unauthorized',
        json: async () => ({ error: { message: 'Invalid API key' } }),
      });

      const res = await testConnection(mockConfig);
      expect(res.success).toBe(false);
      expect(res.error).toContain('Invalid API key');
    });
  });

  describe('fetchModels', () => {
    it('解析 models 列表', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          data: [{ id: 'deepseek-chat' }, { id: 'deepseek-coder' }],
        }),
      });

      const models = await fetchModels(mockConfig);
      expect(models).toEqual(['deepseek-chat', 'deepseek-coder']);
    });
  });

  describe('safeJsonArguments', () => {
    it('将空值、空字符串或仅包含空格的字符串安全回退为 "{}"', () => {
      expect(safeJsonArguments('')).toBe('{}');
      expect(safeJsonArguments('   ')).toBe('{}');
      expect(safeJsonArguments(null)).toBe('{}');
      expect(safeJsonArguments(undefined)).toBe('{}');
    });

    it('将残缺或非法的 JSON 字符串回退为 "{}"，防止 OpenAI 报 invalid tool call arguments', () => {
      expect(safeJsonArguments('{"question":')).toBe('{}');
      expect(safeJsonArguments('not a json')).toBe('{}');
    });

    it('保留合法的 JSON 字符串', () => {
      expect(safeJsonArguments('{"question":"dev"}')).toBe('{"question":"dev"}');
      expect(safeJsonArguments('{}')).toBe('{}');
    });
  });

  describe('sanitizeChatMessages', () => {
    it('剔除 status 为 error 的 UI 报错卡片消息，避免污染上下文', () => {
      const messages: AIMessage[] = [
        { id: '1', role: 'user', content: 'hello', timestamp: 1 },
        {
          id: '2',
          role: 'assistant',
          status: 'error',
          content: 'HTTP 400 Bad Request',
          error: 'HTTP 400',
          timestamp: 2,
        },
      ];
      const sanitized = sanitizeChatMessages(messages);
      expect(sanitized).toHaveLength(1);
      expect(sanitized[0].id).toBe('1');
    });

    it('自动修复被 user 纠偏消息错误插队的 tool 结果顺序 (Assistant -> Tool -> User)', () => {
      const messages: AIMessage[] = [
        { id: 'u1', role: 'user', content: '创建工作流', timestamp: 1 },
        {
          id: 'a1',
          role: 'assistant',
          content: '',
          tool_calls: [
            {
              id: 'call_1',
              type: 'function',
              function: { name: 'ask_user', arguments: '{"question":"环境?"}' },
            },
          ],
          timestamp: 2,
        },
        // 错误插在 tool 前面的 user 消息
        { id: 'u2', role: 'user', content: '我用 dev 环境', isCorrection: true, timestamp: 3 },
        // 稍后完成的 tool 消息
        {
          id: 't1',
          role: 'tool',
          tool_call_id: 'call_1',
          name: 'ask_user',
          content: '{"success":true,"answer":"dev"}',
          timestamp: 4,
        },
      ];

      const sanitized = sanitizeChatMessages(messages);
      expect(sanitized).toHaveLength(4);
      expect(sanitized[0].id).toBe('u1');
      expect(sanitized[1].id).toBe('a1');
      // Tool 结果被正确前置紧跟 Assistant
      expect(sanitized[2].id).toBe('t1');
      expect(sanitized[2].role).toBe('tool');
      // User 消息紧随 Tool 之后
      expect(sanitized[3].id).toBe('u2');
      expect(sanitized[3].role).toBe('user');
    });

    it('为未完成或中断的 tool_call 自动补齐 cancelled 的 tool 响应 (避免 Gemini 与 WorkBuddy 400)', () => {
      const messages: AIMessage[] = [
        { id: 'u1', role: 'user', content: '执行测试', timestamp: 1 },
        {
          id: 'a1',
          role: 'assistant',
          content: '',
          tool_calls: [
            {
              id: 'call_unfulfilled',
              type: 'function',
              function: { name: 'ask_user', arguments: '' },
            },
          ],
          timestamp: 2,
        },
        { id: 'u2', role: 'user', content: '算了不用了', timestamp: 3 },
      ];

      const sanitized = sanitizeChatMessages(messages);
      // 应包含: u1, a1, synth_tool, u2
      expect(sanitized).toHaveLength(4);
      expect(sanitized[1].id).toBe('a1');
      // a1 的 arguments 已被修正为 "{}"
      expect(sanitized[1].tool_calls?.[0].function.arguments).toBe('{}');
      // 自动补齐的 tool 响应
      expect(sanitized[2].role).toBe('tool');
      expect(sanitized[2].tool_call_id).toBe('call_unfulfilled');
      expect(JSON.parse(sanitized[2].content)).toEqual(
        expect.objectContaining({ cancelled: true }),
      );
      // user 紧随其后
      expect(sanitized[3].id).toBe('u2');
    });

    it('丢弃没有对应 assistant.tool_calls 的孤立 tool 消息', () => {
      const messages: AIMessage[] = [
        {
          id: 'orphan_tool',
          role: 'tool',
          tool_call_id: 'non_existent',
          content: '{"data":1}',
          timestamp: 1,
        },
        { id: 'u1', role: 'user', content: 'hello', timestamp: 2 },
      ];
      const sanitized = sanitizeChatMessages(messages);
      expect(sanitized).toHaveLength(1);
      expect(sanitized[0].id).toBe('u1');
    });
  });
});
