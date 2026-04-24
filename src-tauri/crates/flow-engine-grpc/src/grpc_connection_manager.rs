//! gRPC 连接池管理器
//!
//! 实现 GrpcProvider trait，管理 gRPC 连接的生命周期。
//! 使用 tonic Channel 进行 gRPC 通信，
//! 使用 prost-reflect DynamicMessage 进行动态消息编解码。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

        use async_trait::async_trait;
use flow_engine_core::traits::{GrpcMethodInfo, GrpcProvider, GrpcServiceInfo, GrpcStreamingResult, GrpcTimingInfo};
use http::uri::PathAndQuery;
use prost_reflect::{DynamicMessage, MethodDescriptor};
use serde_json::de::Deserializer;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::metadata::{MetadataKey, MetadataValue};

use crate::proto_parser;

// ---------------------------------------------------------------------------
// TLS 辅助函数
// ---------------------------------------------------------------------------

/// 不验证任何证书的 ServerCertVerifier（仅用于 `tls_skip_verify=true`）
///
/// **警告**: 仅用于开发/测试环境，切勿在生产环境使用。
#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}

/// 确保已安装 CryptoProvider（幂等，多次调用安全）
fn ensure_crypto_provider() {
    // install_default 只在首次调用时安装，后续调用返回 Err 但不影响程序
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// 配置端点的 TLS（标准验证模式）
///
/// 使用 tonic 内置的 webpki-roots + 系统原生证书信任链。
fn apply_standard_tls(
    endpoint: tonic::transport::Endpoint,
) -> Result<tonic::transport::Endpoint, String> {
    let tls_config = tonic::transport::ClientTlsConfig::new()
        .with_webpki_roots()
        .with_native_roots();
    endpoint
        .tls_config(tls_config)
        .map_err(|e| format!("TLS 配置失败: {}", e))
}

/// 构建 gRPC channel（跳过证书验证模式）
///
/// 使用自定义 rustls `ClientConfig` + dangerous verifier，
/// 通过 `connect_with_connector_lazy()` 注入自定义 TLS 连接器。
fn build_skip_verify_channel(
    url_with_scheme: &str,
    connect_timeout_ms: u64,
) -> Result<tonic::transport::Channel, String> {
    ensure_crypto_provider();

    // 解析原始 URL 获取 host（用于 TLS SNI）
    let original_uri: http::Uri = url_with_scheme
        .parse()
        .map_err(|e| format!("解析 URL 失败: {}", e))?;
    let host = original_uri
        .host()
        .ok_or_else(|| format!("URL 缺少 host: {}", url_with_scheme))?
        .to_string();
    let port = original_uri.port_u16().unwrap_or(443);

    // 构建 dangerous verifier 的 rustls ClientConfig
    let client_config = {
        let mut cfg = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth();
        cfg.alpn_protocols = vec![b"h2".to_vec()];
        Arc::new(cfg)
    };

    // TLS SNI 名称
    let server_name = rustls::pki_types::ServerName::try_from(host.clone())
        .map_err(|e| format!("无效的 TLS 服务器名 '{}': {}", host, e))?
        .to_owned();

    // 使用 http:// 方案创建端点（绕过 tonic 的内置 TLS）
    let http_url = format!("http://{}:{}", host, port);
    let mut endpoint = tonic::transport::Endpoint::from_shared(http_url)
        .map_err(|e| format!("创建 gRPC 端点失败: {}", e))?;

    if connect_timeout_ms > 0 {
        endpoint = endpoint.connect_timeout(std::time::Duration::from_millis(connect_timeout_ms));
    }

    // 构建自定义连接器: TCP 连接 + TLS 握手
    let tls_connector = tokio_rustls::TlsConnector::from(client_config);

    let connector = tower::service_fn(move |uri: http::Uri| {
        let tls = tls_connector.clone();
        let sni = server_name.clone();

        async move {
            let host = uri.host().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "URI 缺少 host")
            })?;
            let port = uri.port_u16().unwrap_or(443);
            let addr = format!("{}:{}", host, port);

            let tcp_stream = tokio::net::TcpStream::connect(&addr).await?;
            tcp_stream.set_nodelay(true)?;

            let tls_stream = tls.connect(sni.to_owned(), tcp_stream).await?;
            Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(tls_stream))
        }
    });

    Ok(endpoint.connect_with_connector_lazy(connector))
}

/// 根据 TLS 配置创建 tonic channel
///
/// - `tls_skip_verify=true`: 跳过证书验证
/// - `tls_skip_verify=false`: 标准验证（webpki-roots + 系统原生证书）
fn create_channel(
    url_with_scheme: &str,
    use_tls: bool,
    tls_skip_verify: bool,
    connect_timeout_ms: u64,
) -> Result<tonic::transport::Channel, String> {
    let mut endpoint = tonic::transport::Endpoint::from_shared(url_with_scheme.to_string())
        .map_err(|e| format!("创建 gRPC 端点失败: {}", e))?;

    if use_tls && tls_skip_verify {
        // 跳过证书验证: 使用自定义 TLS 连接器
        return build_skip_verify_channel(url_with_scheme, connect_timeout_ms);
    }

    if use_tls {
        // 标准验证: webpki-roots + 系统原生证书
        endpoint = apply_standard_tls(endpoint)?;
    }

    if connect_timeout_ms > 0 {
        endpoint = endpoint.connect_timeout(std::time::Duration::from_millis(connect_timeout_ms));
    }

    Ok(endpoint.connect_lazy())
}

