//! Kafka 提供者抽象 trait

use async_trait::async_trait;
use crate::node_config::{
    KafkaConnectConfig, KafkaProduceConfig, KafkaConsumeConfig,
    KafkaConsumeBatchConfig, KafkaAdminConfig, KafkaSchemaRegistryConfig,
    KafkaCloseConfig,
};

/// Kafka 连接输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaConnectOutput {
    /// 是否成功
    pub success: bool,
    /// 连接信息
    #[serde(default)]
    pub connection_info: Option<KafkaConnectionInfo>,
    /// 错误信息
    #[serde(default)]
    pub error: Option<String>,
}

/// Kafka 连接信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaConnectionInfo {
    /// 连接 UUID
    pub uuid: String,
    /// Bootstrap Servers
    pub bootstrap_servers: String,
    /// 客户端 ID
    #[serde(default)]
    pub client_id: Option<String>,
    /// 安全协议
    #[serde(default)]
    pub security_protocol: Option<String>,
    /// 连接状态
    pub status: String,
    /// 连接时间戳（毫秒）
    pub connected_at: i64,
}

/// Kafka Broker 信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaBrokerInfo {
    /// Broker ID
    pub broker_id: i32,
    /// Broker 主机
    pub host: String,
    /// Broker 端口
    pub port: i32,
    /// Rack 信息
    #[serde(default)]
    pub rack: Option<String>,
}

/// Kafka 主题信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaTopicInfo {
    /// 主题名称
    pub name: String,
    /// 是否为内部主题
    #[serde(default)]
    pub is_internal: bool,
    /// 分区数
    #[serde(default)]
    pub partition_count: Option<i32>,
    /// 副本因子
    #[serde(default)]
    pub replication_factor: Option<i32>,
    /// 主题配置
    #[serde(default)]
    pub config: Option<std::collections::HashMap<String, String>>,
}

/// Kafka 分区信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaPartitionInfo {
    /// 分区 ID
    pub partition_id: i32,
    /// Leader Broker ID
    #[serde(default)]
    pub leader: Option<i32>,
    /// 副本列表
    #[serde(default)]
    pub replicas: Vec<i32>,
    /// ISR 列表
    #[serde(default)]
    pub isr: Vec<i32>,
    /// 起始偏移量
    #[serde(default)]
    pub beginning_offset: Option<i64>,
    /// 结束偏移量
    #[serde(default)]
    pub end_offset: Option<i64>,
}

/// Kafka 消费者组信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaConsumerGroupInfo {
    /// 消费者组 ID
    pub group_id: String,
    /// 状态
    #[serde(default)]
    pub state: Option<String>,
    /// 成员数
    #[serde(default)]
    pub members: Option<i32>,
    /// 协议类型
    #[serde(default)]
    pub protocol_type: Option<String>,
}

/// Kafka 消息输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaMessageOutput {
    /// 主题
    pub topic: String,
    /// 分区
    pub partition: i32,
    /// 偏移量
    pub offset: i64,
    /// 消息键
    #[serde(default)]
    pub key: Option<String>,
    /// 消息值
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    /// 消息头
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    /// 时间戳（毫秒）
    #[serde(default)]
    pub timestamp: Option<i64>,
}

/// Kafka 生产输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaProduceOutput {
    /// 是否成功
    pub success: bool,
    /// 消息所在分区
    #[serde(default)]
    pub partition: Option<i32>,
    /// 消息偏移量
    #[serde(default)]
    pub offset: Option<i64>,
    /// 时间信息
    #[serde(default)]
    pub timing: Option<KafkaTimingInfo>,
    /// 错误信息
    #[serde(default)]
    pub error: Option<String>,
}

/// Kafka 消费输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaConsumeOutput {
    /// 是否成功
    pub success: bool,
    /// 消费到的消息列表
    #[serde(default)]
    pub messages: Vec<KafkaMessageOutput>,
    /// 时间信息
    #[serde(default)]
    pub timing: Option<KafkaTimingInfo>,
    /// 错误信息
    #[serde(default)]
    pub error: Option<String>,
}

