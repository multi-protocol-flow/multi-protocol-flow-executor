import { describe, it, expect, beforeEach } from 'vitest';
import {
  buildSystemPrompt,
  buildActiveStateSummary,
  formatNodeConfigSummary,
  buildCompactionSystemPrompt,
  buildCompactionUserPrompt,
  estimateTokens,
  pruneMessagesForSending,
} from '@/services/ai/contextBuilder';
import { useFlowStore } from '@/stores/flow';
import { useEnvironmentStore } from '@/stores/environment';
import { useExecutionStore } from '@/stores/execution';
import type { Flow, FlowExecutionReport, HttpConfig, ScriptNodeConfig } from '@/types';
import type { AIMessage } from '@/types/ai';

describe('contextBuilder', () => {
  beforeEach(() => {
    useFlowStore.getState().reset();
    useEnvironmentStore.getState().reset();
  });

  describe('buildSystemPrompt', () => {
    it('包含 MPE 核心架构说明和 JavaScript 沙箱规范', () => {
      const prompt = buildSystemPrompt();
      expect(prompt).toContain('MPE Copilot & Flow Architect');
      expect(prompt).toContain('RouteToFalse');
      expect(prompt).toContain('auto_layout');
      expect(prompt).toContain('Error Diagnosis & Collaborative Repair Protocol');
      expect(prompt).toContain('是否需要我帮您修复？您倾向于按哪个方向修复？');
      expect(prompt).toContain('get_node_type_schema');
      expect(prompt).toContain('get_node_detail');
      expect(prompt).toContain('run_terminal_command');
      expect(prompt).toContain('Local Terminal & Shell Execution Protocol');
    });
    it('包含 flow:sub 子流程节点分类、端口规范与配置模式', () => {
      const prompt = buildSystemPrompt();
      expect(prompt).toContain("'flow:sub'");
      expect(prompt).toContain("flow:sub");
      expect(prompt).toContain("target_flow_path");
      expect(prompt).toContain("target_flow_id");
      expect(prompt).toContain("inherit_all_variables");
    });

    it('包含强制 YAML 格式流程文件规范与禁止 JSON 说明', () => {
      const prompt = buildSystemPrompt();
      expect(prompt).toContain('MANDATORY YAML FORMAT');
      expect(prompt).toContain('.mpf');
      expect(prompt).toContain('YAML');
    });

    it('包含内置动态变量规范与常见类别示例', () => {
      const prompt = buildSystemPrompt();
      expect(prompt).toContain('Built-in Dynamic Variables');
      expect(prompt).toContain('{{$guid}}');
      expect(prompt).toContain('{{$timestamp}}');
      expect(prompt).toContain('{{$randomEmail}}');
      expect(prompt).toContain('{{$randomInt}}');
      // 脚本沙箱只暴露 fn.* API；旧文档里的 context.* 在运行期是 undefined
      expect(prompt).toContain('fn.variables.get(key)');
      expect(prompt).toContain('fn.response.body');
    });

    it('包含外挂环境文件与变量分层规范', () => {
      const prompt = buildSystemPrompt();
      expect(prompt).toContain('Global Environment Files');
      expect(prompt).toContain('flow.initial_variables');
      expect(prompt).toContain('.local.yaml');
      // 引擎按裸键解析变量，不存在 env./variables. 命名空间前缀
      expect(prompt).toContain('NEVER emit');
      // evalexpr 没有 null 字面量：示例必须是可求值的形式，并显式告知模型该限制
      expect(prompt).toContain('advanced_expression');
      expect(prompt).toContain('status == 200 && TOKEN != \\"\\"');
      expect(prompt).toContain('NO `null` literal');
    });

    it('包含目标驱动自主执行与完工原则', () => {
      const prompt = buildSystemPrompt();
      expect(prompt).toContain('Goal-Driven Autonomous Execution Protocol');
      expect(prompt).toContain('ACTION-FIRST AGENT PARADIGM');
      expect(prompt).toContain('CONTINUOUS EXECUTION UNTIL GOAL ATTAINED');
      expect(prompt).toContain('DISTINGUISHING ACTION VS INQUIRY');
      expect(prompt).toContain('GENUINE CLARIFICATION VS HESITATION');
      expect(prompt).toContain('SELF-HEALING & ERROR RECOVERY');
      expect(prompt).toContain('FINAL SUMMARY UPON COMPLETION');
    });

    it('追加用户自定义 System Prompt', () => {
      const custom = 'Always use camelCase for variables.';
      const prompt = buildSystemPrompt(custom);
      expect(prompt).toContain(custom);
    });
  });

  describe('buildActiveStateSummary', () => {
    it('当没有打开流程时输出对应状态', () => {
      const summary = buildActiveStateSummary();
      expect(summary).toContain('No flow currently opened');
    });

    it('当打开流程时输出节点和连线摘要', () => {
      const mockFlow: Flow = {
        uuid: 'flow-test-uuid',
        name: 'My Test Flow',
        description: 'Test flow description',
        initial_variables: {},
        nodes: [
          {
            uuid: 'node-1',
            type: 'http:request',
            name: 'Login Step',
            position: { x: 100, y: 100 },
            data: { type: 'http:request', url: 'https://api.com/login', method: 'post' } as HttpConfig,
          },
          {
            uuid: 'node-2',
            type: 'script',
            name: 'Parse Token',
            position: { x: 300, y: 100 },
            data: { type: 'script', script: 'console.log(1);' } as ScriptNodeConfig,
          },
        ],
        connections: [
          {
            id: 'c1',
            source_node_uuid: 'node-1',
            source_port_id: 'default',
            target_node_uuid: 'node-2',
            target_port_id: 'default',
          },
        ],
      };

      useFlowStore.getState().setFlow(mockFlow);
      useEnvironmentStore.setState({ activeEnvironmentName: 'dev' });
      const summary = buildActiveStateSummary();
      expect(summary).toContain('flow-test-uuid');
      expect(summary).toContain('My Test Flow');
      expect(summary).toContain('Login Step');
      expect(summary).toContain('Parse Token');
      expect(summary).toContain('Active Environment: "dev"');
      // 验证包含节点配置摘要与小流程下的 Config JSON
      expect(summary).toContain('Config Summary:');
      expect(summary).toContain('Config JSON:');
      expect(summary).toContain('https://api.com/login');
    });

    it('当流程执行失败时，在上下文中输出详细的失败节点错误与根因线索', () => {
      const mockFlow: Flow = {
        uuid: 'flow-err-uuid',
        name: 'Failing Flow',
        initial_variables: {},
        nodes: [
          {
            uuid: 'node-err-1',
            type: 'http:request',
            name: 'Broken HTTP Request',
            position: { x: 100, y: 100 },
            data: { type: 'http:request', url: 'http://127.0.0.1:8080/api', method: 'get' } as HttpConfig,
          },
        ],
        connections: [],
      };
      useFlowStore.getState().setFlow(mockFlow);

      const mockReport: FlowExecutionReport = {
        uuid: 'report-uuid-1',
        flow_uuid: 'flow-err-uuid',
        flow_name: 'Failing Flow',
        start_time: 1000,
        end_time: 1150,
        duration_ms: 150,
        total_nodes: 1,
        executed_count: 1,
        status: 'failed',
        flow_error: {
          error_kind: 'execution',
          error_code: 'E_CONN_REFUSED',
          message: 'Flow execution stopped due to node failure',
        },
        statistics: { success_count: 0, failed_count: 1 },
        node_reports: [
          {
            node_uuid: 'node-err-1',
            node_name: 'Broken HTTP Request',
            node_type: 'http:request',
            start_time: 1000,
            end_time: 1150,
            duration_ms: 150,
            status: 'failed',
            error: {
              error_kind: 'connection',
              error_code: 'E_CONN_REFUSED',
              message: 'Connection refused (os error 111)',
              stack_trace: 'at connect (/src/net.rs:42)\nat execute (/src/http.rs:88)',
            },
            resolved_config: { url: 'http://127.0.0.1:8080/api', method: 'GET' },
            output_data: { error: 'connect refused' },
          },
        ],
      };
      useExecutionStore.getState().setReport(mockReport);

      const summary = buildActiveStateSummary();
      expect(summary).toContain('Last Execution Report');
      expect(summary).toContain('Failed Nodes Details (CRITICAL - USE TO DIAGNOSE & PROPOSE REPAIRS)');
      expect(summary).toContain('Broken HTTP Request');
      expect(summary).toContain('Error Kind: connection');
      expect(summary).toContain('Error Code: E_CONN_REFUSED');
      expect(summary).toContain('Connection refused (os error 111)');
      expect(summary).toContain('Resolved Config:');
      expect(summary).toContain('Flow-Level Error: [execution]');
    });
  });

  describe('formatNodeConfigSummary', () => {
    it('格式化 http 节点摘要', () => {
      const res = formatNodeConfigSummary('http:request', {
        method: 'post',
        url: 'https://example.com/api',
        body_type: 'body',
        timeout_ms: 5000,
      });
      expect(res).toBe('POST | https://example.com/api | body_type: body | timeout: 5000ms');
    });

    it('格式化 assertion 节点摘要', () => {
      const res = formatNodeConfigSummary('assertion', {
        mode: 'all',
        assertions: [
          { target: 'status_code', operator: 'equal', expected: 200 },
        ],
      });
      expect(res).toContain('mode: all');
      expect(res).toContain('status_code equal 200');
    });

    it('格式化 conditional 节点摘要', () => {
      const res = formatNodeConfigSummary('conditional', {
        condition_type: 'advanced',
        advanced_expression: '{{status}} == 200',
      });
      expect(res).toBe('advanced: {{status}} == 200');
    });

    it('空配置兜底', () => {
      expect(formatNodeConfigSummary('http:request', undefined)).toBe('empty config');
    });
    it('格式化 flow:sub 节点摘要', () => {
      const res = formatNodeConfigSummary('flow:sub', {
        target_flow_path: './subflows/login.json',
        inherit_all_variables: true,
        export_all_variables: true,
      });
      expect(res).toContain('target: ./subflows/login.json');
      expect(res).toContain('inherit_vars: true');
      expect(res).toContain('export_vars: true');
    });
  });

  describe('compaction & pruning utilities', () => {
    it('buildCompactionSystemPrompt 返回规范的会话压缩系统指令', () => {
      const prompt = buildCompactionSystemPrompt();
      expect(prompt).toContain('Context Compaction Engine');
      expect(prompt).toContain('Objective & Flow Architecture');
      expect(prompt).toContain('Key Variables & Specifications');
      expect(prompt).toContain('Current Status & Next Actions');
    });

    it('buildCompactionUserPrompt 将多角色消息转为结构化纯文本', () => {
      const messages: AIMessage[] = [
        { id: '1', role: 'user', content: '创建登录流程', timestamp: 1 },
        {
          id: '2',
          role: 'assistant',
          content: '好的，正在创建',
          tool_calls: [
            {
              id: 'tc1',
              type: 'function',
              function: { name: 'add_node', arguments: '{"type":"http:request"}' },
            },
          ],
          timestamp: 2,
        },
        { id: '3', role: 'tool', name: 'add_node', content: 'node created ok', timestamp: 3 },
      ];
      const res = buildCompactionUserPrompt(messages);
      expect(res).toContain('[User]: 创建登录流程');
      expect(res).toContain('[Assistant]: 好的，正在创建');
      expect(res).toContain('[Tool Calls]: add_node({"type":"http:request"})');
      expect(res).toContain('[Tool Output (add_node)]: node created ok');
    });

    it('estimateTokens 预估字符消耗', () => {
      const messages: AIMessage[] = [
        { id: '1', role: 'user', content: 'Hello world', timestamp: 1 },
      ];
      const tokens = estimateTokens(messages);
      expect(tokens).toBeGreaterThan(0);
    });

    it('pruneMessagesForSending 正确过滤归档消息并截断超长历史工具输出', () => {
      const messages: AIMessage[] = [
        { id: '1', role: 'user', content: '已归档内容', isArchived: true, timestamp: 1 },
        { id: '2', role: 'assistant', content: '历史摘要', isSummary: true, timestamp: 2 },
        {
          id: '3',
          role: 'assistant',
          content: '中间步骤',
          thought: '深度思考中...',
          timestamp: 3,
        },
        {
          id: '4',
          role: 'tool',
          name: 'scan',
          content: 'A'.repeat(1200),
          timestamp: 4,
        },
        { id: '5', role: 'user', content: '最新提问', isCorrection: true, timestamp: 5 },
        { id: '6', role: 'assistant', content: '最新回答', thought: '保留最新思考', timestamp: 6 },
        { id: '7', role: 'tool', name: 'latest_tool', content: 'B'.repeat(1200), timestamp: 7 },
      ];

      const pruned = pruneMessagesForSending(messages, '[补充要求]:');
      // 1. 已归档消息被过滤
      expect(pruned.find((m) => m.id === '1')).toBeUndefined();
      // 2. isSummary 消息被注入语义前缀
      const summaryMsg = pruned.find((m) => m.id === '2');
      expect(summaryMsg?.content).toContain('[Previous Conversation Summary & Key Milestones]:');
      // 3. 历史 assistant 消息的旧 thought 被剥离，而最新 assistant 的 thought 保留
      const oldAssistant = pruned.find((m) => m.id === '3');
      const latestAssistant = pruned.find((m) => m.id === '6');
      expect(oldAssistant?.thought).toBeUndefined();
      expect(latestAssistant?.thought).toBe('保留最新思考');
      // 4. 较旧的超长 tool 消息被截断
      const oldTool = pruned.find((m) => m.id === '4');
      expect(oldTool?.content).toContain('... [output truncated for context limit]');
      // 5. 最新的 tool 消息保留
      const latestTool = pruned.find((m) => m.id === '7');
      expect(latestTool?.content).toBe('B'.repeat(1200));
      // 6. user 纠偏指令带上前缀
      const userMsg = pruned.find((m) => m.id === '5');
      expect(userMsg?.content).toBe('[补充要求]: 最新提问');
    });

    it('pruneMessagesForSending 仅保留最近两轮 user 消息的图片附件，较旧轮次剥离 images', () => {
      const messages: AIMessage[] = [
        { id: 'u1', role: 'user', content: '图1', images: ['data:image1'], timestamp: 1 },
        { id: 'a1', role: 'assistant', content: '答1', timestamp: 2 },
        { id: 'u2', role: 'user', content: '图2', images: ['data:image2'], timestamp: 3 },
        { id: 'a2', role: 'assistant', content: '答2', timestamp: 4 },
        { id: 'u3', role: 'user', content: '图3', images: ['data:image3'], timestamp: 5 },
      ];

      const pruned = pruneMessagesForSending(messages, '[补充要求]:');
      const u1Pruned = pruned.find((m) => m.id === 'u1');
      const u2Pruned = pruned.find((m) => m.id === 'u2');
      const u3Pruned = pruned.find((m) => m.id === 'u3');

      expect(u1Pruned?.images).toBeUndefined();
      expect(u2Pruned?.images).toEqual(['data:image2']);
      expect(u3Pruned?.images).toEqual(['data:image3']);
    });
  });
});