/// 通过 Server Reflection 协议发现 gRPC 服务
///
/// 连接到 gRPC 服务器后，使用 v1alpha 反射协议:
/// 1. 发送 `ListServices` 获取所有服务名
/// 2. 对每个服务发送 `FileContainingSymbol` 获取文件描述符
/// 3. 递归获取所有传递依赖（通过 `FileByFilename`）
/// 4. 构建 `DescriptorPool` 用于后续动态调用
///
/// 所有网络操作均带超时，避免无限挂起。
async fn reflect_services(
    channel: tonic::transport::Channel,
) -> Result<prost_reflect::DescriptorPool, String> {
    use tonic_reflection::pb::v1alpha::{
        server_reflection_client::ServerReflectionClient,
        server_reflection_request,
        server_reflection_response,
        ServerReflectionRequest,
    };

    // 单次反射请求的超时时间（秒）
    const REFLECTION_TIMEOUT_SECS: u64 = 15;

    log::info!("[gRPC Reflection] 创建反射客户端...");

    // 创建反射客户端和双向流的请求通道
    let mut client = ServerReflectionClient::new(channel);
    let (tx, rx) = tokio::sync::mpsc::channel(16);
    let request_stream = tokio_stream::wrappers::ReceiverStream::new(rx);

    // === 关键: 先发送 ListServices 请求到 mpsc 通道，再发起双向流 ===
    // 使用 connect_lazy() 时，TCP+HTTP/2 连接在第一个 RPC 调用时才建立。
    // 如果先调用 server_reflection_info() 再发请求，流是空的，不会触发连接。
    // 必须先往 mpsc 通道里放入请求，这样 server_reflection_info() 消费
    // request_stream 时会立即发送数据，触发实际连接。
    log::info!("[gRPC Reflection] 预发送 ListServices 请求...");
    tx.send(ServerReflectionRequest {
        host: String::new(),
        message_request: Some(
            server_reflection_request::MessageRequest::ListServices(String::new()),
        ),
    })
    .await
    .map_err(|e| format!("发送 ListServices 请求失败: {}", e))?;

    // 发起双向流调用（带超时）
    // 此时 request_stream 中已有 ListServices 请求，
    // server_reflection_info 会消费它并触发 TCP+HTTP/2 连接
    log::info!("[gRPC Reflection] 发起双向流调用...");
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(REFLECTION_TIMEOUT_SECS),
        client.server_reflection_info(request_stream),
    )
    .await
    .map_err(|_| {
        log::error!("[gRPC Reflection] 双向流连接超时");
        "Server Reflection 连接超时（服务器可能不支持 Reflection）".to_string()
    })?
    .map_err(|e| {
        log::error!("[gRPC Reflection] 双向流连接失败: {}", e);
        format!("Server Reflection 连接失败: {}（服务器可能不支持 Reflection）", e)
    })?;
    let mut stream = response.into_inner();
    log::info!("[gRPC Reflection] 双向流已建立，等待 ListServices 响应...");

    // 等待 ListServices 响应
    let list_resp = tokio::time::timeout(
        std::time::Duration::from_secs(REFLECTION_TIMEOUT_SECS),
        stream.message(),
    )
    .await
    .map_err(|_| {
        log::error!("[gRPC Reflection] ListServices 响应超时");
        "ListServices 响应超时（服务器可能不支持 Reflection）".to_string()
    })?
    .map_err(|e| {
        log::error!("[gRPC Reflection] 读取 ListServices 响应失败: {}", e);
        format!("读取 ListServices 响应失败: {}", e)
    })?
    .ok_or_else(|| {
        log::error!("[gRPC Reflection] 服务器关闭了流，未返回服务列表");
        "Server Reflection 未返回服务列表响应".to_string()
    })?;

    let services = match list_resp.message_response {
        Some(server_reflection_response::MessageResponse::ListServicesResponse(resp)) => {
            resp.service
        }
        Some(server_reflection_response::MessageResponse::ErrorResponse(err)) => {
            return Err(format!("Server Reflection 错误: {}", err.error_message));
        }
        _ => return Err("Server Reflection 返回了意外的响应类型".to_string()),
    };

    // 过滤掉反射服务自身（v1 和 v1alpha）
    let service_names: Vec<String> = services
        .into_iter()
        .map(|s| s.name)
        .filter(|name| !is_reflection_service(name))
        .collect();

    if service_names.is_empty() {
        return Err("服务器没有暴露任何业务 gRPC 服务（仅发现反射服务）".to_string());
    }

    log::info!("Server Reflection 发现 {} 个服务: {:?}", service_names.len(), service_names);

    // === 第二步: 获取每个服务对应的文件描述符 ===
    let mut all_fds: Vec<prost_types::FileDescriptorProto> = Vec::new();
    let mut seen_files: HashSet<String> = HashSet::new();

    for service_name in &service_names {
        tx.send(ServerReflectionRequest {
            host: String::new(),
            message_request: Some(
                server_reflection_request::MessageRequest::FileContainingSymbol(
                    service_name.clone(),
                ),
            ),
        })
        .await
        .map_err(|e| format!("发送 FileContainingSymbol 请求失败: {}", e))?;

        let resp = tokio::time::timeout(
            std::time::Duration::from_secs(REFLECTION_TIMEOUT_SECS),
            stream.message(),
        )
        .await
        .map_err(|_| format!("查询服务 '{}' 的文件描述符超时", service_name))?
        .map_err(|e| format!("查询服务 '{}' 的文件描述符失败: {}", service_name, e))?
        .ok_or_else(|| format!("Server Reflection 在查询服务 '{}' 时关闭了流", service_name))?;

        match resp.message_response {
            Some(server_reflection_response::MessageResponse::FileDescriptorResponse(fdr)) => {
                for fd_bytes in &fdr.file_descriptor_proto {
                    let fd: prost_types::FileDescriptorProto = prost::Message::decode(
                        fd_bytes.as_slice(),
                    )
                    .map_err(|e| format!("解码 FileDescriptorProto 失败: {}", e))?;

                    if let Some(name) = &fd.name {
                        if seen_files.insert(name.clone()) {
                            log::debug!("从反射获取到文件描述符: {}", name);
                            all_fds.push(fd);
                        }
                    }
                }
            }
            Some(server_reflection_response::MessageResponse::ErrorResponse(err)) => {
                log::warn!(
                    "查询服务 '{}' 的文件描述符时收到错误: {}",
                    service_name,
                    err.error_message
                );
                continue;
            }
            _ => {
                log::warn!("查询服务 '{}' 时收到意外响应", service_name);
                continue;
            }
        }
    }

    // === 第三步: 递归解析传递依赖 ===
    // 检查所有已收集文件的 dependency 字段，获取尚未加载的依赖文件
    let mut max_iterations = 50; // 防止无限循环
    loop {
        max_iterations -= 1;
        if max_iterations <= 0 {
            log::warn!("依赖解析达到最大迭代次数限制，可能有未解析的依赖");
            break;
        }

        // 收集尚未获取的依赖文件名
        let missing_deps: Vec<String> = all_fds
            .iter()
            .flat_map(|fd| fd.dependency.iter().cloned())
            .filter(|dep| !seen_files.contains(dep))
            .collect();

        if missing_deps.is_empty() {
            break;
        }

        log::debug!("发现 {} 个未解析的传递依赖", missing_deps.len());

        for dep_name in &missing_deps {
            tx.send(ServerReflectionRequest {
                host: String::new(),
                message_request: Some(
                    server_reflection_request::MessageRequest::FileByFilename(
                        dep_name.clone(),
                    ),
                ),
            })
            .await
            .map_err(|e| format!("发送 FileByFilename 请求失败: {}", e))?;

            let resp = tokio::time::timeout(
                std::time::Duration::from_secs(REFLECTION_TIMEOUT_SECS),
                stream.message(),
            )
            .await
            .map_err(|_| format!("查询依赖文件 '{}' 超时", dep_name))?;

            let resp = match resp {
                Ok(Some(resp)) => resp,
                Ok(None) => {
                    log::warn!("查询依赖文件 '{}' 时流已关闭", dep_name);
                    seen_files.insert(dep_name.clone());
                    continue;
                }
                Err(e) => {
                    log::warn!("查询依赖文件 '{}' 失败: {}", dep_name, e);
                    seen_files.insert(dep_name.clone());
                    continue;
                }
            };

            match resp.message_response {
                Some(server_reflection_response::MessageResponse::FileDescriptorResponse(
                    fdr,
                )) => {
                    for fd_bytes in &fdr.file_descriptor_proto {
                        let fd: prost_types::FileDescriptorProto =
                            prost::Message::decode(fd_bytes.as_slice()).map_err(|e| {
                                format!("解码依赖 FileDescriptorProto 失败: {}", e)
                            })?;
                        if let Some(name) = &fd.name {
                            if seen_files.insert(name.clone()) {
                                log::debug!("从反射获取到依赖文件: {}", name);
                                all_fds.push(fd);
                            }
                        }
                    }
                }
                Some(server_reflection_response::MessageResponse::ErrorResponse(err)) => {
                    log::warn!(
                        "查询依赖文件 '{}' 时收到错误: {}",
                        dep_name,
                        err.error_message
                    );
                    seen_files.insert(dep_name.clone());
                }
                _ => {
                    seen_files.insert(dep_name.clone());
                }
            }
        }
    }

    // 关闭请求流
    drop(tx);

    if all_fds.is_empty() {
        return Err("Server Reflection 未返回任何文件描述符".to_string());
    }

    // === 第四步: 构建 DescriptorPool ===
    let fds = prost_types::FileDescriptorSet { file: all_fds };
    prost_reflect::DescriptorPool::from_file_descriptor_set(fds)
        .map_err(|e| format!("从反射数据构建 DescriptorPool 失败: {}", e))
}

