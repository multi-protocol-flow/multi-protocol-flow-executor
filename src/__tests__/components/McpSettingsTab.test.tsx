import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { McpSettingsTab } from '@/components/settings/McpSettingsTab';
import { useMcpStore } from '@/stores/mcp';
import * as mcpService from '@/services/tauri/mcp';
import { useFlowStore } from '@/stores/flow';
import { useUIStore } from '@/stores/ui';
import { flowStorage } from '@/services/storage';

vi.mock('@/services/tauri/mcp');
vi.mock('@/services/storage', () => ({
  flowStorage: {
    listFlows: vi.fn(),
    loadFlow: vi.fn(),
    saveFlow: vi.fn(),
  },
}));

describe('McpSettingsTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useMcpStore.getState().reset();
  });

  it('renders plugin status badge and empty state when no instances', async () => {
    vi.mocked(mcpService.mcpCheckPluginStatus).mockResolvedValue({
      installed: true,
      binary_path: '/usr/local/bin/mpe-plugin-mcp',
      version: '0.1.3',
    });
    vi.mocked(mcpService.mcpListServers).mockResolvedValue([]);

    useMcpStore.setState({
      pluginStatus: {
        installed: true,
        binary_path: '/usr/local/bin/mpe-plugin-mcp',
        version: '0.1.3',
      },
    });

    render(<McpSettingsTab />);

    expect(await screen.findByText('MCP 插件运行状态')).toBeInTheDocument();
    expect(await screen.findByText(/已安装就绪/)).toBeInTheDocument();
    expect(await screen.findByText('暂无活跃的 MCP 服务')).toBeInTheDocument();
  });

  it('renders server instances in table', async () => {
    vi.mocked(mcpService.mcpCheckPluginStatus).mockResolvedValue({
      installed: true,
      binary_path: '/usr/local/bin/mpe-plugin-mcp',
      version: '0.1.3',
    });
    vi.mocked(mcpService.mcpListServers).mockResolvedValue([
      {
        id: 'srv-1',
        name: 'User Flow',
        pid: 3042,
        transport: 'streamable',
        host: '127.0.0.1',
        port: 8000,
        flow_target: 'flows/user.mpf',
        dir_target: null,
        tools_count: 1,
        started_at: Date.now(),
        managed_by: 'gui',
        status: 'running',
        tool_name: 'user_flow',
        slug: 'user-flow',
        shared: true,
      },
    ]);

    useMcpStore.setState({
      pluginStatus: { installed: true },
      servers: [
        {
          id: 'srv-1',
          name: 'User Flow',
          pid: 3042,
          transport: 'streamable',
          host: '127.0.0.1',
          port: 8000,
          flow_target: 'flows/user.mpf',
          dir_target: null,
          tools_count: 1,
          started_at: Date.now(),
          managed_by: 'gui',
          status: 'running',
        },
      ],
    });

    render(<McpSettingsTab />);

    expect(await screen.findByText('User Flow')).toBeInTheDocument();
    expect(screen.getByText('3042')).toBeInTheDocument();
    expect(screen.getByText('http://127.0.0.1:8000/mcp')).toBeInTheDocument();
    // Status is a colour-only dot; text lives in the accessible label.
    expect(screen.getByLabelText('running')).toBeInTheDocument();
  });

  it('renders one gateway endpoint for many flows sharing the port', async () => {
    vi.mocked(mcpService.mcpCheckPluginStatus).mockResolvedValue({ installed: true });
    vi.mocked(mcpService.mcpListServers).mockResolvedValue([
      {
        id: 'flow-a-1',
        name: 'Flow A',
        pid: 500,
        transport: 'streamable',
        host: '127.0.0.1',
        port: 9528,
        flow_target: '/flows/a.mpf',
        dir_target: null,
        tools_count: 1,
        started_at: Date.now(),
        managed_by: 'gui',
        status: 'running',
        tool_name: 'flow_a',
        slug: 'flow-a',
        shared: true,
      },
      {
        id: 'flow-b-1',
        name: 'Flow B',
        pid: 500,
        transport: 'streamable',
        host: '127.0.0.1',
        port: 9528,
        flow_target: '/flows/b.mpf',
        dir_target: null,
        tools_count: 1,
        started_at: Date.now(),
        managed_by: 'gui',
        status: 'running',
        tool_name: 'flow_b',
        slug: 'flow-b',
        shared: true,
      },
    ]);

    useMcpStore.setState({ pluginStatus: { installed: true } });

    render(<McpSettingsTab />);

    // Endpoint is rendered once per gateway, not once per flow.
    expect(await screen.findAllByText('http://127.0.0.1:9528/mcp')).toHaveLength(1);
    expect(screen.getByText('flow_a')).toBeInTheDocument();
    expect(screen.getByText('flow_b')).toBeInTheDocument();
    expect(screen.getByText('http://127.0.0.1:9528/flow-a/mcp')).toBeInTheDocument();
    expect(screen.getByText('http://127.0.0.1:9528/flow-b/mcp')).toBeInTheDocument();
  });

  it('restores a stopped published instance from its row', async () => {
    vi.mocked(mcpService.mcpCheckPluginStatus).mockResolvedValue({ installed: true });
    vi.mocked(mcpService.mcpRestoreServers).mockResolvedValue([]);
    vi.mocked(mcpService.mcpListServers).mockResolvedValue([
      {
        id: 'flow-a-1',
        name: 'Flow A',
        pid: 0,
        transport: 'streamable',
        host: '127.0.0.1',
        port: 9528,
        flow_target: '/flows/a.mpf',
        dir_target: null,
        tools_count: 1,
        started_at: Date.now(),
        managed_by: 'gui',
        status: 'stopped',
        tool_name: 'flow_a',
        slug: 'flow-a',
        shared: true,
      },
    ]);

    useMcpStore.setState({ pluginStatus: { installed: true } });

    render(<McpSettingsTab />);

    const restoreBtn = await screen.findByTitle('恢复服务（重新拉起）');
    fireEvent.click(restoreBtn);

    await vi.waitFor(() => {
      expect(mcpService.mcpRestoreServers).toHaveBeenCalledWith(['flow-a-1']);
    });
  });

  it('opens the flow on the canvas when its name is clicked', async () => {
    vi.mocked(mcpService.mcpCheckPluginStatus).mockResolvedValue({ installed: true });
    vi.mocked(mcpService.mcpListServers).mockResolvedValue([
      {
        id: 'flow-a-1',
        name: 'Flow A',
        pid: 500,
        transport: 'streamable',
        host: '127.0.0.1',
        port: 9528,
        flow_target: '/flows/a.mpf',
        dir_target: null,
        tools_count: 1,
        started_at: Date.now(),
        managed_by: 'gui',
        status: 'running',
        tool_name: 'flow_a',
        slug: 'flow-a',
        shared: true,
      },
    ]);
    vi.mocked(flowStorage.listFlows).mockResolvedValue([
      { uuid: 'flow-a-uuid', name: 'Flow A', filePath: '/flows/a.mpf' } as never,
    ]);
    vi.mocked(flowStorage.loadFlow).mockResolvedValue({
      uuid: 'flow-a-uuid',
      name: 'Flow A',
      nodes: [],
      connections: [],
    } as never);

    useMcpStore.setState({ pluginStatus: { installed: true } });
    useUIStore.setState({ showSettingsDialog: true });
    useFlowStore.setState({ tabs: [], activeTabUuid: null });

    render(<McpSettingsTab />);

    fireEvent.click(await screen.findByText('Flow A'));

    await vi.waitFor(() => {
      expect(useFlowStore.getState().tabs.some((t) => t.uuid === 'flow-a-uuid')).toBe(true);
    });
    expect(useUIStore.getState().showSettingsDialog).toBe(false);
  });

  it('unpublishes an instance from the trash action', async () => {
    vi.mocked(mcpService.mcpCheckPluginStatus).mockResolvedValue({ installed: true });
    vi.mocked(mcpService.mcpRemoveServer).mockResolvedValue(true);
    vi.mocked(mcpService.mcpListServers).mockResolvedValue([
      {
        id: 'flow-a-1',
        name: 'Flow A',
        pid: 500,
        transport: 'streamable',
        host: '127.0.0.1',
        port: 9528,
        flow_target: '/flows/a.mpf',
        dir_target: null,
        tools_count: 1,
        started_at: Date.now(),
        managed_by: 'gui',
        status: 'running',
        tool_name: 'flow_a',
        slug: 'flow-a',
        shared: true,
      },
    ]);

    useMcpStore.setState({ pluginStatus: { installed: true } });

    render(<McpSettingsTab />);

    fireEvent.click(await screen.findByTitle('取消发布（删除发布记录）'));

    await vi.waitFor(() => {
      expect(mcpService.mcpRemoveServer).toHaveBeenCalledWith('flow-a-1');
    });
  });

  it('triggers stopServer when stop button is clicked', async () => {
    const testServers = [
      {
        id: 'srv-stop',
        name: 'To Stop Flow',
        pid: 5055,
        transport: 'streamable' as const,
        host: '127.0.0.1',
        port: 8005,
        flow_target: 'flows/stop.mpf',
        dir_target: null,
        tools_count: 1,
        started_at: Date.now(),
        managed_by: 'gui',
        status: 'running' as const,
      },
    ];
    vi.mocked(mcpService.mcpCheckPluginStatus).mockResolvedValue({
      installed: true,
    });
    vi.mocked(mcpService.mcpStopServer).mockResolvedValue(true);
    vi.mocked(mcpService.mcpListServers).mockResolvedValue(testServers);

    useMcpStore.setState({
      pluginStatus: { installed: true },
      servers: testServers,
    });

    render(<McpSettingsTab />);

    const stopBtn = await screen.findByTitle(/^停止服务/);
    fireEvent.click(stopBtn);

    await vi.waitFor(() => {
      expect(mcpService.mcpStopServer).toHaveBeenCalledWith('srv-stop');
    });
  });
});
