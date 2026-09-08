/**
 * AI Datasource Plugin Tools
 */

import { useDatasourceStore } from '@/stores/datasource';
import { useFlowStore } from '@/stores/flow';
import { datasourceGetTemplates } from '@/services/tauri/datasource';
import type { Node, NodeType, NodeData } from '@/types';
import type { AIToolDefinition } from '@/types/ai';

export const datasourcePluginToolDefinitions: AIToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'list_datasource_plugins',
      description: 'List all registered datasource plugins and active connections (e.g. Postman, Eolink, Apifox).',
      parameters: {
        type: 'object',
        properties: {},
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'browse_datasource',
      description: 'Browse remote assets, collections, projects, or API endpoints from a configured datasource plugin.',
      parameters: {
        type: 'object',
        properties: {
          datasource_id: {
            type: 'string',
            description: 'The ID of the datasource (e.g. postman, apifox, eolink).',
          },
          resource_id: {
            type: 'string',
            description: 'The resource ID to browse under this datasource.',
          },
          parent_id: {
            type: 'string',
            description: 'Optional parent item ID for folder hierarchy browsing (null for top level).',
          },
          connection_id: {
            type: 'string',
            description: 'Optional connection ID if multiple connections are configured for this datasource.',
          },
        },
        required: ['datasource_id', 'resource_id'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'import_datasource_item_to_flow',
      description: 'Import an API item from a datasource plugin directly into the active flow canvas as a configured HTTP node.',
      parameters: {
        type: 'object',
        properties: {
          datasource_id: {
            type: 'string',
            description: 'Datasource ID.',
          },
          item_id: {
            type: 'string',
            description: 'Remote item ID to convert and instantiate on the canvas.',
          },
          name: {
            type: 'string',
            description: 'Optional custom display name for the imported node.',
          },
          connection_id: {
            type: 'string',
            description: 'Optional connection ID if multiple connections are configured for this datasource.',
          },
        },
        required: ['datasource_id', 'item_id'],
      },
    },
  },
];

export const datasourcePluginToolHandlers = {
  async list_datasource_plugins() {
    const dsStore = useDatasourceStore.getState();
    await dsStore.loadDescriptions();
    await dsStore.loadConnections();

    const descriptions = dsStore.descriptions || [];
    const connections = dsStore.connections || [];

    return {
      success: true,
      datasources: descriptions.map((ds) => {
        const conns = connections.filter((c) => c.datasource_id === ds.id);
        return {
          id: ds.id,
          display_name: ds.display_name,
          auth: ds.auth,
          resources: ds.resources,
          connected: conns.length > 0,
          connections: conns.map((c) => ({
            id: c.id,
            name: c.name || c.display_name,
          })),
        };
      }),
    };
  },

  async browse_datasource(args: {
    datasource_id: string;
    resource_id: string;
    parent_id?: string | null;
    connection_id?: string;
  }) {
    const dsStore = useDatasourceStore.getState();
    const items = await dsStore.browse(
      args.datasource_id,
      args.resource_id,
      args.parent_id ?? null,
      false,
      args.connection_id,
    );
    return {
      success: true,
      datasource_id: args.datasource_id,
      resource_id: args.resource_id,
      parent_id: args.parent_id ?? null,
      count: items.length,
      items: items.map((it) => ({
        id: it.id,
        name: it.name,
        protocol: it.protocol,
        method: it.method,
        path: it.path,
        summary: it.summary,
        extra: it.extra,
      })),
    };
  },

  async import_datasource_item_to_flow(args: {
    datasource_id: string;
    item_id: string;
    name?: string;
    connection_id?: string;
  }) {
    const dsStore = useDatasourceStore.getState();
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow to import datasource item into.');
    }

    const conn = args.connection_id
      ? dsStore.connections.find((c) => c.id === args.connection_id)
      : dsStore.connections.find((c) => c.datasource_id === args.datasource_id);
    const config = conn?.config || {};

    const templates = await datasourceGetTemplates(
      args.datasource_id,
      [args.item_id],
      config,
    );
    if (templates.length === 0) {
      throw new Error(`Failed to convert datasource item "${args.item_id}" into node template.`);
    }

    const tpl = templates[0];
    const newUuid = crypto.randomUUID();
    const nodeName = args.name || tpl.name || `IMPORT_${args.item_id}`;

    const lastNode = currentFlow.nodes[currentFlow.nodes.length - 1];
    const position = lastNode
      ? { x: lastNode.position.x + 220, y: lastNode.position.y }
      : { x: 150, y: 150 };

    const newNode: Node = {
      uuid: newUuid,
      type: (tpl.type_id || 'http') as NodeType,
      name: nodeName,
      position,
      data: (tpl.data || { type: 'http' }) as unknown as NodeData,
    };

    flowState.addNode(newNode);
    flowState.selectNode(newUuid);
    flowState.centerOnNode(newUuid);

    return {
      success: true,
      node_uuid: newUuid,
      name: nodeName,
      type: newNode.type,
    };
  },
};
