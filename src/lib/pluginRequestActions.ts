/**
 * pluginRequestActions —— 插件 iframe `requestAction` 白名单（PR-5 安全门）
 *
 * 安全模型：沙箱 iframe（仅 `allow-scripts`）内的插件内容是**不可信**的 ——
 * sandbox 只保护宿主 UI，不是二进制隔离边界。requestAction 是插件触发宿主
 * 能力的唯一通道，本模块是这条通道上**唯一的闸门**：
 *   - 只放行白名单动作（`testConnection` / `openFileDialog`）；
 *   - 未知动作 → `resolvePluginAction` 返回 undefined → 调用方立即以 rejected
 *     Promise 拒绝 → usePluginIframe 捕获后**立即**经 `actionResult` 回传错误
 *     （插件永不挂起，PR-3 锁定的 no-handler 拒绝路径）；
 *   - `hostApi` 能力声明（`PLUGIN_HOST_API`）由白名单生成，二者不可能漂移。
 *
 * 动作审计（每个动作按攻击面审查）：
 *
 * 1. `testConnection` —— **SSRF 原语**，三项约束：
 *    (a) 目标来源：只允许取用户**输入并审阅过**的节点 config（`context.config`，
 *        即流程文件中持久化的节点数据）。插件 `params` **绝不携带目标 URL 值** ——
 *        params 最多携带一个 `field`（config 字段名，默认 `'url'`），由宿主从
 *        config 中解析出目标字符串。攻击者推导出的 URL（如插件内容里抓来的内网
 *        地址）无法注入请求目标；目标一定是用户在 config 里亲手填过、在流程
 *        文件中可见的地址。
 *    (b) 硬超时：`TEST_CONNECTION_TIMEOUT_MS`（5s），AbortController 中止 ——
 *        插件无法让宿主无限挂起。
 *    (c) 返回脱敏：请求以 `mode: 'no-cors'` 发出 → 响应是 **opaque** 的，
 *        浏览器层面就不可读 body/headers/status（脱敏由构造保证，不是约定）；
 *        只回传 `{ ok, latency_ms?, error? }`，失败只回稳定错误码
 *        （`invalid_target` / `timeout` / `connection_failed`），绝不透传
 *        `err.message`（可能含内部 URL / 端口等敏感信息）。
 *
 * 2. `openFileDialog` —— 原生文件选择器，**只返回路径字符串**（取消 → null）：
 *    不返回文件内容、不做目录枚举。对话框选项固定 `{ multiple: false }`，
 *    插件 params 无法影响对话框行为（不能诱导打开特定目录/过滤器）。
 */
import { invoke } from '@tauri-apps/api/core';
import type { PluginIframeHostApi } from '@/types/plugin-iframe';

/** 白名单动作（新增动作 = 显式安全审查；与 HANDLERS 的 Record 完整性互锁） */
export const PLUGIN_REQUEST_ACTIONS = ['testConnection', 'openFileDialog', 'uiCall'] as const;
export type PluginRequestAction = (typeof PLUGIN_REQUEST_ACTIONS)[number];

/** testConnection 硬超时（ms）—— 插件不能挂起宿主 */
export const TEST_CONNECTION_TIMEOUT_MS = 5000;

/** 动作调用上下文（由宿主调用方构造，携带用户 config —— 各动作的目标/参数来源） */
export interface PluginActionContext {
  /** 用户输入并审阅过的节点 config（testConnection 目标的唯一合法来源） */
  config: unknown;
  /** 当前编辑节点的类型 id（uiCall 路由到对应插件进程；如 `mcp:connect`） */
  nodeType: string;
}

export type PluginActionHandler = (
  params?: unknown,
  context?: PluginActionContext
) => Promise<unknown>;

/** testConnection 脱敏结果 —— 绝不携带 body/headers/status 等原始响应信息 */
export interface TestConnectionResult {
  ok: boolean;
  /** 往返耗时（ms，请求已发出时存在） */
  latency_ms?: number;
  /** 失败原因 —— 稳定错误码，不透传原始错误信息 */
  error?: 'invalid_target' | 'timeout' | 'connection_failed';
}

/* ============================================================================
 * 工具（纯函数，直接可测）
 * ========================================================================== */

