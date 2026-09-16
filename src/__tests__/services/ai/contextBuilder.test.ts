import { describe, it, expect, beforeEach } from 'vitest';
import {
  buildSystemPrompt,
  buildActiveStateSummary,
  formatNodeConfigSummary,
} from '@/services/ai/contextBuilder';
import { useFlowStore } from '@/stores/flow';
import { useExecutionStore } from '@/stores/execution';
import type { Flow, FlowExecutionReport, HttpConfig, ScriptNodeConfig } from '@/types';

describe('contextBuilder', () => {
  beforeEach(() => {
    useFlowStore.getState().reset();
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
    it('包含 sub_flow 子流程节点分类、端口规范与配置模式', () => {
      const prompt = buildSystemPrompt();
      expect(prompt).toContain("'sub_flow'");
      expect(prompt).toContain("sub_flow");
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
      expect(prompt).toContain('context.variables.get(\'$guid\')');
    });

    it('包含透明可解释自主执行规划原则', () => {
      const prompt = buildSystemPrompt();
      expect(prompt).toContain('TRANSPARENT PLANNING FIRST');
      expect(prompt).toContain('已为您规划');
    });

    it('追加用户自定义 System Prompt', () => {
      const custom = 'Always use camelCase for variables.';
      const prompt = buildSystemPrompt(custom);
      expect(prompt).toContain(custom);
    });

    it('开启自主执行到底模式时包含 AUTONOMOUS EXECUTION 核心指令', () => {
      const prompt = buildSystemPrompt(undefined, true);
      expect(prompt).toContain('AUTONOMOUS EXECUTION MODE IS CURRENTLY ACTIVE');
      expect(prompt).toContain('执行到底');
      expect(prompt).toContain('DEEP INTENT COMPREHENSION');
      expect(prompt).toContain('SELF-HEALING & ERROR RECOVERY');
    });

    it('未开启自主模式时不输出 AUTONOMOUS EXECUTION 专有段落', () => {
      const prompt = buildSystemPrompt(undefined, false);
      expect(prompt).not.toContain('AUTONOMOUS EXECUTION MODE IS CURRENTLY ACTIVE');
      // 但依然包含通用的执行到底与工具使用原则
      expect(prompt).toContain('Autonomous End-to-End Execution Protocol');
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
        environments: [{ name: 'dev', variables: { API_KEY: '123' } }],
        active_environment: 'dev',
        nodes: [
          {
            uuid: 'node-1',
            type: 'http',
            name: 'Login Step',
            position: { x: 100, y: 100 },
            data: { type: 'http', url: 'https://api.com/login', method: 'post' } as HttpConfig,
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
            type: 'http',
            name: 'Broken HTTP Request',
            position: { x: 100, y: 100 },
            data: { type: 'http', url: 'http://127.0.0.1:8080/api', method: 'get' } as HttpConfig,
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
            node_type: 'http',
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
      const res = formatNodeConfigSummary('http', {
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
      expect(formatNodeConfigSummary('http', undefined)).toBe('empty config');
    });
    it('格式化 sub_flow 节点摘要', () => {
      const res = formatNodeConfigSummary('sub_flow', {
        target_flow_path: './subflows/login.json',
        inherit_all_variables: true,
        export_all_variables: true,
      });
      expect(res).toContain('target: ./subflows/login.json');
      expect(res).toContain('inherit_vars: true');
      expect(res).toContain('export_vars: true');
    });
  });
});
