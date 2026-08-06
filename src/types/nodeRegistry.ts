/**
 * 节点注册表类型定义
 * 与后端 flow_engine_core::plugin::NodeMetadata 对齐
 */

/**
 * 端口定义
 */
export interface PortDef {
  /** 端口标识，如 "true"、"false"、"output" */
  id: string;
  /** 端口显示名称，如 "成功"、"失败"、"输出" */
  label: string;
  /** 端口位置，如 "right"、"bottom" */
  position: string;
}

/**
 * 节点元数据（从后端注册表获取）
 */
export interface NodeMetadata {
  /** 节点类型标识，如 "http"、"db_connect" */
  type_id: string;
  /** 显示名称，如 "HTTP 请求" */
  display_name: string;
  /** 分组分类，如 "http"、"database"、"logic"、"websocket"、"tools" */
  category: string;
  /** 图标标识 */
  icon: string;
  /** 节点颜色（十六进制），如 "#3B82F6" */
  color: string;
  /** 端口定义列表 */
  ports: PortDef[];
  /** 拖拽创建节点时的默认配置 */
  default_config: Record<string, unknown>;
  /** 配置 JSON Schema（插件节点动态表单，可选） */
  config_schema?: unknown;
  /**
   * 前端描述（P0 仅支持 inline；P1 支持 iframe 等）
   * `url` 由宿主生成（D5/F3）：macOS/Linux 为
   * `mpe-plugin://<plugin-id>.localhost/config-panel`，Windows/Android 为
   * `http://<plugin-id>.localhost/config-panel`（P1 iframe 配置面板用）
   */
  frontend?: { type: string; content?: string; url?: string };
}
