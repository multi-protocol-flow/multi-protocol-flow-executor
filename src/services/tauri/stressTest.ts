/**
 * Tauri IPC 压力测试相关调用
 * 封装压力测试命令的 invoke 调用
 */

import { invoke, Channel } from '@tauri-apps/api/core';
import type { Flow } from '@/types/flow';
import type {
  StressTestConfig,
  StressTestEvent,
  GetStressTestStatusOutput,
} from '@/types/stressTest';
import { formatError } from './flow';

/**
 * 启动压力测试
 * 使用 Channel API 接收压测事件
 *
 * @param flow - 要压测的流程对象
 * @param config - 压测配置
 * @param onEvent - 压测事件回调函数
 * @throws IpcError 如果启动失败
 */
export async function startStressTest(
  flow: Flow,
  config: StressTestConfig,
  onEvent: (event: StressTestEvent) => void
): Promise<void> {
  try {
    // 创建 Channel 用于接收压测事件
    const eventChannel = new Channel<StressTestEvent>();
    eventChannel.onmessage = (event) => {
      onEvent(event);
    };

    await invoke<void>('start_stress_test', {
      input: {
        flow,
        config,
      },
      onEvent: eventChannel,
    });
  } catch (error) {
    throw formatError(error, 'start_stress_test');
  }
}

/**
 * 停止压力测试
 *
 * @throws IpcError 如果停止失败
 */
export async function stopStressTest(): Promise<void> {
  try {
    await invoke<void>('stop_stress_test');
  } catch (error) {
    throw formatError(error, 'stop_stress_test');
  }
}

/**
 * 获取压力测试状态
 *
 * @returns 当前压测状态和进度信息
 * @throws IpcError 如果查询失败
 */
export async function getStressTestStatus(): Promise<GetStressTestStatusOutput> {
  try {
    return await invoke<GetStressTestStatusOutput>('get_stress_test_status');
  } catch (error) {
    throw formatError(error, 'get_stress_test_status');
  }
}
