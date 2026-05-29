// 节点模块 - 完整结构体实现

use crate::node_config::NodeData;

/// 节点类型枚举
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    /// 入口节点
    Entry,
    /// 结束节点
    #[serde(rename = "end")]
    End,
    /// HTTP 请求节点
    Http,
    /// 条件分支节点
    Conditional,
    /// WebSocket 连接节点
    #[serde(rename = "ws_connect")]
    WsConnect,
    /// WebSocket 发送采集节点
    #[serde(rename = "ws_send_collect")]
    WsSendCollect,
    /// WebSocket 关闭节点
    #[serde(rename = "ws_close")]
    WsClose,
    /// 断言节点
    #[serde(rename = "assertion")]
    Assertion,
    /// 数据库连接节点
    #[serde(rename = "db_connect")]
    DbConnect,
    /// 数据库查询节点
    #[serde(rename = "db_query")]
    DbQuery,
    /// 数据库事务开始节点
    #[serde(rename = "db_begin")]
    DbBegin,
    /// 数据库事务提交节点
    #[serde(rename = "db_commit")]
    DbCommit,
    /// 数据库事务回滚节点
    #[serde(rename = "db_rollback")]
    DbRollback,
    /// 数据库关闭节点
    #[serde(rename = "db_close")]
    DbClose,
    /// 脚本节点
    #[serde(rename = "script")]
    Script,
    /// gRPC 连接节点
    #[serde(rename = "grpc_connect")]
    GrpcConnect,
    /// gRPC 调用节点
    #[serde(rename = "grpc_call")]
    GrpcCall,
    /// gRPC 关闭节点
    #[serde(rename = "grpc_close")]
    GrpcClose,
    /// MCP 连接节点
    #[serde(rename = "mcp_connect")]
    McpConnect,
    /// MCP 调用工具节点
    #[serde(rename = "mcp_call_tool")]
    McpCallTool,
    /// MCP 读取资源节点
    #[serde(rename = "mcp_read_resource")]
    McpReadResource,
    /// MCP 关闭连接节点
    #[serde(rename = "mcp_close")]
    McpClose,
    /// MCP 获取提示词节点
    #[serde(rename = "mcp_get_prompt")]
    McpGetPrompt,
    /// TCP 连接节点
    #[serde(rename = "tcp_connect")]
    TcpConnect,
    /// TCP 发送节点
    #[serde(rename = "tcp_send")]
    TcpSend,
    /// TCP 关闭节点
    #[serde(rename = "tcp_close")]
    TcpClose,
    /// TCP 接收节点
    #[serde(rename = "tcp_receive")]
    TcpReceive,
    /// UDP 绑定节点
    #[serde(rename = "udp_bind")]
    UdpBind,
    /// UDP 发送节点
    #[serde(rename = "udp_send_to")]
    UdpSendTo,
    /// UDP 接收节点
    #[serde(rename = "udp_recv_from")]
    UdpRecvFrom,
    /// UDP 关闭节点
    #[serde(rename = "udp_close")]
    UdpClose,
    /// MQTT 连接节点
    #[serde(rename = "mqtt_connect")]
    MqttConnect,
    /// MQTT 发布节点
    #[serde(rename = "mqtt_publish")]
    MqttPublish,
    /// MQTT 订阅节点
    #[serde(rename = "mqtt_subscribe")]
    MqttSubscribe,
    /// MQTT 取消订阅节点
    #[serde(rename = "mqtt_unsubscribe")]
    MqttUnsubscribe,
    /// MQTT 断开连接节点
    #[serde(rename = "mqtt_disconnect")]
    MqttDisconnect,
    /// MQTT 等待消息节点
    #[serde(rename = "mqtt_wait_message")]
    MqttWaitMessage,
    /// MQTT 请求-响应节点
    #[serde(rename = "mqtt_request")]
    MqttRequest,
    /// Kafka 连接节点
    #[serde(rename = "kafka_connect")]
    KafkaConnect,
    /// Kafka 生产消息节点
    #[serde(rename = "kafka_produce")]
    KafkaProduce,
    /// Kafka 消费消息节点
    #[serde(rename = "kafka_consume")]
    KafkaConsume,
    /// Kafka 批量消费消息节点
    #[serde(rename = "kafka_consume_batch")]
    KafkaConsumeBatch,
    /// Kafka 管理操作节点
    #[serde(rename = "kafka_admin")]
    KafkaAdmin,
    /// Kafka Schema Registry 节点
    #[serde(rename = "kafka_schema_registry")]
    KafkaSchemaRegistry,
    /// Kafka 关闭连接节点
    #[serde(rename = "kafka_close")]
    KafkaClose,
    /// SSE 连接节点
    #[serde(rename = "sse_connect")]
    SseConnect,
    /// SSE 监听事件节点
    #[serde(rename = "sse_listen")]
    SseListen,
    /// SSE 断开连接节点
    #[serde(rename = "sse_disconnect")]
    SseDisconnect,
    /// GraphQL 连接节点
    #[serde(rename = "graphql_connect")]
    GraphqlConnect,
    /// GraphQL 查询节点 (Query + Mutation)
    #[serde(rename = "graphql_query")]
    GraphqlQuery,
    /// GraphQL 订阅节点
    #[serde(rename = "graphql_subscribe")]
    GraphqlSubscribe,
    /// GraphQL Schema 探索节点
    #[serde(rename = "graphql_introspect")]
    GraphqlIntrospect,
    /// GraphQL 断开连接节点
    #[serde(rename = "graphql_disconnect")]
    GraphqlDisconnect,
    /// AMQP 连接节点
    #[serde(rename = "amqp_connect")]
    AmqpConnect,
    /// AMQP 关闭连接节点
    #[serde(rename = "amqp_close")]
    AmqpClose,
    /// AMQP 声明交换器节点
    #[serde(rename = "amqp_exchange_declare")]
    AmqpExchangeDeclare,
    /// AMQP 删除交换器节点
    #[serde(rename = "amqp_exchange_delete")]
    AmqpExchangeDelete,
    /// AMQP 声明队列节点
    #[serde(rename = "amqp_queue_declare")]
    AmqpQueueDeclare,
    /// AMQP 删除队列节点
    #[serde(rename = "amqp_queue_delete")]
    AmqpQueueDelete,
    /// AMQP 绑定队列节点
    #[serde(rename = "amqp_queue_bind")]
    AmqpQueueBind,
    /// AMQP 发布消息节点
    #[serde(rename = "amqp_publish")]
    AmqpPublish,
    /// AMQP 消费消息节点
    #[serde(rename = "amqp_consume")]
    AmqpConsume,
    /// AMQP 获取消息节点
    #[serde(rename = "amqp_get")]
    AmqpGet,
    /// AMQP 确认消息节点
    #[serde(rename = "amqp_ack")]
    AmqpAck,
    /// AMQP 管理节点 (可选 feature)
    #[serde(rename = "amqp_management")]
    AmqpManagement,
    /// AMQP 1.0 连接节点
    #[serde(rename = "amqp10_connect")]
    Amqp10Connect,
    /// AMQP 1.0 发送消息节点
    #[serde(rename = "amqp10_send")]
    Amqp10Send,
    /// AMQP 1.0 接收消息节点
    #[serde(rename = "amqp10_receive")]
    Amqp10Receive,
    /// AMQP 1.0 确认消息节点
    #[serde(rename = "amqp10_accept")]
    Amqp10Accept,
    /// AMQP 1.0 拒绝消息节点
    #[serde(rename = "amqp10_reject")]
    Amqp10Reject,
    /// AMQP 1.0 断开连接节点
    #[serde(rename = "amqp10_disconnect")]
    Amqp10Disconnect,
    /// 变量提取节点
    #[serde(rename = "variable_extractor")]
    VariableExtractor,
    /// 代理设置节点
    #[serde(rename = "proxy")]
    Proxy,
    // SMTP 节点
    /// SMTP 连接节点
    #[serde(rename = "smtp_connect")]
    SmtpConnect,
    /// SMTP 发送邮件节点
    #[serde(rename = "smtp_send")]
    SmtpSend,
    /// SMTP 断开连接节点
    #[serde(rename = "smtp_disconnect")]
    SmtpDisconnect,
    // IMAP 节点
    /// IMAP 连接节点
    #[serde(rename = "imap_connect")]
    ImapConnect,
    /// IMAP 选择邮箱节点
    #[serde(rename = "imap_select")]
    ImapSelect,
    /// IMAP 搜索邮件节点
    #[serde(rename = "imap_search")]
    ImapSearch,
    /// IMAP 获取邮件节点
    #[serde(rename = "imap_fetch")]
    ImapFetch,
    /// IMAP 设置标志节点
    #[serde(rename = "imap_store_flags")]
    ImapStoreFlags,
    /// IMAP 复制邮件节点
    #[serde(rename = "imap_copy")]
    ImapCopy,
    /// IMAP 移动邮件节点
    #[serde(rename = "imap_move")]
    ImapMove,
    /// IMAP 永久删除节点
    #[serde(rename = "imap_expunge")]
    ImapExpunge,
    /// IMAP 列出邮箱节点
    #[serde(rename = "imap_list")]
    ImapList,
    /// IMAP 创建文件夹节点
    #[serde(rename = "imap_folder_create")]
    ImapFolderCreate,
    /// IMAP 删除文件夹节点
    #[serde(rename = "imap_folder_delete")]
    ImapFolderDelete,
    /// IMAP IDLE 监听节点
    #[serde(rename = "imap_idle")]
    ImapIdle,
    /// IMAP 关闭连接节点
    #[serde(rename = "imap_close")]
    ImapClose,
}

