// gRPC 协议集成测试
//
// 测试 gRPC crate 的跨模块集成：
// 1. Config 序列化/反序列化 round-trip
// 2. GrpcPluginGroup 注册验证
// 3. Plugin 元数据完整性
// 4. Connect → Call → Close 流程接线（使用 mock）

use flow_engine_core::node_config::{
    GrpcCallConfig, GrpcCloseConfig, GrpcConnectConfig, GrpcMetadataEntry, NodeData, ProtoFile,
};
use flow_engine_core::plugin::NodePlugin;
use flow_engine_core::registry::NodeRegistry;
use flow_engine_grpc::plugin::{GrpcCallPlugin, GrpcClosePlugin, GrpcConnectPlugin, GrpcPluginGroup};
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// 1. Config 序列化/反序列化 round-trip
// ============================================================================

#[test]
fn test_grpc_connect_config_serde_roundtrip() {
    let config = GrpcConnectConfig {
        url: "http://localhost:50051".to_string(),
        use_tls: true,
        tls_skip_verify: false,
        enable_reflection: true,
        proto_files: vec![ProtoFile {
            path: "test.proto".to_string(),
            content: "syntax = \"proto3\";".to_string(),
        }],
        default_metadata: vec![GrpcMetadataEntry {
            key: "authorization".to_string(),
            value: "Bearer token123".to_string(),
        }],
        connect_timeout_ms: 60000,
    };

    // 序列化 → JSON → 反序列化
    let json = serde_json::to_string(&config).expect("序列化应成功");
    let deserialized: GrpcConnectConfig =
        serde_json::from_str(&json).expect("反序列化应成功");

    assert_eq!(deserialized, config);
}

#[test]
fn test_grpc_connect_config_serde_minimal() {
    // 只有 url 是必须字段，其他都有 default
    let json = r#"{"url":"http://example.com:50051"}"#;
    let config: GrpcConnectConfig = serde_json::from_str(json).expect("最小 JSON 应反序列化成功");

    assert_eq!(config.url, "http://example.com:50051");
    assert_eq!(config.use_tls, false);
    assert_eq!(config.tls_skip_verify, false);
    assert_eq!(config.enable_reflection, false);
    assert!(config.proto_files.is_empty());
    assert!(config.default_metadata.is_empty());
    assert_eq!(config.connect_timeout_ms, 30000);
}

#[test]
fn test_grpc_connect_config_via_node_data_roundtrip() {
    let config = GrpcConnectConfig {
        url: "https://grpc.example.com".to_string(),
        use_tls: true,
        tls_skip_verify: true,
        enable_reflection: false,
        proto_files: vec![],
        default_metadata: vec![],
        connect_timeout_ms: 10000,
    };
    let node_data = NodeData::GrpcConnect(config.clone());

    let json = serde_json::to_string(&node_data).expect("序列化 NodeData 应成功");
    let deserialized: NodeData =
        serde_json::from_str(&json).expect("反序列化 NodeData 应成功");

    assert_eq!(deserialized, node_data);
    assert_eq!(deserialized.type_name(), "grpc_connect");
}

#[test]
fn test_grpc_call_config_serde_roundtrip() {
    let config = GrpcCallConfig {
        connection_id: "conn-001".to_string(),
        service_name: "test.TestService".to_string(),
        method_name: "GetUser".to_string(),
        request_json: r#"{"user_id":"123"}"#.to_string(),
        timeout_ms: 15000,
        metadata: vec![
            GrpcMetadataEntry {
                key: "x-request-id".to_string(),
                value: "abc-456".to_string(),
            },
        ],
    };

    let json = serde_json::to_string(&config).expect("序列化应成功");
    let deserialized: GrpcCallConfig =
        serde_json::from_str(&json).expect("反序列化应成功");

    assert_eq!(deserialized, config);
}

#[test]
fn test_grpc_call_config_via_node_data_roundtrip() {
    let config = GrpcCallConfig {
        connection_id: "conn-002".to_string(),
        service_name: "api.OrderService".to_string(),
        method_name: "CreateOrder".to_string(),
        request_json: r#"{"product_id":"p1","qty":3}"#.to_string(),
        timeout_ms: 5000,
        metadata: vec![],
    };
    let node_data = NodeData::GrpcCall(config.clone());

    let json = serde_json::to_string(&node_data).expect("序列化应成功");
    let deserialized: NodeData =
        serde_json::from_str(&json).expect("反序列化应成功");

    assert_eq!(deserialized, node_data);
    assert_eq!(deserialized.type_name(), "grpc_call");
}

