/**
 * 数据源插件类型定义
 * 与后端 mpe_plugin_sdk::protocol::datasource 严格对齐（snake_case wire 字段）
 */

/** 数据条目协议族（对应宿主节点家族） */
export type DataItemProtocol =
  | 'http'
  | 'websocket'
  | 'sse'
  | 'tcp'
  | 'graphql'
  | 'mqtt'
  | 'kafka'
  | 'other';

/** 数据源描述（插件 describe 注册；`list_datasources` 返回） */
export interface DatasourceDescription {
  /** 数据源 id（唯一，如 "eolink"） */
  id: string;
  /** 展示名称（左侧栏面板显示） */
  display_name: string;
  /** 认证方式：`api_key` | `oauth2` | `none` */
  auth: { type: string };
  /** 连接配置 JSON Schema（驱动内联配置表单；缺省 = 无表单校验） */
  config_schema?: unknown;
  /** 顶层可浏览资源（项目列表 / 接口列表…） */
  resources: DatasourceResource[];
}

/** 一个可浏览的资源层级 */
export interface DatasourceResource {
  /** 资源 id（如 "project" / "api"） */
  id: string;
  /** 展示标签（如 "项目" / "接口"） */
  label: string;
  /** 子资源 id（展开父条目时浏览）；null = 无子级 */
  children?: string | null;
  /** true = 条目为可拖拽的叶子；false = 条目为可展开的分组 */
  leaf: boolean;
}

/** 一个可浏览的平台条目（`browse` 返回） */
export interface DataItem {
  /** 条目 id（叶子条目传给 get_item/get_templates） */
  id: string;
  /** 展示名称 */
  name: string;
  /** 协议族 */
  protocol: DataItemProtocol;
  /** HTTP 方法（HTTP 族条目） */
  method?: string | null;
  /** 路径 / 端点（HTTP 族条目） */
  path?: string | null;
  /** 一句话描述 */
  summary?: string | null;
  /** 平台特有展示字段（key-value） */
  extra: { label: string; value: string }[];
  /** 原始平台对象（get_item/get_templates 原样转发） */
  raw?: unknown;
}

/** 平台条目 → 可落画布节点模板（`get_templates` 返回） */
export interface NodeTemplate {
  /** 宿主已知节点 type_id（内置如 "http:request"，插件如 "kafka:produce"） */
  type_id: string;
  /** 落画布节点的展示名 */
  name: string;
  /** 直接作为画布节点 data */
  data: Record<string, unknown>;
  /** 溯源（数据源 + 条目 id；可选） */
  source_ref?: { datasource: string; item_id: string; updated_at?: string | null } | null;
}

/** 连接配置校验结果（`validate_config` 返回） */
export interface ValidateConfigResult {
  /** 配置是否可用（如 api_key 能认证） */
  valid: boolean;
  /** 拒绝原因（valid=false 时通常非空） */
  message?: string | null;
}

/** 一个已保存的数据源连接（前端持久化，`{appDataDir}/store/datasources.bin`） */
export interface DatasourceConnection {
  /** 连接 id（UUID） */
  id: string;
  /** 数据源 id（对应插件 describe 注册的 datasource.id） */
  datasource_id: string;
  /** 连接自定义名称（如 "支付中心项目"、"测试空间"），缺省时回退到 display_name */
  name?: string;
  /** 数据源插件展示名称（冗余自描述，便于面板展示） */
  display_name: string;
  /** 连接配置（api_key 等，保存时原样写入） */
  config: Record<string, unknown>;
  /** 创建时间（ISO 8601） */
  created_at: string;
}

/** 拖拽到画布的数据源条目 Payload */
export interface DatasourceDragPayload {
  datasource_id: string;
  connection_id?: string;
  item_ids: string[];
}
