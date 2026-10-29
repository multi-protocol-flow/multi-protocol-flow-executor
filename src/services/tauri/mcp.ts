/**
 * Tauri IPC service for MCP Server management and Flow-as-an-MCP-Server export.
 */

import { invoke } from '@tauri-apps/api/core';
import { isTauriEnv } from '@/lib/platform';
import { formatError } from './flow';
import type {
  McpPluginStatus,
  McpStartConfig,
  McpServerInstance,
  McpToolInspection,
  McpClientSnippets,
} from '@/types/mcp';

/**
 * Check if MCP sidecar plugin is installed and ready
 */
export async function mcpCheckPluginStatus(): Promise<McpPluginStatus> {
  if (isTauriEnv()) {
    try {
      return await invoke<McpPluginStatus>('mcp_check_plugin_status');
    } catch (error) {
      throw formatError(error, 'mcp_check_plugin_status');
    }
  }
  return {
    installed: true,
    plugin_path: '/mock/plugins/mcp',
    binary_path: '/mock/plugins/mcp/mpe_plugin_mcp',
    version: '0.1.3',
  };
}

/**
 * List all running MCP server instances managed by host (with self-healing)
 */
export async function mcpListServers(): Promise<McpServerInstance[]> {
  if (isTauriEnv()) {
    try {
      return await invoke<McpServerInstance[]>('mcp_list_servers');
    } catch (error) {
      throw formatError(error, 'mcp_list_servers');
    }
  }
  return [];
}

/**
 * Start a new MCP Server instance
 */
export async function mcpStartServer(config: McpStartConfig): Promise<McpServerInstance> {
  if (isTauriEnv()) {
    try {
      return await invoke<McpServerInstance>('mcp_start_server', { config });
    } catch (error) {
      throw formatError(error, 'mcp_start_server');
    }
  }
  return {
    id: `${config.name}-mock`,
    name: config.name,
    pid: 12345,
    transport: config.transport,
    host: config.host,
    port: config.port,
    flow_target: config.flow_target,
    dir_target: config.dir_target,
    tools_count: 1,
    started_at: Date.now(),
    managed_by: 'gui',
    status: 'running',
  };
}

/**
 * Stop an MCP Server instance by ID or name
 */
export async function mcpStopServer(id: string): Promise<boolean> {
  if (isTauriEnv()) {
    try {
      return await invoke<boolean>('mcp_stop_server', { id });
    } catch (error) {
      throw formatError(error, 'mcp_stop_server');
    }
  }
  return true;
}

/**
 * Unpublish an MCP server: stop it and delete its published record
 */
export async function mcpRemoveServer(id: string): Promise<boolean> {
  if (isTauriEnv()) {
    try {
      return await invoke<boolean>('mcp_remove_server', { id });
    } catch (error) {
      throw formatError(error, 'mcp_remove_server');
    }
  }
  return true;
}

/**
 * Restore published-but-stopped MCP servers (after app restart or reboot).
 * Omit `ids` to restore every published instance.
 */
export async function mcpRestoreServers(ids?: string[]): Promise<McpServerInstance[]> {
  if (isTauriEnv()) {
    try {
      return await invoke<McpServerInstance[]>('mcp_restore_servers', { ids: ids ?? null });
    } catch (error) {
      throw formatError(error, 'mcp_restore_servers');
    }
  }
  return [];
}

/**
 * Reload tools and flows for an MCP server instance
 */
export async function mcpReloadServer(id: string): Promise<boolean> {
  if (isTauriEnv()) {
    try {
      return await invoke<boolean>('mcp_reload_server', { id });
    } catch (error) {
      throw formatError(error, 'mcp_reload_server');
    }
  }
  return true;
}

/**
 * Statically inspect a flow file for MCP Tool definition and parameters
 */
export async function mcpInspectFlowTools(flowPath: string): Promise<McpToolInspection> {
  if (isTauriEnv()) {
    try {
      return await invoke<McpToolInspection>('mcp_inspect_flow_tools', { flowPath });
    } catch (error) {
      throw formatError(error, 'mcp_inspect_flow_tools');
    }
  }
  return {
    tool_name: 'mock_tool',
    description: 'Mock MCP Tool',
    input_schema: { type: 'object', properties: {} },
    variables: [],
  };
}

/**
 * Get client config snippets for Claude Desktop and Cursor
 */
export async function mcpGetClientConfig(
  server: McpServerInstance,
): Promise<McpClientSnippets> {
  if (isTauriEnv()) {
    try {
      return await invoke<McpClientSnippets>('mcp_get_client_config', { server });
    } catch (error) {
      throw formatError(error, 'mcp_get_client_config');
    }
  }
  return {
    claude_desktop: { mcpServers: { [server.name]: { command: 'mpe-plugin-mcp' } } },
    cursor: { mcpServers: { [server.name]: { command: 'mpe-plugin-mcp' } } },
  };
}