/** 非 null 的普通对象（非数组） */
function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/**
 * 从 params 解析 testConnection 的 config 字段名（默认 'url'）。
 * 注意：这里解析的是**字段名**，不是目标值 —— 目标值永远从 context.config 读取。
 */
function readTargetFieldFromParams(params: unknown): string {
  if (isPlainObject(params)) {
    const field = params['field'];
    if (typeof field === 'string' && field.trim() !== '') return field.trim();
  }
  return 'url';
}

/**
 * 从用户 config 解析连接测试目标。
 * 安全约束：
 *   - 字段名必须是纯标识符（`/^[A-Za-z_][A-Za-z0-9_]*$/`）—— 封死路径穿越
 *     （`a/b`）、原型链键（`__proto__` / `constructor`）等键名注入形态；
 *   - 值必须是非空字符串且为 http/https URL —— 拒绝 file:/ws:/data: 等
 *     任何非 HTTP(S) 协议（fetch 只允许 http(s)，前置校验是纵深防御）。
 * 解析失败一律返回 null（调用方归入 `invalid_target`，不发请求）。
 */
function resolveTargetFromConfig(config: unknown, field: string): string | null {
  if (!isPlainObject(config)) return null;
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(field)) return null;
  const value = config[field];
  if (typeof value !== 'string' || value.trim() === '') return null;
  const trimmed = value.trim();
  try {
    const parsed = new URL(trimmed);
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return null;
    return trimmed;
  } catch {
    return null;
  }
}

/* ============================================================================
 * 动作实现
 * ========================================================================== */

/**
 * testConnection —— 连接探测（SSRF 原语，见模块头审计 (a)(b)(c)）。
 * 探测方式：HEAD + `no-cors`。no-cors 让 fetch 在网络层成功时 resolve、
 * 失败时 reject，且响应为 opaque —— 状态码/头/体对宿主都不可见，
 * 天然满足"不泄漏内部服务信息"；HEAD 是最轻的探测方法。
 */
async function handleTestConnection(
  params: unknown,
  context?: PluginActionContext
): Promise<TestConnectionResult> {
  // (a) 目标来源：只从用户 config 解析，params 中的任何 URL 值都被忽略
  const field = readTargetFieldFromParams(params);
  const target = resolveTargetFromConfig(context?.config, field);
  if (target === null) return { ok: false, error: 'invalid_target' };

  // (b) 硬超时：插件无法让宿主无限挂起
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), TEST_CONNECTION_TIMEOUT_MS);
  const startedAt = performance.now();
  try {
    // (c) 脱敏：no-cors → opaque 响应，body/headers/status 不可读
    await fetch(target, {
      method: 'HEAD',
      mode: 'no-cors',
      cache: 'no-store',
      signal: controller.signal,
    });
    return { ok: true, latency_ms: Math.round(performance.now() - startedAt) };
  } catch {
    // 只回稳定错误码 —— 不透传 err.message（可能含内部 URL 等敏感信息）
    const error: TestConnectionResult['error'] = controller.signal.aborted
      ? 'timeout'
      : 'connection_failed';
    return {
      ok: false,
      latency_ms: Math.round(performance.now() - startedAt),
      error,
    };
  } finally {
    clearTimeout(timer);
  }
}

/**
 * openFileDialog —— 原生文件选择器（见模块头审计 2）。
 * 动态导入 @tauri-apps/plugin-dialog：Tauri 环境走原生对话框，
 * 浏览器模式（playground/测试）走 browser-shim（恒返回 null = 取消）。
 * 参数（插件可选提供）：
 *   - `multiple?: boolean` — 多选（返回 string[]）；
 *   - `filters?: {name: string; extensions: string[]}[]` — 文件类型过滤；
 *   - `directory?: boolean` — 目录选择。
 * 返回 `string | string[] | null`。
 */
async function handleOpenFileDialog(params?: unknown): Promise<string | string[] | null> {
  const { open } = await import('@tauri-apps/plugin-dialog');
  const options = isPlainObject(params) ? (params as Record<string, unknown>) : {};
  const multiple = options['multiple'] === true;
  const directory = options['directory'] === true;
  const filters = Array.isArray(options['filters'])
    ? (options['filters'] as { name: string; extensions: string[] }[])
    : undefined;
  const result = await open({ multiple, directory, filters });
  if (multiple) {
    if (Array.isArray(result)) {
      const strings = result.filter((r): r is string => typeof r === 'string');
      return strings;
    }
    if (typeof result === 'string') return [result];
    return null;
  }
  if (typeof result === 'string') return result;
  if (Array.isArray(result)) {
    const first = result[0];
    if (typeof first === 'string') return first;
  }
  return null;
}

