/**
 * 执行组件共享常量
 */

import { getLL } from '@/i18n/i18n-lazy';
import { getNodeDisplayName, getNodeMeta } from '@/lib/nodeMeta';

/**
 * 节点类型标签映射 — 动态代理
 *
 * 通过 getNodeDisplayName() 获取运行时当前 locale 的显示名称，
 * 覆盖全部 ~70 种节点类型，支持 zh-CN / en-US 动态切换。
 */
export const NODE_TYPE_NAMES: Record<string, string> = new Proxy(
  {} as Record<string, string>,
  {
    get(_target, prop: string) {
      return getNodeDisplayName(prop);
    },
  }
);

/**
 * 节点类型颜色映射 — 动态代理
 *
 * 通过 getNodeMeta() 获取每个节点类型的 badge 颜色 class。
 */
export const NODE_TYPE_COLORS: Record<string, string> = new Proxy(
  {} as Record<string, string>,
  {
    get(_target, prop: string) {
      return getNodeMeta(prop)?.badge ?? '';
    },
  }
);

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