/// 节点位置结构体
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NodePosition {
    /// X 坐标
    pub x: f64,
    /// Y 坐标
    pub y: f64,
}

impl Default for NodePosition {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// 端口类型枚举
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortType {
    /// 输入端口
    Input,
    /// 输出端口
    Output,
}

/// 端口结构体
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Port {
    /// 端口唯一标识符
    pub id: String,
    /// 端口类型
    #[serde(rename = "type")]
    pub port_type: PortType,
    /// 端口标签（可选）
    pub label: Option<String>,
}

/// 节点数据结构
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    /// 节点唯一标识符（前端生成UUID，用于连接关联）
    pub uuid: String,
    /// 节点类型（字符串形式，使用 `node_types` 模块中的常量进行比较）
    #[serde(rename = "type")]
    pub node_type: String,
    /// 节点名称
    pub name: String,
    /// 节点位置
    pub position: NodePosition,
    /// 节点特定数据（强类型配置）
    pub data: NodeData,
    /// 端口数组（可选）
    pub ports: Option<Vec<Port>>,
    /// 是否设置断点（可选，默认 false）
    #[serde(default)]
    pub has_breakpoint: Option<bool>,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            uuid: String::new(),
            node_type: crate::node_types::ENTRY.to_string(),
            name: String::new(),
            position: NodePosition::default(),
            data: NodeData::default(),
            ports: None,
            has_breakpoint: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_config::{ConditionOperator, NodeData};
    use serde_json::json;

