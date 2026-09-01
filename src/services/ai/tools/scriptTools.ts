/**
 * AI Script Generation & Manipulation Tools
 */

import { useFlowStore } from '@/stores/flow';
import { useUIStore } from '@/stores/ui';
import type { Node, ScriptNodeConfig } from '@/types';
import type { AIToolDefinition } from '@/types/ai';

export const scriptToolDefinitions: AIToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'generate_or_update_script',
      description: 'Generate or update JavaScript code for a Script node and focus/open its editor in the configuration panel.',
      parameters: {
        type: 'object',
        properties: {
          node_uuid: {
            type: 'string',
            description: 'The UUID of the script node to update. If omitted, uses the currently selected script node, or creates a new one.',
          },
          script: {
            type: 'string',
            description: 'The JavaScript code snippet to inject into the script node sandbox.',
          },
          node_name: {
            type: 'string',
            description: 'Display name if creating a new script node.',
          },
        },
        required: ['script'],
      },
    },
  },
];

export const scriptToolHandlers = {
  async generate_or_update_script(args: {
    node_uuid?: string;
    script: string;
    node_name?: string;
  }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow to edit scripts in.');
    }

    let targetNode: Node | undefined;

    if (args.node_uuid) {
      targetNode = currentFlow.nodes.find((n) => n.uuid === args.node_uuid);
    } else if (flowState.selectedNodeId) {
      const selected = currentFlow.nodes.find((n) => n.uuid === flowState.selectedNodeId);
      if (selected && selected.type === 'script') {
        targetNode = selected;
      }
    }

    if (targetNode) {
      // 更新现有 Script 节点
      flowState.updateNode(targetNode.uuid, {
        data: {
          ...targetNode.data,
          type: 'script',
          script: args.script,
        } as ScriptNodeConfig,
      });

      flowState.selectNode(targetNode.uuid);
      flowState.centerOnNode(targetNode.uuid);
      useUIStore.getState().setConfigPanelOpen(true);

      return {
        success: true,
        node_uuid: targetNode.uuid,
        action: 'updated',
        name: targetNode.name,
      };
    } else {
      // 创建新的 Script 节点
      const newUuid = crypto.randomUUID();
      const nodeName = args.node_name || `SCRIPT_${currentFlow.nodes.length + 1}`;

      const lastNode = currentFlow.nodes[currentFlow.nodes.length - 1];
      const position = lastNode
        ? { x: lastNode.position.x + 220, y: lastNode.position.y }
        : { x: 150, y: 150 };

      const newNode: Node = {
        uuid: newUuid,
        type: 'script',
        name: nodeName,
        position,
        data: {
          type: 'script',
          script: args.script,
        } as ScriptNodeConfig,
      };

      flowState.addNode(newNode);
      flowState.selectNode(newUuid);
      flowState.centerOnNode(newUuid);
      useUIStore.getState().setConfigPanelOpen(true);

      return {
        success: true,
        node_uuid: newUuid,
        action: 'created',
        name: nodeName,
      };
    }
  },
};
