/**
 * AI Flow File Management Tools
 */

import { flowStorage } from '@/services/storage/flowStorage';
import { useFlowStore } from '@/stores/flow';
import { useFlowListStore } from '@/stores/flowList';
import type { Flow, EntryConfig } from '@/types';
import type { AIToolDefinition } from '@/types/ai';

export const flowFileToolDefinitions: AIToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'list_flows',
      description: 'List all existing workflow files and their metadata in the workspace.',
      parameters: {
        type: 'object',
        properties: {},
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'create_flow',
      description: 'Create a new blank workflow with a given name and make it active.',
      parameters: {
        type: 'object',
        properties: {
          name: {
            type: 'string',
            description: 'Name of the new workflow.',
          },
          description: {
            type: 'string',
            description: 'Optional description for the workflow.',
          },
          folder_path: {
            type: 'string',
            description: 'Relative folder path where to store the flow (defaults to root).',
          },
        },
        required: ['name'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'open_flow',
      description: 'Open and activate a workflow by its UUID or name.',
      parameters: {
        type: 'object',
        properties: {
          uuid: {
            type: 'string',
            description: 'The UUID of the workflow to open.',
          },
          name: {
            type: 'string',
            description: 'Name of the workflow to open if UUID is not known.',
          },
        },
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'save_current_flow',
      description: 'Save the currently active workflow to disk.',
      parameters: {
        type: 'object',
        properties: {},
      },
    },
  },
];

export const flowFileToolHandlers = {
  async list_flows() {
    const list = await flowStorage.listFlows();
    return {
      success: true,
      count: list.length,
      flows: list.map((f) => ({
        uuid: f.uuid,
        name: f.name,
        filePath: f.filePath,
        updatedAt: f.updatedAt,
        folderPath: f.folderPath,
      })),
    };
  },

  async create_flow(args: {
    name: string;
    description?: string;
    folder_path?: string;
  }) {
    const uuid = crypto.randomUUID();
    const newFlow: Flow = {
      uuid,
      name: args.name,
      description: args.description,
      initial_variables: {},
      nodes: [
        {
          uuid: crypto.randomUUID(),
          type: 'entry',
          name: 'ENTRY',
          position: { x: 100, y: 150 },
          data: { type: 'entry' } as EntryConfig,
        },
      ],
      connections: [],
    };

    await flowStorage.saveFlow(newFlow);

    let actualFolderPath = '';
    const folderPath = args.folder_path?.trim();
    if (folderPath) {
      try {
        await flowStorage.moveFlowToFolder(newFlow.uuid, folderPath);
        actualFolderPath = folderPath;
      } catch (err) {
        console.warn('[flowFileTools] Failed to move flow to target folder:', folderPath, err);
      }
    }

    // 刷新流程列表
    const flows = await flowStorage.listFlows();
    useFlowListStore.getState().setFlows(
      flows.map((f) => ({
        uuid: f.uuid,
        name: f.name,
        updated_at: f.updatedAt,
        folder_path: f.folderPath,
        file_path: f.filePath,
      })),
    );

    // 匹配刚创建并落盘的流程绝对路径，绑定至标签页
    const folderMatches = flows.filter((f) =>
      actualFolderPath ? f.folderPath === actualFolderPath : !f.folderPath
    );
    const matchedMeta =
      folderMatches
        .filter((f) => f.name === newFlow.name || f.name.startsWith(`${newFlow.name}_`))
        .sort((a, b) => b.updatedAt - a.updatedAt)[0] ??
      folderMatches.find((f) => f.name === newFlow.name) ??
      flows.find((f) => f.name === newFlow.name);

    useFlowStore.getState().openTab(newFlow, matchedMeta?.filePath);

    return {
      success: true,
      uuid: newFlow.uuid,
      name: newFlow.name,
      file_path: matchedMeta?.filePath,
      folder_path: actualFolderPath,
    };
  },

  async open_flow(args: { uuid?: string; name?: string }) {
    let targetUuid = args.uuid;
    let targetFilePath: string | undefined;

    const list = await flowStorage.listFlows();
    if (!targetUuid && args.name) {
      const match = list.find(
        (f) => f.name.toLowerCase() === args.name!.toLowerCase(),
      );
      if (match) {
        targetUuid = match.uuid;
        targetFilePath = match.filePath;
      }
    } else if (targetUuid) {
      const match = list.find(
        (f) => f.uuid === targetUuid || (f.filePath && f.filePath === targetUuid),
      );
      if (match) {
        targetFilePath = match.filePath;
      }
    }

    if (!targetUuid) {
      throw new Error('Flow UUID or valid name must be provided to open a flow.');
    }

    const flow = await flowStorage.loadFlow(targetUuid);

    // 若通过内部 Flow UUID 打开导致先前相对路径未匹配，通过 flow.name 回退解析绝对路径
    if (!targetFilePath && flow.name) {
      const matches = list.filter(
        (f) => f.name.toLowerCase() === flow.name.toLowerCase(),
      );
      if (matches.length === 1) {
        targetFilePath = matches[0].filePath;
      } else if (matches.length > 1) {
        targetFilePath = matches.sort((a, b) => b.updatedAt - a.updatedAt)[0].filePath;
      }
    }

    useFlowStore.getState().openTab(flow, targetFilePath);

    return {
      success: true,
      uuid: flow.uuid,
      name: flow.name,
      file_path: targetFilePath,
      node_count: flow.nodes.length,
      connection_count: flow.connections.length,
    };
  },

  async save_current_flow() {
    const currentFlow = useFlowStore.getState().flow;
    if (!currentFlow) {
      throw new Error('No active flow to save.');
    }

    await flowStorage.saveFlow(currentFlow);
    useFlowStore.getState().markSaved();

    // 重新获取保存后最新的 Tab 状态，防止异步 I/O 期间 activeTab 被切换
    const latestTabs = useFlowStore.getState().tabs;
    const targetTab =
      latestTabs.find((t) => t.uuid === currentFlow.uuid) ??
      latestTabs.find((t) => t.uuid === useFlowStore.getState().activeTabUuid);

    let resolvedFilePath = targetTab?.filePath;
    if (!resolvedFilePath) {
      const list = await flowStorage.listFlows();
      const matches = list.filter((f) => f.name === currentFlow.name);
      const match =
        matches.length === 1 ? matches[0] : matches.sort((a, b) => b.updatedAt - a.updatedAt)[0];
      if (match?.filePath && targetTab) {
        resolvedFilePath = match.filePath;
        useFlowStore.getState().updateTabFilePath(targetTab.uuid, match.filePath);
      }
    }

    return {
      success: true,
      uuid: currentFlow.uuid,
      name: currentFlow.name,
      file_path: resolvedFilePath,
    };
  },
};