#[test]
fn test_grpc_close_config_serde_roundtrip() {
    let config = GrpcCloseConfig {
        connection_id: "conn-001".to_string(),
    };

    let json = serde_json::to_string(&config).expect("序列化应成功");
    let deserialized: GrpcCloseConfig =
        serde_json::from_str(&json).expect("反序列化应成功");

    assert_eq!(deserialized, config);
}

#[test]
fn test_grpc_close_config_via_node_data_roundtrip() {
    let config = GrpcCloseConfig {
        connection_id: "conn-close-001".to_string(),
    };
    let node_data = NodeData::GrpcClose(config.clone());

    let json = serde_json::to_string(&node_data).expect("序列化应成功");
    let deserialized: NodeData =
        serde_json::from_str(&json).expect("反序列化应成功");

    assert_eq!(deserialized, node_data);
    assert_eq!(deserialized.type_name(), "grpc_close");
}

#[test]
fn test_grpc_metadata_entry_serde() {
    let entry = GrpcMetadataEntry {
        key: "authorization".to_string(),
        value: "Bearer my-token".to_string(),
    };

    let json = serde_json::to_string(&entry).expect("序列化应成功");
    let deserialized: GrpcMetadataEntry =
        serde_json::from_str(&json).expect("反序列化应成功");

    assert_eq!(deserialized, entry);
}

#[test]
fn test_proto_file_serde() {
    let proto = ProtoFile {
        path: "api/v1/user.proto".to_string(),
        content: "syntax = \"proto3\";".to_string(),
    };

    let json = serde_json::to_string(&proto).expect("序列化应成功");
    let deserialized: ProtoFile =
        serde_json::from_str(&json).expect("反序列化应成功");

    assert_eq!(deserialized, proto);
}

#[test]
fn test_connect_config_skip_empty_vectors() {
    // 验证 skip_serializing_if = "Vec::is_empty" 行为
    let config = GrpcConnectConfig {
        url: "http://localhost:50051".to_string(),
        use_tls: false,
        tls_skip_verify: false,
        enable_reflection: false,
        proto_files: vec![],
        default_metadata: vec![],
        connect_timeout_ms: 30000,
    };

    let json = serde_json::to_string(&config).expect("序列化应成功");
    let json_obj: serde_json::Value =
        serde_json::from_str(&json).expect("JSON 解析应成功");

    // 空 vec 应被跳过
    assert!(json_obj.get("proto_files").is_none());
    assert!(json_obj.get("default_metadata").is_none());

    // 但有值时应保留
    let config_with_files = GrpcConnectConfig {
        proto_files: vec![ProtoFile {
            path: "a.proto".to_string(),
            content: "".to_string(),
        }],
        ..config
    };
    let json2 = serde_json::to_string(&config_with_files).expect("序列化应成功");
    let json_obj2: serde_json::Value =
        serde_json::from_str(&json2).expect("JSON 解析应成功");
    assert!(json_obj2.get("proto_files").is_some());
}

// ============================================================================
// 2. GrpcPluginGroup 注册验证
// ============================================================================

#[test]
fn test_plugin_group_registers_all_three_types() {
    let mut registry = NodeRegistry::new();
    assert!(registry.is_empty());

    GrpcPluginGroup::register_all(&mut registry);

    assert_eq!(registry.len(), 3, "应注册 3 个 gRPC 插件");

    // 验证每个 type_id 都存在
    assert!(
        registry.get("grpc_connect").is_some(),
        "grpc_connect 应被注册"
    );
    assert!(
        registry.get("grpc_call").is_some(),
        "grpc_call 应被注册"
    );
    assert!(
        registry.get("grpc_close").is_some(),
        "grpc_close 应被注册"
    );
}

#[test]
fn test_plugin_group_no_extra_registrations() {
    let mut registry = NodeRegistry::new();
    GrpcPluginGroup::register_all(&mut registry);

    // 确保只有 3 个插件，没有多余的
    assert_eq!(registry.len(), 3);

    // 验证不存在的 type_id
    assert!(registry.get("grpc").is_none());
    assert!(registry.get("grpc_connection").is_none());
    assert!(registry.get("http").is_none());
}