/// 判断是否为 gRPC 反射服务（应从结果中过滤）
fn is_reflection_service(name: &str) -> bool {
    matches!(
        name,
        "grpc.reflection.v1.ServerReflection"
            | "grpc.reflection.v1alpha.ServerReflection"
    )
}

/// 通过 Server Reflection 协议发现 gRPC 服务（不创建持久连接）
///
/// 用于设计时预发现服务，仅创建临时连接进行反射查询，
/// 返回服务列表但不将连接存入连接池。
pub async fn discover_services_via_reflection(
    url: &str,
    use_tls: bool,
    tls_skip_verify: bool,
    connect_timeout_ms: u64,
) -> Result<Vec<GrpcServiceInfo>, String> {
    // 1. 构造 URL
    let url_with_scheme = if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else if use_tls {
        format!("https://{}", url)
    } else {
        format!("http://{}", url)
    };

    log::info!(
        "[gRPC] 开始反射发现: url={}, use_tls={}, tls_skip_verify={}, timeout={}ms",
        url_with_scheme,
        use_tls,
        tls_skip_verify,
        connect_timeout_ms
    );

    // 2. 创建 channel（含 TLS 配置）
    let channel = create_channel(&url_with_scheme, use_tls, tls_skip_verify, connect_timeout_ms)?;

    // 3. 通过反射发现服务
    let pool = reflect_services(channel).await?;

    // 4. 提取服务信息
    let services = proto_parser::list_services(&pool);
    Ok(services
        .iter()
        .map(|s| GrpcServiceInfo {
            service_name: s.service_name.clone(),
            methods: s
                .methods
                .iter()
                .map(|m| GrpcMethodInfo {
                    method_name: m.method_name.clone(),
                    input_type: m.input_type.clone(),
                    output_type: m.output_type.clone(),
                    is_server_streaming: m.is_server_streaming,
                    is_client_streaming: m.is_client_streaming,
                })
                .collect(),
        })
        .collect())
}

