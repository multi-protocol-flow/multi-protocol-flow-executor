import type { Flow, Node, Connection, NodeData } from '@/types';

/** 字段级变化 */
export interface FieldChange {
  field: string;
  oldValue: unknown;
  newValue: unknown;
}

/** 节点配置变化 */
export interface NodeDataChange {
  nodeType: string;
  fieldChanges: FieldChange[];
}

/** 节点级 Diff 结果 */
export interface NodeDiff {
  uuid: string;
  name: string;
  nodeType: string;
  dataChanges?: NodeDataChange;
  nameChange?: FieldChange;
}

/** 连接级 Diff 结果 */
export interface ConnectionDiff {
  id: string;
  sourceNodeUuid: string;
  targetNodeUuid: string;
  sourcePortId: string;
  targetPortId: string;
}

/** 流程级属性变化 */
export interface FlowChanges {
  name?: FieldChange;
  initialVariables?: {
    added: string[];
    removed: string[];
    modified: FieldChange[];
  };
}

/** 完整的 Flow Diff 结果 */
export interface FlowDiffResult {
  addedNodes: NodeDiff[];
  removedNodes: NodeDiff[];
  modifiedNodes: NodeDiff[];
  unchangedNodes: NodeDiff[];
  addedConnections: ConnectionDiff[];
  removedConnections: ConnectionDiff[];
  unchangedConnections: ConnectionDiff[];
  flowChanges: FlowChanges;
}

function deepEqual(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

function toNodeDiff(node: Node): NodeDiff {
  return {
    uuid: node.uuid,
    name: node.name,
    nodeType: node.type,
  };
}

function toConnectionDiff(conn: Connection): ConnectionDiff {
  return {
    id: conn.id,
    sourceNodeUuid: conn.source_node_uuid,
    targetNodeUuid: conn.target_node_uuid,
    sourcePortId: conn.source_port_id,
    targetPortId: conn.target_port_id,
  };
}

function diffNodeData(oldData: NodeData, newData: NodeData): NodeDataChange | null {
  const typeChanged = oldData.type !== newData.type;
  const nodeType = typeChanged ? `${oldData.type} → ${newData.type}` : oldData.type;

  const oldRecord = oldData as unknown as Record<string, unknown>;
  const newRecord = newData as unknown as Record<string, unknown>;
  const allKeys = new Set([...Object.keys(oldRecord), ...Object.keys(newRecord)]);

  const fieldChanges: FieldChange[] = [];
  for (const key of allKeys) {
    if (key === 'type') continue;
    if (!deepEqual(oldRecord[key], newRecord[key])) {
      fieldChanges.push({ field: key, oldValue: oldRecord[key], newValue: newRecord[key] });
    }
  }

  if (fieldChanges.length === 0 && !typeChanged) return null;
  return { nodeType, fieldChanges };
}

function diffFlowChanges(snapshot: Flow, current: Flow): FlowChanges {
  const result: FlowChanges = {};

  if (snapshot.name !== current.name) {
    result.name = { field: 'name', oldValue: snapshot.name, newValue: current.name };
  }

  const oldVars = snapshot.initial_variables ?? {};
  const newVars = current.initial_variables ?? {};
  const added: string[] = [];
  const removed: string[] = [];
  const modified: FieldChange[] = [];

  const allVarKeys = new Set([...Object.keys(oldVars), ...Object.keys(newVars)]);
  for (const key of allVarKeys) {
    const inOld = key in oldVars;
    const inNew = key in newVars;
    if (inOld && !inNew) {
      removed.push(key);
    } else if (!inOld && inNew) {
      added.push(key);
    } else if (!deepEqual(oldVars[key], newVars[key])) {
      modified.push({ field: key, oldValue: oldVars[key], newValue: newVars[key] });
    }
  }

  if (added.length > 0 || removed.length > 0 || modified.length > 0) {
    result.initialVariables = { added, removed, modified };
  }

  return result;
}

export function diffFlows(snapshot: Flow, current: Flow): FlowDiffResult {
  // 节点对比
  const snapshotNodeMap = new Map(snapshot.nodes.map((n) => [n.uuid, n]));
  const currentNodeMap = new Map(current.nodes.map((n) => [n.uuid, n]));

  const addedNodes: NodeDiff[] = [];
  const removedNodes: NodeDiff[] = [];
  const modifiedNodes: NodeDiff[] = [];
  const unchangedNodes: NodeDiff[] = [];

  for (const [uuid, node] of snapshotNodeMap) {
    if (!currentNodeMap.has(uuid)) {
      removedNodes.push(toNodeDiff(node));
    }
  }

  for (const [uuid, node] of currentNodeMap) {
    if (!snapshotNodeMap.has(uuid)) {
      addedNodes.push(toNodeDiff(node));
      continue;
    }

    const snapshotNode = snapshotNodeMap.get(uuid)!;
    const dataChanges = diffNodeData(snapshotNode.data, node.data);
    const nameChanged = snapshotNode.name !== node.name;

    if (dataChanges || nameChanged) {
      const diff: NodeDiff = {
        uuid,
        name: snapshotNode.name,
        nodeType: node.type,
      };
      if (dataChanges) diff.dataChanges = dataChanges;
      if (nameChanged) {
        diff.nameChange = {
          field: 'name',
          oldValue: snapshotNode.name,
          newValue: node.name,
        };
      }
      modifiedNodes.push(diff);
    } else {
      unchangedNodes.push(toNodeDiff(node));
    }
  }

  // 连接对比
  const snapshotConnMap = new Map(snapshot.connections.map((c) => [c.id, c]));
  const currentConnMap = new Map(current.connections.map((c) => [c.id, c]));

  const addedConnections: ConnectionDiff[] = [];
  const removedConnections: ConnectionDiff[] = [];
  const unchangedConnections: ConnectionDiff[] = [];

  for (const [id, conn] of snapshotConnMap) {
    if (!currentConnMap.has(id)) {
      removedConnections.push(toConnectionDiff(conn));
    }
  }

  for (const [id, conn] of currentConnMap) {
    if (!snapshotConnMap.has(id)) {
      addedConnections.push(toConnectionDiff(conn));
    } else {
      unchangedConnections.push(toConnectionDiff(conn));
    }
  }

  // 流程级属性对比
  const flowChanges = diffFlowChanges(snapshot, current);

  return {
    addedNodes,
    removedNodes,
    modifiedNodes,
    unchangedNodes,
    addedConnections,
    removedConnections,
    unchangedConnections,
    flowChanges,
  };
}