/**
 * uiCall —— 配置面板运行时发现查询的宿主转发通道（见 docs/plugin-protocol.md §12）。
 *
 * 面板无法直接与插件进程通信；发现类查询（MCP 工具/资源/提示词列表、工具
 * input_schema 等）经宿主 `plugin_ui_call` Tauri 命令转发到 nodeType 对应
 * 插件的 `uiCall` RPC。约束：
 *   - `params` 必须是对象且含非空字符串 `method`（缺 method 一律拒绝，
 *     文案沿用 `Action not allowed` 风格，防止面板把任意键当方法名注入）；
 *   - 目标插件由 `context.nodeType` 决定（宿主侧未知类型即报错，天然防串插件）；
 *   - `payload`（可选）透传给插件；缺省传 `null`（Rust `Option` 侧为 None）。
 * 超时由宿主连接自带 request_timeout（默认 30s）兜底，这里不新造机制。
 */
async function handleUiCall(params: unknown, context?: PluginActionContext): Promise<unknown> {
  if (
    !isPlainObject(params) ||
    typeof params['method'] !== 'string' ||
    params['method'].trim() === ''
  ) {
    throw new Error('Action not allowed: uiCall requires a string `method`');
  }
  const nodeType = context?.nodeType;
  if (!nodeType) {
    throw new Error('Action not allowed: uiCall requires nodeType');
  }
  const payload = (params as { payload?: unknown }).payload;
  return invoke<unknown>('plugin_ui_call', {
    nodeType,
    method: params['method'],
    payload: payload === undefined ? null : payload,
  });
}

/* ============================================================================
 * 白名单 + 解析器
 * ========================================================================== */

/** 动作 → handler 注册表（Record 类型要求覆盖全部白名单动作，声明不漂移） */
const HANDLERS: Readonly<Record<PluginRequestAction, PluginActionHandler>> = {
  testConnection: handleTestConnection,
  openFileDialog: handleOpenFileDialog,
  uiCall: handleUiCall,
};

/**
 * hostApi 能力声明 —— 由白名单自动生成，与 `resolvePluginAction` 不可能漂移。
 * 通过 `init` 消息发送给插件，插件据此决定是否展示"测试连接"等按钮。
 */
export const PLUGIN_HOST_API: PluginIframeHostApi = Object.fromEntries(
  PLUGIN_REQUEST_ACTIONS.map((action) => [action, true])
);

/**
 * 白名单解析器 —— requestAction 的唯一安全闸。
 * 只对白名单动作返回 handler；**未知动作返回 undefined**（调用方必须拒绝）。
 * 返回的 handler 需要调用方传入 PluginActionContext（用户 config）。
 */
export function resolvePluginAction(action: string): PluginActionHandler | undefined {
  // 显式枚举白名单（与 PLUGIN_REQUEST_ACTIONS 由 HANDLERS 的 Record 完整性互锁，
  // 测试另断言枚举 ↔ 解析器一致性）
  switch (action) {
    case 'testConnection':
    case 'openFileDialog':
    case 'uiCall':
      return HANDLERS[action];
    default:
      return undefined;
  }
}

/**
 * 构造 onRequestAction 处理器（NodeConfigPanel 直接使用；也是集成测试的
 * 生产 wiring 来源）。未知动作 → rejected Promise（错误文案固定为
 * `Action not allowed: <action>`），usePluginIframe 捕获后**立即**经
 * actionResult 回传 —— 插件永不挂起。
 *
 * @param config 当前节点 config（用户输入并审阅过的数据）—— testConnection 的目标来源
 * @param nodeType 当前节点类型 id（如 `mcp:connect`）—— uiCall 的路由目标
 */
export function createPluginRequestHandler(
  config: unknown,
  nodeType: string
): (action: string, params?: unknown) => Promise<unknown> {
  return (action, params) => {
    const handler = resolvePluginAction(action);
    if (!handler) {
      return Promise.reject(new Error(`Action not allowed: ${action}`));
    }
    return handler(params, { config, nodeType });
  };
}
