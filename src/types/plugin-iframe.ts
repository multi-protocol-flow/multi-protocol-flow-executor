/**
 * 插件 iframe postMessage 协议类型定义
 *
 * 与 docs/plugin-architecture.md §9.2 wire 契约对齐：
 * 宿主 → iframe: `init` / `configUpdated` / `actionResult`
 * iframe → 宿主: `ready` / `configChanged` / `requestAction` / `resize` / `error`
 *
 * 安全（D3）：所有入站消息必须通过 `isPluginIframeMessage` 守卫
 * （信封形状 + 按 type 的 payload 形状校验），再叠加
 * `event.source` 身份校验（权威）+ origin 纵深校验（沙箱不透明源 'null'
 * 恒接受，见 usePluginIframe）。
 * 未知 type 一律拒绝 —— 新增消息类型必须在此文件显式登记。
 */

/** 宿主 → iframe 消息类型 */
export type PluginIframeHostMessageType =
  | 'init'
  | 'configUpdated'
  | 'actionResult';

/** iframe → 宿主消息类型 */
export type PluginIframePluginMessageType =
  | 'ready'
  | 'configChanged'
  | 'requestAction'
  | 'resize'
  | 'error';

/** 全部协议消息类型 */
export type PluginIframeMessageType =
  | PluginIframeHostMessageType
  | PluginIframePluginMessageType;

/** 宿主声明可提供给插件的 host_api 能力（结构由 PR-5 白名单决定） */
export type PluginIframeHostApi = Record<string, unknown>;

/* ---------- 宿主 → iframe payload ---------- */

/** 同插件节点快照（connection 下拉等跨节点引用用） */
export interface PluginIframeNodeSnapshot {
  /** 画布节点 uuid（即插件 execute 的 node_instance_id） */
  uuid: string;
  /** 节点 label */
  label: string;
  /** 节点类型 id（如 `mcp:connect`） */
  type: string;
  /** 节点 config（含 type 字段） */
  config: Record<string, unknown>;
}

/** `init`：初始 config + 变量快照 + host_api 能力声明 + 节点上下文 */
export interface PluginIframeInitPayload {
  config: unknown;
  variables: Record<string, unknown>;
  host_api: PluginIframeHostApi;
  /** 当前编辑节点的类型 id（如 `mcp:connect`） */
  nodeType: string;
  /** 宿主当前 UI 语言（BCP-47，如 `zh-CN` / `en-US`） */
  locale: string;
  /** 同插件节点快照（按前缀过滤；可选，旧宿主不携带） */
  nodes?: PluginIframeNodeSnapshot[];
}

/** `configUpdated`：宿主侧配置变更推送 */
export interface PluginIframeConfigUpdatedPayload {
  config: unknown;
}

/** `actionResult`：requestAction 响应（requestId 关联） */
export interface PluginIframeActionResultPayload {
  requestId: string;
  result: unknown;
  /** 失败时携带错误信息（成功时省略） */
  error?: string;
}

/* ---------- iframe → 宿主 payload ---------- */

/** `ready`：插件面板初始化完成（无额外字段） */
export interface PluginIframeReadyPayload {}

/** `configChanged`：用户在面板中修改了表单 */
export interface PluginIframeConfigChangedPayload {
  config: unknown;
}

/** `requestAction`：请求宿主能力（testConnection / openFileDialog / 自定义透传） */
export interface PluginIframeRequestActionPayload {
  requestId: string;
  action: string;
  params?: unknown;
}

/** `resize`：面板自适应高度 */
export interface PluginIframeResizePayload {
  height: number;
}

/** `error`：面板内部错误上报 */
export interface PluginIframeErrorPayload {
  message: string;
}

/* ---------- 消息信封（type + payload 判别联合） ---------- */

export interface PluginIframeInitMessage {
  type: 'init';
  payload: PluginIframeInitPayload;
}

export interface PluginIframeConfigUpdatedMessage {
  type: 'configUpdated';
  payload: PluginIframeConfigUpdatedPayload;
}

export interface PluginIframeActionResultMessage {
  type: 'actionResult';
  payload: PluginIframeActionResultPayload;
}

export interface PluginIframeReadyMessage {
  type: 'ready';
  payload: PluginIframeReadyPayload;
}

export interface PluginIframeConfigChangedMessage {
  type: 'configChanged';
  payload: PluginIframeConfigChangedPayload;
}

export interface PluginIframeRequestActionMessage {
  type: 'requestAction';
  payload: PluginIframeRequestActionPayload;
}

export interface PluginIframeResizeMessage {
  type: 'resize';
  payload: PluginIframeResizePayload;
}

