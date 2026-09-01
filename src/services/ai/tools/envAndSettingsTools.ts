/**
 * AI Environment & Settings Management Tools
 */

import { useFlowStore } from '@/stores/flow';
import { useSettingsStore } from '@/stores/settings';
import type { EnvironmentSet } from '@/types';
import type { ThemeMode, Locale } from '@/types/settings';
import type { AIToolDefinition } from '@/types/ai';

export const envAndSettingsToolDefinitions: AIToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'list_environments',
      description: 'List all environments, their variables, and the currently active environment in the active flow.',
      parameters: {
        type: 'object',
        properties: {},
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'set_environment_variable',
      description: 'Add or update an environment variable in a specific environment set.',
      parameters: {
        type: 'object',
        properties: {
          env_name: {
            type: 'string',
            description: 'The name of the target environment (e.g. dev, test, prod). Creates the environment if not present.',
          },
          key: {
            type: 'string',
            description: 'The variable name key (e.g. BASE_URL, API_TOKEN).',
          },
          value: {
            type: 'string',
            description: 'The variable value.',
          },
        },
        required: ['env_name', 'key', 'value'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'switch_active_environment',
      description: 'Switch the currently active environment in the flow.',
      parameters: {
        type: 'object',
        properties: {
          env_name: {
            type: 'string',
            description: 'The environment name to activate, or empty string/null to deactivate.',
          },
        },
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'update_theme_or_locale',
      description: 'Change the application appearance theme or display language.',
      parameters: {
        type: 'object',
        properties: {
          theme: {
            type: 'string',
            enum: ['light', 'dark', 'system'],
            description: 'Appearance theme.',
          },
          locale: {
            type: 'string',
            enum: ['zh-CN', 'en-US'],
            description: 'UI language locale.',
          },
        },
      },
    },
  },
];

export const envAndSettingsToolHandlers = {
  async list_environments() {
    const flow = useFlowStore.getState().flow;
    if (!flow) {
      throw new Error('No active flow.');
    }

    return {
      success: true,
      active_environment: flow.active_environment,
      environments: (flow.environments || []).map((env) => ({
        name: env.name,
        variable_count: Object.keys(env.variables || {}).length,
        variables: env.variables,
      })),
    };
  },

  async set_environment_variable(args: {
    env_name: string;
    key: string;
    value: string;
  }) {
    const flowState = useFlowStore.getState();
    const flow = flowState.flow;
    if (!flow) {
      throw new Error('No active flow.');
    }

    const currentEnvs = [...(flow.environments || [])];
    const existingIndex = currentEnvs.findIndex((e) => e.name === args.env_name);

    if (existingIndex >= 0) {
      const targetEnv = currentEnvs[existingIndex];
      const updatedEnv: EnvironmentSet = {
        ...targetEnv,
        variables: {
          ...targetEnv.variables,
          [args.key]: args.value,
        },
      };
      currentEnvs[existingIndex] = updatedEnv;
    } else {
      currentEnvs.push({
        name: args.env_name,
        variables: {
          [args.key]: args.value,
        },
      });
    }

    flowState.setEnvironments(currentEnvs);

    return {
      success: true,
      env_name: args.env_name,
      key: args.key,
      value: args.value,
    };
  },

  async switch_active_environment(args: { env_name?: string | null }) {
    const flowState = useFlowStore.getState();
    const flow = flowState.flow;
    if (!flow) {
      throw new Error('No active flow.');
    }

    const targetName = args.env_name ? args.env_name : null;
    flowState.setActiveEnvironment(targetName);

    return {
      success: true,
      active_environment: targetName,
    };
  },

  async update_theme_or_locale(args: {
    theme?: ThemeMode;
    locale?: Locale;
  }) {
    const settingsStore = useSettingsStore.getState();
    if (args.theme) {
      await settingsStore.setTheme(args.theme);
    }
    if (args.locale) {
      await settingsStore.setLocale(args.locale);
    }

    return {
      success: true,
      theme: args.theme || settingsStore.theme,
      locale: args.locale || settingsStore.locale,
    };
  },
};
