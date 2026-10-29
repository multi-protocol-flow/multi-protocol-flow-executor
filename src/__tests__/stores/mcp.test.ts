import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useMcpStore } from '@/stores/mcp';
import * as mcpService from '@/services/tauri/mcp';

vi.mock('@/services/tauri/mcp');

describe('useMcpStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useMcpStore.getState().reset();
  });

  it('initial state should be empty and not loading', () => {
    const state = useMcpStore.getState();
    expect(state.pluginStatus).toBeNull();
    expect(state.servers).toEqual([]);
    expect(state.loading).toBe(false);
    expect(state.error).toBeNull();
  });

  it('checkPluginStatus updates pluginStatus', async () => {
    vi.mocked(mcpService.mcpCheckPluginStatus).mockResolvedValueOnce({
      installed: true,
      binary_path: '/path/to/binary',
      version: '0.1.3',
    });

    const status = await useMcpStore.getState().checkPluginStatus();
    expect(status.installed).toBe(true);
    expect(useMcpStore.getState().pluginStatus?.binary_path).toBe('/path/to/binary');
  });

  it('loadServers updates servers array', async () => {
    vi.mocked(mcpService.mcpListServers).mockResolvedValueOnce([
      {
        id: 's1',
        name: 'test-server',
        pid: 1001,
        transport: 'streamable',
        host: '127.0.0.1',
        port: 8000,
        tools_count: 2,
        started_at: 1000,
        managed_by: 'gui',
        status: 'running',
      },
    ]);

    await useMcpStore.getState().loadServers();
    expect(useMcpStore.getState().servers).toHaveLength(1);
    expect(useMcpStore.getState().servers[0].id).toBe('s1');
  });

  it('startServer triggers service and refreshes servers', async () => {
    vi.mocked(mcpService.mcpStartServer).mockResolvedValueOnce({
      id: 's2',
      name: 'new-server',
      pid: 1002,
      transport: 'streamable',
      host: '127.0.0.1',
      port: 8001,
      tools_count: 1,
      started_at: 2000,
      managed_by: 'gui',
      status: 'running',
    });
    vi.mocked(mcpService.mcpListServers).mockResolvedValueOnce([]);

    const res = await useMcpStore.getState().startServer({
      name: 'new-server',
      transport: 'streamable',
      host: '127.0.0.1',
      port: 8001,
      timeout_ms: 28000,
      managed_by: 'gui',
    });

    expect(res).not.toBeNull();
    expect(res?.name).toBe('new-server');
  });

  it('reset() resets store state for test isolation', () => {
    useMcpStore.setState({
      loading: true,
      error: 'some error',
      servers: [
        {
          id: 'test',
          name: 'test',
          pid: 1,
          transport: 'stdio',
          host: '',
          port: 0,
          tools_count: 0,
          started_at: 0,
          managed_by: 'gui',
          status: 'running',
        },
      ],
    });

    useMcpStore.getState().reset();
    const state = useMcpStore.getState();
    expect(state.loading).toBe(false);
    expect(state.error).toBeNull();
    expect(state.servers).toEqual([]);
  });
});
