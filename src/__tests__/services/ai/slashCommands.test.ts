import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  getSlashCommands,
  findMatchingSlashCommands,
  isSlashCommand,
  executeSlashCommand,
} from '@/services/ai/slashCommands';
import { useAIStore } from '@/stores/ai';
import { useExecutionStore } from '@/stores/execution';
import type { FlowExecutionReport } from '@/types';
describe('slashCommands service', () => {
  beforeEach(() => {
    useAIStore.getState().reset();
  });

  describe('getSlashCommands', () => {
    it('返回所有内置已注册的快捷斜杠指令', () => {
      const commands = getSlashCommands();
      const ids = commands.map((c) => c.id);
      expect(ids).toContain('compact');
      expect(ids).toContain('clear');
      expect(ids).toContain('diagnose');
      expect(ids).toContain('review');
      expect(ids).toContain('help');

      const compactCmd = commands.find((c) => c.id === 'compact');
      expect(compactCmd?.name).toBe('/compact');
      expect(compactCmd?.aliases).toContain('/summarize');
      expect(compactCmd?.title()).toBeTruthy();
      expect(compactCmd?.description()).toBeTruthy();
    });
  });

  describe('findMatchingSlashCommands', () => {
    it('按指令名称前缀进行智能匹配', () => {
      const matches = findMatchingSlashCommands('/comp');
      expect(matches.length).toBe(1);
      expect(matches[0].id).toBe('compact');
    });

    it('支持按别名匹配指令', () => {
      const matches = findMatchingSlashCommands('/summarize');
      expect(matches.length).toBe(1);
      expect(matches[0].id).toBe('compact');

      const fixMatches = findMatchingSlashCommands('/fix');
      expect(fixMatches.length).toBe(1);
      expect(fixMatches[0].id).toBe('diagnose');

      const questionMatches = findMatchingSlashCommands('/?');
      expect(questionMatches.length).toBe(1);
      expect(questionMatches[0].id).toBe('help');
    });

    it('当输入多字符匹配多个候选指令', () => {
      // /re matches /reset (alias of clear) and /review
      const matches = findMatchingSlashCommands('/re');
      const ids = matches.map((c) => c.id);
      expect(ids).toContain('clear');
      expect(ids).toContain('review');
    });

    it('非斜杠开头的文本返回空匹配', () => {
      expect(findMatchingSlashCommands('hello')).toEqual([]);
      expect(findMatchingSlashCommands('')).toEqual([]);
    });
  });

  describe('isSlashCommand', () => {
    it('正确识别合法斜杠指令及其别名', () => {
      expect(isSlashCommand('/compact')).toBe(true);
      expect(isSlashCommand('/summarize')).toBe(true);
      expect(isSlashCommand('/clear')).toBe(true);
      expect(isSlashCommand('/reset')).toBe(true);
      expect(isSlashCommand('/diagnose')).toBe(true);
      expect(isSlashCommand('/fix')).toBe(true);
      expect(isSlashCommand('/review')).toBe(true);
      expect(isSlashCommand('/audit')).toBe(true);
      expect(isSlashCommand('/help')).toBe(true);
      expect(isSlashCommand('/?')).toBe(true);
      expect(isSlashCommand('/compact now please')).toBe(true);
    });

    it('过滤未知指令与普通文本', () => {
      expect(isSlashCommand('/unknown')).toBe(false);
      expect(isSlashCommand('normal text')).toBe(false);
      expect(isSlashCommand(' /compact')).toBe(true); // trims before checking
    });
  });

  describe('executeSlashCommand', () => {
    it('执行 /clear 清空会话消息', async () => {
      useAIStore.setState({
        messages: [
          { id: '1', role: 'user', content: 'test', timestamp: 1 },
        ],
      });

      const handled = await executeSlashCommand('/clear');
      expect(handled).toBe(true);
      expect(useAIStore.getState().messages).toHaveLength(0);
    });

    it('执行 /help 注入帮助信息消息', async () => {
      const handled = await executeSlashCommand('/help');
      expect(handled).toBe(true);

      const msgs = useAIStore.getState().messages;
      expect(msgs).toHaveLength(1);
      expect(msgs[0].role).toBe('assistant');
      expect(msgs[0].content).toContain('/compact');
      expect(msgs[0].content).toContain('/clear');
    });

    it('执行 /diagnose 读取 execution report 中的失败节点数组', async () => {
      const sendSpy = vi.spyOn(useAIStore.getState(), 'sendMessage').mockResolvedValue();
      useExecutionStore.setState({
        report: {
          flow_uuid: 'f-1',
          execution_id: 'e-1',
          status: 'failed',
          node_reports: [
            {
              node_uuid: 'node-http-1',
              node_name: '请求订单',
              node_type: 'http:request',
              status: 'failed',
              error: { error_kind: 'network', message: 'connection refused' },
            },
          ],
        } as unknown as FlowExecutionReport,
      });

      const handled = await executeSlashCommand('/diagnose');
      expect(handled).toBe(true);
      expect(sendSpy).toHaveBeenCalledWith(
        expect.stringContaining('请求订单'),
      );
      expect(sendSpy).toHaveBeenCalledWith(
        expect.stringContaining('connection refused'),
      );

      sendSpy.mockRestore();
    });

    it('执行 /review 发送流程审查指令', async () => {
      const sendSpy = vi.spyOn(useAIStore.getState(), 'sendMessage').mockResolvedValue();

      const handled = await executeSlashCommand('/review');
      expect(handled).toBe(true);
      expect(sendSpy).toHaveBeenCalled();

      sendSpy.mockRestore();
    });

    it('执行未识别指令返回 false', async () => {
      const handled = await executeSlashCommand('/foobar');
      expect(handled).toBe(false);
    });

    it('执行普通非斜杠文本返回 false', async () => {
      const handled = await executeSlashCommand('what is a flow?');
      expect(handled).toBe(false);
    });
  });
});
