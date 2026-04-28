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
import { TcpReceiveConfig } from '@/components/panels/TcpReceiveConfig';
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
import { SseConnectConfig } from '@/components/panels/SseConnectConfig';
import { SseListenConfig } from '@/components/panels/SseListenConfig';
import { SseDisconnectConfig } from '@/components/panels/SseDisconnectConfig';
import { GraphqlConnectConfig } from '@/components/panels/GraphqlConnectConfig';
import { GraphqlQueryConfig } from '@/components/panels/GraphqlQueryConfig';
import { GraphqlSubscribeConfig } from '@/components/panels/GraphqlSubscribeConfig';
import { GraphqlUnsubscribeConfig } from '@/components/panels/GraphqlUnsubscribeConfig';
import { GraphqlIntrospectConfig } from '@/components/panels/GraphqlIntrospectConfig';
import { GraphqlDisconnectConfig } from '@/components/panels/GraphqlDisconnectConfig';
import { AmqpConnectConfigPanel as AmqpConnectConfig } from '@/components/panels/AmqpConnectConfigPanel';
import { AmqpCloseConfigPanel as AmqpCloseConfig } from '@/components/panels/AmqpCloseConfigPanel';
import { AmqpExchangeDeclareConfigPanel as AmqpExchangeDeclareConfig } from '@/components/panels/AmqpExchangeDeclareConfigPanel';
import { AmqpExchangeDeleteConfigPanel as AmqpExchangeDeleteConfig } from '@/components/panels/AmqpExchangeDeleteConfigPanel';
import { AmqpQueueDeclareConfigPanel as AmqpQueueDeclareConfig } from '@/components/panels/AmqpQueueDeclareConfigPanel';
import { AmqpQueueDeleteConfigPanel as AmqpQueueDeleteConfig } from '@/components/panels/AmqpQueueDeleteConfigPanel';
import { AmqpQueueBindConfigPanel as AmqpQueueBindConfig } from '@/components/panels/AmqpQueueBindConfigPanel';
import { AmqpPublishConfigPanel as AmqpPublishConfig } from '@/components/panels/AmqpPublishConfigPanel';
import { AmqpConsumeConfigPanel as AmqpConsumeConfig } from '@/components/panels/AmqpConsumeConfigPanel';
import { AmqpGetConfigPanel as AmqpGetConfig } from '@/components/panels/AmqpGetConfigPanel';
import { AmqpAckConfigPanel as AmqpAckConfig } from '@/components/panels/AmqpAckConfigPanel';
import { AmqpManagementConfigPanel as AmqpManagementConfig } from '@/components/panels/AmqpManagementConfigPanel';
import { Amqp10ConnectConfigPanel as Amqp10ConnectConfig } from '@/components/panels/Amqp10ConnectConfigPanel';
import { Amqp10SendConfigPanel as Amqp10SendConfig } from '@/components/panels/Amqp10SendConfigPanel';
import { Amqp10ReceiveConfigPanel as Amqp10ReceiveConfig } from '@/components/panels/Amqp10ReceiveConfigPanel';
import { Amqp10AcceptConfigPanel as Amqp10AcceptConfig } from '@/components/panels/Amqp10AcceptConfigPanel';
import { Amqp10RejectConfigPanel as Amqp10RejectConfig } from '@/components/panels/Amqp10RejectConfigPanel';
import { Amqp10DisconnectConfigPanel as Amqp10DisconnectConfig } from '@/components/panels/Amqp10DisconnectConfigPanel';
import { VariableExtractorConfig as VariableExtractorConfigPanel } from '@/components/panels/VariableExtractorConfig';
import { ImapConnectPanel } from '@/components/panels/ImapConnectPanel';
import { ImapSelectPanel } from '@/components/panels/ImapSelectPanel';
import { ImapSearchPanel } from '@/components/panels/ImapSearchPanel';
import { ImapFetchPanel } from '@/components/panels/ImapFetchPanel';
import { ImapStoreFlagsPanel } from '@/components/panels/ImapStoreFlagsPanel';
import { ImapCopyPanel } from '@/components/panels/ImapCopyPanel';
import { ImapMovePanel } from '@/components/panels/ImapMovePanel';
import { ImapExpungePanel } from '@/components/panels/ImapExpungePanel';
import { ImapListPanel } from '@/components/panels/ImapListPanel';
import { ImapFolderCreatePanel } from '@/components/panels/ImapFolderCreatePanel';
import { ImapFolderDeletePanel } from '@/components/panels/ImapFolderDeletePanel';
import { ImapIdlePanel } from '@/components/panels/ImapIdlePanel';
import { ImapClosePanel } from '@/components/panels/ImapClosePanel';
import { SmtpConnectConfig } from '@/components/panels/SmtpConnectPanel';
import { SmtpSendConfig as SmtpSendPanel } from '@/components/panels/SmtpSendPanel';
import { SmtpDisconnectConfig as SmtpDisconnectPanel } from '@/components/panels/SmtpDisconnectPanel';
import { ProxyConfig as ProxyConfigPanel } from '@/components/panels/ProxyConfig';

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
  tcp_receive: TcpReceiveConfig,
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
  sse_connect: SseConnectConfig,
  sse_listen: SseListenConfig,
  sse_disconnect: SseDisconnectConfig,
  graphql_connect: GraphqlConnectConfig,
  graphql_query: GraphqlQueryConfig,
  graphql_subscribe: GraphqlSubscribeConfig,
  graphql_unsubscribe: GraphqlUnsubscribeConfig,
  graphql_introspect: GraphqlIntrospectConfig,
  graphql_disconnect: GraphqlDisconnectConfig,
  amqp_connect: AmqpConnectConfig,
  amqp_close: AmqpCloseConfig,
  amqp_exchange_declare: AmqpExchangeDeclareConfig,
  amqp_exchange_delete: AmqpExchangeDeleteConfig,
  amqp_queue_declare: AmqpQueueDeclareConfig,
  amqp_queue_delete: AmqpQueueDeleteConfig,
  amqp_queue_bind: AmqpQueueBindConfig,
  amqp_publish: AmqpPublishConfig,
  amqp_consume: AmqpConsumeConfig,
  amqp_get: AmqpGetConfig,
  amqp_ack: AmqpAckConfig,
  amqp_management: AmqpManagementConfig,
  amqp10_connect: Amqp10ConnectConfig,
  amqp10_send: Amqp10SendConfig,
  amqp10_receive: Amqp10ReceiveConfig,
  amqp10_accept: Amqp10AcceptConfig,
  amqp10_reject: Amqp10RejectConfig,
  amqp10_disconnect: Amqp10DisconnectConfig,
  variable_extractor: VariableExtractorConfigPanel,
  // SMTP 节点
  smtp_connect: SmtpConnectConfig,
  smtp_send: SmtpSendPanel,
  smtp_disconnect: SmtpDisconnectPanel,
  // IMAP 节点
  imap_connect: ImapConnectPanel,
  imap_select: ImapSelectPanel,
  imap_search: ImapSearchPanel,
  imap_fetch: ImapFetchPanel,
  imap_store_flags: ImapStoreFlagsPanel,
  imap_copy: ImapCopyPanel,
  imap_move: ImapMovePanel,
  imap_expunge: ImapExpungePanel,
  imap_list: ImapListPanel,
  imap_folder_create: ImapFolderCreatePanel,
  imap_folder_delete: ImapFolderDeletePanel,
  imap_idle: ImapIdlePanel,
  imap_close: ImapClosePanel,
  proxy: ProxyConfigPanel,
};

/** 根据节点类型获取对应的配置面板组件 */
export function getConfigComponent(type: string): ComponentType<any> | undefined {
  return configPanelRegistry[type];
}

/** 判断节点类型是否需要 Schema 驱动渲染（未知类型 → true） */
export function isSchemaDriven(type: string): boolean {
  return !configPanelRegistry[type];
}
