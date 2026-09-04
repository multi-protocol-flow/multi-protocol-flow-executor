/**
 * AI Tool Call 目标解析与标题格式化工具
 * 提取具体操作目标名 (如改了哪个节点、查了哪个数据源、连了哪条线等)，
 * 严格保留原始英文工具名，不添加冗余中文标题，绝不展示冗长丑陋的 UUID。
 */

import { useFlowStore } from '@/stores/flow';
import { useDatasourceStore } from '@/stores/datasource';
import { getLL } from '@/i18n/i18n-lazy';

export interface ToolTitleInfo {
  /** 提取到的具体操作目标名 (如 `"用户登录" (http)`, `apifox · order_center`, `"登录" ➔ "验签"`) */
  target?: string;
  /** 国际化工具显示名称 (如 "修改节点" / "Update Node") */
  toolDisplayName: string;
  /** 完整标题: 先工具名: 目标名，没有目标名就只显示工具名 (如 `修改节点: "用户登录" (http)` 或 `清空画布`) */
  fullTitle: string;
  /** 底层原始函数名 (如 `update_node_config`) */
  rawName: string;
  /** 保持向后兼容的 action 字段 */
  action: string;
}

/**
 * 获取工具名称的国际化显示文本
 */
export function getLocalizedToolName(toolName: string): string {
  try {
    const ll = getLL();
    const toolNames = ll.ai?.tool_names as Record<string, () => string> | undefined;
    if (toolNames && typeof toolNames[toolName] === 'function') {
      return toolNames[toolName]();
    }
  } catch {
    // 忽略加载错误
  }
  return toolName;
}

/**
 * 判断字符串是否为原始 UUID 或包含 UUID 的自动生成 ID
 */
export function isUuid(str?: string): boolean {
  if (!str) return false;
  const trimmed = str.trim();
  // 标准 UUID 格式 (36 字符: 8-4-4-4-12)
  const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
  // 带前缀如 node-xxx, start-xxx, assert_xxx
  const stripped = trimmed.replace(/^(node|start|entry|end|conn)-/, '');
  if (uuidRegex.test(trimmed) || uuidRegex.test(stripped)) {
    return true;
  }
  // 长度大于等于 32 的十六进制/哈希特征字符串，视为非人类可读 ID
  if (trimmed.length >= 32 && /^[0-9a-f-_]+$/i.test(trimmed)) {
    return true;
  }
  return false;
}

/**
 * 提取并过滤非人类可读的名称 (如果是 UUID 则返回 undefined)
 */
export function cleanName(name?: unknown): string | undefined {
  if (typeof name !== 'string') return undefined;
  const trimmed = name.trim();
  if (!trimmed || isUuid(trimmed)) return undefined;
  return trimmed;
}

interface ResolvedNodeInfo {
  name?: string;
  type?: string;
}

function resolveNode(nodeId?: string): ResolvedNodeInfo | undefined {
  if (!nodeId) return undefined;
  const nodes = useFlowStore.getState().flow?.nodes;
  const found = nodes?.find((n) => n.uuid === nodeId);
  if (found) {
    return {
      name: cleanName(found.name),
      type: found.type,
    };
  }
  return undefined;
}

function resolveDatasourceName(datasourceId?: string): string {
  if (!datasourceId) return '';
  const descriptions = useDatasourceStore.getState().descriptions;
  const found = descriptions?.find((d) => d.id === datasourceId);
  return found?.display_name || datasourceId;
}

