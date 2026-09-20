/**
 * Environment Store 单元测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { useEnvironmentStore } from '@/stores/environment';
import { environmentStorageMock } from '@/services/storage/environmentStorage.mock';

describe('EnvironmentStore', () => {
  beforeEach(() => {
    localStorage.clear();
    environmentStorageMock.reset();
    useEnvironmentStore.getState().reset();
  });

  it('初始状态应为空', () => {
    const state = useEnvironmentStore.getState();
    expect(state.environments).toEqual([]);
    expect(state.activeEnvironmentName).toBeNull();
    expect(state.activeEnvironmentData).toBeNull();
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
  });

  it('支持创建环境并加载列表', async () => {
    const store = useEnvironmentStore.getState();
    await store.createEnvironment('dev', { API_HOST: 'https://dev.api' });

    const state = useEnvironmentStore.getState();
    expect(state.environments.length).toBe(1);
    expect(state.environments[0].name).toBe('dev');
    expect(state.environments[0].variableCount).toBe(1);
  });

  it('支持激活环境并缓存其变量数据', async () => {
    const store = useEnvironmentStore.getState();
    await store.createEnvironment('staging', { BASE_URL: 'https://stage.api' });
    await store.setActiveEnvironment('staging');

    const state = useEnvironmentStore.getState();
    expect(state.activeEnvironmentName).toBe('staging');
    expect(state.activeEnvironmentData?.name).toBe('staging');
    expect(state.activeEnvironmentData?.variables.BASE_URL).toBe('https://stage.api');
  });

  it('更新当前激活环境时自动同步刷新 activeEnvironmentData', async () => {
    const store = useEnvironmentStore.getState();
    await store.createEnvironment('prod', { TIMEOUT: 5000 });
    await store.setActiveEnvironment('prod');

    // 保存更新
    await store.saveEnvironment({
      name: 'prod',
      variables: { TIMEOUT: 10000, NEW_KEY: 'val' },
    });

    const state = useEnvironmentStore.getState();
    expect(state.activeEnvironmentData?.variables.TIMEOUT).toBe(10000);
    expect(state.activeEnvironmentData?.variables.NEW_KEY).toBe('val');
  });

  it('删除当前激活环境时自动清除激活状态', async () => {
    const store = useEnvironmentStore.getState();
    await store.createEnvironment('temp', { FOO: 'bar' });
    await store.setActiveEnvironment('temp');
    expect(useEnvironmentStore.getState().activeEnvironmentName).toBe('temp');

    await store.deleteEnvironment('temp');

    const state = useEnvironmentStore.getState();
    expect(state.environments.length).toBe(0);
    expect(state.activeEnvironmentName).toBeNull();
    expect(state.activeEnvironmentData).toBeNull();
  });

  it('重命名当前激活环境时同步更新 activeEnvironmentName', async () => {
    const store = useEnvironmentStore.getState();
    await store.createEnvironment('old_name', { X: 1 });
    await store.setActiveEnvironment('old_name');

    await store.renameEnvironment('old_name', 'new_name');

    const state = useEnvironmentStore.getState();
    expect(state.activeEnvironmentName).toBe('new_name');
    expect(state.activeEnvironmentData?.name).toBe('new_name');
    expect(state.environments[0].name).toBe('new_name');
  });

  it('reset() 应彻底恢复初始状态', async () => {
    const store = useEnvironmentStore.getState();
    await store.createEnvironment('dev');
    await store.setActiveEnvironment('dev');

    useEnvironmentStore.getState().reset();

    const state = useEnvironmentStore.getState();
    expect(state.environments).toEqual([]);
    expect(state.activeEnvironmentName).toBeNull();
    expect(state.activeEnvironmentData).toBeNull();
  });
});
