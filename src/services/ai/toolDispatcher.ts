import type { AIToolDefinition, AIToolCall, AIPendingAction } from '@/types/ai';

/**
 * AI Tool Registry & Dispatcher
 */

import { useFlowStore } from '@/stores/flow';
import { getLL } from '@/i18n/i18n-lazy';

import { canvasToolDefinitions, canvasToolHandlers } from './tools/canvasTools';
import { scriptToolDefinitions, scriptToolHandlers } from './tools/scriptTools';
import { flowFileToolDefinitions, flowFileToolHandlers } from './tools/flowFileTools';
import {
  datasourcePluginToolDefinitions,
  datasourcePluginToolHandlers,
} from './tools/datasourcePluginTools';
import { pluginToolDefinitions, pluginToolHandlers } from './tools/pluginTools';
import {
  envAndSettingsToolDefinitions,
  envAndSettingsToolHandlers,
} from './tools/envAndSettingsTools';
import {
  executionToolDefinitions,
  executionToolHandlers,
} from './tools/executionTools';

/**
 * 获取所有可供 LLM 调用的工具定义列表
 */
export function getAllAIToolDefinitions(): AIToolDefinition[] {
  return [
    ...canvasToolDefinitions,
    ...scriptToolDefinitions,
    ...flowFileToolDefinitions,
    ...datasourcePluginToolDefinitions,
    ...pluginToolDefinitions,
    ...envAndSettingsToolDefinitions,
    ...executionToolDefinitions,
  ];
}

/**
 * 所有工具的执行 Handler 映射表
 */
const ALL_HANDLERS: Record<
  string,
  (args: Record<string, unknown>) => Promise<unknown>
> = {
  add_node: (args) =>
    canvasToolHandlers.add_node(
      args as Parameters<typeof canvasToolHandlers.add_node>[0],
    ),
  update_node_config: (args) =>
    canvasToolHandlers.update_node_config(
      args as Parameters<typeof canvasToolHandlers.update_node_config>[0],
    ),
  connect_nodes: (args) =>
    canvasToolHandlers.connect_nodes(
      args as Parameters<typeof canvasToolHandlers.connect_nodes>[0],
    ),
  disconnect_nodes: (args) =>
    canvasToolHandlers.disconnect_nodes(
      args as Parameters<typeof canvasToolHandlers.disconnect_nodes>[0],
    ),
  auto_layout: (args) =>
    canvasToolHandlers.auto_layout(
      args as Parameters<typeof canvasToolHandlers.auto_layout>[0],
    ),
  delete_nodes: (args) =>
    canvasToolHandlers.delete_nodes(
      args as Parameters<typeof canvasToolHandlers.delete_nodes>[0],
    ),
  clear_canvas: () => canvasToolHandlers.clear_canvas(),

  generate_or_update_script: (args) =>
    scriptToolHandlers.generate_or_update_script(
      args as Parameters<typeof scriptToolHandlers.generate_or_update_script>[0],
    ),

  list_flows: () => flowFileToolHandlers.list_flows(),
  create_flow: (args) =>
    flowFileToolHandlers.create_flow(
      args as Parameters<typeof flowFileToolHandlers.create_flow>[0],
    ),
  open_flow: (args) =>
    flowFileToolHandlers.open_flow(
      args as Parameters<typeof flowFileToolHandlers.open_flow>[0],
    ),
  save_current_flow: () => flowFileToolHandlers.save_current_flow(),

  list_datasource_plugins: () =>
    datasourcePluginToolHandlers.list_datasource_plugins(),
  browse_datasource: (args) =>
    datasourcePluginToolHandlers.browse_datasource(
      args as Parameters<typeof datasourcePluginToolHandlers.browse_datasource>[0],
    ),
  import_datasource_item_to_flow: (args) =>
    datasourcePluginToolHandlers.import_datasource_item_to_flow(
      args as Parameters<typeof datasourcePluginToolHandlers.import_datasource_item_to_flow>[0],
    ),

  list_plugins: () => pluginToolHandlers.list_plugins(),
  install_marketplace_plugin: (args) =>
    pluginToolHandlers.install_marketplace_plugin(
      args as Parameters<typeof pluginToolHandlers.install_marketplace_plugin>[0],
    ),

  list_environments: () => envAndSettingsToolHandlers.list_environments(),
  set_environment_variable: (args) =>
    envAndSettingsToolHandlers.set_environment_variable(
      args as Parameters<typeof envAndSettingsToolHandlers.set_environment_variable>[0],
    ),
  switch_active_environment: (args) =>
    envAndSettingsToolHandlers.switch_active_environment(
      args as Parameters<typeof envAndSettingsToolHandlers.switch_active_environment>[0],
    ),
  update_theme_or_locale: (args) =>
    envAndSettingsToolHandlers.update_theme_or_locale(
      args as Parameters<typeof envAndSettingsToolHandlers.update_theme_or_locale>[0],
    ),

  run_flow_debug: () => executionToolHandlers.run_flow_debug(),
  cancel_flow_execution: () => executionToolHandlers.cancel_flow_execution(),
  get_last_execution_report: () =>
    executionToolHandlers.get_last_execution_report(),
};