    #[test]
    fn test_node_creation() {
        let node = Node {
            uuid: "node-uuid-123".to_string(),
            node_type: "entry".to_string(),
            name: "Entry Node".to_string(),
            position: NodePosition { x: 100.0, y: 200.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        };

        assert_eq!(node.uuid, "node-uuid-123");
        assert_eq!(node.name, "Entry Node");
        assert_eq!(node.position.x, 100.0);
        assert_eq!(node.position.y, 200.0);
        assert_eq!(node.has_breakpoint, None);
    }

    #[test]
    fn test_node_serialization() {
        let node = Node {
            uuid: "node-uuid-456".to_string(),
            node_type: "http".to_string(),
            name: "HTTP Request".to_string(),
            position: NodePosition { x: 150.0, y: 250.0 },
            data: NodeData::http("https://api.example.com"),
            ports: None,
            has_breakpoint: Some(true),
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("node-uuid-456"));
        assert!(json.contains("https://api.example.com"));
        assert!(json.contains("has_breakpoint"));

        // 反序列化测试
        let deserialized: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uuid, "node-uuid-456");
        assert_eq!(deserialized.name, "HTTP Request");
        assert_eq!(deserialized.has_breakpoint, Some(true));
    }

    #[test]
    fn test_port_creation() {
        let port = Port {
            id: "port-123".to_string(),
            port_type: PortType::Input,
            label: Some("Input Port".to_string()),
        };

        assert_eq!(port.id, "port-123");
        assert_eq!(port.port_type, PortType::Input);
        assert_eq!(port.label, Some("Input Port".to_string()));
    }

    #[test]
    fn test_port_id_field_unchanged() {
        // 确认 Port 结构保持 id 字段，没有改为 uuid
        let port = Port {
            id: "port-id-unchanged".to_string(),
            port_type: PortType::Output,
            label: None,
        };

        assert_eq!(port.id, "port-id-unchanged");
    }

    #[test]
    fn test_port_serialization() {
        let port = Port {
            id: "port-789".to_string(),
            port_type: PortType::Input,
            label: Some("Test Port".to_string()),
        };

        let json = serde_json::to_string(&port).unwrap();
        assert!(json.contains("port-789"));
        assert!(json.contains("input"));

        // 反序列化测试
        let deserialized: Port = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "port-789");
        assert_eq!(deserialized.port_type, PortType::Input);
    }

    #[test]
    fn test_node_with_ports() {
        let ports = vec![
            Port {
                id: "port-1".to_string(),
                port_type: PortType::Input,
                label: Some("Input".to_string()),
            },
            Port {
                id: "port-2".to_string(),
                port_type: PortType::Output,
                label: Some("Output".to_string()),
            },
        ];

        let node = Node {
            uuid: "node-uuid-999".to_string(),
            node_type: "conditional".to_string(),
            name: "Conditional Node".to_string(),
            position: NodePosition { x: 50.0, y: 75.0 },
            data: NodeData::conditional_simple("field", ConditionOperator::Equal, json!(1)),
            ports: Some(ports),
            has_breakpoint: Some(false),
        };

        assert_eq!(node.uuid, "node-uuid-999");
        assert!(node.ports.is_some());
        let ports_ref = node.ports.as_ref().unwrap();
        assert_eq!(ports_ref.len(), 2);
        assert_eq!(ports_ref[0].id, "port-1");
        assert_eq!(ports_ref[1].id, "port-2");
    }

    #[test]
    fn test_node_type_enum() {
        assert_eq!(
            serde_json::to_string(&NodeType::Entry).unwrap(),
            r#""entry""#
        );
        assert_eq!(serde_json::to_string(&NodeType::Http).unwrap(), r#""http""#);
        assert_eq!(
            serde_json::to_string(&NodeType::Conditional).unwrap(),
            r#""conditional""#
        );
    }
}
