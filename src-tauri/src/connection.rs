// 连接模块 - 连接结构体实现

/// 连接数据结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Connection {
    /// 连接唯一标识符
    pub id: String,
    /// 源节点的 UUID（引用 node.uuid）
    pub source_node_uuid: String,
    /// 目标节点的 UUID（引用 node.uuid）
    pub target_node_uuid: String,
    /// 源端口 ID
    pub source_port_id: String,
    /// 目标端口 ID
    pub target_port_id: String,
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_creation() {
        let connection = Connection {
            id: "conn-123".to_string(),
            source_node_uuid: "node-uuid-1".to_string(),
            target_node_uuid: "node-uuid-2".to_string(),
            source_port_id: "port-1".to_string(),
            target_port_id: "port-2".to_string(),
        };

        assert_eq!(connection.id, "conn-123");
        assert_eq!(connection.source_node_uuid, "node-uuid-1");
        assert_eq!(connection.target_node_uuid, "node-uuid-2");
        assert_eq!(connection.source_port_id, "port-1");
        assert_eq!(connection.target_port_id, "port-2");
    }

    #[test]
    fn test_connection_serialization() {
        let connection = Connection {
            id: "conn-456".to_string(),
            source_node_uuid: "source-abc".to_string(),
            target_node_uuid: "target-def".to_string(),
            source_port_id: "out-1".to_string(),
            target_port_id: "in-1".to_string(),
        };

        let json = serde_json::to_string(&connection).unwrap();
        assert!(json.contains("conn-456"));
        assert!(json.contains("source-abc"));
        assert!(json.contains("target-def"));
        assert!(json.contains("out-1"));
        assert!(json.contains("in-1"));

        // 反序列化测试
        let deserialized: Connection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "conn-456");
        assert_eq!(deserialized.source_node_uuid, "source-abc");
        assert_eq!(deserialized.target_node_uuid, "target-def");
    }

    #[test]
    fn test_connection_id_field_unchanged() {
        // 确认 Connection 结构保持 id 字段，没有改为 uuid
        let connection = Connection {
            id: "connection-id-unchanged".to_string(),
            source_node_uuid: "node-1".to_string(),
            target_node_uuid: "node-2".to_string(),
            source_port_id: "port-out".to_string(),
            target_port_id: "port-in".to_string(),
        };

        assert_eq!(connection.id, "connection-id-unchanged");
    }

    #[test]
    fn test_connection_source_node_uuid_field() {
        let connection = Connection {
            id: "conn-789".to_string(),
            source_node_uuid: "source-node-uuid".to_string(),
            target_node_uuid: "target-node-uuid".to_string(),
            source_port_id: "out-port".to_string(),
            target_port_id: "in-port".to_string(),
        };

        assert_eq!(connection.source_node_uuid, "source-node-uuid");
    }

    #[test]
    fn test_connection_target_node_uuid_field() {
        let connection = Connection {
            id: "conn-999".to_string(),
            source_node_uuid: "node-a".to_string(),
            target_node_uuid: "node-b".to_string(),
            source_port_id: "port-x".to_string(),
            target_port_id: "port-y".to_string(),
        };

        assert_eq!(connection.target_node_uuid, "node-b");
    }

    #[test]
    fn test_connection_deserialization() {
        let json = r#"{
            "id": "conn-deser",
            "source_node_uuid": "node-src",
            "target_node_uuid": "node-tgt",
            "source_port_id": "out",
            "target_port_id": "in"
        }"#;

        let connection: Connection = serde_json::from_str(json).unwrap();
        assert_eq!(connection.id, "conn-deser");
        assert_eq!(connection.source_node_uuid, "node-src");
        assert_eq!(connection.target_node_uuid, "node-tgt");
        assert_eq!(connection.source_port_id, "out");
        assert_eq!(connection.target_port_id, "in");
    }

    #[test]
    fn test_multiple_connections() {
        let conn1 = Connection {
            id: "conn-1".to_string(),
            source_node_uuid: "node-a".to_string(),
            target_node_uuid: "node-b".to_string(),
            source_port_id: "port-a-out".to_string(),
            target_port_id: "port-b-in".to_string(),
        };

        let conn2 = Connection {
            id: "conn-2".to_string(),
            source_node_uuid: "node-b".to_string(),
            target_node_uuid: "node-c".to_string(),
            source_port_id: "port-b-out".to_string(),
            target_port_id: "port-c-in".to_string(),
        };

        assert_eq!(conn1.id, "conn-1");
        assert_eq!(conn1.source_node_uuid, "node-a");
        assert_eq!(conn1.target_node_uuid, "node-b");

        assert_eq!(conn2.id, "conn-2");
        assert_eq!(conn2.source_node_uuid, "node-b");
        assert_eq!(conn2.target_node_uuid, "node-c");
    }
}
