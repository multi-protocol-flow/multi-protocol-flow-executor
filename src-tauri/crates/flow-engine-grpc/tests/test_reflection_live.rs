//! 实时测试 gRPC Server Reflection（需要 echo Docker 服务运行）
//!
//! 依赖 `echo/` 下的 Docker Compose:
//!   cd echo && docker compose up -d
//!
//! 运行: cargo test -p flow-engine-grpc --test test_reflection_live -- --nocapture --ignored

use flow_engine_grpc::grpc_connection_manager;
use tonic_reflection::pb::v1alpha::server_reflection_response;

const GRPC_PLAINTEXT: &str = "http://localhost:50051";

#[tokio::test]
#[ignore = "需要 echo Docker 容器 (cd echo && docker compose up -d)"]
async fn test_reflection_echo_plaintext() {
    println!("=== 测试 gRPC Server Reflection (echo 明文) ===");

    let result = grpc_connection_manager::discover_services_via_reflection(
        GRPC_PLAINTEXT,
        false,  // use_tls
        false,  // tls_skip_verify
        15000,  // connect_timeout_ms
        None,   // tls_ca_cert
        None,   // tls_client_cert
        None,   // tls_client_key
        None,   // tls_server_name_override
        vec![], // reflection_metadata
    )
    .await;

    match &result {
        Ok(services) => {
            println!("✅ 发现 {} 个服务:", services.len());
            for svc in services {
                println!("  服务: {}", svc.service_name);
                for method in &svc.methods {
                    println!(
                        "    方法: {} ({}) -> {} [client_streaming={}, server_streaming={}]",
                        method.method_name,
                        method.input_type,
                        method.output_type,
                        method.is_client_streaming,
                        method.is_server_streaming,
                    );
                }
            }
        }
        Err(e) => {
            println!("❌ Failed: {}", e);
        }
    }

    assert!(result.is_ok(), "Reflection 应该Success");

    let services = result.unwrap();
    assert!(!services.is_empty(), "应该发现至少一个服务");

    // 验证 echo 服务的 TestService
    let test_service = services.iter().find(|s| s.service_name == "testserver.TestService");
    assert!(test_service.is_some(), "应该发现 testserver.TestService");

    let svc = test_service.unwrap();
    assert_eq!(svc.methods.len(), 6, "TestService 应有 6 个方法");

    // 验证各方法的流式属性
    let say_hello = svc.methods.iter().find(|m| m.method_name == "SayHello").unwrap();
    assert!(!say_hello.is_client_streaming);
    assert!(!say_hello.is_server_streaming);

    let server_stream = svc.methods.iter().find(|m| m.method_name == "ServerStream").unwrap();
    assert!(!server_stream.is_client_streaming);
    assert!(server_stream.is_server_streaming);

    let client_stream = svc.methods.iter().find(|m| m.method_name == "ClientStream").unwrap();
    assert!(client_stream.is_client_streaming);
    assert!(!client_stream.is_server_streaming);

    let bidi = svc.methods.iter().find(|m| m.method_name == "BidiStream").unwrap();
    assert!(bidi.is_client_streaming);
    assert!(bidi.is_server_streaming);
}

