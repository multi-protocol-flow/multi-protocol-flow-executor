/**
 * AI Canvas & Workflow Manipulation Tools
 */

import { useFlowStore } from '@/stores/flow';
import { getLayoutedPositions } from '@/components/canvas/layout';
import type { Node, Connection, NodeType, NodeData, ExecutionErrorStrategy } from '@/types';
import type { AIToolDefinition } from '@/types/ai';

/**
 * 构造默认的 NodeData
 */
export function buildDefaultNodeData(type: string, config?: Record<string, unknown>): NodeData {
  const base: Record<string, unknown> = {
    type,
    ...(config || {}),
  };

  switch (type) {
    case 'entry':
      return { type: 'entry', ...base } as unknown as NodeData;
    case 'end':
      return { type: 'end', ...base } as unknown as NodeData;
    case 'http':
      return {
        type: 'http',
        method: 'get',
        url: 'https://httpbin.org/get',
        headers: {},
        query_params: {},
        body_type: 'none',
        body: '',
        timeout_ms: 30000,
        ...base,
      } as unknown as NodeData;
    case 'script':
      return {
        type: 'script',
        script: '// JavaScript Execution Sandbox\nconsole.log("Running script step");\n',
        ...base,
      } as unknown as NodeData;
    case 'conditional':
      return {
        type: 'conditional',
        conditions: [],
        ...base,
      } as unknown as NodeData;
    case 'assertion':
      return {
        type: 'assertion',
        assertions: [],
        mode: 'all',
        on_failure: 'abort',
        ...base,
      } as unknown as NodeData;
    case 'variable_extractor':
      return {
        type: 'variable_extractor',
        extractions: [],
        ...base,
      } as unknown as NodeData;
    default:
      return base as unknown as NodeData;
  }
}

/**
 * Canvas 工具定义
 */
