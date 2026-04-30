//! gRPC TestService 测试服务器
//!
//! 基于 tonic 0.12 实现 TestService 的全部 6 个 RPC 方法，
//! 覆盖 Unary / Server Streaming / Client Streaming / Bidi Streaming 四种通信模式。

use std::fs::File;
use std::io::{BufReader, Read};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use anyhow::{Context as AnyhowContext, Result};
use futures_util::Stream;
use tonic::transport::{Identity, ServerTlsConfig};
use tonic::{Request, Response, Status, Streaming};

// 引入 build.rs 生成的 protobuf 代码
pub mod proto {
    tonic::include_proto!("testserver");
}

use proto::test_service_server::{TestService, TestServiceServer};

// ============================================================================
// ReceiverStream — 无需额外依赖的 Stream 包装器
// ============================================================================

/// 将 tokio mpsc::Receiver 包装为 futures_core::Stream
struct ReceiverStream<T> {
    inner: tokio::sync::mpsc::Receiver<T>,
}

impl<T> Unpin for ReceiverStream<T> {}

impl<T> Stream for ReceiverStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        self.inner.poll_recv(cx)
    }
}

// ============================================================================
// TestService 实现
// ============================================================================

#[derive(Debug, Default)]
struct TestServiceImpl;

#[tonic::async_trait]
impl TestService for TestServiceImpl {
    // --- Unary RPCs ---

    /// 简单问候：返回 "Hello, {name}!"
    async fn say_hello(
        &self,
        request: Request<proto::HelloRequest>,
    ) -> Result<Response<proto::HelloResponse>, Status> {
        let req = request.into_inner();
        tracing::debug!(name = %req.name, "SayHello 请求");
        Ok(Response::new(proto::HelloResponse {
            message: format!("Hello, {}!", req.name),
        }))
    }

    /// 原样回显消息
    async fn echo(
        &self,
        request: Request<proto::EchoRequest>,
    ) -> Result<Response<proto::EchoResponse>, Status> {
        let req = request.into_inner();
        tracing::debug!(message = %req.message, "Echo 请求");
        Ok(Response::new(proto::EchoResponse {
            message: req.message,
        }))
    }

    /// 四则运算：支持 add / subtract / multiply / divide
    async fn calculate(
        &self,
        request: Request<proto::CalcRequest>,
    ) -> Result<Response<proto::CalcResponse>, Status> {
        let req = request.into_inner();
        tracing::debug!(
            a = req.a,
            b = req.b,
            operation = %req.operation,
            "Calculate 请求"
        );

        let (result, error) = match req.operation.as_str() {
            "add" => (req.a + req.b, String::new()),
            "subtract" => (req.a - req.b, String::new()),
            "multiply" => (req.a * req.b, String::new()),
            "divide" => {
                if req.b == 0.0 {
                    (0.0, "division by zero".to_string())
                } else {
                    (req.a / req.b, String::new())
                }
            }
            _ => {
                return Err(Status::invalid_argument(format!(
                    "未知运算类型: {}",
                    req.operation
                )));
            }
        };

        Ok(Response::new(proto::CalcResponse {
            result,
            operation: req.operation,
            error,
        }))
    }

    // --- Server Streaming ---

    /// 服务端流式响应：返回 count 次 StreamResponse
    ///
    /// 默认返回 5 条消息，可通过 message 字段指定 "count:N" 来设置数量。
    /// 每条消息间隔 100ms，sequence 递增，附带当前时间戳。
    type ServerStreamStream = Pin<Box<dyn Stream<Item = Result<proto::StreamResponse, Status>> + Send>>;

    async fn server_stream(
        &self,
        request: Request<proto::StreamRequest>,
    ) -> Result<Response<Self::ServerStreamStream>, Status> {
        let req = request.into_inner();
        // 解析 count，默认 5
        let count: i32 = req
            .message
            .strip_prefix("count:")
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        tracing::debug!(
            message = %req.message,
            sequence = req.sequence,
            count,
            "ServerStream 请求"
        );

        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let start_seq = req.sequence;

        tokio::spawn(async move {
            for i in 0..count {
                let timestamp_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间早于 UNIX 纪元")
                    .as_millis() as i64;

                let resp = proto::StreamResponse {
                    message: format!("stream #{}", i),
                    sequence: start_seq + i,
                    timestamp_ms,
                };

                if tx.send(Ok(resp)).await.is_err() {
                    // 客户端已断开
                    break;
                }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream { inner: rx })))
    }

    // --- Client Streaming ---

    /// 客户端流式请求：汇总所有收到的消息
    ///
    /// 收集全部 StreamRequest，返回消息数量和逗号拼接的消息内容。
    async fn client_stream(
        &self,
        request: Request<Streaming<proto::StreamRequest>>,
    ) -> Result<Response<proto::StreamSummary>, Status> {
        let mut stream = request.into_inner();
        let mut messages: Vec<String> = Vec::new();

        while let Some(msg) = stream.message().await? {
            tracing::debug!(
                message = %msg.message,
                sequence = msg.sequence,
                "ClientStream 收到消息"
            );
            messages.push(msg.message);
        }

        let count = messages.len() as i32;
        let concatenated = messages.join(",");

        tracing::debug!(count, "ClientStream 汇总完成");

        Ok(Response::new(proto::StreamSummary {
            message_count: count,
            concatenated,
        }))
    }

    // --- Bidi Streaming ---

    /// 双向流：每收到一条消息就立即回一条，消息前缀 "echo: "
    type BidiStreamStream = Pin<Box<dyn Stream<Item = Result<proto::StreamResponse, Status>> + Send>>;

