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
}
