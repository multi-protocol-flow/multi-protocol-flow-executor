/**
 * 执行结果查看器注册表
 * 将节点类型映射到对应的 Viewer 组件，替代 switch/case 分发
 */
import type { ComponentType } from 'react';
import type { NodeExecutionReport } from '@/types';
import { getLL } from '@/i18n/i18n-lazy';
import { HttpNodeViewer } from '@/components/execution/viewers/HttpNodeViewer';
import { WsConnectViewer } from '@/components/execution/viewers/WsConnectViewer';
import { WsSendCollectViewer } from '@/components/execution/viewers/WsSendCollectViewer';
import { WsCloseViewer } from '@/components/execution/viewers/WsCloseViewer';
import { ConditionalViewer } from '@/components/execution/viewers/ConditionalViewer';
import { AssertionViewer } from '@/components/execution/viewers/AssertionViewer';
import { GenericViewer } from '@/components/execution/viewers/GenericViewer';
import { GrpcViewer } from '@/components/execution/viewers/GrpcViewer';
import { TcpUdpViewer } from '@/components/execution/viewers/TcpUdpViewer';
import { McpViewer } from '@/components/execution/viewers/McpViewer';
import { MqttViewer } from '@/components/execution/viewers/MqttViewer';
import { KafkaViewer } from '@/components/execution/viewers/KafkaViewer';
import { SseViewer } from '@/components/execution/viewers/SseViewer';
import { GraphqlViewer } from '@/components/execution/viewers/GraphqlViewer';
import { AmqpViewer } from '@/components/execution/viewers/AmqpViewer';
import { EmailViewer } from '@/components/execution/viewers/EmailViewer';
import { PluginIframeViewer } from '@/components/execution/viewers/PluginIframeViewer';

interface ViewerProps {
  report: NodeExecutionReport;
}

/** End 节点查看器 — 显示流程结束信息 */
function EndNodeViewer({ report }: ViewerProps) {
  const ll = getLL();
  return (
    <div className="p-4 text-center">
      <div className="text-green-600 text-lg font-semibold mb-2">
        {ll.execution.report.end_node_reached()}
      </div>
      <p className="text-sm text-muted-foreground">
        {report.node_name}
      </p>
    </div>
  );
}

/**
 * 节点类型 → 专用 Viewer 组件映射
 * 未注册的类型将使用 GenericViewer 兜底
 */
export const viewerRegistry: Record<string, ComponentType<ViewerProps>> = {
  end: EndNodeViewer,
  http: HttpNodeViewer,
  ws_connect: WsConnectViewer,
  ws_send_collect: WsSendCollectViewer,
  ws_close: WsCloseViewer,
  conditional: ConditionalViewer,
  assertion: AssertionViewer,
  grpc_connect: GrpcViewer,
  grpc_call: GrpcViewer,
  grpc_close: GrpcViewer,
  mcp_connect: McpViewer,
  mcp_call_tool: McpViewer,
  mcp_read_resource: McpViewer,
  mcp_get_prompt: McpViewer,
  mcp_close: McpViewer,
  tcp_connect: TcpUdpViewer,
  tcp_send: TcpUdpViewer,
  tcp_receive: TcpUdpViewer,
  tcp_close: TcpUdpViewer,
  udp_bind: TcpUdpViewer,
  udp_send_to: TcpUdpViewer,
  udp_recv_from: TcpUdpViewer,
  udp_close: TcpUdpViewer,
  mqtt_connect: MqttViewer,
  mqtt_publish: MqttViewer,
  mqtt_subscribe: MqttViewer,
  mqtt_unsubscribe: MqttViewer,
  mqtt_disconnect: MqttViewer,
  mqtt_wait_message: MqttViewer,
  mqtt_request: MqttViewer,
  kafka_connect: KafkaViewer,
  kafka_produce: KafkaViewer,
  kafka_consume: KafkaViewer,
  kafka_consume_batch: KafkaViewer,
  kafka_admin: KafkaViewer,
  kafka_schema_registry: KafkaViewer,
  kafka_close: KafkaViewer,
  sse_connect: SseViewer,
  sse_listen: SseViewer,
  sse_disconnect: SseViewer,
  graphql_connect: GraphqlViewer,
  graphql_query: GraphqlViewer,
  graphql_subscribe: GraphqlViewer,
  graphql_introspect: GraphqlViewer,
  graphql_disconnect: GraphqlViewer,
  amqp_connect: AmqpViewer,
  amqp_close: AmqpViewer,
  amqp_exchange_declare: AmqpViewer,
  amqp_exchange_delete: AmqpViewer,
  amqp_queue_declare: AmqpViewer,
  amqp_queue_delete: AmqpViewer,
  amqp_queue_bind: AmqpViewer,
  amqp_publish: AmqpViewer,
  amqp_consume: AmqpViewer,
  amqp_get: AmqpViewer,
  amqp_ack: AmqpViewer,
  amqp_management: AmqpViewer,
  amqp10_connect: AmqpViewer,
  amqp10_send: AmqpViewer,
  amqp10_receive: AmqpViewer,
  amqp10_accept: AmqpViewer,
  amqp10_reject: AmqpViewer,
  amqp10_disconnect: AmqpViewer,
  variable_extractor: GenericViewer,
  // SMTP 节点
  smtp_connect: GenericViewer,
  smtp_send: GenericViewer,
  smtp_disconnect: GenericViewer,
  // IMAP 节点
  imap_connect: EmailViewer,
  imap_select: EmailViewer,
  imap_search: EmailViewer,
  imap_fetch: EmailViewer,
  imap_store_flags: EmailViewer,
  imap_copy: EmailViewer,
  imap_move: EmailViewer,
  imap_expunge: EmailViewer,
  imap_list: EmailViewer,
  imap_folder_create: EmailViewer,
  imap_folder_delete: EmailViewer,
  imap_idle: EmailViewer,
  imap_close: EmailViewer,
  proxy: GenericViewer,
};

/**
 * 根据节点类型获取对应的 Viewer 组件
 * - 已注册类型 → 精确 viewer（与 PR-4 之前完全一致，零回归）
 * - 未注册类型 + 插件执行载荷（report.plugin_data 存在）→ PluginIframeViewer
 * - 其余未注册类型 → GenericViewer（兜底）
 *
 * 插件检测规则：以「未注册类型 + plugin_data 存在」为准（而非类型名含 `:`），
 * 因为 plugin_data 是插件执行载荷的权威信号；无 plugin_data 的插件节点
 * （如校验失败未执行）继续走 GenericViewer 展示错误，体验更好。
 * `report` 可选：旧调用方不传时行为与 PR-4 之前完全一致。
 */
export function getViewerComponent(
  nodeType: string,
  report?: NodeExecutionReport
): ComponentType<ViewerProps> {
  const registered = viewerRegistry[nodeType];
  if (registered) return registered;
  // 插件分支：仅对未注册类型生效 —— 内置 70+ 类型解析路径不变
  if (report && report.plugin_data !== undefined) {
    return PluginIframeViewer;
  }
  return GenericViewer;
}