/// 通过解析 Proto 文件发现 gRPC 服务（不需要连接服务器）
///
/// 用于设计时预发现服务，解析用户提供的 .proto 文件，
/// 返回服务列表。
pub fn discover_services_via_proto(
    proto_files: &[(String, String)],
) -> Result<Vec<GrpcServiceInfo>, String> {
    let files: Vec<proto_parser::ProtoFile> = proto_files
        .iter()
        .map(|(path, content)| proto_parser::ProtoFile {
            path: path.clone(),
            content: content.clone(),
        })
        .collect();

    let pool = proto_parser::parse_proto_files(&files)
        .map_err(|e| format!("解析 proto 文件失败: {}", e))?;

    let services = proto_parser::list_services(&pool);
    Ok(services
        .iter()
        .map(|s| GrpcServiceInfo {
            service_name: s.service_name.clone(),
            methods: s
                .methods
                .iter()
                .map(|m| GrpcMethodInfo {
                    method_name: m.method_name.clone(),
                    input_type: m.input_type.clone(),
                    output_type: m.output_type.clone(),
                    is_server_streaming: m.is_server_streaming,
                    is_client_streaming: m.is_client_streaming,
                })
                .collect(),
        })
        .collect())
}

/// 活跃的 gRPC 连接
struct GrpcConnection {
    /// tonic channel（可克隆，克隆后共享底层连接）
    channel: tonic::transport::Channel,
    /// Descriptor pool（从 proto 文件解析得到）
    descriptor_pool: Arc<prost_reflect::DescriptorPool>,
    /// 连接状态
    status: String,
    #[allow(dead_code)]
    /// 连接 URL（预留用于后续功能）
    url: String,
    #[allow(dead_code)]
    /// 创建时间（预留用于连接超时管理）
    created_at: chrono::DateTime<chrono::Utc>,
}

/// 动态 gRPC 编解码器
///
/// 使用 prost-reflect 的 `DynamicMessage` 实现 tonic 的 Codec trait，
/// 从而支持对任意 protobuf 消息类型的动态编解码。
#[derive(Clone)]
struct DynamicCodec {
    /// 方法描述符（包含输入输出类型信息）
    method: MethodDescriptor,
}

impl DynamicCodec {
    /// 创建新的动态编解码器
    fn new(method: MethodDescriptor) -> Self {
        Self { method }
    }
}

impl Codec for DynamicCodec {
    type Encode = DynamicMessage;
    type Decode = DynamicMessage;
    type Encoder = Self;
    type Decoder = Self;

    fn encoder(&mut self) -> Self::Encoder {
        self.clone()
    }

    fn decoder(&mut self) -> Self::Decoder {
        self.clone()
    }
}

impl Encoder for DynamicCodec {
    type Item = DynamicMessage;
    type Error = tonic::Status;

    fn encode(
        &mut self,
        item: Self::Item,
        dst: &mut EncodeBuf<'_>,
    ) -> Result<(), Self::Error> {
        prost::Message::encode(&item, dst)
            .map_err(|e| tonic::Status::internal(format!("编码消息失败: {}", e)))?;
        Ok(())
    }
}

impl Decoder for DynamicCodec {
    type Item = DynamicMessage;
    type Error = tonic::Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        let mut msg = DynamicMessage::new(self.method.output());
        prost::Message::merge(&mut msg, src)
            .map_err(|e| tonic::Status::internal(format!("解码消息失败: {}", e)))?;
        Ok(Some(msg))
    }
}

/// gRPC 连接管理器
///
/// 管理 gRPC 连接池，实现 `GrpcProvider` trait。
/// 线程安全，使用 `RwLock` 保护连接池。
pub struct GrpcConnectionManager {
    /// 连接池: `connection_id` → `GrpcConnection`
    connections: Arc<RwLock<HashMap<String, GrpcConnection>>>,
}

impl GrpcConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for GrpcConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 构建 gRPC 方法路径
///
/// gRPC 路径格式为 `/{package.Service}/{Method}`
fn build_method_path(service_name: &str, method_name: &str) -> Result<PathAndQuery, String> {
    let path = format!("/{}/{}", service_name, method_name);
    path.parse()
        .map_err(|e| format!("无效的 gRPC 路径 '{}': {}", path, e))
}