/// 直接使用 tonic 底层 API 测试反射协议
#[tokio::test]
#[ignore = "需要 echo Docker 容器"]
async fn test_raw_tonic_reflection_echo() {
    use tonic_reflection::pb::v1alpha::{
        server_reflection_client::ServerReflectionClient, server_reflection_request,
        ServerReflectionRequest,
    };

    println!("=== 直接测试 tonic 连接 + 反射 (echo 明文) ===");

    let endpoint = tonic::transport::Endpoint::from_shared(GRPC_PLAINTEXT.to_string())
        .expect("创建端点Failed")
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30));

    println!("[1] 连接中...");
    let channel = tokio::time::timeout(std::time::Duration::from_secs(15), endpoint.connect())
        .await
        .expect("TCP 连接超时")
        .expect("TCP connection failed");

    println!("[2] TCP 连接Success! 创建反射客户端...");

    let mut client = ServerReflectionClient::new(channel);
    let (tx, rx) = tokio::sync::mpsc::channel(16);
    let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);

    println!("[3] 发起双向流 server_reflection_info...");
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.server_reflection_info(request_stream),
    )
    .await;

    match response {
        Ok(Ok(response)) => {
            println!("[4] 双向流建立Success!");
            let mut stream = response.into_inner();

            // 发送 ListServices
            println!("[5] 发送 ListServices...");
            tx.send(ServerReflectionRequest {
                host: String::new(),
                message_request: Some(
                    server_reflection_request::MessageRequest::ListServices(String::new()),
                ),
            })
            .await
            .expect("发送Failed");

            println!("[6] 等待响应...");
            let list_resp =
                tokio::time::timeout(std::time::Duration::from_secs(15), stream.message()).await;

            match list_resp {
                Ok(Ok(Some(resp))) => {
                    println!("[7] 收到响应: {:?}", resp);
                    // 验证包含 TestService
                    if let Some(server_reflection_response::MessageResponse::ListServicesResponse(ref services)) = resp.message_response {
                        assert!(
                            services.service.iter().any(|s| s.name == "testserver.TestService"),
                            "应包含 testserver.TestService"
                        );
                    }
                }
                Ok(Ok(None)) => println!("[7] 服务器关闭了流"),
                Ok(Err(e)) => println!("[7] 读取响应Error: {} (code={:?})", e, e.code()),
                Err(_) => println!("[7] 等待响应超时"),
            }
        }
        Ok(Err(e)) => println!("[5] Failed: {} (code={:?})", e, e.code()),
        Err(_) => println!("[5] 超时 (30s)"),
    }
}

/// 测试 connect_lazy + 反射
#[tokio::test]
#[ignore = "需要 echo Docker 容器"]
async fn test_lazy_connect_reflection() {
    use tonic_reflection::pb::v1alpha::{
        server_reflection_client::ServerReflectionClient, server_reflection_request,
        ServerReflectionRequest,
    };

    println!("=== 测试 connect_lazy + 反射 (echo 明文) ===");

    let channel = tonic::transport::Endpoint::from_shared(GRPC_PLAINTEXT.to_string())
        .expect("创建端点Failed")
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .connect_lazy();

    println!("[1] 懒连接 channel 已创建");

    let mut client = ServerReflectionClient::new(channel);
    let (tx, rx) = tokio::sync::mpsc::channel(16);
    let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);

    // 先发送 ListServices，再发起流
    tx.send(ServerReflectionRequest {
        host: String::new(),
        message_request: Some(server_reflection_request::MessageRequest::ListServices(
            String::new(),
        )),
    })
    .await
    .expect("发送Failed");

    println!("[2] 已发送 ListServices 请求到通道");

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.server_reflection_info(request_stream),
    )
    .await;

    match response {
        Ok(Ok(response)) => {
            println!("[3] 双向流建立Success!");
            let mut stream = response.into_inner();

            let list_resp =
                tokio::time::timeout(std::time::Duration::from_secs(15), stream.message()).await;

            match list_resp {
                Ok(Ok(Some(resp))) => {
                    println!("[4] 收到响应: {:?}", resp.message_response);
                    let found = match resp.message_response {
                        Some(server_reflection_response::MessageResponse::ListServicesResponse(ref r)) =>
                            r.service.iter().any(|s| s.name == "testserver.TestService"),
                        _ => false,
                    };
                    assert!(found, "应通过反射发现 testserver.TestService");
                }
                Ok(Ok(None)) => println!("[4] 流关闭 (None)"),
                Ok(Err(e)) => println!("[4] Error: {} (code={:?})", e, e.code()),
                Err(_) => println!("[4] 超时"),
            }
        }
        Ok(Err(e)) => println!("[3] Failed: {} (code={:?})", e, e.code()),
        Err(_) => println!("[3] 超时 (30s)"),
    }
}
