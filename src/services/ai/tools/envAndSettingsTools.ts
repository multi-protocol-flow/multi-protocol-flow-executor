/**
 * AI Environment & Settings Management Tools
 */

import { useEnvironmentStore } from '@/stores/environment';
import { useSettingsStore } from '@/stores/settings';
import { environmentStorage } from '@/services/storage/environmentStorage';
import type { EnvironmentSet } from '@/types';
import type { ThemeMode, Locale } from '@/types/settings';
import type { AIToolDefinition } from '@/types/ai';

export const envAndSettingsToolDefinitions: AIToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'list_environments',
      description:
        'List all environments with their shared base variables, plus the currently active environment. Machine-local override values (.local.yaml) are reported as a flag only and never exposed.',
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
      description:
        'Add or update a variable in the shared base environment file (<environments_dir>/<name>.yaml). The machine-local override file (<name>.local.yaml) is neither modified nor read by this tool, so private values stay machine-private.',
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
      description:
        'Switch the globally active environment (shared by every flow, not flow-scoped). Its variables are merged into executions after flow fallback variables and before runtime overrides.',
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
    await useEnvironmentStore.getState().loadEnvironments();
    // `loadEnvironments` 会替换 store 状态，必须在 await 之后重新取快照，
    // 否则读到的是加载前的空列表。
    const envStore = useEnvironmentStore.getState();
    const envsWithVars: Array<{
      name: string;
      variable_count: number;
      variables: Record<string, unknown>;
      has_local_override: boolean;
    }> = [];

    for (const meta of envStore.environments) {
      // 只读共享的 base 文件：`loadEnvironment(name)` 会自动叠加 `.local.yaml`，
      // 把机器私有覆盖（常含密钥）回传给模型等于把密钥发给模型供应商。
      let variables: Record<string, unknown> = {};
      try {
        const baseEnv = await environmentStorage.loadEnvironment(meta.name, false);
        variables = baseEnv.variables || {};
      } catch {
        // base 文件缺失（例如仅存在 .local 覆盖）时退化为元数据
      }
      envsWithVars.push({
        name: meta.name,
        variable_count: Object.keys(variables).length,
        variables,
        has_local_override: meta.hasLocalOverride,
      });
    }

    return {
      success: true,
      active_environment: envStore.activeEnvironmentName,
      environments: envsWithVars,
    };
  },

  async set_environment_variable(args: {
    env_name: string;
    key: string;
    value: string;
  }) {
    const envStore = useEnvironmentStore.getState();
    let targetEnv: EnvironmentSet;
    try {
      // 与读取同理：必须显式取 base（isLocal=false）。若读取合并结果再写回 base，
      // `.local.yaml` 的私有值会被固化进会进 git 的共享环境文件。
      targetEnv = await environmentStorage.loadEnvironment(args.env_name, false);
    } catch {
      targetEnv = { name: args.env_name, variables: {} };
    }

    targetEnv.variables = {
      ...targetEnv.variables,
      [args.key]: args.value,
    };

    await envStore.saveEnvironment(targetEnv, false);

    return {
      success: true,
      env_name: args.env_name,
      key: args.key,
      value: args.value,
    };
  },

  async switch_active_environment(args: { env_name?: string | null }) {
    const envStore = useEnvironmentStore.getState();
    const targetName = args.env_name ? args.env_name : null;
    await envStore.setActiveEnvironment(targetName);

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