/**
 * 判断是否为高风险操作（需 UI 拦截确认）
 */
export function isHighRiskTool(toolName: string): boolean {
  return [
    'delete_nodes',
    'clear_canvas',
    'install_marketplace_plugin',
  ].includes(toolName);
}

/**
 * 构造高风险操作的展示描述与 Diff Summary
 */
export function buildActionDescription(
  toolName: string,
  args: Record<string, unknown>,
): { description: string; diffSummary?: string } {
  const ll = getLL();
  const currentFlow = useFlowStore.getState().flow;

  switch (toolName) {
    case 'clear_canvas':
      return {
        description: ll.ai.confirm_card.clear_canvas_desc(),
        diffSummary: ll.ai.confirm_card.clear_canvas_diff({
          nodes: currentFlow?.nodes.length ?? 0,
          connections: currentFlow?.connections.length ?? 0,
        }),
      };

    case 'delete_nodes': {
      const uuids = Array.isArray(args.node_uuids) ? (args.node_uuids as string[]) : [];
      const nodeNames = uuids
        .map((id) => currentFlow?.nodes.find((n) => n.uuid === id)?.name || id)
        .join(', ');
      return {
        description: ll.ai.confirm_card.delete_nodes_desc({ count: uuids.length }),
        diffSummary: ll.ai.confirm_card.delete_nodes_diff({ names: nodeNames }),
      };
    }

    case 'install_marketplace_plugin':
      return {
        description: ll.ai.confirm_card.install_plugin_desc({ name: String(args.plugin_name || '') }),
        diffSummary: ll.ai.confirm_card.install_plugin_diff({
          version: String(args.version || 'latest'),
          url: String(args.marketplace_url || 'official marketplace'),
        }),
      };

    default:
      return {
        description: `Execute: ${toolName}`,
      };
  }
}

/**
 * 创建高风险待确认操作对象
 */
export function createPendingAction(
  toolCall: AIToolCall,
  parsedArgs: Record<string, unknown>,
): AIPendingAction {
  const { description, diffSummary } = buildActionDescription(
    toolCall.function.name,
    parsedArgs,
  );

  return {
    id: `action_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`,
    toolCallId: toolCall.id,
    toolName: toolCall.function.name,
    args: parsedArgs,
    description,
    diffSummary,
    status: 'pending',
    createdAt: Date.now(),
  };
}

/**
 * 分发并执行工具调用
 */
export async function dispatchToolCall(
  toolName: string,
  args: Record<string, unknown>,
): Promise<unknown> {
  const handler = ALL_HANDLERS[toolName];
  if (!handler) {
    throw new Error(`Unknown AI tool: "${toolName}"`);
  }

  return handler(args);
}