export interface PluginIframeErrorMessage {
  type: 'error';
  payload: PluginIframeErrorPayload;
}

/** 宿主发往 iframe 的消息 */
export type PluginIframeHostMessage =
  | PluginIframeInitMessage
  | PluginIframeConfigUpdatedMessage
  | PluginIframeActionResultMessage;

/** iframe 发往宿主的消息 */
export type PluginIframePluginMessage =
  | PluginIframeReadyMessage
  | PluginIframeConfigChangedMessage
  | PluginIframeRequestActionMessage
  | PluginIframeResizeMessage
  | PluginIframeErrorMessage;

/** 完整协议消息联合（宿主侧消费） */
export type PluginIframeMessage = PluginIframeHostMessage | PluginIframePluginMessage;

/* ---------- 守卫 ---------- */

/** 非 null 的普通对象（非数组） */
function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isString(value: unknown): value is string {
  return typeof value === 'string';
}

/** 非负有限数字（resize height 用） */
function isNonNegativeNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0;
}

/** payload 形状校验：按 type 分派（未知 type 一律 false） */
function isValidPayload(
  type: PluginIframeMessageType,
  payload: Record<string, unknown>
): boolean {
  switch (type) {
    case 'init': {
      // config 为任意 JSON 值（unknown），但键必须存在且非 undefined
      // （structured clone 会丢弃 undefined，传输后键不存在）
      const nodes = payload['nodes'];
      return (
        'config' in payload &&
        payload['config'] !== undefined &&
        isPlainObject(payload['variables']) &&
        isPlainObject(payload['host_api']) &&
        isString(payload['nodeType']) &&
        payload['nodeType'].length > 0 &&
        isString(payload['locale']) &&
        payload['locale'].length > 0 &&
        (nodes === undefined ||
          (Array.isArray(nodes) &&
            nodes.every(
              (node) =>
                isPlainObject(node) &&
                isString(node['uuid']) &&
                isString(node['label']) &&
                isString(node['type']) &&
                isPlainObject(node['config'])
            )))
      );
    }
    case 'configUpdated':
    case 'configChanged':
      return 'config' in payload && payload['config'] !== undefined;
    case 'actionResult': {
      return (
        isString(payload['requestId']) &&
        (payload['error'] === undefined || isString(payload['error']))
      );
    }
    case 'ready':
      return true;
    case 'requestAction':
      return isString(payload['requestId']) && isString(payload['action']);
    case 'resize':
      return isNonNegativeNumber(payload['height']);
    case 'error':
      return isString(payload['message']);
  }
}

/**
 * 协议消息类型守卫 —— postMessage 边界的解析层（parse, don't validate）。
 * 校验信封（type + payload 均为必需字段）并按 type 校验 payload 形状。
 * 未知 type / 畸形 payload 一律拒绝。
 */
export function isPluginIframeMessage(data: unknown): data is PluginIframeMessage {
  if (!isPlainObject(data)) return false;
  const { type, payload } = data;
  if (!isString(type) || !isPlainObject(payload)) return false;
  // 强制要求 type 属于已登记集合（union 成员判断）
  switch (type as PluginIframeMessageType) {
    case 'init':
    case 'configUpdated':
    case 'actionResult':
    case 'ready':
    case 'configChanged':
    case 'requestAction':
    case 'resize':
    case 'error':
      return isValidPayload(type as PluginIframeMessageType, payload);
    default:
      return false;
  }
}

/* ---------- 宿主 → iframe 消息构造器 ---------- */

/** 构造 `init` 消息（config + 变量快照 + host_api 能力声明 + 节点上下文） */
export function createInitMessage(
  config: unknown,
  variables: Record<string, unknown>,
  hostApi: PluginIframeHostApi,
  nodeType: string,
  locale: string,
  nodes?: PluginIframeNodeSnapshot[]
): PluginIframeInitMessage {
  const payload: PluginIframeInitPayload = {
    config,
    variables,
    host_api: hostApi,
    nodeType,
    locale,
  };
  if (nodes !== undefined) {
    payload.nodes = nodes;
  }
  return { type: 'init', payload };
}

/** 构造 `configUpdated` 消息（宿主侧配置变更推送） */
export function createConfigUpdatedMessage(config: unknown): PluginIframeConfigUpdatedMessage {
  return { type: 'configUpdated', payload: { config } };
}

/** 构造 `actionResult` 消息（requestAction 响应，成功省略 error） */
export function createActionResultMessage(
  requestId: string,
  result: unknown,
  error?: string
): PluginIframeActionResultMessage {
  return error === undefined
    ? { type: 'actionResult', payload: { requestId, result } }
    : { type: 'actionResult', payload: { requestId, result, error } };
}
