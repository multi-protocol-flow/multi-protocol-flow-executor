/**
 * 终端远程指令与双向联动 Hook
 *
 * 监听来自 CLI (`mpe open` / `mpe ai`) 经由 Local Loopback HTTP 广播的 Tauri 全局事件：
 * - `mpe://open-workspace`: 自动将指定路径设为当前工作区，并展开 AI 侧边栏
 * - `mpe://trigger-ai`: 自动聚焦/展开 AI 侧边栏，绑定对应工作区，并自动触发消息发送
 */
import { useEffect } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { isTauriEnv } from '@/lib/platform';
import { useAIStore } from '@/stores/ai';
import type { AIWorkspaceContext } from '@/types/ai';

export function useRemoteCommands() {
  useEffect(() => {
    if (!isTauriEnv()) return;
    let unlistenOpen: UnlistenFn | undefined;
    let unlistenAi: UnlistenFn | undefined;

    const setupListeners = async () => {
      try {
        // 1. 监听打开或关联工作区指令
        unlistenOpen = await listen<{ path: string }>('mpe://open-workspace', (event) => {
          const rawPath = event.payload?.path;
          if (!rawPath) return;

          const cleanPath = rawPath.replace(/[/\\]+$/, '');
          const segments = cleanPath.split(/[/\\]/);
          const folderName = segments[segments.length - 1] || 'workspace';

          const ws: AIWorkspaceContext = {
            path: rawPath,
            name: folderName,
          };

          const aiStore = useAIStore.getState();
          aiStore.setSessionWorkspace(ws);
          aiStore.setCopilotOpen(true);
        });

        // 2. 监听终端触发的 AI 分析与生成任务
        unlistenAi = await listen<{ workspace: string; prompt: string }>(
          'mpe://trigger-ai',
          async (event) => {
            const { workspace: rawWs, prompt } = event.payload || {};
            if (!prompt) return;

            const aiStore = useAIStore.getState();
            aiStore.setCopilotOpen(true);

            if (rawWs) {
              const cleanPath = rawWs.replace(/[/\\]+$/, '');
              const segments = cleanPath.split(/[/\\]/);
              const folderName = segments[segments.length - 1] || 'workspace';
              const ws: AIWorkspaceContext = {
                path: rawWs,
                name: folderName,
              };
              aiStore.setSessionWorkspace(ws);
            }

            // 发送提示词并触发 ReAct Agent 循环
            await aiStore.sendMessage(prompt);
          },
        );
      } catch (err) {
        console.warn('[useRemoteCommands] Failed to setup remote command listeners:', err);
      }
    };

    setupListeners();

    return () => {
      if (unlistenOpen) unlistenOpen();
      if (unlistenAi) unlistenAi();
    };
  }, []);
}
