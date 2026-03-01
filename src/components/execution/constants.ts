/**
 * 执行组件共享常量
 */

/** 节点类型中文名映射 */
export const NODE_TYPE_NAMES: Record<string, string> = {
  entry: '入口',
  http: 'HTTP 请求',
  conditional: '条件分支',
  end: '终止',
};

/** 节点类型颜色映射 */
export const NODE_TYPE_COLORS: Record<string, string> = {
  entry: 'bg-emerald-100 text-emerald-700',
  http: 'bg-blue-100 text-blue-700',
  conditional: 'bg-amber-100 text-amber-700',
  end: 'bg-slate-100 text-slate-700',
};
