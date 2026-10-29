/**
 * MCP Server Sidecar State Management Store.
 * Zustand 5 store supporting testing isolation via `reset()`.
 */

import { create } from 'zustand';
import type {
  McpPluginStatus,
  McpStartConfig,
  McpServerInstance,
  McpToolInspection,
  McpClientSnippets,
} from '@/types/mcp';
import * as mcpService from '@/services/tauri/mcp';
import { notifyError, notifySuccess } from '@/lib/notify';

interface McpState {
  pluginStatus: McpPluginStatus | null;
  servers: McpServerInstance[];
  loading: boolean;
  error: string | null;
  activeInspection: McpToolInspection | null;
  activeSnippets: McpClientSnippets | null;

  // Actions
  checkPluginStatus: () => Promise<McpPluginStatus>;
  loadServers: () => Promise<void>;
  startServer: (config: McpStartConfig) => Promise<McpServerInstance | null>;
  stopServer: (id: string) => Promise<boolean>;
  removeServer: (id: string) => Promise<boolean>;
  restoreServers: (ids?: string[]) => Promise<number>;
  reloadServer: (id: string) => Promise<boolean>;
  inspectFlow: (flowPath: string) => Promise<McpToolInspection | null>;
  loadClientConfig: (server: McpServerInstance) => Promise<McpClientSnippets | null>;
  installMcpPlugin: () => Promise<boolean>;
  reset: () => void;
}

const initialState = {
  pluginStatus: null,
  servers: [],
  loading: false,
  error: null,
  activeInspection: null,
  activeSnippets: null,
};

export const useMcpStore = create<McpState>((set, get) => ({
  ...initialState,

  checkPluginStatus: async () => {
    try {
      const status = await mcpService.mcpCheckPluginStatus();
      set({ pluginStatus: status, error: null });
      return status;
    } catch (e) {
      const err = e instanceof Error ? e.message : String(e);
      set({ error: err });
      return { installed: false, error: err };
    }
  },

  loadServers: async () => {
    set({ loading: true });
    try {
      const servers = await mcpService.mcpListServers();
      set({ servers, loading: false, error: null });
    } catch (e) {
      const err = e instanceof Error ? e.message : String(e);
      set({ loading: false, error: err });
    }
  },

  startServer: async (config: McpStartConfig) => {
    set({ loading: true, error: null });
    try {
      const instance = await mcpService.mcpStartServer(config);
      await get().loadServers();
      notifySuccess(`MCP server '${instance.name}' started successfully`);
      return instance;
    } catch (e) {
      const err = e instanceof Error ? e.message : String(e);
      set({ loading: false, error: err });
      notifyError('Failed to start MCP server', err);
      return null;
    }
  },

  stopServer: async (id: string) => {
    set({ loading: true });
    try {
      const success = await mcpService.mcpStopServer(id);
      await get().loadServers();
      if (success) {
        notifySuccess('MCP server stopped');
      }
      return success;
    } catch (e) {
      const err = e instanceof Error ? e.message : String(e);
      set({ loading: false, error: err });
      notifyError('Failed to stop MCP server', err);
      return false;
    }
  },

  restoreServers: async (ids?: string[]) => {
    set({ loading: true, error: null });
    try {
      const restored = await mcpService.mcpRestoreServers(ids);
      await get().loadServers();
      if (restored.length > 0) {
        notifySuccess(`Restored ${restored.length} MCP server(s)`);
      }
      return restored.length;
    } catch (e) {
      const err = e instanceof Error ? e.message : String(e);
      set({ loading: false, error: err });
      notifyError('Failed to restore MCP servers', err);
      return 0;
    }
  },

  removeServer: async (id: string) => {
    set({ loading: true });
    try {
      const success = await mcpService.mcpRemoveServer(id);
      await get().loadServers();
      if (success) {
        notifySuccess('MCP server unpublished');
      }
      return success;
    } catch (e) {
      const err = e instanceof Error ? e.message : String(e);
      set({ loading: false, error: err });
      notifyError('Failed to unpublish MCP server', err);
      return false;
    }
  },

  reloadServer: async (id: string) => {
    set({ loading: true });
    try {
      const success = await mcpService.mcpReloadServer(id);
      await get().loadServers();
      if (success) {
        notifySuccess('MCP server reloaded');
      }
      return success;
    } catch (e) {
      const err = e instanceof Error ? e.message : String(e);
      set({ loading: false, error: err });
      notifyError('Failed to reload MCP server', err);
      return false;
    }
  },

  inspectFlow: async (flowPath: string) => {
    try {
      const inspection = await mcpService.mcpInspectFlowTools(flowPath);
      set({ activeInspection: inspection, error: null });
      return inspection;
    } catch (e) {
      const err = e instanceof Error ? e.message : String(e);
      set({ error: err });
      return null;
    }
  },

  loadClientConfig: async (server: McpServerInstance) => {
    try {
      const snippets = await mcpService.mcpGetClientConfig(server);
      set({ activeSnippets: snippets, error: null });
      return snippets;
    } catch (e) {
      const err = e instanceof Error ? e.message : String(e);
      set({ error: err });
      return null;
    }
  },

  installMcpPlugin: async () => {
    set({ loading: true });
    try {
      // Use marketplace installation IPC
      const { installMarketplacePlugin } = await import('@/services/tauri/plugins');
      const registryUrl = 'https://raw.githubusercontent.com/multi-protocol-flow/marketplace/main';
      await installMarketplacePlugin(registryUrl, 'mcp', undefined, true);
      await get().checkPluginStatus();
      set({ loading: false });
      notifySuccess('MCP plugin installed successfully from marketplace');
      return true;
    } catch (e) {
      const err = e instanceof Error ? e.message : String(e);
      set({ loading: false, error: err });
      notifyError('Failed to install MCP plugin', err);
      return false;
    }
  },

  reset: () => {
    set({ ...initialState });
  },
}));
