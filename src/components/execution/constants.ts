/**
 * 执行组件共享常量
 */

import { i18nObject } from '@/i18n/i18n-util';
import { baseLocale } from '@/i18n/i18n-util';
import type { TranslationFunctions } from '@/i18n/i18n-types';
import type { NodeMetadata } from '@/types/nodeRegistry';

let _LL: TranslationFunctions | null = null;
const getLL = () => {
  if (!_LL) _LL = i18nObject(baseLocale);
  return _LL;
};

/** 节点类型标签映射 — 静态回退值 */
const FALLBACK_NODE_TYPE_NAMES: Record<string, string> = {
  entry: getLL().execution.node_types.entry(),
  http: getLL().execution.node_types.http(),
  conditional: getLL().execution.node_types.conditional(),
  script: getLL().execution.node_types.script(),
  assertion: getLL().execution.node_types.assertion(),
  end: getLL().execution.node_types.end(),
  ws_connect: getLL().execution.node_types.ws_connect(),
  ws_send_collect: getLL().execution.node_types.ws_send_collect(),
  ws_close: getLL().execution.node_types.ws_close(),
  db_connect: getLL().execution.node_types.db_connect(),
  db_query: getLL().execution.node_types.db_query(),
  db_begin: getLL().execution.node_types.db_begin(),
  db_commit: getLL().execution.node_types.db_commit(),
  db_rollback: getLL().execution.node_types.db_rollback(),
  db_close: getLL().execution.node_types.db_close(),
};

/** 节点类型颜色映射 — 静态回退值 */
const FALLBACK_NODE_TYPE_COLORS: Record<string, string> = {
  entry: 'bg-emerald-100 text-emerald-700',
  http: 'bg-blue-100 text-blue-700',
  conditional: 'bg-orange-100 text-orange-700',
  script: 'bg-indigo-100 text-indigo-700',
  assertion: 'bg-purple-100 text-purple-700',
  end: 'bg-slate-100 text-slate-700',
  ws_connect: 'bg-amber-100 text-amber-700',
  ws_send_collect: 'bg-teal-100 text-teal-700',
  ws_close: 'bg-rose-100 text-rose-700',
  db_connect: 'bg-violet-100 text-violet-700',
  db_query: 'bg-violet-100 text-violet-700',
  db_begin: 'bg-violet-100 text-violet-700',
  db_commit: 'bg-violet-100 text-violet-700',
  db_rollback: 'bg-violet-100 text-violet-700',
  db_close: 'bg-violet-100 text-violet-700',
};

/**
 * 动态构建节点类型标签映射
 *
 * 优先使用后端元数据的 display_name，回退到静态映射。
 *
 * @param metadata - 从后端获取的节点元数据
 * @returns 合并后的标签映射
 */
export function buildNodeTypeNames(
  metadata: NodeMetadata[]
): Record<string, string> {
  const dynamic: Record<string, string> = {};
  for (const m of metadata) {
    dynamic[m.type_id] = m.display_name;
  }
  return { ...FALLBACK_NODE_TYPE_NAMES, ...dynamic };
}

/**
 * 动态构建节点类型颜色映射
 *
 * 优先使用后端元数据的 color 转换为 Tailwind class，回退到静态映射。
 *
 * @param metadata - 从后端获取的节点元数据
 * @returns 合并后的颜色映射
 */
export function buildNodeTypeColors(
  _metadata: NodeMetadata[]
): Record<string, string> {
  // 后端返回十六进制颜色，需要转为 Tailwind class
  // 目前保留静态映射确保视觉一致性
  return { ...FALLBACK_NODE_TYPE_COLORS };
}

/**
 * 节点类型标签映射（与画布节点标签保持一致）
 *
 * 初始使用静态值，可由组件调用 buildNodeTypeNames 动态更新。
 */
export const NODE_TYPE_NAMES: Record<string, string> = {
  ...FALLBACK_NODE_TYPE_NAMES,
};

/**
 * 节点类型颜色映射（与画布节点主题色保持一致）
 *
 * 初始使用静态值，可由组件调用 buildNodeTypeColors 动态更新。
 */
export const NODE_TYPE_COLORS: Record<string, string> = {
  ...FALLBACK_NODE_TYPE_COLORS,
};

// ============================================================================
// 断言节点常量
// ============================================================================

