//! 实时测试 gRPC Server Reflection (需要网络连接)
//!
//! 运行: cargo test -p flow-engine-grpc --test test_reflection_live -- --nocapture --ignored

use flow_engine_grpc::grpc_connection_manager;

#[tokio::test]
#[ignore] // 需要网络，默认跳过
async fn test_reflection_grpcb_in_plaintext() {
    println!("=== 测试 grpcb.in:9000 (明文) Server Reflection ===");

    let result = grpc_connection_manager::discover_services_via_reflection(
        "grpcb.in:9000",
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
                        "    方法: {} ({}) -> {} [streaming={}]",
                        method.method_name,
                        method.input_type,
                        method.output_type,
                        method.is_server_streaming
                    );
                }
            }
        }
        Err(e) => {
            println!("❌ Failed: {}", e);
        }
    }

    assert!(result.is_ok(), "Reflection 应该Success");
    assert!(!result.unwrap().is_empty(), "应该发现至少一个服务");
}

/// 直接使用 tonic 底层 API 测试，诊断反射Failed的确切原因
#[tokio::test]
#[ignore]
async fn test_raw_tonic_reflection_grpcb_in() {
    use tonic_reflection::pb::v1alpha::{
        server_reflection_client::ServerReflectionClient, server_reflection_request,
        ServerReflectionRequest,
    };

    println!("=== 直接测试 tonic 连接 + 反射 ===");

    // 步骤 1: 建立 TCP + HTTP/2 连接
    println!("[1] 创建 endpoint...");
    let endpoint = tonic::transport::Endpoint::from_shared("http://grpcb.in:9000".to_string())
        .expect("创建端点Failed")
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .http2_keep_alive_interval(std::time::Duration::from_secs(5))
        .keep_alive_timeout(std::time::Duration::from_secs(10));

    println!("[2] 连接中...");
    let channel = tokio::time::timeout(std::time::Duration::from_secs(15), endpoint.connect())
        .await
        .expect("TCP 连接超时")
        .expect("TCP connection failed");

    println!("[3] TCP 连接Success! 创建反射客户端...");

    // 步骤 2: 创建反射客户端
    let mut client = ServerReflectionClient::new(channel);
    let (tx, rx) = tokio::sync::mpsc::channel(16);
    let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);

    println!("[4] 发起双向流 server_reflection_info...");
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.server_reflection_info(request_stream),
    )
    .await;

    match response {
        Ok(Ok(response)) => {
            println!("[5] 双向流建立Success!");
            let mut stream = response.into_inner();

            // 步骤 3: 发送 ListServices
            println!("[6] 发送 ListServices...");
            tx.send(ServerReflectionRequest {
                host: String::new(),
                message_request: Some(server_reflection_request::MessageRequest::ListServices(
                    String::new(),
                )),
            })
            .await
            .expect("发送Failed");

            println!("[7] 等待响应...");
            let list_resp =
                tokio::time::timeout(std::time::Duration::from_secs(15), stream.message()).await;

            match list_resp {
                Ok(Ok(Some(resp))) => {
                    println!("[8] 收到响应: {:?}", resp);
                }
                Ok(Ok(None)) => {
                    println!("[8] 服务器关闭了流 (返回 None)");
                }
                Ok(Err(e)) => {
                    println!(
                        "[8] 读取响应Error: {} (code={:?}, message={})",
                        e,
                        e.code(),
                        e.message()
                    );
                }
                Err(_) => {
                    println!("[8] 等待响应超时 (15s)");
                }
            }
        }
        Ok(Err(e)) => {
            println!(
                "[5] 双向流建立Failed: {} (code={:?}, message={})",
                e,
                e.code(),
                e.message()
            );
            println!("    details: {:?}", e.details());
        }
        Err(_) => {
            println!("[5] 双向流建立超时 (30s)");
        }
    }
}

/// 测试 connect_lazy + 不同的 endpoint 配置
#[tokio::test]
#[ignore]
async fn test_lazy_connect_reflection() {
    use tonic_reflection::pb::v1alpha::{
        server_reflection_client::ServerReflectionClient, server_reflection_request,
        ServerReflectionRequest,
    };

    println!("=== 测试 connect_lazy + 反射 ===");

    let channel = tonic::transport::Endpoint::from_shared("http://grpcb.in:9000".to_string())
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
                }
                Ok(Ok(None)) => println!("[4] 流关闭 (None)"),
                Ok(Err(e)) => println!("[4] Error: {} (code={:?})", e, e.code()),
                Err(_) => println!("[4] 超时"),
            }
        }
        Ok(Err(e)) => {
            println!("[3] Failed: {} (code={:?})", e, e.code());
        }
        Err(_) => {
            println!("[3] 超时 (30s)");
        }
    }
}