function parseJsonSafe(str?: string): Record<string, unknown> | null {
  if (!str) return null;
  try {
    const val = JSON.parse(str);
    return val && typeof val === 'object' ? (val as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}

/**
 * 解析并格式化 Tool Call 的具体操作目标 (无中文前缀，无 UUID)
 */
export function formatToolTitle(
  toolName: string,
  rawArgs?: string | Record<string, unknown>,
  rawResult?: string,
): ToolTitleInfo {
  const args = typeof rawArgs === 'string' ? parseJsonSafe(rawArgs) || {} : rawArgs || {};
  const resultObj = parseJsonSafe(rawResult);

  let target: string | undefined;

  switch (toolName) {
    // ========================================================================
    // 1. 画布节点与连线操作
    // ========================================================================
    case 'update_node_config': {
      const nodeId = String(args.node_uuid || args.node_id || args.uuid || args.id || '');
      const resolved = resolveNode(nodeId);
      const name =
        cleanName(resultObj?.node_name) ||
        cleanName(args.name) ||
        cleanName(args.node_name) ||
        cleanName(resolved?.name);

      const type = resolved?.type || (typeof resultObj?.node_type === 'string' ? resultObj.node_type : undefined);
      if (name) {
        target = `"${name}"`;
      } else if (type) {
        target = type;
      }
      break;
    }

    case 'add_node': {
      const name = cleanName(resultObj?.name) || cleanName(args.name) || cleanName(args.node_name);
      const type = String(resultObj?.node_type || args.node_type || args.type || '');
      if (name) {
        target = `"${name}"`;
      } else if (type) {
        target = type;
      }
      break;
    }

    case 'batch_create_pipeline': {
      const name = cleanName(args.pipeline_name) || cleanName(args.name);
      const count =
        (typeof resultObj?.created_nodes_count === 'number'
          ? resultObj.created_nodes_count
          : undefined) ||
        (Array.isArray(args.nodes) ? args.nodes.length : undefined);
      if (name) {
        target = `"${name}"`;
      } else if (count) {
        target = `${count} nodes`;
      }
      break;
    }

    case 'connect_nodes': {
      const srcId = String(args.source_node_uuid || args.source_node_id || args.source || '');
      const tgtId = String(args.target_node_uuid || args.target_node_id || args.target || '');
      const srcNode = resolveNode(srcId);
      const tgtNode = resolveNode(tgtId);

      const srcName =
        cleanName(resultObj?.source) ||
        srcNode?.name ||
        srcNode?.type;
      const tgtName =
        cleanName(resultObj?.target) ||
        tgtNode?.name ||
        tgtNode?.type;

      if (srcName && tgtName) {
        target = `"${srcName}" ➔ "${tgtName}"`;
      }
      break;
    }

    case 'disconnect_nodes': {
      const srcId = String(args.source_node_uuid || args.source_node_id || '');
      const tgtId = String(args.target_node_uuid || args.target_node_id || '');
      const srcNode = resolveNode(srcId);
      const tgtNode = resolveNode(tgtId);

      const srcName = srcNode?.name || srcNode?.type;
      const tgtName = tgtNode?.name || tgtNode?.type;

      if (srcName && tgtName) {
        target = `"${srcName}" ➔ "${tgtName}"`;
      }
      break;
    }

    case 'delete_nodes': {
      const uuids = Array.isArray(args.node_uuids)
        ? (args.node_uuids as string[])
        : Array.isArray(args.node_ids)
          ? (args.node_ids as string[])
          : [];
      const names = uuids.map((id) => resolveNode(id)?.name).filter(Boolean) as string[];
      if (names.length > 0) {
        target = names.map((n) => `"${n}"`).join(', ');
      } else if (uuids.length > 0) {
        target = `${uuids.length} nodes`;
      }
      break;
    }

    case 'auto_layout':
    case 'clear_canvas':
      break;

    case 'get_node_detail': {
      const nodeId = String(args.node_uuid || args.node_id || '');
      const resolved = resolveNode(nodeId);
      const name = resolved?.name || resolved?.type;
      if (name) {
        target = `"${name}"`;
      }
      break;
    }

    case 'get_node_type_schema': {
      const typeId = cleanName(args.type_id || args.node_type);
      if (typeId) {
        target = typeId;
      }
      break;
    }

    case 'get_flow_nodes_config':
      break;

    // ========================================================================
    // 2. 脚本生成与更新
    // ========================================================================
    case 'generate_or_update_script': {
      const nodeId = String(args.node_uuid || args.node_id || '');
      const resolved = resolveNode(nodeId);
      const name = cleanName(args.node_name) || resolved?.name;
      if (name) {
        target = `"${name}"`;
      }
      break;
    }
    // ========================================================================
    // 3. 数据源插件相关
    // ========================================================================
    case 'browse_datasource': {
      const dsId = String(args.datasource_id || '');
      const dsName = resolveDatasourceName(dsId);
      const resId = cleanName(args.resource_id);
      if (dsName && resId) {
        target = `${dsName} · ${resId}`;
      } else if (dsName) {
        target = dsName;
      } else if (resId) {
        target = resId;
      }
      break;
    }

    case 'import_datasource_item_to_flow': {
      const itemName = cleanName(args.name) || cleanName(args.item_id);
      if (itemName) {
        target = `"${itemName}"`;
      }
      break;
    }

    // ========================================================================
    // 4. 流程文件操作
    // ========================================================================
    case 'create_flow': {
      const name = cleanName(args.name) || cleanName(args.flow_name);
      if (name) target = `"${name}"`;
      break;
    }

    case 'open_flow': {
      const name = cleanName(args.name) || cleanName(args.flow_id);
      if (name) target = `"${name}"`;
      break;
    }

    case 'save_current_flow':
    case 'run_flow':
    case 'run_flow_debug':
    case 'cancel_flow_execution':
      break;

    case 'diagnose_flow_errors': {
      const nodeId = String(args.failed_node_id || '');
      const resolved = resolveNode(nodeId);
      const name = cleanName(args.failed_node_name) || resolved?.name;
      if (name) target = `"${name}"`;
      break;
    }
    case 'switch_active_environment': {
      const name = cleanName(args.env_name) || cleanName(args.env_id);
      if (name) target = `"${name}"`;
      break;
    }

    case 'install_marketplace_plugin': {
      const plugin = cleanName(args.plugin_name || args.name);
      const ver = cleanName(args.version);
      if (plugin) {
        target = ver ? `"${plugin}" (${ver})` : `"${plugin}"`;
      }
      break;
    }

    default:
      break;
  }

  const toolDisplayName = getLocalizedToolName(toolName);
  const fullTitle = target ? `${toolDisplayName}: ${target}` : toolDisplayName;

  return {
    target,
    toolDisplayName,
    fullTitle,
    rawName: toolName,
    action: toolDisplayName,
  };
}