import type { AssertionOperator, AssertionMode, OnFailureMode, AssertionFormData } from '../panels/schemas';

/** 断言操作符选项 (15 种) */
export const ASSERTION_OPERATOR_OPTIONS: { value: AssertionOperator; label: string }[] = [
  { value: 'equal', label: getLL().execution.assertion.operators.equal() },
  { value: 'not_equal', label: getLL().execution.assertion.operators.not_equal() },
  { value: 'greater', label: getLL().execution.assertion.operators.greater() },
  { value: 'greater_or_equal', label: getLL().execution.assertion.operators.greater_or_equal() },
  { value: 'less', label: getLL().execution.assertion.operators.less() },
  { value: 'less_or_equal', label: getLL().execution.assertion.operators.less_or_equal() },
  { value: 'contains', label: getLL().execution.assertion.operators.contains() },
  { value: 'not_contains', label: getLL().execution.assertion.operators.not_contains() },
  { value: 'starts_with', label: getLL().execution.assertion.operators.starts_with() },
  { value: 'ends_with', label: getLL().execution.assertion.operators.ends_with() },
  { value: 'exists', label: getLL().execution.assertion.operators.exists() },
  { value: 'not_exists', label: getLL().execution.assertion.operators.not_exists() },
  { value: 'is_empty', label: getLL().execution.assertion.operators.is_empty() },
  { value: 'is_not_empty', label: getLL().execution.assertion.operators.is_not_empty() },
  { value: 'matches_regex', label: getLL().execution.assertion.operators.matches_regex() },
];

/** 断言组合模式选项 */
export const ASSERTION_MODE_OPTIONS: { value: AssertionMode; label: string }[] = [
  { value: 'all', label: getLL().execution.assertion.modes.all() },
  { value: 'any', label: getLL().execution.assertion.modes.any() },
];

/** 失败处理模式选项 */
export const ON_FAILURE_MODE_OPTIONS: { value: OnFailureMode; label: string }[] = [
  { value: 'route_false', label: getLL().execution.assertion.failure_modes.route_false() },
  { value: 'throw', label: getLL().execution.assertion.failure_modes.throw() },
];

/** 断言预设模板 (5 种) */
export const ASSERTION_PRESET_OPTIONS: { value: string; label: string; assertions: AssertionFormData[] }[] = [
  {
    value: 'http_status',
    label: getLL().execution.assertion.presets.http_status(),
    assertions: [
      { target: '$.status', operator: 'equal', expected: '200', enabled: true, description: getLL().execution.assertion.descriptions.status_200() },
    ],
  },
  {
    value: 'http_success',
    label: getLL().execution.assertion.presets.http_success(),
    assertions: [
      { target: '$.status', operator: 'greater_or_equal', expected: '200', enabled: true, description: getLL().execution.assertion.descriptions.status_ge_200() },
      { target: '$.status', operator: 'less', expected: '300', enabled: true, description: getLL().execution.assertion.descriptions.status_lt_300() },
    ],
  },
  {
    value: 'response_body_exists',
    label: getLL().execution.assertion.presets.response_body_exists(),
    assertions: [
      { target: '$.body', operator: 'exists', expected: '', enabled: true, description: getLL().execution.assertion.descriptions.body_exists() },
    ],
  },
  {
    value: 'json_field_exists',
    label: getLL().execution.assertion.presets.json_field_exists(),
    assertions: [
      { target: '$.body.data', operator: 'exists', expected: '', enabled: true, description: getLL().execution.assertion.descriptions.data_field_exists() },
    ],
  },
  {
    value: 'empty_check',
    label: getLL().execution.assertion.presets.empty_check(),
    assertions: [
      { target: '$.body', operator: 'is_not_empty', expected: '', enabled: true, description: getLL().execution.assertion.descriptions.body_not_empty() },
    ],
  },
];

/** 默认断言配置 */
export const DEFAULT_ASSERTION_CONFIG = {
  type: 'assertion' as const,
  preset: undefined,
  mode: 'all' as AssertionMode,
  on_failure: 'route_false' as OnFailureMode,
  continue_on_failure: false,
  assertions: [] as AssertionFormData[],
};

/** 默认单条断言 */
export const DEFAULT_ASSERTION: AssertionFormData = {
  target: '',
  operator: 'equal',
  expected: '',
  enabled: true,
  description: '',
};