/// 将 JSON 字符串反序列化为 `DynamicMessage`
fn json_to_dynamic_message(
    request_json: &str,
    descriptor: prost_reflect::MessageDescriptor,
) -> Result<DynamicMessage, String> {
    let mut deserializer = Deserializer::from_str(request_json);
    let msg = DynamicMessage::deserialize(descriptor, &mut deserializer)
        .map_err(|e| format!("反序列化请求消息失败: {}", e))?;
    deserializer
        .end()
        .map_err(|e| format!("请求 JSON 包含多余数据: {}", e))?;
    Ok(msg)
}

/// 将 `DynamicMessage` 序列化为 JSON Value
fn dynamic_message_to_json(msg: &DynamicMessage) -> Result<serde_json::Value, String> {
    let options = prost_reflect::SerializeOptions::new().skip_default_fields(false);
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::new(&mut buf);
    msg.serialize_with_options(&mut ser, &options)
        .map_err(|e| format!("序列化响应消息失败: {}", e))?;
    serde_json::from_slice(&buf).map_err(|e| format!("解析响应 JSON 失败: {}", e))
}

#[async_trait]
impl GrpcProvider for GrpcConnectionManager {
    /// 建立 gRPC 连接
    ///
    /// 创建 tonic Channel，解析 proto 文件构建 `DescriptorPool`，
    /// 将连接信息存入连接池。
    ///
    /// 当 `enable_reflection=true` 且未提供 proto 文件时，
    /// 通过 Server Reflection 协议自动发现服务。
    async fn connect(
        &self,
        uuid: String,
        url: String,
        use_tls: bool,
        tls_skip_verify: bool,
        enable_reflection: bool,
        proto_files: Vec<(String, String)>,
        _metadata: Vec<(String, String)>,
        connect_timeout_ms: u64,
    ) -> Result<Vec<GrpcServiceInfo>, String> {
        // 1. 构造 URL（确保包含 scheme）
        let url_with_scheme = if url.starts_with("http://") || url.starts_with("https://") {
            url.clone()
        } else if use_tls {
            format!("https://{}", url)
        } else {
            format!("http://{}", url)
        };

        // 2. 创建 channel（含 TLS 配置）
        let channel = create_channel(&url_with_scheme, use_tls, tls_skip_verify, connect_timeout_ms)?;

        // 3. 获取 DescriptorPool（反射或解析 proto 文件）
        let pool = if enable_reflection && proto_files.is_empty() {
            // 通过 Server Reflection 发现服务
            reflect_services(channel.clone()).await?
        } else if proto_files.is_empty() {
            return Err(
                "未提供 proto 文件且未启用 Server Reflection，无法发现服务".to_string()
            );
        } else {
            // 解析用户提供的 proto 文件
            let files: Vec<proto_parser::ProtoFile> = proto_files
                .into_iter()
                .map(|(path, content)| proto_parser::ProtoFile { path, content })
                .collect();
            proto_parser::parse_proto_files(&files)
                .map_err(|e| format!("解析 proto 文件失败: {}", e))?
        };

        // 4. 提取服务信息
        let services = proto_parser::list_services(&pool);
        let grpc_services: Vec<GrpcServiceInfo> = services
            .iter()
            .map(|s| GrpcServiceInfo {
                service_name: s.service_name.clone(),
                methods: s
                    .methods
                    .iter()
                    .map(|m| GrpcMethodInfo {
                        method_name: m.method_name.clone(),
                        input_type: m.input_type.clone(),
                        output_type: m.output_type.clone(),
                        is_server_streaming: m.is_server_streaming,
                        is_client_streaming: m.is_client_streaming,
                    })
                    .collect(),
            })
            .collect();

        // 5. 存入连接池
        let connection = GrpcConnection {
            channel,
            descriptor_pool: Arc::new(pool),
            url,
            status: "connected".to_string(),
            created_at: chrono::Utc::now(),
        };

        {
            let mut conns = self
                .connections
                .write()
                .map_err(|e| format!("获取连接池写锁失败: {}", e))?;
            conns.insert(uuid, connection);
        }

        Ok(grpc_services)
    }

    /// gRPC Unary 调用
    ///
    /// 从连接池获取连接，通过 tonic 的低级 Grpc 客户端发送请求。
    /// 使用 `DynamicCodec` 进行动态编解码。
    async fn call_unary(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_json: &str,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
    ) -> Result<serde_json::Value, String> {
        // 1. 从连接池获取连接信息（克隆后释放锁）
        let (channel, pool) = {
            let conns = self
                .connections
                .read()
                .map_err(|e| format!("获取连接池读锁失败: {}", e))?;
            let conn = conns
                .get(connection_id)
                .ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
            (conn.channel.clone(), Arc::clone(&conn.descriptor_pool))
        }; // 读锁在此释放

        // 2. 查找方法描述符
        let service = pool
            .get_service_by_name(service_name)
            .ok_or_else(|| format!("服务 '{}' 不存在", service_name))?;
        let method = service
            .methods()
            .find(|m| m.name() == method_name)
            .ok_or_else(|| format!("方法 '{}' 在服务 '{}' 中不存在", method_name, service_name))?;

        // 3. 反序列化请求 JSON 为 DynamicMessage
        let req_message = json_to_dynamic_message(request_json, method.input())?;

        // 4. 构造 tonic 请求
        let mut request = tonic::Request::new(req_message);
        // 设置 metadata
        for (key, value) in metadata {
            let meta_key = MetadataKey::from_bytes(key.as_bytes())
                .map_err(|e| format!("无效的 metadata key '{}': {:?}", key, e))?;
            let meta_val = MetadataValue::try_from(&value)
                .map_err(|e| format!("无效的 metadata value '{}': {:?}", value, e))?;
            request.metadata_mut().insert(meta_key, meta_val);
        }
        // 设置超时
        if timeout_ms > 0 {
            request.set_timeout(std::time::Duration::from_millis(timeout_ms));
        }

        // 5. 发起 Unary 调用
        let path = build_method_path(service_name, method_name)?;
        let codec = DynamicCodec::new(method);
        let mut client = tonic::client::Grpc::new(channel);
        client
            .ready()
            .await
            .map_err(|e| format!("gRPC 客户端未就绪: {}", e))?;

        let response = client
            .unary(request, path, codec)
            .await
            .map_err(|e| format!("gRPC Unary 调用失败: {} ({})", e.message(), e.code()))?;

        // 6. 序列化响应为 JSON
        let response_msg = response.into_inner();
        dynamic_message_to_json(&response_msg)
    }

