/**
 * Tauri IPC 压测服务测试
 *
 * 覆盖压测启动时必须携带激活环境名：缺失会让压测静默跑在流程兜底变量指向的端点上。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { startStressTest } from '@/services/tauri/stressTest';
import { useEnvironmentStore } from '@/stores/environment';
import type { Flow } from '@/types/flow';
import type { StressTestConfig } from '@/types/stressTest';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

function createTestFlow(): Flow {
  return {
    uuid: 'stress-flow-uuid',
    name: '压测流程',
    nodes: [],
    connections: [],
    initial_variables: {},
  };
}

function createConfig(): StressTestConfig {
  return {
    concurrentUsers: 1,
    rampUpSeconds: 0,
    durationSeconds: 1,
    flowUuid: 'stress-flow-uuid',
  } as StressTestConfig;
}

describe('startStressTest', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(undefined);
    useEnvironmentStore.setState({ activeEnvironmentName: null, activeEnvironmentData: null });
  });

  it('激活环境时应携带 environmentName', async () => {
    useEnvironmentStore.setState({ activeEnvironmentName: 'dev', activeEnvironmentData: null });

    await startStressTest(createTestFlow(), createConfig(), vi.fn());

    const callArgs = mockInvoke.mock.calls[0];
    expect(callArgs[0]).toBe('start_stress_test');
    expect(callArgs[1]).toMatchObject({
      input: { config: expect.objectContaining({ concurrentUsers: 1 }) },
      environmentName: 'dev',
    });
  });

  it('未激活环境时不应携带 environmentName', async () => {
    await startStressTest(createTestFlow(), createConfig(), vi.fn());

    expect(mockInvoke.mock.calls[0][1]).not.toHaveProperty('environmentName');
  });
});
