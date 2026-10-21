import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import {
  AIAssistantTurnCard,
  groupMessagesIntoTurns,
} from '@/components/ai/AIAssistantTurnCard';
import type { AIMessage } from '@/types/ai';
import { getLL } from '@/i18n/i18n-lazy';

describe('AIAssistantTurnCard & groupMessagesIntoTurns', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  describe('groupMessagesIntoTurns', () => {
    it('正确将用户、多轮 ReAct 工具调用与最终回复聚合为单 Turn', () => {
      const messages: AIMessage[] = [
        {
          id: 'msg-user-1',
          role: 'user',
          content: '创建一个测试流水线并执行验证',
          timestamp: 1000,
        },
        {
          id: 'msg-asst-1',
          role: 'assistant',
          content: '正在规划接口测试...',
          thought: '首先需要创建登录和查询节点',
          tool_calls: [
            {
              id: 'call-1',
              type: 'function',
              function: {
                name: 'batch_create_pipeline',
                arguments: JSON.stringify({ nodes: [{ type: 'http:request' }] }),
              },
            },
          ],
          timestamp: 1001,
        },
        {
          id: 'msg-tool-1',
          role: 'tool',
          tool_call_id: 'call-1',
          name: 'batch_create_pipeline',
          content: JSON.stringify({ success: true, created_count: 2 }),
          timestamp: 1002,
        },
        {
          id: 'msg-asst-2',
          role: 'assistant',
          content: '节点已创建，现在执行测试...',
          tool_calls: [
            {
              id: 'call-2',
              type: 'function',
              function: {
                name: 'run_flow',
                arguments: '{}',
              },
            },
          ],
          timestamp: 1003,
        },
        {
          id: 'msg-tool-2',
          role: 'tool',
          tool_call_id: 'call-2',
          name: 'run_flow',
          content: JSON.stringify({ success: true, status: 'success' }),
          timestamp: 1004,
        },
        {
          id: 'msg-asst-3',
          role: 'assistant',
          content: '🎉 流水线创建并执行成功！全部 2 个节点均已通过验证。',
          timestamp: 1005,
        },
      ];

      const turns = groupMessagesIntoTurns(messages);

      // 6 条平铺消息应精确聚合为 2 个 Turn：1 个用户 Turn，1 个 Assistant 复合 Turn
      expect(turns).toHaveLength(2);
      expect(turns[0].type).toBe('user');
      if (turns[0].type === 'user') {
        expect(turns[0].message.content).toBe('创建一个测试流水线并执行验证');
      }

      expect(turns[1].type).toBe('assistant');
      if (turns[1].type === 'assistant') {
        const { turn } = turns[1];
        expect(turn.id).toBe('msg-asst-1');
        // 包含全部 5 条助手/工具消息
        expect(turn.messages).toHaveLength(5);
        // 聚合了 2 次工具调用
        expect(turn.toolCalls).toHaveLength(2);
        expect(turn.toolCalls[0].toolCall.function.name).toBe('batch_create_pipeline');
        expect(turn.toolCalls[1].toolCall.function.name).toBe('run_flow');
        // 聚合了思考过程
        expect(turn.thoughts).toHaveLength(1);
        expect(turn.thoughts[0].thought).toBe('首先需要创建登录和查询节点');
        // 最终交付消息为最后一轮结论
        expect(turn.finalMessage?.content).toBe(
          '🎉 流水线创建并执行成功！全部 2 个节点均已通过验证。',
        );
        // 中间过渡文本被记录为 intermediateTexts
        expect(turn.intermediateTexts).toHaveLength(2);
        expect(turn.intermediateTexts[0].content).toBe('正在规划接口测试...');
        expect(turn.intermediateTexts[1].content).toBe('节点已创建，现在执行测试...');
      }
    });

    it('单条纯文本助手回复正确归入单个 AssistantTurn 且无多余操作', () => {
      const messages: AIMessage[] = [
        { id: 'u1', role: 'user', content: '你好', timestamp: 100 },
        { id: 'a1', role: 'assistant', content: '你好！我是 MPE Copilot。', timestamp: 101 },
      ];

      const turns = groupMessagesIntoTurns(messages);
      expect(turns).toHaveLength(2);
      expect(turns[1].type).toBe('assistant');
      if (turns[1].type === 'assistant') {
        expect(turns[1].turn.toolCalls).toHaveLength(0);
        expect(turns[1].turn.thoughts).toHaveLength(0);
        expect(turns[1].turn.intermediateTexts).toHaveLength(0);
        expect(turns[1].turn.finalMessage?.content).toBe('你好！我是 MPE Copilot。');
      }
    });
  });

  describe('AIAssistantTurnCard 组件渲染', () => {
    it('多工具调用聚合后仅展示单个 Bot 头像与执行过程折叠卡片', () => {
      const messages: AIMessage[] = [
        {
          id: 'asst-1',
          role: 'assistant',
          content: '正在创建...',
          thought: '设计拓扑结构',
          tool_calls: [
            {
              id: 'c1',
              type: 'function',
              function: { name: 'add_node', arguments: '{"type":"http:request"}' },
            },
          ],
          timestamp: 200,
        },
        {
          id: 'tool-1',
          role: 'tool',
          tool_call_id: 'c1',
          name: 'add_node',
          content: '{"success":true}',
          timestamp: 201,
        },
        {
          id: 'asst-2',
          role: 'assistant',
          content: '流程已就绪。',
          timestamp: 202,
        },
      ];

      const turns = groupMessagesIntoTurns(messages);
      expect(turns).toHaveLength(1);
      const turn = (turns[0] as { type: 'assistant'; turn: any }).turn;

      const { container } = render(
        <AIAssistantTurnCard turn={turn} isStreaming={false} isGenerating={false} isLatest={false} />,
      );

      // 仅渲染 1 个 Bot 头像 (lucide-bot)
      const botIcons = container.querySelectorAll('svg.lucide-bot');
      expect(botIcons).toHaveLength(1);

      // 展示执行过程折叠卡片头部
      const ll = getLL();
      // 1 tool call + 1 thought = 2 operations
      expect(screen.getByText(ll.ai.copilot.execution_steps({ count: 2 }))).toBeInTheDocument();

      // 展示最终回复交付正文
      expect(screen.getByText('流程已就绪。')).toBeInTheDocument();
    });

    it('点击折叠卡片头部可以手动展开与收起执行详情', () => {
      const messages: AIMessage[] = [
        {
          id: 'asst-1',
          role: 'assistant',
          content: '操作中...',
          tool_calls: [
            {
              id: 'c1',
              type: 'function',
              function: { name: 'run_flow', arguments: '{}' },
            },
          ],
          timestamp: 300,
        },
        {
          id: 'tool-1',
          role: 'tool',
          tool_call_id: 'c1',
          name: 'run_flow',
          content: '{"success":true}',
          timestamp: 301,
        },
        {
          id: 'asst-2',
          role: 'assistant',
          content: '已完成。',
          timestamp: 302,
        },
      ];

      const turns = groupMessagesIntoTurns(messages);
      const turn = (turns[0] as { type: 'assistant'; turn: any }).turn;

      render(
        <AIAssistantTurnCard turn={turn} isStreaming={false} isGenerating={false} isLatest={false} />,
      );

      const ll = getLL();
      const accordionHeader = screen.getByText(ll.ai.copilot.execution_steps({ count: 1 }));

      // 历史已完成 Turn 默认折叠，不直接展示工具详情
      expect(screen.queryByText('run_flow')).not.toBeInTheDocument();

      // 点击展开
      fireEvent.click(accordionHeader);
      expect(screen.getByText('run_flow')).toBeInTheDocument();

      // 再次点击收起
      fireEvent.click(accordionHeader);
      expect(screen.queryByText('run_flow')).not.toBeInTheDocument();
    });

    it('发生错误状态时渲染错误卡片', () => {
      const messages: AIMessage[] = [
        {
          id: 'asst-err',
          role: 'assistant',
          status: 'error',
          error: 'Network connection timed out.',
          content: 'Network connection timed out.',
          timestamp: 400,
        },
      ];

      const turns = groupMessagesIntoTurns(messages);
      const turn = (turns[0] as { type: 'assistant'; turn: any }).turn;

      render(
        <AIAssistantTurnCard turn={turn} isStreaming={false} isGenerating={false} />,
      );
      const ll = getLL();
      expect(screen.getByText(ll.ai.copilot.error_title())).toBeInTheDocument();
    });
    it('多操作按顺序执行时：已完成展示已完成、当前执行项展示执行中、后续任务展示等待执行，不全部显示为执行中', () => {
      const turn = {
        id: 'turn-multi',
        messages: [],
        finalMessage: undefined,
        thoughts: [],
        intermediateTexts: [],
        toolCalls: [
          {
            toolCall: {
              id: 'call-1',
              type: 'function' as const,
              function: { name: 'add_node', arguments: '{"type":"http:request"}' },
              output: '{"success":true}',
            },
            result: '{"success":true}',
          },
          {
            toolCall: {
              id: 'call-2',
              type: 'function' as const,
              function: { name: 'connect_nodes', arguments: '{"from":"n1","to":"n2"}' },
            },
            result: undefined,
          },
          {
            toolCall: {
              id: 'call-3',
              type: 'function' as const,
              function: { name: 'run_flow', arguments: '{}' },
            },
            result: undefined,
          },
        ],
      };

      render(
        <AIAssistantTurnCard
          turn={turn}
          isStreaming={false}
          isGenerating={true}
          isLatest={true}
          activeExecutingToolCallId="call-2"
        />
      );

      const ll = getLL();
      // call-1 已完成
      expect(screen.getByText(ll.ai.tool_call.done())).toBeInTheDocument();
      // call-2 正在执行
      expect(screen.getByText(ll.ai.tool_call.running())).toBeInTheDocument();
      // call-3 等待执行
      expect(screen.getByText(ll.ai.tool_call.pending())).toBeInTheDocument();
    });

    it('多操作全部执行完毕后，后续单轮流式输出时状态保持稳定，绝不来回跳变为正在执行', () => {
      const turn = {
        id: 'turn-all-done',
        messages: [],
        finalMessage: {
          id: 'asst-final',
          role: 'assistant' as const,
          content: '正在生成最终回复总结...',
          status: 'streaming' as const,
          timestamp: 500,
        },
        thoughts: [],
        intermediateTexts: [],
        toolCalls: [
          {
            toolCall: {
              id: 'call-1',
              type: 'function' as const,
              function: { name: 'add_node', arguments: '{}' },
              output: '{"success":true}',
            },
            result: '{"success":true}',
          },
          {
            toolCall: {
              id: 'call-2',
              type: 'function' as const,
              function: { name: 'run_flow', arguments: '{}' },
              output: '{"success":true}',
            },
            result: '{"success":true}',
          },
        ],
      };

      // 模拟下一轮 ReAct 流式输出最终总结，此时 isStreaming=true，且所有工具已完成
      render(
        <AIAssistantTurnCard
          turn={turn}
          isStreaming={true}
          isGenerating={true}
          isLatest={true}
          activeExecutingToolCallId={null}
        />
      );

      const ll = getLL();
      // 折叠卡片默认已展开（正在生成阶段）
      const accordionHeader = screen.getByText(ll.ai.copilot.execution_steps({ count: 2 }));
      expect(accordionHeader).toBeInTheDocument();
      // 所有工具均应显示"执行成功"，而不是来回跳变为"正在执行"
      const doneElements = screen.getAllByText(ll.ai.tool_call.done());
      expect(doneElements).toHaveLength(2);
      expect(screen.queryByText(ll.ai.tool_call.running())).not.toBeInTheDocument();
    });
  });
});
