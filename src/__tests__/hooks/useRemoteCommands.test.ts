/**
 * useRemoteCommands Hook 测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { useRemoteCommands } from '@/hooks/useRemoteCommands';
import { useAIStore } from '@/stores/ai';

// Mock platform: return true for isTauriEnv
vi.mock('@/lib/platform', () => ({
  isTauriEnv: () => true,
}));

// Mock @tauri-apps/api/event
type EventHandler = (event: { payload: unknown }) => void;
const registeredHandlers = new Map<string, EventHandler>();

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((event: string, handler: EventHandler) => {
    registeredHandlers.set(event, handler);
    return Promise.resolve(() => {
      if (registeredHandlers.get(event) === handler) {
        registeredHandlers.delete(event);
      }
    });
  }),
}));

describe('useRemoteCommands', () => {
  beforeEach(() => {
    useAIStore.getState().reset();
    registeredHandlers.clear();
  });

  it('监听并在收到 mpe://open-workspace 时设置工作区并展开面板', async () => {
    renderHook(() => useRemoteCommands());

    await waitFor(() => {
      expect(registeredHandlers.get('mpe://open-workspace')).toBeDefined();
    });

    const openHandler = registeredHandlers.get('mpe://open-workspace');
    openHandler!({
      payload: { path: '/home/developer/cool-project' },
    });

    const state = useAIStore.getState();
    expect(state.isCopilotOpen).toBe(true);
    expect(state.activeWorkspace).toEqual({
      path: '/home/developer/cool-project',
      name: 'cool-project',
    });
  });

  it('监听并在收到 mpe://trigger-ai 时展开面板并触发 sendMessage', async () => {
    const sendSpy = vi.spyOn(useAIStore.getState(), 'sendMessage').mockResolvedValue();

    renderHook(() => useRemoteCommands());

    await waitFor(() => {
      expect(registeredHandlers.get('mpe://trigger-ai')).toBeDefined();
    });

    const aiHandler = registeredHandlers.get('mpe://trigger-ai');
    aiHandler!({
      payload: {
        workspace: '/var/www/api-server',
        prompt: '分析当前项目接口并自动编排工作流',
      },
    });

    const state = useAIStore.getState();
    expect(state.isCopilotOpen).toBe(true);
    expect(state.activeWorkspace).toEqual({
      path: '/var/www/api-server',
      name: 'api-server',
    });
    expect(sendSpy).toHaveBeenCalledWith('分析当前项目接口并自动编排工作流');

    sendSpy.mockRestore();
  });
});
