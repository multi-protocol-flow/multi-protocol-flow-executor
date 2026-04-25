/**
 * MQTT 服务层 — MQTT 协议相关的 Tauri IPC 调用
 */

import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// 类型定义
// ============================================================================

/** MQTT 测试连接请求参数（对应 Rust MqttTestConnectionInput） */
export interface MqttTestConnectionInput {
  /** MQTT Broker 地址（如 mqtt://localhost:1883） */
  url: string;
  /** 客户端 ID（可选，服务器可能自动分配） */
  client_id?: string;
  /** 用户名（可选） */
  username?: string;
  /** 密码（可选） */
  password?: string;
  /** 是否使用 TLS */
  use_tls?: boolean;
  /** 是否跳过 TLS 证书验证 */
  tls_skip_verify?: boolean;
  /** 连接超时（毫秒） */
  connect_timeout_ms?: number;
}

/** MQTT Broker 信息（对应 Rust MqttBrokerInfo） */
export interface MqttBrokerInfo {
  /** Broker 支持的 MQTT 协议版本 */
  version?: string;
  /** 是否存在持久会话 */
  session_present?: boolean;
  /** 服务器分配的客户端 ID */
  assigned_client_id?: string;
  /** 服务器指定的 Keep Alive 时间（秒） */
  server_keep_alive?: number;
  /** 最大 Topic Alias 数量 */
  topic_alias_max?: number;
  /** 最大接收配额（QoS 1/2 消息） */
  receive_max?: number;
  /** 是否支持共享订阅 */
  shared_subscription_available?: boolean;
  /** 是否支持通配符订阅 */
  wildcard_sub_available?: boolean;
}

/** MQTT 测试连接响应（对应 Rust MqttTestConnectionOutput） */
export interface MqttTestConnectionOutput {
  /** 是否成功 */
  success: boolean;
  /** Broker 信息（成功时） */
  broker_info?: MqttBrokerInfo;
  /** 错误信息（失败时） */
  error?: string;
}

// ============================================================================
// IPC 调用
// ============================================================================

/**
 * 测试 MQTT Broker 连接
 *
 * 创建临时连接到 MQTT Broker，获取 Broker 信息后断开。
 * 不创建持久连接，仅返回 Broker 能力信息供前端展示。
 *
 * @param input - 测试连接请求参数
 * @returns 测试连接结果
 */
export async function testMqttConnection(
  input: MqttTestConnectionInput
): Promise<MqttTestConnectionOutput> {
  return invoke<MqttTestConnectionOutput>('mqtt_test_connection', { input });
}