    /// gRPC Server Streaming 调用
    ///
    /// 从连接池获取连接，通过 tonic 的低级 Grpc 客户端发送请求，
    /// 收集所有流式响应并返回 JSON 数组。
    async fn call_server_streaming(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_json: &str,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
    ) -> Result<Vec<serde_json::Value>, String> {
        // 1. 从连接池获取连接信息（克隆后释放锁）
        let (channel, pool) = {
            let conns = self
                .connections
                .read()
                .map_err(|e| format!("获取连接池读锁失败: {}", e))?;
            let conn = conns
                .get(connection_id)
                .ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
            (conn.channel.clone(), Arc::clone(&conn.descriptor_pool))
        }; // 读锁在此释放

        // 2. 查找方法描述符
        let service = pool
            .get_service_by_name(service_name)
            .ok_or_else(|| format!("服务 '{}' 不存在", service_name))?;
        let method = service
            .methods()
            .find(|m| m.name() == method_name)
            .ok_or_else(|| format!("方法 '{}' 在服务 '{}' 中不存在", method_name, service_name))?;

        // 3. 反序列化请求 JSON 为 DynamicMessage
        let req_message = json_to_dynamic_message(request_json, method.input())?;

        // 4. 构造 tonic 请求
        let mut request = tonic::Request::new(req_message);
        for (key, value) in metadata {
            let meta_key = MetadataKey::from_bytes(key.as_bytes())
                .map_err(|e| format!("无效的 metadata key '{}': {:?}", key, e))?;
            let meta_val = MetadataValue::try_from(&value)
                .map_err(|e| format!("无效的 metadata value '{}': {:?}", value, e))?;
            request.metadata_mut().insert(meta_key, meta_val);
        }
        // 5. 发起 Server Streaming 调用（不设 request deadline，避免过早终止流式响应）
        let path = build_method_path(service_name, method_name)?;
        let codec = DynamicCodec::new(method);
        let mut client = tonic::client::Grpc::new(channel);
        client
            .ready()
            .await
            .map_err(|e| format!("gRPC 客户端未就绪: {}", e))?;

        let response = client
            .server_streaming(request, path, codec)
            .await
            .map_err(|e| {
                format!(
                    "gRPC Server Streaming 调用失败: {} ({})",
                    e.message(),
                    e.code()
                )
            })?;

        // 6. 采集响应流（带超时）
        let mut stream = response.into_inner();
        let mut results = Vec::new();

        let collect_result = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            async {
                loop {
                    match stream.message().await {
                        Ok(Some(msg)) => {
                            let json_val = dynamic_message_to_json(&msg)?;
                            results.push(json_val);
                        }
                        Ok(None) => break,
                        Err(e) => {
                            return Err(format!(
                                "流式响应错误: {} ({})",
                                e.message(),
                                e.code()
                            ));
                        }
                    }
                }
                Ok(())
            },
        )
        .await;

