/**
 * AI 环境工具单元测试
 *
 * 重点覆盖 `.local.yaml` 私有层与 AI 写路径的隔离：AI 只能读写共享的 base 环境文件。
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { environmentStorageMock } from '@/services/storage/environmentStorage.mock';
import { environmentStorage } from '@/services/storage/environmentStorage';
import { useEnvironmentStore } from '@/stores/environment';
import { envAndSettingsToolHandlers } from '@/services/ai/tools/envAndSettingsTools';

describe('envAndSettingsTools', () => {
  beforeEach(() => {
    localStorage.clear();
    environmentStorageMock.reset();
    useEnvironmentStore.getState().reset();
  });

  it('set_environment_variable 不得把本地私有覆盖写进共享环境文件', async () => {
    await environmentStorageMock.saveEnvironment(
      { name: 'dev', variables: { API: 'https://dev.api' } },
      false,
    );
    await environmentStorageMock.saveEnvironment(
      { name: 'dev', variables: { LOCAL_SECRET: 'machine-private' } },
      true,
    );

    await envAndSettingsToolHandlers.set_environment_variable({
      env_name: 'dev',
      key: 'NEW_KEY',
      value: 'v',
    });

    const base = await environmentStorage.loadEnvironment('dev', false);
    const local = await environmentStorage.loadEnvironment('dev', true);
    expect(base.variables).toEqual({ API: 'https://dev.api', NEW_KEY: 'v' });
    expect(local.variables).toEqual({ LOCAL_SECRET: 'machine-private' });
  });

  it('set_environment_variable 对不存在的环境创建共享环境文件', async () => {
    await envAndSettingsToolHandlers.set_environment_variable({
      env_name: 'staging',
      key: 'BASE_URL',
      value: 'https://stage.api',
    });

    const base = await environmentStorage.loadEnvironment('staging', false);
    expect(base).toEqual({ name: 'staging', variables: { BASE_URL: 'https://stage.api' } });
  });

  it('list_environments 只暴露共享变量并标记本地覆盖', async () => {
    await environmentStorageMock.saveEnvironment(
      { name: 'dev', variables: { API: 'https://dev.api' } },
      false,
    );
    await environmentStorageMock.saveEnvironment(
      { name: 'dev', variables: { LOCAL_SECRET: 'machine-private' } },
      true,
    );

    const result = (await envAndSettingsToolHandlers.list_environments()) as {
      environments: Array<{
        name: string;
        variables: Record<string, unknown>;
        has_local_override: boolean;
      }>;
    };

    expect(result.environments).toHaveLength(1);
    expect(result.environments[0].variables).toEqual({ API: 'https://dev.api' });
    expect(result.environments[0].has_local_override).toBe(true);
  });
});
