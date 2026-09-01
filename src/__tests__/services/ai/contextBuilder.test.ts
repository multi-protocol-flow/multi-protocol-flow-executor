import { describe, it, expect, beforeEach } from 'vitest';
import {
  buildSystemPrompt,
  buildActiveStateSummary,
} from '@/services/ai/contextBuilder';
import { useFlowStore } from '@/stores/flow';
import type { Flow, HttpConfig, ScriptNodeConfig } from '@/types';

describe('contextBuilder', () => {
  beforeEach(() => {
    useFlowStore.getState().reset();
  });

  describe('buildSystemPrompt', () => {
    it('包含 MPE 核心架构说明和 JavaScript 沙箱规范', () => {
      const prompt = buildSystemPrompt();
      expect(prompt).toContain('MPE Copilot & Flow Architect');
      expect(prompt).toContain('context.steps.<step_id>.response.body');
      expect(prompt).toContain('RouteToFalse');
      expect(prompt).toContain('auto_layout');
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
    });
  });
});
