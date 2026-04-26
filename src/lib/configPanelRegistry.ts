/**
 * 配置面板注册表
 * 将 14 种节点类型映射到对应的 Config 组件，消除 if/else 分支分发
 * 未知节点类型通过 isSchemaDriven() 标记，由消费者使用 SchemaConfigPanel 渲染
 */
import type { ComponentType } from 'react';

import { EntryConfig } from '@/components/panels/EntryConfig';
import { HttpConfig } from '@/components/panels/HttpConfig';
import { ConditionalConfig } from '@/components/panels/ConditionalConfig';
import { WsConnectConfig } from '@/components/panels/WsConnectConfig';
import { WsSendCollectConfig } from '@/components/panels/WsSendCollectConfig';
import { WsCloseConfig } from '@/components/panels/WsCloseConfig';
import { AssertionConfig } from '@/components/panels/AssertionConfig';
import { DbConnectConfig } from '@/components/panels/DbConnectConfig';
import { DbQueryConfig } from '@/components/panels/DbQueryConfig';
import { DbBeginConfig } from '@/components/panels/DbBeginConfig';
import { DbCommitConfig } from '@/components/panels/DbCommitConfig';
import { DbRollbackConfig } from '@/components/panels/DbRollbackConfig';
import { DbCloseConfig } from '@/components/panels/DbCloseConfig';
import { ScriptConfig } from '@/components/panels/ScriptConfig';
import { GrpcConnectConfig } from '@/components/panels/GrpcConnectConfig';
import { GrpcCallConfig } from '@/components/panels/GrpcCallConfig';
import { GrpcCloseConfig } from '@/components/panels/GrpcCloseConfig';
import { McpConnectConfig } from '@/components/panels/McpConnectConfig';
import { McpCallToolConfig } from '@/components/panels/McpCallToolConfig';
import { McpReadResourceConfig } from '@/components/panels/McpReadResourceConfig';
import { McpCloseConfig } from '@/components/panels/McpCloseConfig';
import { McpGetPromptConfig } from '@/components/panels/McpGetPromptConfig';
import { TcpConnectConfig } from '@/components/panels/TcpConnectConfig';
import { TcpSendConfig } from '@/components/panels/TcpSendConfig';
import { TcpCloseConfig } from '@/components/panels/TcpCloseConfig';
import { UdpBindConfig } from '@/components/panels/UdpBindConfig';
import { UdpSendToConfig } from '@/components/panels/UdpSendToConfig';
import { UdpRecvFromConfig } from '@/components/panels/UdpRecvFromConfig';
import { UdpCloseConfig } from '@/components/panels/UdpCloseConfig';
import { MqttConnectConfig } from '@/components/panels/MqttConnectConfig';
import { MqttPublishConfig } from '@/components/panels/MqttPublishConfig';
import { MqttSubscribeConfig } from '@/components/panels/MqttSubscribeConfig';
import { MqttUnsubscribeConfig } from '@/components/panels/MqttUnsubscribeConfig';
import { MqttDisconnectConfig } from '@/components/panels/MqttDisconnectConfig';
import { MqttWaitMessageConfig } from '@/components/panels/MqttWaitMessageConfig';
import { MqttRequestConfig } from '@/components/panels/MqttRequestConfig';
import { KafkaConnectConfig } from '@/components/panels/KafkaConnectConfig';
import { KafkaProduceConfig } from '@/components/panels/KafkaProduceConfig';
import { KafkaConsumeConfig } from '@/components/panels/KafkaConsumeConfig';
import { KafkaConsumeBatchConfig } from '@/components/panels/KafkaConsumeBatchConfig';
import { KafkaCloseConfig } from '@/components/panels/KafkaCloseConfig';
import { KafkaAdminConfig } from '@/components/panels/KafkaAdminConfig';
import { KafkaSchemaRegistryConfig } from '@/components/panels/KafkaSchemaRegistryConfig';
import { VariableExtractorConfig as VariableExtractorConfigPanel } from '@/components/panels/VariableExtractorConfig';

export const configPanelRegistry: Record<string, ComponentType<any>> = {
  entry: EntryConfig,
  http: HttpConfig,
  conditional: ConditionalConfig,
  ws_connect: WsConnectConfig,
  ws_send_collect: WsSendCollectConfig,
  ws_close: WsCloseConfig,
  assertion: AssertionConfig,
  db_connect: DbConnectConfig,
  db_query: DbQueryConfig,
  db_begin: DbBeginConfig,
  db_commit: DbCommitConfig,
  db_rollback: DbRollbackConfig,
  db_close: DbCloseConfig,
  script: ScriptConfig,
  grpc_connect: GrpcConnectConfig,
  grpc_call: GrpcCallConfig,
  grpc_close: GrpcCloseConfig,
  mcp_connect: McpConnectConfig,
  mcp_call_tool: McpCallToolConfig,
  mcp_read_resource: McpReadResourceConfig,
  mcp_close: McpCloseConfig,
  mcp_get_prompt: McpGetPromptConfig,
  tcp_connect: TcpConnectConfig,
  tcp_send: TcpSendConfig,
  tcp_close: TcpCloseConfig,
  udp_bind: UdpBindConfig,
  udp_send_to: UdpSendToConfig,
  udp_recv_from: UdpRecvFromConfig,
  udp_close: UdpCloseConfig,
  mqtt_connect: MqttConnectConfig,
  mqtt_publish: MqttPublishConfig,
  mqtt_subscribe: MqttSubscribeConfig,
  mqtt_unsubscribe: MqttUnsubscribeConfig,
  mqtt_disconnect: MqttDisconnectConfig,
  mqtt_wait_message: MqttWaitMessageConfig,
  mqtt_request: MqttRequestConfig,
  kafka_connect: KafkaConnectConfig,
  kafka_produce: KafkaProduceConfig,
  kafka_consume: KafkaConsumeConfig,
  kafka_consume_batch: KafkaConsumeBatchConfig,
  kafka_close: KafkaCloseConfig,
  kafka_admin: KafkaAdminConfig,
  kafka_schema_registry: KafkaSchemaRegistryConfig,
  variable_extractor: VariableExtractorConfigPanel,
};

/** 根据节点类型获取对应的配置面板组件 */
export function getConfigComponent(type: string): ComponentType<any> | undefined {
  return configPanelRegistry[type];
}

/** 判断节点类型是否需要 Schema 驱动渲染（未知类型 → true） */
export function isSchemaDriven(type: string): boolean {
  return !configPanelRegistry[type];
}
