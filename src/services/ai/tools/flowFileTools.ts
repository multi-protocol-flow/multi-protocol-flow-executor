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
      environments: [],
      active_environment: undefined,
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
    useFlowStore.getState().setFlow(newFlow);

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

    return {
      success: true,
      uuid: newFlow.uuid,
      name: newFlow.name,
    };
  },

  async open_flow(args: { uuid?: string; name?: string }) {
    let targetUuid = args.uuid;

    if (!targetUuid && args.name) {
      const list = await flowStorage.listFlows();
      const match = list.find(
        (f) => f.name.toLowerCase() === args.name!.toLowerCase(),
      );
      if (match) {
        targetUuid = match.uuid;
      }
    }

    if (!targetUuid) {
      throw new Error('Flow UUID or valid name must be provided to open a flow.');
    }

    const flow = await flowStorage.loadFlow(targetUuid);
    useFlowStore.getState().setFlow(flow);

    return {
      success: true,
      uuid: flow.uuid,
      name: flow.name,
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

    return {
      success: true,
      uuid: currentFlow.uuid,
      name: currentFlow.name,
    };
  },
};