/// Kafka 批量消费输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaConsumeBatchOutput {
    /// 是否成功
    pub success: bool,
    /// 消费到的消息列表
    #[serde(default)]
    pub messages: Vec<KafkaMessageOutput>,
    /// 批量大小
    #[serde(default)]
    pub batch_size: Option<u32>,
    /// 时间信息
    #[serde(default)]
    pub timing: Option<KafkaTimingInfo>,
    /// 错误信息
    #[serde(default)]
    pub error: Option<String>,
}

/// Kafka 管理操作输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaAdminOutput {
    /// 是否成功
    pub success: bool,
    /// 操作类型
    #[serde(default)]
    pub operation: Option<String>,
    /// 主题列表
    #[serde(default)]
    pub topics: Vec<KafkaTopicInfo>,
    /// 分区列表
    #[serde(default)]
    pub partitions: Vec<KafkaPartitionInfo>,
    /// 消费者组列表
    #[serde(default)]
    pub consumer_groups: Vec<KafkaConsumerGroupInfo>,
    /// Broker 列表
    #[serde(default)]
    pub brokers: Vec<KafkaBrokerInfo>,
    /// 配置信息
    #[serde(default)]
    pub config: Option<std::collections::HashMap<String, String>>,
    /// 错误信息
    #[serde(default)]
    pub error: Option<String>,
}

/// Kafka Schema 信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaSchemaInfo {
    /// Schema 主题
    pub subject: String,
    /// 版本号
    pub version: i32,
    /// Schema ID
    #[serde(default)]
    pub schema_id: Option<i32>,
    /// Schema 类型
    #[serde(default)]
    pub schema_type: Option<String>,
    /// Schema 定义
    #[serde(default)]
    pub schema_definition: Option<String>,
    /// 兼容性级别
    #[serde(default)]
    pub compatibility: Option<String>,
}

/// Kafka Schema Registry 输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaSchemaRegistryOutput {
    /// 是否成功
    pub success: bool,
    /// 操作类型
    #[serde(default)]
    pub operation: Option<String>,
    /// 主题列表
    #[serde(default)]
    pub subjects: Vec<String>,
    /// 版本列表
    #[serde(default)]
    pub versions: Vec<i32>,
    /// Schema 信息
    #[serde(default)]
    pub schema: Option<KafkaSchemaInfo>,
    /// 是否兼容
    #[serde(default)]
    pub is_compatible: Option<bool>,
    /// 注册的 Schema ID
    #[serde(default)]
    pub registered_schema_id: Option<i32>,
    /// 错误信息
    #[serde(default)]
    pub error: Option<String>,
}

/// Kafka 关闭输出
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaCloseOutput {
    /// 是否成功
    pub success: bool,
    /// 错误信息
    #[serde(default)]
    pub error: Option<String>,
}

/// Kafka 时间信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KafkaTimingInfo {
    /// 总耗时（毫秒）
    pub total_ms: u64,
    /// 首条消息耗时（毫秒）
    #[serde(default)]
    pub first_message_ms: Option<u64>,
}

/// Kafka 提供者 trait
///
/// 抽象 Kafka 连接管理功能，使得执行器可以与具体的 Kafka 实现解耦。
#[async_trait]
pub trait KafkaProvider: Send + Sync {
    /// 建立 Kafka 连接
    async fn connect(
        &self,
        uuid: &str,
        config: &KafkaConnectConfig,
    ) -> Result<KafkaConnectOutput, String>;

    /// 生产消息
    async fn produce(
        &self,
        uuid: &str,
        config: &KafkaProduceConfig,
    ) -> Result<KafkaProduceOutput, String>;

    /// 消费消息
    async fn consume(
        &self,
        uuid: &str,
        config: &KafkaConsumeConfig,
    ) -> Result<KafkaConsumeOutput, String>;