#[test]
fn test_registry_all_metadata_returns_grpc_plugins() {
    let mut registry = NodeRegistry::new();
    GrpcPluginGroup::register_all(&mut registry);

    let metadata = registry.all_metadata();
    assert_eq!(metadata.len(), 3);

    // 验证所有 category 都是 "grpc"
    for meta in &metadata {
        assert_eq!(meta.category, "grpc", "所有 gRPC 插件 category 应为 'grpc'");
    }

    // 验证 type_id 集合
    let type_ids: std::collections::HashSet<&str> =
        metadata.iter().map(|m| m.type_id.as_str()).collect();
    assert!(type_ids.contains("grpc_connect"));
    assert!(type_ids.contains("grpc_call"));
    assert!(type_ids.contains("grpc_close"));
}

// ============================================================================
// 3. Plugin 元数据正确性
// ============================================================================

#[test]
fn test_grpc_connect_plugin_full_metadata() {
    let plugin = GrpcConnectPlugin;

    assert_eq!(plugin.type_id(), "grpc_connect");
    assert_eq!(plugin.display_name(), "gRPC 连接");
    assert_eq!(plugin.category(), "grpc");
    assert_eq!(plugin.icon(), "radio");
    assert_eq!(plugin.color(), "#8B5CF6");
    assert!(plugin.is_protocol_node());
    assert!(plugin.supports_false_port());

    let ports = plugin.ports();
    assert_eq!(ports.len(), 2);
    assert_eq!(ports[0].id, "true");
    assert_eq!(ports[0].label, "连接成功");
    assert_eq!(ports[0].position, "right");
    assert_eq!(ports[1].id, "false");
    assert_eq!(ports[1].label, "连接失败");
    assert_eq!(ports[1].position, "bottom");

    let config = plugin.default_config();
    assert!(config.is_object());
    assert_eq!(config["url"], "");
    assert_eq!(config["use_tls"], false);
    assert_eq!(config["tls_skip_verify"], false);
    assert_eq!(config["enable_reflection"], false);
    assert!(config["proto_files"].is_array());
    assert!(config["default_metadata"].is_array());
    assert_eq!(config["connect_timeout_ms"], 30000);
}

#[test]
fn test_grpc_call_plugin_full_metadata() {
    let plugin = GrpcCallPlugin;

    assert_eq!(plugin.type_id(), "grpc_call");
    assert_eq!(plugin.display_name(), "gRPC 调用");
    assert_eq!(plugin.category(), "grpc");
    assert_eq!(plugin.icon(), "send");
    assert_eq!(plugin.color(), "#8B5CF6");
    assert!(plugin.is_protocol_node());
    assert!(plugin.supports_false_port());

    let ports = plugin.ports();
    assert_eq!(ports.len(), 2);
    assert_eq!(ports[0].id, "true");
    assert_eq!(ports[0].label, "调用成功");
    assert_eq!(ports[0].position, "right");
    assert_eq!(ports[1].id, "false");
    assert_eq!(ports[1].label, "调用失败");
    assert_eq!(ports[1].position, "bottom");

    let config = plugin.default_config();
    assert!(config.is_object());
    assert_eq!(config["connection_id"], "");
    assert_eq!(config["service_name"], "");
    assert_eq!(config["method_name"], "");
    assert_eq!(config["request_json"], "{}");
    assert_eq!(config["timeout_ms"], 30000);
    assert!(config["metadata"].is_array());
}

#[test]
fn test_grpc_close_plugin_full_metadata() {
    let plugin = GrpcClosePlugin;

    assert_eq!(plugin.type_id(), "grpc_close");
    assert_eq!(plugin.display_name(), "gRPC 关闭");
    assert_eq!(plugin.category(), "grpc");
    assert_eq!(plugin.icon(), "x-circle");
    assert_eq!(plugin.color(), "#8B5CF6");
    assert!(plugin.supports_false_port());

    let ports = plugin.ports();
    assert_eq!(ports.len(), 1);
    assert_eq!(ports[0].id, "out");
    assert_eq!(ports[0].label, "输出");
    assert_eq!(ports[0].position, "right");

    let config = plugin.default_config();
    assert!(config.is_object());
    assert_eq!(config["connection_id"], "");
}