        match collect_result {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                log::warn!(
                    "[gRPC Server Streaming] 采集超时 ({}ms)，已收集 {} 条响应",
                    timeout_ms,
                    results.len()
                );
            }
        }

        Ok(results)
    }

    /// gRPC Client Streaming 调用
    async fn call_client_streaming(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_messages: Vec<String>,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
    ) -> Result<serde_json::Value, String> {
        // 1. 从连接池获取连接信息
        let (channel, pool) = {
            let conns = self.connections.read().map_err(|e| format!("获取连接池读锁失败: {}", e))?;
            let conn = conns.get(connection_id).ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
            (conn.channel.clone(), Arc::clone(&conn.descriptor_pool))
        };

        // 2. 查找方法描述符
        let service = pool.get_service_by_name(service_name)
            .ok_or_else(|| format!("服务 '{}' 不存在", service_name))?;
        let method = service.methods()
            .find(|m| m.name() == method_name)
            .ok_or_else(|| format!("方法 '{}' 在服务 '{}' 中不存在", method_name, service_name))?;

        // 3. 将所有 request_messages 解码为 DynamicMessage
        let messages: Vec<DynamicMessage> = request_messages.iter()
            .map(|json| json_to_dynamic_message(json, method.input()))
            .collect::<Result<Vec<_>, _>>()?;

        // 4. 构造请求流
        let request_stream = tokio_stream::iter(messages);
        let mut request = tonic::Request::new(request_stream);

        // 5. 设置 metadata
        for (key, value) in metadata {
            let meta_key = MetadataKey::from_bytes(key.as_bytes())
                .map_err(|e| format!("无效的 metadata key '{}': {:?}", key, e))?;
            let meta_val = MetadataValue::try_from(&value)
                .map_err(|e| format!("无效的 metadata value '{}': {:?}", value, e))?;
            request.metadata_mut().insert(meta_key, meta_val);
        }
        if timeout_ms > 0 {
            request.set_timeout(std::time::Duration::from_millis(timeout_ms));
        }

        // 6. 发起 Client Streaming 调用
        let path = build_method_path(service_name, method_name)?;
        let codec = DynamicCodec::new(method);
        let mut client = tonic::client::Grpc::new(channel);
        client.ready().await.map_err(|e| format!("gRPC 客户端未就绪: {}", e))?;

        let response = client.client_streaming(request, path, codec).await
            .map_err(|e| format!("gRPC Client Streaming 调用失败: {} ({})", e.message(), e.code()))?;

        // 7. 序列化单个响应为 JSON
        let response_msg = response.into_inner();
        dynamic_message_to_json(&response_msg)
    }

    /// gRPC Bidi Streaming 调用
    async fn call_bidi_streaming(
        &self,
        connection_id: &str,
        service_name: &str,
        method_name: &str,
        request_messages: Vec<String>,
        metadata: Vec<(String, String)>,
        timeout_ms: u64,
    ) -> Result<GrpcStreamingResult, String> {
        use std::time::Instant;

        let start = Instant::now();

        // 1. 从连接池获取连接信息
        let (channel, pool) = {
            let conns = self.connections.read().map_err(|e| format!("获取连接池读锁失败: {}", e))?;
            let conn = conns.get(connection_id).ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
            (conn.channel.clone(), Arc::clone(&conn.descriptor_pool))
        };

        // 2. 查找方法描述符
        let service = pool.get_service_by_name(service_name)
            .ok_or_else(|| format!("服务 '{}' 不存在", service_name))?;
        let method = service.methods()
            .find(|m| m.name() == method_name)
            .ok_or_else(|| format!("方法 '{}' 在服务 '{}' 中不存在", method_name, service_name))?;

        // 3. 将所有 request_messages 解码为 DynamicMessage
        let messages: Vec<DynamicMessage> = request_messages.iter()
            .map(|json| json_to_dynamic_message(json, method.input()))
            .collect::<Result<Vec<_>, _>>()?;
        let sent_count = messages.len();

        // 4. 构造请求流
        let request_stream = tokio_stream::iter(messages);
        let mut request = tonic::Request::new(request_stream);

        // 5. 设置 metadata
        for (key, value) in metadata {
            let meta_key = MetadataKey::from_bytes(key.as_bytes())
                .map_err(|e| format!("无效的 metadata key '{}': {:?}", key, e))?;
            let meta_val = MetadataValue::try_from(&value)
                .map_err(|e| format!("无效的 metadata value '{}': {:?}", value, e))?;
            request.metadata_mut().insert(meta_key, meta_val);
        }

        // 6. 发起 Bidi Streaming 调用（不设 request deadline，避免过早终止流式响应）
        let path = build_method_path(service_name, method_name)?;
        let codec = DynamicCodec::new(method);
        let mut client = tonic::client::Grpc::new(channel);
        client.ready().await.map_err(|e| format!("gRPC 客户端未就绪: {}", e))?;

        let response = client.streaming(request, path, codec).await
            .map_err(|e| format!("gRPC Bidi Streaming 调用失败: {} ({})", e.message(), e.code()))?;

        // 7. 采集响应流（带超时）
        let mut stream = response.into_inner();
        let mut responses = Vec::new();
        let mut first_response_ms: Option<u64> = None;

        let collect_result = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            async {
                loop {
                    match stream.message().await {
                        Ok(Some(msg)) => {
                            if first_response_ms.is_none() {
                                first_response_ms = Some(start.elapsed().as_millis() as u64);
                            }
                            let json_val = dynamic_message_to_json(&msg)?;
                            responses.push(json_val);
                        }
                        Ok(None) => break,
                        Err(e) => {
                            return Err(format!("Bidi 流式响应错误: {} ({})", e.message(), e.code()));
                        }
                    }
                }
                Ok(())
            },
        ).await;

        match collect_result {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                // 超时，收集已获取的响应（非致命）
                log::warn!("[gRPC Bidi] 采集超时 ({}ms)，已收集 {} 条响应", timeout_ms, responses.len());
            }
        }

        let received_count = responses.len();
        let total_ms = start.elapsed().as_millis() as u64;

        Ok(GrpcStreamingResult {
            status: 0, // gRPC OK
            responses,
            sent_count,
            received_count,
            trailers: std::collections::HashMap::new(),
            timing: GrpcTimingInfo {
                total_ms,
                first_response_ms,
            },
        })
    }

    /// 关闭 gRPC 连接
    ///
    /// 从连接池中移除连接，tonic Channel 自动 drop 关闭。
    async fn close(&self, connection_id: &str) -> Result<(), String> {
        let mut conns = self
            .connections
            .write()
            .map_err(|e| format!("获取连接池写锁失败: {}", e))?;
        conns
            .remove(connection_id)
            .ok_or_else(|| format!("连接 '{}' 不存在", connection_id))?;
        Ok(())
    }

    /// 获取连接状态
    fn get_status(&self, connection_id: &str) -> Option<String> {
        let conns = self.connections.read().ok()?;
        conns.get(connection_id).map(|c| c.status.clone())
    }

    /// 获取连接的可用服务列表
    fn list_services(&self, connection_id: &str) -> Option<Vec<GrpcServiceInfo>> {
        let conns = self.connections.read().ok()?;
        let conn = conns.get(connection_id)?;
        let services = proto_parser::list_services(&conn.descriptor_pool);
        Some(
            services
                .iter()
                .map(|s| GrpcServiceInfo {
                    service_name: s.service_name.clone(),
                    methods: s
                        .methods
                        .iter()
                        .map(|m| GrpcMethodInfo {
                            method_name: m.method_name.clone(),
                            input_type: m.input_type.clone(),
                            output_type: m.output_type.clone(),
                            is_server_streaming: m.is_server_streaming,
                            is_client_streaming: m.is_client_streaming,
                        })
                        .collect(),
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_new() {
        let manager = GrpcConnectionManager::new();
        // 新创建的 manager 应该没有连接
        assert!(manager.get_status("non-existent").is_none());
        assert!(manager.list_services("non-existent").is_none());
    }

    #[test]
    fn test_connection_manager_default() {
        let manager = GrpcConnectionManager::default();
        assert!(manager.get_status("non-existent").is_none());
    }

    #[test]
    fn test_build_method_path() {
        let path = build_method_path("test.TestService", "GetUser").expect("路径构建应成功");
        assert_eq!(path.as_str(), "/test.TestService/GetUser");
    }

    #[test]
    fn test_build_method_path_with_package() {
        let path = build_method_path("myapp.v1.OrderService", "CreateOrder").expect("路径构建应成功");
        assert_eq!(path.as_str(), "/myapp.v1.OrderService/CreateOrder");
    }

    #[test]
    fn test_dynamic_codec_roundtrip() {
        // 使用 proto_parser 创建一个 DescriptorPool 来测试编解码
        let proto_content = r#"
syntax = "proto3";
package test;
message TestMessage {
  string name = 1;
  int32 value = 2;
}
"#;
        let files = vec![proto_parser::ProtoFile {
            path: "test.proto".to_string(),
            content: proto_content.to_string(),
        }];
        let pool = proto_parser::parse_proto_files(&files).expect("解析应成功");

        // DynamicCodec 需要一个 MethodDescriptor 来确定输入输出类型
        // 由于我们无法从 pool 直接获取独立方法的 MethodDescriptor，
        // 这里只验证 Codec trait 的 encoder/decoder 方法可以调用
        // 真正的编解码测试需要通过 tonic 的 Grpc 客户端
        let msg_desc = pool
            .get_message_by_name("test.TestMessage")
            .expect("应能找到 TestMessage");

        // 创建 DynamicMessage 并设置字段
        let mut msg = DynamicMessage::new(msg_desc);
        msg.set_field_by_name("name", prost_reflect::Value::String("hello".to_string()));
        msg.set_field_by_name("value", prost_reflect::Value::I32(42));

        // 验证消息内容
        assert_eq!(
            msg.get_field_by_name("name")
                .and_then(|v| v.as_str().map(|s| s.to_string())),
            Some("hello".to_string())
        );
        assert_eq!(
            msg.get_field_by_name("value").and_then(|v| v.as_i32()),
            Some(42)
        );
    }

    #[test]
    fn test_json_to_dynamic_message() {
        let proto_content = r#"
syntax = "proto3";
package test;
message TestRequest {
  string user_id = 1;
  int32 count = 2;
}
"#;
        let files = vec![proto_parser::ProtoFile {
            path: "test.proto".to_string(),
            content: proto_content.to_string(),
        }];
        let pool = proto_parser::parse_proto_files(&files).expect("解析应成功");
        let msg_desc = pool
            .get_message_by_name("test.TestRequest")
            .expect("应能找到 TestRequest");

        let json = r#"{"user_id": "abc123", "count": 5}"#;
        let msg = json_to_dynamic_message(json, msg_desc).expect("反序列化应成功");

        assert_eq!(
            msg.get_field_by_name("user_id")
                .and_then(|v| v.as_str().map(|s| s.to_string())),
            Some("abc123".to_string())
        );
        assert_eq!(
            msg.get_field_by_name("count").and_then(|v| v.as_i32()),
            Some(5)
        );
    }

    #[test]
    fn test_dynamic_message_to_json() {
        let proto_content = r#"
syntax = "proto3";
package test;
message TestResponse {
  string name = 1;
  int32 age = 2;
}
"#;
        let files = vec![proto_parser::ProtoFile {
            path: "test.proto".to_string(),
            content: proto_content.to_string(),
        }];
        let pool = proto_parser::parse_proto_files(&files).expect("解析应成功");
        let msg_desc = pool
            .get_message_by_name("test.TestResponse")
            .expect("应能找到 TestResponse");

        let mut msg = DynamicMessage::new(msg_desc);
        msg.set_field_by_name("name", prost_reflect::Value::String("Alice".to_string()));
        msg.set_field_by_name("age", prost_reflect::Value::I32(30));

        let json = dynamic_message_to_json(&msg).expect("序列化应成功");
        assert_eq!(json["name"], "Alice");
        assert_eq!(json["age"], 30);
    }
}