    /// 批量消费消息
    async fn consume_batch(
        &self,
        uuid: &str,
        config: &KafkaConsumeBatchConfig,
    ) -> Result<KafkaConsumeBatchOutput, String>;

    /// 管理操作
    async fn admin(
        &self,
        uuid: &str,
        config: &KafkaAdminConfig,
    ) -> Result<KafkaAdminOutput, String>;

    /// Schema Registry 操作
    async fn schema_registry(
        &self,
        uuid: &str,
        config: &KafkaSchemaRegistryConfig,
    ) -> Result<KafkaSchemaRegistryOutput, String>;

    /// 关闭连接
    async fn close(
        &self,
        uuid: &str,
        config: &KafkaCloseConfig,
    ) -> Result<KafkaCloseOutput, String>;

    /// 获取连接状态
    fn get_status(&self, uuid: &str) -> Option<String>;

    /// 列出所有连接
    fn list_connections(&self) -> Vec<KafkaConnectionInfo>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kafka_connect_output_serialization() {
        let output = KafkaConnectOutput {
            success: true,
            connection_info: Some(KafkaConnectionInfo {
                uuid: "conn-001".to_string(),
                bootstrap_servers: "localhost:9092".to_string(),
                client_id: Some("client-001".to_string()),
                security_protocol: Some("plaintext".to_string()),
                status: "connected".to_string(),
                connected_at: 1700000000000,
            }),
            error: None,
        };

        let json = serde_json::to_string(&output).expect("序列化应成功");
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("conn-001"));
        assert!(json.contains("localhost:9092"));