#[test]
fn test_connect_plugin_resolve_port() {
    let plugin = GrpcConnectPlugin;

    // 成功
    assert_eq!(
        plugin.resolve_port(&Some(serde_json::json!({"success": true}))),
        "true"
    );
    // 失败
    assert_eq!(
        plugin.resolve_port(&Some(serde_json::json!({"success": false}))),
        "false"
    );
    // None
    assert_eq!(plugin.resolve_port(&None), "false");
    // 空对象
    assert_eq!(
        plugin.resolve_port(&Some(serde_json::json!({}))),
        "false"
    );
}

#[test]
fn test_call_plugin_resolve_port() {
    let plugin = GrpcCallPlugin;

    assert_eq!(
        plugin.resolve_port(&Some(serde_json::json!({"success": true}))),
        "true"
    );
    assert_eq!(
        plugin.resolve_port(&Some(serde_json::json!({"success": false}))),
        "false"
    );
    assert_eq!(plugin.resolve_port(&None), "false");
}

#[test]
fn test_close_plugin_resolve_port() {
    let plugin = GrpcClosePlugin;

    assert_eq!(
        plugin.resolve_port(&Some(serde_json::json!({"success": true}))),
        "true"
    );
    assert_eq!(
        plugin.resolve_port(&Some(serde_json::json!({"success": false}))),
        "false"
    );
    assert_eq!(plugin.resolve_port(&None), "false");
}

#[test]
fn test_all_grpc_plugins_share_same_color() {
    // 所有 gRPC 插件应使用统一的颜色
    let connect = GrpcConnectPlugin;
    let call = GrpcCallPlugin;
    let close = GrpcClosePlugin;

    assert_eq!(connect.color(), "#8B5CF6");
    assert_eq!(call.color(), "#8B5CF6");
    assert_eq!(close.color(), "#8B5CF6");
}

#[test]
fn test_build_report_input_connect() {
    let plugin = GrpcConnectPlugin;
    let config = serde_json::json!({
        "url": "http://localhost:50051",
        "use_tls": true,
        "tls_skip_verify": false,
        "enable_reflection": true,
        "proto_files": [],
        "default_metadata": [],
        "connect_timeout_ms": 60000
    });

    let report = plugin.build_report_input(&config);
    assert!(report.is_some());
    let report = report.unwrap();
    assert_eq!(report["url"], "http://localhost:50051");
    assert_eq!(report["use_tls"], true);
    assert_eq!(report["enable_reflection"], true);
    assert_eq!(report["connect_timeout_ms"], 60000);
}

#[test]
fn test_build_report_input_call() {
    let plugin = GrpcCallPlugin;
    let config = serde_json::json!({
        "connection_id": "conn-1",
        "service_name": "test.Service",
        "method_name": "GetUser",
        "request_json": "{}",
        "timeout_ms": 10000,
        "metadata": []
    });

    let report = plugin.build_report_input(&config);
    assert!(report.is_some());
    let report = report.unwrap();
    assert_eq!(report["connection_id"], "conn-1");
    assert_eq!(report["service_name"], "test.Service");
    assert_eq!(report["method_name"], "GetUser");
}

#[test]
fn test_build_report_input_close() {
    let plugin = GrpcClosePlugin;
    let config = serde_json::json!({
        "connection_id": "conn-1"
    });

    let report = plugin.build_report_input(&config);
    assert!(report.is_some());
    let report = report.unwrap();
    assert_eq!(report["connection_id"], "conn-1");
}

// ============================================================================
// 4. Connect → Call → Close 流程接线测试（使用 GrpcConnectionManager）
// ============================================================================

/// 辅助：创建测试用 ExecutionContext
fn create_test_context(flow_id: &str) -> flow_engine_core::context::ExecutionContext {
    use flow_engine_core::context::VariableStorage;
    use std::sync::Arc;

    flow_engine_core::context::ExecutionContext {
        flow_uuid: flow_id.to_string(),
        current_node_uuid: None,
        variables: VariableStorage::new(),
        node_reports: vec![],
        flow_trigger_time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("系统时间错误")
            .as_millis() as u64,
        log_sender: None,
        flow_file_path: None,
        ws_manager: None,
        ws_connection_urls: HashMap::new(),
        ws_streams: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        db_manager: None,
        last_node_output: None,
        debug_event_sender: None,
        execution_event_sender: None,
        cancel_receiver: None,
        grpc_manager: None,
        providers: HashMap::new(),
    }
}