    async fn bidi_stream(
        &self,
        request: Request<Streaming<proto::StreamRequest>>,
    ) -> Result<Response<Self::BidiStreamStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(128);

        tracing::debug!("BidiStream 连接建立");

        tokio::spawn(async move {
            while let Ok(Some(msg)) = stream.message().await {
                tracing::debug!(
                    message = %msg.message,
                    sequence = msg.sequence,
                    "BidiStream 收到消息"
                );

                let timestamp_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("系统时间早于 UNIX 纪元")
                    .as_millis() as i64;

                let resp = proto::StreamResponse {
                    message: format!("echo: {}", msg.message),
                    sequence: msg.sequence,
                    timestamp_ms,
                };

                if tx.send(Ok(resp)).await.is_err() {
                    // 客户端已断开
                    break;
                }
            }
            tracing::debug!("BidiStream 连接结束");
        });

        Ok(Response::new(Box::pin(ReceiverStream { inner: rx })))
    }
}

// ============================================================================
// 公共入口
// ============================================================================

/// 启动 gRPC TestService 服务器（明文）
///
/// 端口可通过 `GRPC_PORT` 环境变量配置，默认 50051。
pub async fn run(port: u16) -> Result<()> {
    let addr = format!("0.0.0.0:{}", port).parse()?;
    tracing::info!("gRPC TestService 服务启动 → 0.0.0.0:{}", port);

    tonic::transport::Server::builder()
        .add_service(TestServiceServer::new(TestServiceImpl::default()))
        .serve(addr)
        .await?;

    Ok(())
}

/// 加载 PEM 证书和私钥
fn load_pem_cert(cert_path: &str) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    File::open(cert_path)
        .with_context(|| format!("打开证书文件失败: {}", cert_path))?
        .read_to_end(&mut buf)?;
    Ok(buf)
}

fn load_pem_key(key_path: &str) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    File::open(key_path)
        .with_context(|| format!("打开私钥文件失败: {}", key_path))?
        .read_to_end(&mut buf)?;
    Ok(buf)
}

/// 启动 TLS gRPC TestService 服务器
///
/// 端口可通过 `GRPC_TLS_PORT` 环境变量配置，默认 50052。
/// 证书路径可通过 `TLS_CERT_PATH` / `TLS_KEY_PATH` 配置。
pub async fn run_tls(port: u16) -> Result<()> {
    let cert_path = std::env::var("TLS_CERT_PATH")
        .unwrap_or_else(|_| "/app/certs/server.crt".to_string());
    let key_path = std::env::var("TLS_KEY_PATH")
        .unwrap_or_else(|_| "/app/certs/server.key".to_string());

    let cert_pem = match load_pem_cert(&cert_path) {
        Ok(c) => c,
        Err(e) => {
            // 尝试备用路径
            let fallback_cert = "./certs/server.crt";
            let fallback_key = "./certs/server.key";
            tracing::warn!(
                "主证书路径加载失败 ({}: {}), 尝试备用: {}, {}",
                cert_path, e, fallback_cert, fallback_key
            );
            load_pem_cert(fallback_cert)?
        }
    };
    let key_pem = load_pem_key(&key_path).or_else(|_| load_pem_key("./certs/server.key"))?;

    let identity = Identity::from_pem(cert_pem, key_pem);
    let tls_config = ServerTlsConfig::new().identity(identity);

    let addr = format!("0.0.0.0:{}", port).parse()?;
    tracing::info!("gRPC TLS TestService 服务启动 → 0.0.0.0:{}", port);

    tonic::transport::Server::builder()
        .tls_config(tls_config)
        .with_context(|| "gRPC TLS 配置失败")?
        .add_service(TestServiceServer::new(TestServiceImpl::default()))
        .serve(addr)
        .await?;

    Ok(())
}

/// 启动 mTLS gRPC TestService 服务器（需要客户端证书）
///
/// 端口可通过 `GRPC_MTLS_PORT` 环境变量配置，默认 50053。
/// CA 证书路径通过 `TLS_CA_CERT_PATH` 配置。
pub async fn run_mtls(port: u16) -> Result<()> {
    let cert_path = std::env::var("TLS_CERT_PATH")
        .unwrap_or_else(|_| "/app/certs/server.crt".to_string());
    let key_path = std::env::var("TLS_KEY_PATH")
        .unwrap_or_else(|_| "/app/certs/server.key".to_string());
    let ca_cert_path = std::env::var("TLS_CA_CERT_PATH")
        .unwrap_or_else(|_| "/app/certs/ca.pem".to_string());

    let cert_pem = load_pem_cert(&cert_path)
        .or_else(|_| load_pem_cert("./certs/server.crt"))?;
    let key_pem = load_pem_key(&key_path)
        .or_else(|_| load_pem_key("./certs/server.key"))?;

    // 加载 CA 证书用于客户端验证
    let mut ca_buf = Vec::new();
    File::open(&ca_cert_path)
        .with_context(|| format!("打开 CA 证书文件失败: {}", ca_cert_path))?
        .read_to_end(&mut ca_buf)?;

    let identity = Identity::from_pem(cert_pem, key_pem);
    let tls_config = ServerTlsConfig::new()
        .identity(identity)
        .client_ca_root(tonic::transport::Certificate::from_pem(ca_buf));

    let addr = format!("0.0.0.0:{}", port).parse()?;
    tracing::info!("gRPC mTLS TestService 服务启动 → 0.0.0.0:{}", port);

    tonic::transport::Server::builder()
        .tls_config(tls_config)
        .with_context(|| "gRPC mTLS 配置失败")?
        .add_service(TestServiceServer::new(TestServiceImpl::default()))
        .serve(addr)
        .await?;

    Ok(())
}