export const canvasToolDefinitions: AIToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'add_node',
      description: 'Add a new node to the current workflow canvas.',
      parameters: {
        type: 'object',
        properties: {
          type: {
            type: 'string',
            description: 'The node type (e.g. http, script, conditional, assertion, entry, end, ws_connect, etc.).',
          },
          name: {
            type: 'string',
            description: 'Human-readable name for the node.',
          },
          position: {
            type: 'object',
            description: 'Position on the canvas {x, y}. If omitted, automatically placed next to existing nodes.',
            properties: {
              x: { type: 'number' },
              y: { type: 'number' },
            },
          },
          config: {
            type: 'object',
            description: 'Node configuration parameters specific to the node type.',
          },
          on_error: {
            type: 'string',
            enum: ['route_to_false', 'ignore', 'abort_flow'],
            description: 'Error routing policy if execution fails. Defaults to route_to_false.',
          },
        },
        required: ['type'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'update_node_config',
      description: 'Update the properties or configuration of an existing node in the workflow.',
      parameters: {
        type: 'object',
        properties: {
          node_uuid: {
            type: 'string',
            description: 'The UUID of the node to update.',
          },
          name: {
            type: 'string',
            description: 'Updated node display name.',
          },
          config: {
            type: 'object',
            description: 'Updated configuration fields to merge into node.data.',
          },
          on_error: {
            type: 'string',
            enum: ['route_to_false', 'ignore', 'abort_flow'],
            description: 'Updated error routing policy.',
          },
        },
        required: ['node_uuid'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'connect_nodes',
      description: 'Connect two nodes in the workflow canvas from source port to target port.',
      parameters: {
        type: 'object',
        properties: {
          source_node_uuid: {
            type: 'string',
            description: 'UUID of the source node.',
          },
          target_node_uuid: {
            type: 'string',
            description: 'UUID of the target node.',
          },
          source_port_id: {
            type: 'string',
            description: "Source port ID ('true', 'false', 'out', or 'default'). Defaults to 'default' or 'out'.",
          },
          target_port_id: {
            type: 'string',
            description: "Target port ID ('in' or 'default'). Defaults to 'default' or 'in'.",
          },
        },
        required: ['source_node_uuid', 'target_node_uuid'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'disconnect_nodes',
      description: 'Remove a connection between nodes.',
      parameters: {
        type: 'object',
        properties: {
          connection_id: {
            type: 'string',
            description: 'ID of the connection to remove (if known).',
          },
          source_node_uuid: {
            type: 'string',
            description: 'Source node UUID (used if connection_id not provided).',
          },
          target_node_uuid: {
            type: 'string',
            description: 'Target node UUID (used if connection_id not provided).',
          },
        },
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'auto_layout',
      description: 'Automatically organize and arrange all nodes and connections on the canvas into a clean DAG layout.',
      parameters: {
        type: 'object',
        properties: {
          direction: {
            type: 'string',
            enum: ['LR'],
            description: 'Layout direction (horizontal LR). Defaults to LR.',
          },
        },
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'delete_nodes',
      description: 'Delete one or more nodes and their attached connections from the workflow. [HIGH RISK]',
      parameters: {
        type: 'object',
        properties: {
          node_uuids: {
            type: 'array',
            items: { type: 'string' },
            description: 'List of node UUIDs to delete.',
          },
        },
        required: ['node_uuids'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'clear_canvas',
      description: 'Delete all nodes and connections from the current workflow canvas. [HIGH RISK]',
      parameters: {
        type: 'object',
        properties: {},
      },
    },
  },
];

/**
 * Canvas 工具执行实现
 */
export const canvasToolHandlers = {
  async add_node(args: {
    type: string;
    name?: string;
    position?: { x: number; y: number };
    config?: Record<string, unknown>;
    on_error?: ExecutionErrorStrategy;
  }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow to add node to.');
    }

    const uuid = crypto.randomUUID();
    const name = args.name || `${args.type.toUpperCase()}_${currentFlow.nodes.length + 1}`;

    // 计算合理初始位置
    let position = args.position;
    if (!position) {
      if (currentFlow.nodes.length === 0) {
        position = { x: 100, y: 150 };
      } else {
        const lastNode = currentFlow.nodes[currentFlow.nodes.length - 1];
        position = { x: lastNode.position.x + 220, y: lastNode.position.y };
      }
    }

    const data = buildDefaultNodeData(args.type, args.config);

    const newNode: Node = {
      uuid,
      type: args.type as NodeType,
      name,
      position,
      data,
      on_error: args.on_error,
    };

    flowState.addNode(newNode);
    flowState.selectNode(uuid);
    flowState.centerOnNode(uuid);

    return {
      success: true,
      node_uuid: uuid,
      name,
      type: args.type,
      position,
    };
  },

  async update_node_config(args: {
    node_uuid: string;
    name?: string;
    config?: Record<string, unknown>;
    on_error?: ExecutionErrorStrategy;
  }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow.');
    }

    const node = currentFlow.nodes.find((n) => n.uuid === args.node_uuid);
    if (!node) {
      throw new Error(`Node with UUID "${args.node_uuid}" not found.`);
    }

    const updates: Partial<Node> = {};
    if (args.name !== undefined) {
      updates.name = args.name;
    }
    if (args.on_error !== undefined) {
      updates.on_error = args.on_error;
    }
    if (args.config !== undefined) {
      updates.data = {
        ...node.data,
        ...args.config,
      } as NodeData;
    }

    flowState.updateNode(args.node_uuid, updates);

    return {
      success: true,
      node_uuid: args.node_uuid,
      updated_fields: Object.keys(updates),
    };
  },

  async connect_nodes(args: {
    source_node_uuid: string;
    target_node_uuid: string;
    source_port_id?: string;
    target_port_id?: string;
  }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow.');
    }

    const sourceNode = currentFlow.nodes.find((n) => n.uuid === args.source_node_uuid);
    if (!sourceNode) {
      throw new Error(`Source node "${args.source_node_uuid}" not found.`);
    }
    const targetNode = currentFlow.nodes.find((n) => n.uuid === args.target_node_uuid);
    if (!targetNode) {
      throw new Error(`Target node "${args.target_node_uuid}" not found.`);
    }

    const sourcePort =
      args.source_port_id ||
      (sourceNode.type === 'conditional' ? 'true' : 'default');
    const targetPort = args.target_port_id || 'default';

    const connId = `conn_${args.source_node_uuid}_${sourcePort}_to_${args.target_node_uuid}_${targetPort}`;

    const connection: Connection = {
      id: connId,
      source_node_uuid: args.source_node_uuid,
      source_port_id: sourcePort,
      target_node_uuid: args.target_node_uuid,
      target_port_id: targetPort,
    };

    flowState.addConnection(connection);

    return {
      success: true,
      connection_id: connId,
      source: `${args.source_node_uuid}:${sourcePort}`,
      target: `${args.target_node_uuid}:${targetPort}`,
    };
  },

  async disconnect_nodes(args: {
    connection_id?: string;
    source_node_uuid?: string;
    target_node_uuid?: string;
  }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow.');
    }

    if (args.connection_id) {
      flowState.removeConnection(args.connection_id);
      return { success: true, removed_connection_id: args.connection_id };
    }

    if (args.source_node_uuid && args.target_node_uuid) {
      const match = currentFlow.connections.find(
        (c) =>
          c.source_node_uuid === args.source_node_uuid &&
          c.target_node_uuid === args.target_node_uuid,
      );
      if (match) {
        flowState.removeConnection(match.id);
        return { success: true, removed_connection_id: match.id };
      }
      throw new Error('No matching connection found between specified nodes.');
    }

    throw new Error('Either connection_id or both source and target UUIDs must be provided.');
  },

  async auto_layout(args?: { direction?: 'LR' }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow || currentFlow.nodes.length === 0) {
      return { success: true, node_count: 0 };
    }

    const direction = args?.direction || 'LR';
    const positions = getLayoutedPositions(currentFlow.nodes, currentFlow.connections, direction);
    flowState.batchUpdateNodePositions(positions);

    return {
      success: true,
      node_count: currentFlow.nodes.length,
      direction,
    };
  },

  async delete_nodes(args: { node_uuids: string[] }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow.');
    }

    flowState.removeNodes(args.node_uuids);
    return {
      success: true,
      deleted_count: args.node_uuids.length,
    };
  },

  async clear_canvas() {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      return { success: true, cleared_nodes: 0 };
    }

    const nodeCount = currentFlow.nodes.length;
    const allUuids = currentFlow.nodes.map((n) => n.uuid);
    flowState.removeNodes(allUuids);

    return {
      success: true,
      cleared_nodes: nodeCount,
    };
  },
};
