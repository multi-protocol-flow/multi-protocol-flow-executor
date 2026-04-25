/**
 * 执行结果查看器注册表
 * 将节点类型映射到对应的 Viewer 组件，替代 switch/case 分发
 */
import type { ComponentType } from 'react';
import type { NodeExecutionReport } from '@/types';
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

interface ViewerProps {
  report: NodeExecutionReport;
}

/**
 * 节点类型 → 专用 Viewer 组件映射
 * 未注册的类型将使用 GenericViewer 兜底
 */
export const viewerRegistry: Record<string, ComponentType<ViewerProps>> = {
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
  variable_extractor: GenericViewer,
};

/**
 * 根据节点类型获取对应的 Viewer 组件
 * 未注册的类型返回 GenericViewer
 */
export function getViewerComponent(nodeType: string): ComponentType<ViewerProps> {
  return viewerRegistry[nodeType] ?? GenericViewer;
}