#[tokio::test]
async fn test_connect_call_close_flow_wiring() {
    // 测试 Connect → Call → Close 流程的接线
    // 使用真实的 GrpcConnectionManager（不连接实际服务器），
    // 验证执行器能正确地被实例化和串联执行。

    use flow_engine_core::executor::NodeExecutor;
    use flow_engine_core::node::{Node, NodePosition};
    use flow_engine_grpc::grpc_call_executor::GrpcCallExecutor;
    use flow_engine_grpc::grpc_close_executor::GrpcCloseExecutor;
    use flow_engine_grpc::grpc_connect_executor::GrpcConnectExecutor;
    use flow_engine_grpc::grpc_connection_manager::GrpcConnectionManager;

    let grpc_manager = Arc::new(GrpcConnectionManager::new());

    // --- Step 1: Connect (没有 proto 文件，预期失败) ---
    let connect_node = Node {
        uuid: "connect-1".to_string(),
        node_type: "grpc_connect".to_string(),
        name: "gRPC 连接".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::GrpcConnect(GrpcConnectConfig {
            url: "http://localhost:50051".to_string(),
            use_tls: false,
            tls_skip_verify: false,
            enable_reflection: false,
            proto_files: vec![],
            default_metadata: vec![],
            connect_timeout_ms: 2000,
        }),
        ports: None,
        has_breakpoint: None,
    };

    let connect_executor = GrpcConnectExecutor::new(grpc_manager.clone());
    let mut context = create_test_context("test-flow");

    let (next_nodes, output) = connect_executor
        .execute(&connect_node, &mut context, &[], &HashMap::new())
        .await
        .expect("Connect 执行应返回 Ok");

    // 没有 proto 文件，应走 false 端口
    assert_eq!(next_nodes, vec!["false"]);
    let output_data = output.expect("应有输出数据");
    assert_eq!(output_data["success"], false);
    assert!(output_data["error"].is_string());

    // --- Step 2: Call（连接不存在，预期失败）---
    let call_node = Node {
        uuid: "call-1".to_string(),
        node_type: "grpc_call".to_string(),
        name: "gRPC 调用".to_string(),
        position: NodePosition { x: 200.0, y: 0.0 },
        data: NodeData::GrpcCall(GrpcCallConfig {
            connection_id: "nonexistent".to_string(),
            service_name: "test.TestService".to_string(),
            method_name: "GetUser".to_string(),
            request_json: r#"{"user_id":"1"}"#.to_string(),
            timeout_ms: 5000,
            metadata: vec![],
        }),
        ports: None,
        has_breakpoint: None,
    };

    let call_executor = GrpcCallExecutor::new(grpc_manager.clone());

    let (next_nodes, output) = call_executor
        .execute(&call_node, &mut context, &[], &HashMap::new())
        .await
        .expect("Call 执行应返回 Ok");

    assert_eq!(next_nodes, vec!["false"]);
    let output_data = output.expect("应有输出数据");
    assert_eq!(output_data["success"], false);
    assert!(output_data["error"].is_string());

    // --- Step 3: Close（连接不存在，预期失败）---
    let close_node = Node {
        uuid: "close-1".to_string(),
        node_type: "grpc_close".to_string(),
        name: "gRPC 关闭".to_string(),
        position: NodePosition { x: 400.0, y: 0.0 },
        data: NodeData::GrpcClose(GrpcCloseConfig {
            connection_id: "nonexistent".to_string(),
        }),
        ports: None,
        has_breakpoint: None,
    };

    let close_executor = GrpcCloseExecutor::new(grpc_manager.clone());

    let (next_nodes, output) = close_executor
        .execute(&close_node, &mut context, &[], &HashMap::new())
        .await
        .expect("Close 执行应返回 Ok");

    assert_eq!(next_nodes, Vec::<String>::new());
    let output_data = output.expect("应有输出数据");
    assert_eq!(output_data["success"], false);
    assert!(output_data["error"].is_string());
}

