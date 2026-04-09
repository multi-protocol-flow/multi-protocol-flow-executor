/**
 * 执行组件共享常量
 */

/** 节点类型中文名映射 */
export const NODE_TYPE_NAMES: Record<string, string> = {
  entry: '入口',
  http: 'HTTP 请求',
  conditional: '条件分支',
  assertion: '断言',
  end: '终止',
  ws_connect: 'WebSocket 连接',
  ws_send_collect: 'WebSocket 发送采集',
  ws_close: 'WebSocket 关闭',
};

/** 节点类型颜色映射 */
export const NODE_TYPE_COLORS: Record<string, string> = {
  entry: 'bg-emerald-100 text-emerald-700',
  http: 'bg-blue-100 text-blue-700',
  conditional: 'bg-amber-100 text-amber-700',
  assertion: 'bg-rose-100 text-rose-700',
  end: 'bg-slate-100 text-slate-700',
  ws_connect: 'bg-violet-100 text-violet-700',
  ws_send_collect: 'bg-violet-100 text-violet-700',
  ws_close: 'bg-violet-100 text-violet-700',
};

// ============================================================================
// 断言节点常量
// ============================================================================

import type { AssertionOperator, AssertionMode, OnFailureMode, AssertionFormData } from '../panels/schemas';

/** 断言操作符选项 (15 种) */
export const ASSERTION_OPERATOR_OPTIONS: { value: AssertionOperator; label: string }[] = [
  { value: 'equal', label: '等于' },
  { value: 'not_equal', label: '不等于' },
  { value: 'greater', label: '大于' },
  { value: 'greater_or_equal', label: '大于等于' },
  { value: 'less', label: '小于' },
  { value: 'less_or_equal', label: '小于等于' },
  { value: 'contains', label: '包含' },
  { value: 'not_contains', label: '不包含' },
  { value: 'starts_with', label: '以...开始' },
  { value: 'ends_with', label: '以...结束' },
  { value: 'exists', label: '存在' },
  { value: 'not_exists', label: '不存在' },
  { value: 'is_empty', label: '为空' },
  { value: 'is_not_empty', label: '不为空' },
  { value: 'matches_regex', label: '匹配正则' },
];

/** 断言组合模式选项 */
export const ASSERTION_MODE_OPTIONS: { value: AssertionMode; label: string }[] = [
  { value: 'all', label: '全部通过 (AND)' },
  { value: 'any', label: '任一通过 (OR)' },
];

/** 失败处理模式选项 */
export const ON_FAILURE_MODE_OPTIONS: { value: OnFailureMode; label: string }[] = [
  { value: 'route_false', label: '路由到 false 分支' },
  { value: 'throw', label: '抛出异常终止流程' },
];

/** 断言预设模板 (5 种) */
export const ASSERTION_PRESET_OPTIONS: { value: string; label: string; assertions: AssertionFormData[] }[] = [
  {
    value: 'http_status',
    label: 'HTTP 状态码检查',
    assertions: [
      { target: '$.status', operator: 'equal', expected: '200', enabled: true, description: '状态码为 200' },
    ],
  },
  {
    value: 'http_success',
    label: 'HTTP 成功响应',
    assertions: [
      { target: '$.status', operator: 'greater_or_equal', expected: '200', enabled: true, description: '状态码 >= 200' },
      { target: '$.status', operator: 'less', expected: '300', enabled: true, description: '状态码 < 300' },
    ],
  },
  {
    value: 'response_body_exists',
    label: '响应体存在',
    assertions: [
      { target: '$.body', operator: 'exists', expected: '', enabled: true, description: '响应体存在' },
    ],
  },
  {
    value: 'json_field_exists',
    label: 'JSON 字段存在',
    assertions: [
      { target: '$.body.data', operator: 'exists', expected: '', enabled: true, description: 'data 字段存在' },
    ],
  },
  {
    value: 'empty_check',
    label: '非空检查',
    assertions: [
      { target: '$.body', operator: 'is_not_empty', expected: '', enabled: true, description: '响应体不为空' },
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