        let deserialized: KafkaConnectOutput =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert!(deserialized.success);
        assert!(deserialized.connection_info.is_some());
        assert_eq!(deserialized.connection_info.unwrap().uuid, "conn-001");
    }

    #[test]
    fn test_kafka_connection_info_serialization() {
        let info = KafkaConnectionInfo {
            uuid: "conn-uuid-001".to_string(),
            bootstrap_servers: "broker1:9092,broker2:9092".to_string(),
            client_id: Some("my-client".to_string()),
            security_protocol: Some("sasl_ssl".to_string()),
            status: "connected".to_string(),
            connected_at: 1700000000000,
        };

        let json = serde_json::to_string(&info).expect("序列化应成功");
        assert!(json.contains("conn-uuid-001"));
        assert!(json.contains("broker1:9092,broker2:9092"));

        let deserialized: KafkaConnectionInfo =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(deserialized.uuid, "conn-uuid-001");
        assert_eq!(deserialized.status, "connected");
    }

    #[test]
    fn test_kafka_produce_output_serialization() {
        let output = KafkaProduceOutput {
            success: true,
            partition: Some(3),
            offset: Some(12345),
            timing: Some(KafkaTimingInfo {
                total_ms: 50,
                first_message_ms: None,
            }),
            error: None,
        };

        let json = serde_json::to_string(&output).expect("序列化应成功");
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("12345"));

        let deserialized: KafkaProduceOutput =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert!(deserialized.success);
        assert_eq!(deserialized.partition, Some(3));
        assert_eq!(deserialized.offset, Some(12345));
    }

    #[test]
    fn test_kafka_consume_output_serialization() {
        let output = KafkaConsumeOutput {
            success: true,
            messages: vec![KafkaMessageOutput {
                topic: "test-topic".to_string(),
                partition: 0,
                offset: 100,
                key: Some("key-1".to_string()),
                value: Some(serde_json::json!({"message": "hello"})),
                headers: vec![("trace-id".to_string(), "abc123".to_string())],
                timestamp: Some(1700000000000),
            }],
            timing: Some(KafkaTimingInfo {
                total_ms: 200,
                first_message_ms: Some(50),
            }),
            error: None,
        };

        let json = serde_json::to_string(&output).expect("序列化应成功");
        assert!(json.contains("test-topic"));
        assert!(json.contains("key-1"));
        assert!(json.contains("hello"));

        let deserialized: KafkaConsumeOutput =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert!(deserialized.success);
        assert_eq!(deserialized.messages.len(), 1);
        assert_eq!(deserialized.messages[0].topic, "test-topic");
    }

    #[test]
    fn test_kafka_consume_batch_output_serialization() {
        let output = KafkaConsumeBatchOutput {
            success: true,
            messages: vec![
                KafkaMessageOutput {
                    topic: "batch-topic".to_string(),
                    partition: 0,
                    offset: 200,
                    key: None,
                    value: Some(serde_json::json!("msg1")),
                    headers: vec![],
                    timestamp: None,
                },
                KafkaMessageOutput {
                    topic: "batch-topic".to_string(),
                    partition: 0,
                    offset: 201,
                    key: None,
                    value: Some(serde_json::json!("msg2")),
                    headers: vec![],
                    timestamp: None,
                },
            ],
            batch_size: Some(2),
            timing: Some(KafkaTimingInfo {
                total_ms: 500,
                first_message_ms: Some(100),
            }),
            error: None,
        };

        let json = serde_json::to_string(&output).expect("序列化应成功");
        assert!(json.contains("batch-topic"));
        assert!(json.contains("\"batch_size\":2"));

        let deserialized: KafkaConsumeBatchOutput =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert!(deserialized.success);
        assert_eq!(deserialized.messages.len(), 2);
    }

    #[test]
    fn test_kafka_admin_output_serialization() {
        let output = KafkaAdminOutput {
            success: true,
            operation: Some("topic_list".to_string()),
            topics: vec![KafkaTopicInfo {
                name: "my-topic".to_string(),
                is_internal: false,
                partition_count: Some(3),
                replication_factor: Some(2),
                config: None,
            }],
            partitions: vec![],
            consumer_groups: vec![],
            brokers: vec![],
            config: None,
            error: None,
        };

        let json = serde_json::to_string(&output).expect("序列化应成功");
        assert!(json.contains("topic_list"));
        assert!(json.contains("my-topic"));

        let deserialized: KafkaAdminOutput =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert!(deserialized.success);
        assert_eq!(deserialized.topics.len(), 1);
        assert_eq!(deserialized.topics[0].name, "my-topic");
    }

    #[test]
    fn test_kafka_schema_registry_output_serialization() {
        let output = KafkaSchemaRegistryOutput {
            success: true,
            operation: Some("list_subjects".to_string()),
            subjects: vec!["test-value".to_string(), "test-key".to_string()],
            versions: vec![],
            schema: None,
            is_compatible: None,
            registered_schema_id: None,
            error: None,
        };

        let json = serde_json::to_string(&output).expect("序列化应成功");
        assert!(json.contains("list_subjects"));
        assert!(json.contains("test-value"));

        let deserialized: KafkaSchemaRegistryOutput =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert!(deserialized.success);
        assert_eq!(deserialized.subjects.len(), 2);
    }

    #[test]
    fn test_kafka_close_output_serialization() {
        let output = KafkaCloseOutput {
            success: true,
            error: None,
        };

        let json = serde_json::to_string(&output).expect("序列化应成功");
        assert!(json.contains("\"success\":true"));

        let deserialized: KafkaCloseOutput =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert!(deserialized.success);
    }

    #[test]
    fn test_kafka_broker_info_serialization() {
        let info = KafkaBrokerInfo {
            broker_id: 1,
            host: "kafka-broker1".to_string(),
            port: 9092,
            rack: Some("rack-1".to_string()),
        };

        let json = serde_json::to_string(&info).expect("序列化应成功");
        assert!(json.contains("kafka-broker1"));
        assert!(json.contains("rack-1"));

        let deserialized: KafkaBrokerInfo =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(deserialized.broker_id, 1);
        assert_eq!(deserialized.port, 9092);
    }

    #[test]
    fn test_kafka_topic_info_serialization() {
        let info = KafkaTopicInfo {
            name: "__consumer_offsets".to_string(),
            is_internal: true,
            partition_count: Some(50),
            replication_factor: Some(3),
            config: None,
        };

        let json = serde_json::to_string(&info).expect("序列化应成功");
        assert!(json.contains("__consumer_offsets"));
        assert!(json.contains("true"));

        let deserialized: KafkaTopicInfo =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert!(deserialized.is_internal);
        assert_eq!(deserialized.partition_count, Some(50));
    }

    #[test]
    fn test_kafka_partition_info_serialization() {
        let info = KafkaPartitionInfo {
            partition_id: 0,
            leader: Some(1),
            replicas: vec![1, 2, 3],
            isr: vec![1, 2],
            beginning_offset: Some(0),
            end_offset: Some(1000000),
        };

        let json = serde_json::to_string(&info).expect("序列化应成功");
        assert!(json.contains("1000000"));

        let deserialized: KafkaPartitionInfo =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(deserialized.leader, Some(1));
        assert_eq!(deserialized.replicas, vec![1, 2, 3]);
        assert_eq!(deserialized.isr, vec![1, 2]);
    }

    #[test]
    fn test_kafka_consumer_group_info_serialization() {
        let info = KafkaConsumerGroupInfo {
            group_id: "my-group".to_string(),
            state: Some("Stable".to_string()),
            members: Some(3),
            protocol_type: Some("consumer".to_string()),
        };

        let json = serde_json::to_string(&info).expect("序列化应成功");
        assert!(json.contains("my-group"));
        assert!(json.contains("Stable"));

        let deserialized: KafkaConsumerGroupInfo =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(deserialized.group_id, "my-group");
        assert_eq!(deserialized.members, Some(3));
    }

    #[test]
    fn test_kafka_message_output_serialization() {
        let msg = KafkaMessageOutput {
            topic: "orders".to_string(),
            partition: 2,
            offset: 9999,
            key: Some("order-123".to_string()),
            value: Some(serde_json::json!({"id": 123, "status": "created"})),
            headers: vec![
                ("content-type".to_string(), "application/json".to_string()),
            ],
            timestamp: Some(1700000000000),
        };

        let json = serde_json::to_string(&msg).expect("序列化应成功");
        assert!(json.contains("orders"));
        assert!(json.contains("order-123"));
        assert!(json.contains("created"));

        let deserialized: KafkaMessageOutput =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(deserialized.topic, "orders");
        assert_eq!(deserialized.partition, 2);
        assert_eq!(deserialized.offset, 9999);
    }

    #[test]
    fn test_kafka_schema_info_serialization() {
        let info = KafkaSchemaInfo {
            subject: "orders-value".to_string(),
            version: 3,
            schema_id: Some(42),
            schema_type: Some("AVRO".to_string()),
            schema_definition: Some(r#"{"type":"record","name":"Order"}"#.to_string()),
            compatibility: Some("BACKWARD".to_string()),
        };

        let json = serde_json::to_string(&info).expect("序列化应成功");
        assert!(json.contains("orders-value"));
        assert!(json.contains("AVRO"));
        assert!(json.contains("BACKWARD"));

        let deserialized: KafkaSchemaInfo =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(deserialized.subject, "orders-value");
        assert_eq!(deserialized.version, 3);
        assert_eq!(deserialized.schema_id, Some(42));
    }

    #[test]
    fn test_kafka_timing_info_serialization() {
        let timing = KafkaTimingInfo {
            total_ms: 300,
            first_message_ms: Some(80),
        };

        let json = serde_json::to_string(&timing).expect("序列化应成功");
        let deserialized: KafkaTimingInfo =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(deserialized.total_ms, 300);
        assert_eq!(deserialized.first_message_ms, Some(80));
    }
}