#[tokio::test]
async fn test_connect_with_proto_and_call_flow() {
    // 测试带 proto 文件的 Connect 流程（连接不可达，但 proto 解析应通过）
    use flow_engine_core::executor::NodeExecutor;
    use flow_engine_core::node::{Node, NodePosition};
    use flow_engine_grpc::grpc_connect_executor::GrpcConnectExecutor;
    use flow_engine_grpc::grpc_connection_manager::GrpcConnectionManager;

    let grpc_manager = Arc::new(GrpcConnectionManager::new());

    let proto_content = r#"
syntax = "proto3";
package integration_test;
service TestService {
  rpc Ping(PingRequest) returns (PongResponse);
}
message PingRequest { string message = 1; }
message PongResponse { string message = 1; }
"#;

    let connect_node = Node {
        uuid: "connect-proto-1".to_string(),
        node_type: "grpc_connect".to_string(),
        name: "带 Proto 的连接".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::GrpcConnect(GrpcConnectConfig {
            url: "http://192.0.2.1:50051".to_string(), // 不可路由地址
            use_tls: false,
            tls_skip_verify: false,
            enable_reflection: false,
            proto_files: vec![ProtoFile {
                path: "test.proto".to_string(),
                content: proto_content.to_string(),
            }],
            default_metadata: vec![],
            connect_timeout_ms: 2000,
        }),
        ports: None,
        has_breakpoint: None,
    };

    let connect_executor = GrpcConnectExecutor::new(grpc_manager);
    let mut context = create_test_context("test-flow-proto");

    let result = connect_executor
        .execute(&connect_node, &mut context, &[], &HashMap::new())
        .await;

    // tonic 的 endpoint.connect() 是惰性的，proto 解析通过后可能返回成功
    // 无论成功还是失败，关键是执行器不崩溃且返回有效结构
    assert!(result.is_ok());
    let (_next_nodes, output_data) = result.unwrap();
    assert!(output_data.is_some());
    let data = output_data.unwrap();
    assert!(data.get("success").is_some());
    assert!(data.get("url").is_some());
}

#[tokio::test]
async fn test_plugin_execute_delegates_to_executor() {
    // 验证 Plugin.execute() 能正确委托给底层执行器
    use flow_engine_core::node::{Node, NodePosition};
    use flow_engine_core::plugin::NodePlugin;

    let connect_plugin = GrpcConnectPlugin;

    let node = Node {
        uuid: "plugin-test-1".to_string(),
        node_type: "grpc_connect".to_string(),
        name: "Plugin 测试".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::GrpcConnect(GrpcConnectConfig {
            url: "http://localhost:99999".to_string(),
            use_tls: false,
            tls_skip_verify: false,
            enable_reflection: false,
            proto_files: vec![],
            default_metadata: vec![],
            connect_timeout_ms: 1000,
        }),
        ports: None,
        has_breakpoint: None,
    };

    let mut context = create_test_context("plugin-flow");

    let result = connect_plugin
        .execute(&node, &mut context, &[], &HashMap::new())
        .await;

    assert!(result.is_ok());
    let (next_nodes, output) = result.unwrap();
    assert_eq!(next_nodes, vec!["false"]);
    assert!(output.is_some());
}

#[tokio::test]
async fn test_call_and_close_plugins_delegate() {
    // 验证 Call 和 Close Plugin 的 execute 委托
    use flow_engine_core::node::{Node, NodePosition};
    use flow_engine_core::plugin::NodePlugin;

    // Call Plugin
    let call_plugin = GrpcCallPlugin;
    let call_node = Node {
        uuid: "call-plugin-1".to_string(),
        node_type: "grpc_call".to_string(),
        name: "Call Plugin 测试".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::GrpcCall(GrpcCallConfig {
            connection_id: "missing".to_string(),
            service_name: "test.Svc".to_string(),
            method_name: "Do".to_string(),
            request_json: "{}".to_string(),
            timeout_ms: 1000,
            metadata: vec![],
        }),
        ports: None,
        has_breakpoint: None,
    };
    let mut ctx = create_test_context("test");
    let result = call_plugin
        .execute(&call_node, &mut ctx, &[], &HashMap::new())
        .await;
    assert!(result.is_ok());
    let (next, out) = result.unwrap();
    assert_eq!(next, vec!["false"]);
    assert!(out.is_some());

    // Close Plugin
    let close_plugin = GrpcClosePlugin;
    let close_node = Node {
        uuid: "close-plugin-1".to_string(),
        node_type: "grpc_close".to_string(),
        name: "Close Plugin 测试".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::GrpcClose(GrpcCloseConfig {
            connection_id: "missing".to_string(),
        }),
        ports: None,
        has_breakpoint: None,
    };
    let mut ctx2 = create_test_context("test");
    let result = close_plugin
        .execute(&close_node, &mut ctx2, &[], &HashMap::new())
        .await;
    assert!(result.is_ok());
    let (_, out) = result.unwrap();
    assert!(out.is_some());
}
