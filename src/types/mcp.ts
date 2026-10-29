/**
 * MCP Server Sidecar types for MPE flow export & governance.
 * Strictly aligned with `flow-engine/src/mcp_supervisor/mod.rs`.
 */

/**
 * Default port of the shared MCP gateway (mirrors `flow-engine` `DEFAULT_MCP_PORT`).
 * Sits next to the Ingress gateway default (9527).
 */
export const DEFAULT_MCP_PORT = 9528;

export interface McpPluginStatus {
  installed: boolean;
  plugin_path?: string | null;
  binary_path?: string | null;
  version?: string | null;
  error?: string | null;
}

export interface McpStartConfig {
  name: string;
  flow_target?: string | null;
  dir_target?: string | null;
  transport: 'stdio' | 'streamable';
  host: string;
  port: number;
  timeout_ms: number;
  managed_by: 'gui' | 'cli';
  /** 加入共享网关（按 `/{slug}/mcp` 路由隔离），而非独占端口 */
  shared?: boolean;
}

export interface McpServerInstance {
  id: string;
  name: string;
  pid: number;
  transport: 'stdio' | 'streamable';
  host: string;
  port: number;
  flow_target?: string | null;
  dir_target?: string | null;
  tools_count: number;
  started_at: number;
  managed_by: string;
  status: 'running' | 'stopped' | 'error';
  /** 注册的 MCP 工具名 */
  tool_name?: string | null;
  /** 共享网关下的路由 slug（`/{slug}/mcp`） */
  slug?: string | null;
  /** 是否宿于共享网关端口 */
  shared?: boolean;
}

export interface McpToolInspection {
  tool_name: string;
  description: string;
  input_schema: Record<string, unknown>;
  variables: string[];
}

export interface McpClientSnippets {
  claude_desktop: Record<string, unknown>;
  cursor: Record<string, unknown>;
}
