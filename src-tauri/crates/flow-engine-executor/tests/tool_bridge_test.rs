//! Tests for Tool Bridge loopback HTTP service.
use flow_engine_core::context::ExecutionContext;
use flow_engine_executor::executor::FlowExecutor;
use flow_engine_executor::provider_builder::ProviderBuilderExt;
use flow_engine_executor::tool_bridge::{ToolBridgeServer, BRIDGE_TOKEN_HEADER};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_tool_bridge_unauthorized() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let executor = Arc::new(FlowExecutor::new());
    let handle = ToolBridgeServer::start(executor)
        .await
        .expect("Failed to start ToolBridgeServer");

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/tool/execute", handle.url))
        .json(&json!({
            "execution_id": "test_exec_1",
            "node_type": "script",
            "config": {}
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(resp.status().as_u16(), 401);
    handle.shutdown().await;
}

#[tokio::test]
async fn test_tool_bridge_not_found_route() {
    let executor = Arc::new(FlowExecutor::new());
    let _ = rustls::crypto::ring::default_provider().install_default();
    let handle = ToolBridgeServer::start(executor)
        .await
        .expect("Failed to start ToolBridgeServer");

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/unknown_route", handle.url))
        .header(BRIDGE_TOKEN_HEADER, &handle.token)
        .json(&json!({}))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(resp.status().as_u16(), 404);
    handle.shutdown().await;
}

#[tokio::test]
async fn test_tool_bridge_execute_script_success() {
    let executor = Arc::new(FlowExecutor::new());
    let _ = rustls::crypto::ring::default_provider().install_default();
    let handle = ToolBridgeServer::start(executor)
        .await
        .expect("Failed to start ToolBridgeServer");

    let execution_id = "exec_script_100".to_string();
    let mut context = ExecutionContext::new("flow_1".to_string(), 1000).with_default_providers();
    context
        .variables
        .set_variable("init_val".to_string(), json!(42), "".to_string());
    handle.register_context(execution_id.clone(), context).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/tool/execute", handle.url))
        .header(BRIDGE_TOKEN_HEADER, &handle.token)
        .json(&json!({
            "execution_id": execution_id,
            "node_type": "script",
            "config": {
                "script": "let answer = 42; fn.variables.set('answer', answer); ({ answer, text: 'hello from bridge' });",
                "language": "javascript",
                "timeout_ms": 5000
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.expect("Failed to parse JSON");
    eprintln!("SCRIPT BODY: {:?}", body);
    assert_eq!(body["success"], true);
    assert!(
        body["duration_ms"].as_u64().unwrap_or(0) > 0 || body["duration_ms"].as_u64().is_some()
    );
    assert!(body["output"].is_object());

    handle.shutdown().await;
}

#[tokio::test]
async fn test_tool_bridge_execute_error_recovery() {
    let executor = Arc::new(FlowExecutor::new());
    let _ = rustls::crypto::ring::default_provider().install_default();
    let handle = ToolBridgeServer::start(executor)
        .await
        .expect("Failed to start ToolBridgeServer");

    let execution_id = "exec_script_err".to_string();
    let client = reqwest::Client::new();

    // Execute a script that produces a runtime syntax/eval error
    let resp = client
        .post(format!("{}/tool/execute", handle.url))
        .header(BRIDGE_TOKEN_HEADER, &handle.token)
        .json(&json!({
            "execution_id": execution_id,
            "node_type": "script",
            "config": {
                "script": "throw new Error('Intentional tool error for agent recovery');",
                "language": "javascript",
                "timeout_ms": 5000
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    // HTTP status must be 200 so the agent receives the error payload for reasoning and self-correction
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], false);
    assert!(body["error_message"].is_string());
    assert!(body["raw_response"].as_str().unwrap().contains("error"));

    handle.shutdown().await;
}
