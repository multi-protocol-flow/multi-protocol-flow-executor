/**
 * McpViewer 组件测试
 * - MCP Connect/Call Tool/Read Resource/Get Prompt/Close 五种节点的执行结果展示
 * - 内容块（text/image/resource）渲染
 * - 消息列表（role + content）渲染
 * - 资源内容（MIME 类型、文本）渲染
 * - 注册表验证
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { McpViewer } from '@/components/execution/viewers/McpViewer';
import { viewerRegistry } from '@/lib/viewerRegistry';
import type { NodeExecutionReport } from '@/types';

// Mock react-json-view-lite
vi.mock('react-json-view-lite', () => ({
  JsonView: ({ data }: { data: unknown }) => (
    <pre data-testid="json-view">{JSON.stringify(data, null, 2)}</pre>
  ),
  allExpanded: {},
  defaultStyles: {},
}));

// Helper: 创建 mock report
function createMockReport(
  nodeType: string,
  outputData?: Record<string, unknown>,
  overrides?: Partial<NodeExecutionReport>,
): NodeExecutionReport {
  return {
    node_uuid: `node-${nodeType}-001`,
    node_type: nodeType,
    node_name: `MCP ${nodeType}`,
    start_time: Date.now() - 200,
    end_time: Date.now(),
    duration_ms: 200,
    status: 'success',
    output_data: outputData,
    ...overrides,
  };
}

// Helper: 展开折叠面板
async function expandSection(user: ReturnType<typeof userEvent.setup>, title: string) {
  const buttons = screen.getAllByRole('button');
  const btn = buttons.find(b => b.textContent?.includes(title));
  if (btn && btn.getAttribute('aria-expanded') === 'false') {
    await user.click(btn);
  }
}

describe('McpViewer', () => {
  describe('MCP Connect 节点', () => {
    it('显示连接配置 URL', () => {
      const report = createMockReport('mcp_connect', {
        success: true,
        url: 'http://localhost:8080/mcp',
        request: { url: 'http://localhost:8080/mcp', transport: 'streamable_http' },
      });
      render(<McpViewer report={report} />);

      expect(screen.getByText('http://localhost:8080/mcp')).toBeDefined();
    });

    it('显示已连接状态和服务器信息', async () => {
      const user = userEvent.setup();
      const report = createMockReport('mcp_connect', {
        success: true,
        connection_id: 'mcp-conn-001',
        server_info: { name: 'test-server', version: '1.0.0' },
      });
      render(<McpViewer report={report} />);

      await expandSection(user, '连接状态');
      expect(screen.getByText('已连接')).toBeDefined();
      expect(screen.getByText(/test-server/)).toBeDefined();
      expect(screen.getByText(/1\.0\.0/)).toBeDefined();
    });

    it('显示连接失败状态', async () => {
      const user = userEvent.setup();
      const report = createMockReport('mcp_connect', {
        success: false,
        error: '连接超时',
      });
      render(<McpViewer report={report} />);

      await expandSection(user, '连接状态');
      expect(screen.getByText('连接失败')).toBeDefined();
    });

    it('显示工具/资源/提示词发现列表', async () => {
      const user = userEvent.setup();
      const report = createMockReport('mcp_connect', {
        success: true,
        tools: [
          { name: 'get_weather', description: '获取天气信息' },
        ],
        resources: [
          { uri: 'file:///data/config.json', name: 'config', mime_type: 'application/json' },
        ],
        prompts: [
          { name: 'greet', description: '问候', arguments: [{ name: 'name', required: true }] },
        ],
      });
      render(<McpViewer report={report} />);

      // 工具列表
      expect(screen.getByText(/工具列表/)).toBeDefined();
      await expandSection(user, '工具列表');
      expect(screen.getByText('get_weather')).toBeDefined();

      // 资源列表
      expect(screen.getByText(/资源列表/)).toBeDefined();
      await expandSection(user, '资源列表');
      expect(screen.getByText('config')).toBeDefined();

      // 提示词列表
      expect(screen.getByText(/提示词列表/)).toBeDefined();
      await expandSection(user, '提示词列表');
      expect(screen.getByText('greet')).toBeDefined();
    });
  });

  describe('MCP Call Tool 节点', () => {
    it('显示工具名称和调用配置', () => {
      const report = createMockReport('mcp_call_tool', {
        success: true,
        tool_name: 'get_weather',
        request: {
          connection_id: 'conn-1',
          tool_name: 'get_weather',
          arguments: { city: 'Beijing' },
        },
      });
      render(<McpViewer report={report} />);

      expect(screen.getByText('get_weather')).toBeDefined();
    });

    it('显示 text 类型的内容块', () => {
      const report = createMockReport('mcp_call_tool', {
        success: true,
        tool_name: 'get_weather',
        result: {
          content: [
            { type: 'text', text: '北京今天晴，气温25°C' },
          ],
          is_error: false,
        },
      });
      render(<McpViewer report={report} />);

      expect(screen.getByText('北京今天晴，气温25°C')).toBeDefined();
    });

    it('显示 image 类型的内容块', () => {
      const report = createMockReport('mcp_call_tool', {
        success: true,
        tool_name: 'generate_image',
        result: {
          content: [
            { type: 'image', mimeType: 'image/png', data: 'base64data...' },
          ],
          is_error: false,
        },
      });
      render(<McpViewer report={report} />);

      expect(screen.getByText('image/png')).toBeDefined();
    });

    it('显示 resource 类型的内容块', () => {
      const report = createMockReport('mcp_call_tool', {
        success: true,
        tool_name: 'read_config',
        result: {
          content: [
            { type: 'resource', resource: { uri: 'file:///config.json', mimeType: 'application/json', text: '{"key":"value"}' } },
          ],
          is_error: false,
        },
      });
      render(<McpViewer report={report} />);

      expect(screen.getByText('file:///config.json')).toBeDefined();
      expect(screen.getByText('application/json')).toBeDefined();
    });

    it('显示调用失败状态', async () => {
      const user = userEvent.setup();
      const report = createMockReport('mcp_call_tool', {
        success: false,
        tool_name: 'bad_tool',
        error: '工具不存在',
      }, { status: 'failed' });
      render(<McpViewer report={report} />);

      await expandSection(user, '调用状态');
      expect(screen.getByText('调用失败')).toBeDefined();
    });
  });

  describe('MCP Read Resource 节点', () => {
    it('显示资源 URI 和 MIME 类型', () => {
      const report = createMockReport('mcp_read_resource', {
        success: true,
        resource_uri: 'file:///data/readme.txt',
        contents: [
          { mimeType: 'text/plain', text: 'Hello World' },
        ],
      });
      render(<McpViewer report={report} />);

      expect(screen.getByText('file:///data/readme.txt')).toBeDefined();
      expect(screen.getByText('text/plain')).toBeDefined();
      expect(screen.getByText('Hello World')).toBeDefined();
    });

    it('显示读取失败状态', async () => {
      const user = userEvent.setup();
      const report = createMockReport('mcp_read_resource', {
        success: false,
        resource_uri: 'file:///not-found',
        error: '资源不存在',
      }, { status: 'failed' });
      render(<McpViewer report={report} />);

      await expandSection(user, '读取状态');
      expect(screen.getByText('读取失败')).toBeDefined();
    });
  });

  describe('MCP Get Prompt 节点', () => {
    it('显示提示词名称和消息列表', () => {
      const report = createMockReport('mcp_get_prompt', {
        success: true,
        prompt_name: 'greet',
        result: {
          description: '问候提示词',
          messages: [
            { role: 'user', content: '你好' },
            { role: 'assistant', content: '你好！有什么可以帮助你的？' },
          ],
        },
      });
      render(<McpViewer report={report} />);

      expect(screen.getByText('greet')).toBeDefined();
      expect(screen.getByText('问候提示词')).toBeDefined();
      expect(screen.getByText('user')).toBeDefined();
      expect(screen.getByText('assistant')).toBeDefined();
      expect(screen.getByText('你好')).toBeDefined();
      expect(screen.getByText('你好！有什么可以帮助你的？')).toBeDefined();
    });

    it('显示获取失败状态', async () => {
      const user = userEvent.setup();
      const report = createMockReport('mcp_get_prompt', {
        success: false,
        prompt_name: 'missing',
        error: '提示词不存在',
      }, { status: 'failed' });
      render(<McpViewer report={report} />);

      await expandSection(user, '获取状态');
      expect(screen.getByText('获取失败')).toBeDefined();
    });
  });

  describe('MCP Close 节点', () => {
    it('显示已关闭状态', async () => {
      const user = userEvent.setup();
      const report = createMockReport('mcp_close', {
        success: true,
        connection_id: 'conn-123',
      });
      render(<McpViewer report={report} />);

      await expandSection(user, '关闭状态');
      expect(screen.getByText('已关闭')).toBeDefined();
    });

    it('显示关闭失败状态', async () => {
      const user = userEvent.setup();
      const report = createMockReport('mcp_close', {
        success: false,
        error: '连接不存在',
      }, { status: 'failed' });
      render(<McpViewer report={report} />);

      await expandSection(user, '关闭状态');
      expect(screen.getByText('关闭失败')).toBeDefined();
    });
  });

  describe('未知节点类型', () => {
    it('显示未知类型提示', () => {
      const report = createMockReport('mcp_unknown', {});
      render(<McpViewer report={report} />);

      expect(screen.getByText(/未知的 MCP 节点类型/)).toBeDefined();
    });
  });

  describe('注册表验证', () => {
    const mcpNodeTypes = ['mcp_connect', 'mcp_call_tool', 'mcp_read_resource', 'mcp_get_prompt', 'mcp_close'];

    it.each(mcpNodeTypes)('MCP 节点 %s 注册到 McpViewer', (nodeType) => {
      expect(viewerRegistry[nodeType]).toBe(McpViewer);
    });

    it('5 个 MCP 节点全部注册', () => {
      const registered = mcpNodeTypes.filter(t => viewerRegistry[t] === McpViewer);
      expect(registered.length).toBe(5);
    });
  });
});

vi.restoreAllMocks();
