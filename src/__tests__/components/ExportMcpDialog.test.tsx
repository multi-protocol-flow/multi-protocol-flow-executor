import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ExportMcpDialog } from '@/components/dialogs/ExportMcpDialog';
import { useUIStore } from '@/stores/ui';
import { useMcpStore } from '@/stores/mcp';
import { useSettingsStore } from '@/stores/settings';
import * as mcpService from '@/services/tauri/mcp';

vi.mock('@/services/tauri/mcp');

describe('ExportMcpDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useUIStore.setState({
      showExportMcpDialog: true,
      exportMcpTarget: {
        uuid: 'test-flow-uuid',
        name: 'Demo Flow',
        nodes: [],
        connections: [],
      },
    });
    useMcpStore.getState().reset();
    useSettingsStore.setState({ defaultMcpPort: 9528, defaultIngressPort: 9527 });
  });

  /** Flush the inspect/list promises triggered by the dialog's mount effect. */
  async function primeDialog(toolName = 'demo_flow') {
    vi.mocked(mcpService.mcpCheckPluginStatus).mockResolvedValue({ installed: false });
    vi.mocked(mcpService.mcpListServers).mockResolvedValue([]);
    vi.mocked(mcpService.mcpInspectFlowTools).mockResolvedValue({
      tool_name: toolName,
      description: 'Execute Demo Flow',
      input_schema: { type: 'object', properties: {} },
      variables: [],
    });
    render(<ExportMcpDialog />);
    await screen.findByText('发布流程为 MCP Server');
    await vi.waitFor(() =>
      expect(mcpService.mcpInspectFlowTools).toHaveBeenCalled(),
    );
  }

  it('renders uninstalled banner when plugin is not installed', async () => {
    vi.mocked(mcpService.mcpCheckPluginStatus).mockResolvedValue({
      installed: false,
      error: 'Not installed',
    });
    vi.mocked(mcpService.mcpListServers).mockResolvedValue([]);

    useMcpStore.setState({
      pluginStatus: { installed: false, error: 'Not installed' },
    });

    render(<ExportMcpDialog />);

    expect(await screen.findByText('发布流程为 MCP Server')).toBeInTheDocument();
    expect(await screen.findByText('MCP 客户端节点插件未安装')).toBeInTheDocument();
    expect(screen.getByText('一键从市场安装')).toBeInTheDocument();
  });

  it('renders tool name and client configurations when plugin is installed', async () => {
    vi.mocked(mcpService.mcpCheckPluginStatus).mockResolvedValue({
      installed: true,
      binary_path: '/usr/local/bin/mpe-plugin-mcp',
      version: '0.1.3',
    });
    vi.mocked(mcpService.mcpListServers).mockResolvedValue([]);
    vi.mocked(mcpService.mcpInspectFlowTools).mockResolvedValue({
      tool_name: 'demo_flow',
      description: 'Execute Demo Flow',
      input_schema: {
        type: 'object',
        properties: {
          user_id: { type: 'string', default: '100' },
        },
      },
      variables: ['user_id'],
    });

    useMcpStore.setState({
      pluginStatus: {
        installed: true,
        binary_path: '/usr/local/bin/mpe-plugin-mcp',
        version: '0.1.3',
      },
    });

    render(<ExportMcpDialog />);

    expect(await screen.findByText('发布流程为 MCP Server')).toBeInTheDocument();
    expect(await screen.findByText('demo_flow')).toBeInTheDocument();
    expect(screen.getAllByText(/Claude Desktop/).length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText(/Cursor/).length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('复制配置 JSON')).toBeInTheDocument();
  });

  it('supports switching to Streamable HTTP mode', async () => {
    vi.mocked(mcpService.mcpCheckPluginStatus).mockResolvedValueOnce({
      installed: true,
      binary_path: '/usr/local/bin/mpe-plugin-mcp',
      version: '0.1.3',
    });
    vi.mocked(mcpService.mcpListServers).mockResolvedValueOnce([]);
    vi.mocked(mcpService.mcpInspectFlowTools).mockResolvedValueOnce({
      tool_name: 'demo_flow',
      description: 'Execute Demo Flow',
      input_schema: { type: 'object', properties: {} },
      variables: [],
    });

    render(<ExportMcpDialog />);

    const streamableTabBtn = await screen.findByText(/Streamable HTTP 模式/);
    fireEvent.click(streamableTabBtn);

    expect(screen.getByText('本地一键启动 Streamable HTTP 服务')).toBeInTheDocument();
  });

  it('keeps Cursor on stdio so no port is opened', async () => {
    await primeDialog();
    fireEvent.click(screen.getByText('Cursor'));

    const snippet = document.querySelector('pre')?.textContent ?? '';
    const config = JSON.parse(snippet) as {
      mcpServers: Record<string, { command: string; args: string[] }>;
    };
    const entry = Object.values(config.mcpServers)[0];

    expect(entry.command).toBe('mpe');
    expect(entry.args).toEqual([
      'mcp',
      'serve',
      '--flow',
      expect.any(String),
      '--transport',
      'stdio',
    ]);
  });

  it('joins the shared gateway on the configured default port by default', async () => {
    await primeDialog();
    fireEvent.click(screen.getByText(/Streamable HTTP 模式/));

    // Default gateway port with the aggregate endpoint: one registration
    // discovers every published flow's tools.
    expect(await screen.findByText('http://127.0.0.1:9528/mcp')).toBeInTheDocument();
    expect(screen.getByText('http://127.0.0.1:9528/demo-flow/mcp')).toBeInTheDocument();

    vi.mocked(mcpService.mcpStartServer).mockResolvedValue({
      id: 'demo_flow-1',
      name: 'demo_flow',
      pid: 123,
      transport: 'streamable',
      host: '127.0.0.1',
      port: 9528,
      flow_target: '/tmp/demo.mpf',
      dir_target: null,
      tools_count: 1,
      started_at: Date.now(),
      managed_by: 'gui',
      status: 'running',
      tool_name: 'demo_flow',
      slug: 'demo-flow',
      shared: true,
    });

    fireEvent.click(screen.getByText('本地一键启动 Streamable HTTP 服务'));

    await vi.waitFor(() => expect(mcpService.mcpStartServer).toHaveBeenCalled());
    expect(vi.mocked(mcpService.mcpStartServer).mock.calls[0][0]).toMatchObject({
      transport: 'streamable',
      port: 9528,
      shared: true,
    });
  });
});
