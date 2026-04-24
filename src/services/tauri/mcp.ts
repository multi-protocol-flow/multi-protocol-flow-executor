/**
 * MCP 服务层 — MCP 协议相关的 Tauri IPC 调用
 */

import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// 类型定义
// ============================================================================

/** MCP 工具信息 */
export interface McpToolInfo {
  name: string;
  description?: string;
  input_schema?: Record<string, unknown>;
}

/** MCP 资源信息 */
export interface McpResourceInfo {
  uri: string;
  name: string;
  description?: string;
  mime_type?: string;
}

/** MCP 提示词参数信息 */
export interface McpPromptArgumentInfo {
  name: string;
  description?: string;
  required?: boolean;
}

/** MCP 提示词信息 */
export interface McpPromptInfo {
  name: string;
  description?: string;
  arguments?: McpPromptArgumentInfo[];
}

/** MCP 服务器信息 */
export interface McpServerInfo {
  name: string;
  version: string;
}

/** MCP 提示词信息 */
export interface McpPromptInfo {
  name: string;
  description?: string;
  arguments?: Array<{
    name: string;
    description?: string;
    required?: boolean;
  }>;
}

/** MCP 发现工具输出 */
export interface McpDiscoverToolsOutput {
  success: boolean;
  tools: McpToolInfo[];
  resources: McpResourceInfo[];
  prompts: McpPromptInfo[];
  server_info?: McpServerInfo;
  error?: string;
}

/** MCP 发现工具输入 */
export interface McpDiscoverToolsInput {
  url: string;
  headers?: Record<string, string>;
  protocol_version?: string;
  /** 传输模式：streamable_http (默认) | stdio */
  transport?: 'streamable_http' | 'stdio';
  /** Stdio 模式：可执行文件路径 */
  command?: string;
  /** Stdio 模式：命令参数 */
  args?: string[];
  /** Stdio 模式：环境变量 */
  env?: Record<string, string>;
}

/**
 * 发现 MCP 服务器的工具和资源
 *
 * 创建临时连接到 MCP 服务器，获取可用工具和资源列表，然后断开连接。
 */
export async function discoverMcpTools(
  input: McpDiscoverToolsInput
): Promise<McpDiscoverToolsOutput> {
  return invoke<McpDiscoverToolsOutput>('mcp_discover_tools', { input });
}
