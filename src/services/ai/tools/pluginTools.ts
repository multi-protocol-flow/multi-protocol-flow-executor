/**
 * AI Protocol & Extension Plugin Tools
 */

import { usePluginsStore } from '@/stores/plugins';
import { useSettingsStore } from '@/stores/settings';
import type { AIToolDefinition } from '@/types/ai';

export const pluginToolDefinitions: AIToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'list_plugins',
      description: 'List all currently installed protocol and capability plugins in MPE.',
      parameters: {
        type: 'object',
        properties: {},
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'install_marketplace_plugin',
      description: 'Download and install a plugin from the plugin marketplace. [HIGH RISK]',
      parameters: {
        type: 'object',
        properties: {
          plugin_name: {
            type: 'string',
            description: 'Name of the plugin to install (e.g. mpe-plugin-redis, mpe-plugin-grpc).',
          },
          version: {
            type: 'string',
            description: 'Optional specific version to install.',
          },
          marketplace_url: {
            type: 'string',
            description: 'Optional custom marketplace URL override.',
          },
        },
        required: ['plugin_name'],
      },
    },
  },
];

export const pluginToolHandlers = {
  async list_plugins() {
    const pluginsStore = usePluginsStore.getState();
    await pluginsStore.loadInstalled();

    const installed = pluginsStore.installed || [];

    return {
      success: true,
      count: installed.length,
      installed: installed.map((p) => ({
        name: p.name,
        version: p.version,
        description: p.description,
        path: p.path,
        status: p.status,
        error: p.error,
      })),
    };
  },

  async install_marketplace_plugin(args: {
    plugin_name: string;
    version?: string;
    marketplace_url?: string;
  }) {
    const pluginsStore = usePluginsStore.getState();
    const settingsStore = useSettingsStore.getState();

    const url =
      args.marketplace_url ||
      settingsStore.pluginRegistryUrl ||
      'https://multi-protocol-flow.github.io/mpe-plugin-registry';

    await pluginsStore.installMarketplacePlugin(
      url,
      args.plugin_name,
      args.version,
      true,
    );

    return {
      success: true,
      plugin_name: args.plugin_name,
      version: args.version || 'latest',
      message: `Plugin "${args.plugin_name}" installed successfully.`,
    };
  },
};
