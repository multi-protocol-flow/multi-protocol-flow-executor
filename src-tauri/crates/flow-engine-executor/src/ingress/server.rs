//! Ingress HTTP 监听服务与流程分发器
//! 接收外部请求，转换上下文变量，驱动流程执行并等待 `http:respond` 节点完成响应

use flow_engine_core::flow::Flow;
use flow_engine_core::node_config::NodeData;
use flow_engine_core::registry::NodeRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, Mutex};

/// HTTP 响应负载
#[derive(Debug, Clone)]
pub struct IngressResponsePayload {
    /// HTTP 响应状态码
    pub status_code: u16,
    /// 响应头
    pub headers: HashMap<String, String>,
    /// 响应体字节流
    pub body: Vec<u8>,
}
/// 注入到 `ExecutionContext` 中的响应回传通道
#[derive(Clone)]
pub struct IngressResponseChannel {
    /// 响应发送端
    pub tx: Arc<Mutex<Option<oneshot::Sender<IngressResponsePayload>>>>,
}

/// Ingress 响应回传通道在 `ExecutionContext.providers` 中的注册名
pub const INGRESS_RESPONSE_PROVIDER: &str = "ingress_response";

/// 一次 Ingress 请求的调试执行请求
pub struct IngressRequest {
    /// 触发本次请求的流程
    pub flow: Flow,
    /// 已合并流程初始变量的入参变量（`request.*` 等）
    pub variables: serde_json::Value,
    /// 流程内已声明的断点节点 UUID（由引擎统一提取，避免各实现重复解析）
    pub breakpoints: Vec<String>,
    /// 节点注册表
    pub registry: Arc<NodeRegistry>,
    /// 该请求对应的响应回传通道
    pub response_channel: Arc<IngressResponseChannel>,
}

/// Ingress 请求的调试启动器
///
/// 由应用层实现（需要持有调试事件通道与调试会话管理器）。存在启动器时，
/// 外部请求经由可调试执行器运行：断点/单步生效，HTTP 连接保持挂起，
/// 直到流程执行到 `http:respond` 后客户端才收到响应。
#[async_trait::async_trait]
pub trait IngressDebugLauncher: Send + Sync {
    /// 当前是否可接受新的调试会话
    ///
    /// 例如已有其它调试会话占用命令通道时返回 false，调用方将回退到普通执行，
    /// 避免两个会话争抢同一命令通道。
    fn is_available(&self) -> bool;

    /// 以调试模式执行一次 Ingress 请求，返回时该会话已结束
    async fn launch(&self, request: IngressRequest) -> Result<(), String>;
}

/// 全局默认共享网关端口
pub const DEFAULT_GATEWAY_PORT: u16 = 9527;

/// 流程 Ingress 单条路由定义
#[derive(Debug, Clone)]
pub struct IngressRoute {
    /// 目标流程完整定义
    pub flow: Flow,
    /// 共享网关前缀（如 Some("stripe-pay")），专属独立端口为 None
    pub route_prefix: Option<String>,
    /// 备用前缀（如同时支持流程名 Slug 与 UUID Slug）
    pub alt_prefix: Option<String>,
    /// 匹配路径模式（如 /*, /webhook 等）
    pub path_pattern: String,
    /// 允许的 HTTP 方法列表（空表示全部允许）
    pub methods: Vec<String>,
}

/// 路由匹配结果
#[derive(Debug)]
pub enum RouteMatchResult<'a> {
    /// 匹配成功，返回目标 Flow 引用及剥离网关前缀后的相对路径
    Found(&'a Flow, String),
    /// 路径匹配但 HTTP 方法不被允许
    MethodNotAllowed,
    /// 未找到匹配的路由
    NotFound,
}

/// Ingress 路由表，支持多 Flow 挂载与路径隔离
#[derive(Debug, Clone, Default)]
pub struct IngressRouter {
    /// 路由列表
    pub routes: Vec<IngressRoute>,
}

impl IngressRouter {
    /// 创建空路由表
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加一条路由
    pub fn add_route(&mut self, route: IngressRoute) {
        self.routes.push(route);
    }

    /// 匹配外部请求（支持共享网关前缀剥离与独立端口直连）
    pub fn match_request<'a>(&'a self, path: &str, method: &str) -> RouteMatchResult<'a> {
        let mut path_matched_but_method_failed = false;

        // 1. 前缀路由匹配（如 /demo-http-testing/a 或 /http-node-example-flow/a）
        for route in &self.routes {
            if let Some(prefix) = &route.route_prefix {
                let clean_prefix = prefix.trim_matches('/');
                let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
                if let Some(first) = segments.first() {
                    let matches_prefix = *first == clean_prefix
                        || route
                            .alt_prefix
                            .as_deref()
                            .map(|alt| *first == alt.trim_matches('/'))
                            .unwrap_or(false);

                    if matches_prefix {
                        let remaining = if segments.len() > 1 {
                            format!("/{}", segments[1..].join("/"))
                        } else {
                            "/".to_string()
                        };

                        if matches_path(&route.path_pattern, &remaining) {
                            if route.methods.is_empty()
                                || route.methods.iter().any(|m| m.eq_ignore_ascii_case(method))
                            {
                                return RouteMatchResult::Found(&route.flow, remaining);
                            } else {
                                path_matched_but_method_failed = true;
                            }
                        }
                    }
                }
            } else {
                // 专属独立端口：无前缀，直接以根路径匹配
                if matches_path(&route.path_pattern, path) {
                    if route.methods.is_empty()
                        || route.methods.iter().any(|m| m.eq_ignore_ascii_case(method))
                    {
                        return RouteMatchResult::Found(&route.flow, path.to_string());
                    } else {
                        path_matched_but_method_failed = true;
                    }
                }
            }
        }

        // 2. 根路径直连兜底匹配（若客户端未加前缀直接请求具体路径，如直接访问 /a）
        let allow_wildcard_fallback = self.routes.len() == 1
            || {
                let first_uuid = self.routes.first().map(|r| &r.flow.uuid);
                self.routes.iter().all(|r| Some(&r.flow.uuid) == first_uuid)
            };

        for route in &self.routes {
            let is_wildcard = route.path_pattern == "/*" || route.path_pattern == "*";
            if (!is_wildcard || allow_wildcard_fallback) && matches_path(&route.path_pattern, path) {
                if route.methods.is_empty()
                    || route.methods.iter().any(|m| m.eq_ignore_ascii_case(method))
                {
                    return RouteMatchResult::Found(&route.flow, path.to_string());
                } else {
                    path_matched_but_method_failed = true;
                }
            }
        }
        if path_matched_but_method_failed {
            RouteMatchResult::MethodNotAllowed
        } else {
            RouteMatchResult::NotFound
        }
    }

    /// 批量分析流程中的 `http:ingress` 节点，按共享网关与专属端口规划路由表
    pub fn plan_from_flows(
        flows: &[Flow],
        default_port_override: Option<u16>,
    ) -> Result<IngressMountPlan, String> {
        let default_port = default_port_override.unwrap_or(DEFAULT_GATEWAY_PORT);
        let mut port_routers: HashMap<u16, IngressRouter> = HashMap::new();
        let mut mounted_routes = Vec::new();
        let mut seen_slugs: HashMap<u16, std::collections::HashSet<String>> = HashMap::new();
        let mut custom_port_flows: HashMap<u16, (String, String)> = HashMap::new();

        for flow in flows {
            let ingress_nodes: Vec<_> = flow
                .nodes
                .iter()
                .filter(|n| {
                    n.node_type == "http:ingress"
                        || n.node_type == flow_engine_core::node_types::HTTP_INGRESS
                })
                .collect();

            if ingress_nodes.is_empty() {
                continue;
            }

            let mut flow_port_slug: HashMap<u16, (String, Option<String>)> = HashMap::new();

            for node in ingress_nodes {
                let (cfg_port, path_pattern, methods) = match &node.data {
                    NodeData::HttpIngress(cfg) => (
                        cfg.port,
                        if cfg.path_pattern.is_empty() {
                            "/*".to_string()
                        } else {
                            cfg.path_pattern.clone()
                        },
                        cfg.methods.clone(),
                    ),
                    _ => (0, "/*".to_string(), vec![]),
                };

                let effective_port = if cfg_port == 0 || cfg_port == DEFAULT_GATEWAY_PORT || cfg_port == default_port {
                    default_port
                } else {
                    cfg_port
                };
                let is_shared_gateway = effective_port == default_port;

                let (base_slug, alt_slug) = extract_flow_slug(flow);
                let (route_prefix, alt_prefix) = if is_shared_gateway {
                    let (slug, alt) = if let Some(assigned) = flow_port_slug.get(&effective_port) {
                        assigned.clone()
                    } else {
                        let port_slugs = seen_slugs.entry(effective_port).or_default();
                        let short_uuid = if flow.uuid.len() >= 6 {
                            flow_engine_core::dsl::slugify(&flow.uuid[..6])
                        } else {
                            flow_engine_core::dsl::slugify(&flow.uuid)
                        };

                        // 循环探测确保前缀绝对全局唯一，杜绝哈希或同名碰撞
                        let mut resolved_slug = base_slug.clone();
                        let mut counter = 2;
                        while port_slugs.contains(&resolved_slug) {
                            let candidate = if !short_uuid.is_empty() {
                                format!("{}-{}", base_slug, short_uuid)
                            } else {
                                format!("{}-{}", base_slug, counter)
                            };
                            if !port_slugs.contains(&candidate) {
                                resolved_slug = candidate;
                                break;
                            }
                            resolved_slug = format!("{}-{}-{}", base_slug, short_uuid, counter);
                            counter += 1;
                        }

                        port_slugs.insert(resolved_slug.clone());

                        // 仅当 alt_slug 未被其他流程的主路由占用时才作为别名挂载，防止路由覆盖
                        let safe_alt_slug = alt_slug.filter(|alt| !port_slugs.contains(alt));
                        if let Some(ref a) = safe_alt_slug {
                            port_slugs.insert(a.clone());
                        }

                        let pair = (resolved_slug.clone(), safe_alt_slug);
                        flow_port_slug.insert(effective_port, pair.clone());
                        pair
                    };
                    (Some(slug), alt)
                } else {
                    if let Some((existing_uuid, existing_name)) = custom_port_flows.get(&effective_port) {
                        if existing_uuid != &flow.uuid {
                            return Err(format!(
                                "Custom port {} conflict: used by flow '{}' ({}) and flow '{}' ({})",
                                effective_port, existing_name, existing_uuid, flow.name, flow.uuid
                            ));
                        }
                    } else {
                        custom_port_flows.insert(effective_port, (flow.uuid.clone(), flow.name.clone()));
                    }
                    (None, None)
                };

                let route = IngressRoute {
                    flow: flow.clone(),
                    route_prefix: route_prefix.clone(),
                    alt_prefix,
                    path_pattern,
                    methods,
                };

                mounted_routes.push((flow.name.clone(), effective_port, route_prefix));
                port_routers
                    .entry(effective_port)
                    .or_default()
                    .routes
                    .push(route);
            }
        }

        Ok(IngressMountPlan {
            port_routers,
            mounted_routes,
        })
    }
}

/// 路由挂载规划结果
#[derive(Debug)]
pub struct IngressMountPlan {
    /// 各端口对应的 `IngressRouter`
    pub port_routers: HashMap<u16, IngressRouter>,
    /// 成功挂载的流程及路由前缀信息：(`flow_name`, `port`, `route_prefix`)
    pub mounted_routes: Vec<(String, u16, Option<String>)>,
}

/// 提取流程的路由 Slug 标识
pub fn extract_flow_slug(flow: &Flow) -> (String, Option<String>) {
    let name_slug = flow_engine_core::dsl::slugify(&flow.name);
    let uuid_clean = if !flow.uuid.is_empty()
        && !flow.uuid.contains(' ')
        && flow.uuid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && flow.uuid.len() <= 36
    {
        Some(flow.uuid.to_lowercase())
    } else if flow.uuid.len() >= 8 {
        Some(flow_engine_core::dsl::slugify(&flow.uuid[..8]))
    } else {
        None
    };

    if !name_slug.is_empty() {
        if let Some(ref u) = uuid_clean {
            if u != &name_slug {
                return (name_slug, Some(u.clone()));
            }
        }
        (name_slug, None)
    } else if let Some(u) = uuid_clean {
        (u, None)
    } else {
        ("flow".to_string(), None)
    }
}

/// Ingress 监听器配置
pub struct IngressListenerConfig {
    /// 被监听的流程（单流程监听模式）
    pub flow: Flow,
    /// 路由表（多 Flow 挂载共享网关时优先使用）
    pub router: Option<IngressRouter>,
    /// 节点注册表
    pub registry: Arc<NodeRegistry>,
    /// 调试启动器；为 `None` 时该端口的请求走普通执行
    pub debug_launcher: Option<Arc<dyn IngressDebugLauncher>>,
    /// 客户端等待响应的上限
    pub request_timeout: Duration,
}

impl IngressListenerConfig {
    /// 创建单流程监听配置
    pub fn new(
        flow: Flow,
        registry: Arc<NodeRegistry>,
        debug_launcher: Option<Arc<dyn IngressDebugLauncher>>,
        request_timeout: Duration,
    ) -> Self {
        Self {
            flow,
            router: None,
            registry,
            debug_launcher,
            request_timeout,
        }
    }

    /// 对外部请求进行路由决议
    pub fn match_route<'a>(&'a self, path: &str, method: &str) -> RouteMatchResult<'a> {
        if let Some(router) = &self.router {
            return router.match_request(path, method);
        }

        let ingress_node = self.flow.nodes.iter().find(|n| {
            n.node_type == "http:ingress"
                || n.node_type == flow_engine_core::node_types::HTTP_INGRESS
        });

        if let Some(node) = ingress_node {
            if let NodeData::HttpIngress(cfg) = &node.data {
                if !matches_path(&cfg.path_pattern, path) {
                    return RouteMatchResult::NotFound;
                }
                if !cfg.methods.is_empty()
                    && !cfg.methods.iter().any(|m| m.eq_ignore_ascii_case(method))
                {
                    return RouteMatchResult::MethodNotAllowed;
                }
            }
        }

        RouteMatchResult::Found(&self.flow, path.to_string())
    }
}

/// Ingress 监听服务句柄
pub struct IngressServerHandle {
    /// 监听端口
    pub port: u16,
    /// 监听流程 UUID
    pub flow_uuid: String,
    /// 监听流程名称
    pub flow_name: String,
    /// 停止信号发送端
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// accept 循环任务句柄（用于停止时等待端口真正释放）
    task: Option<tokio::task::JoinHandle<()>>,
}

impl IngressServerHandle {
    /// 发送停止信号（不等待任务退出）
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }

    /// 发送停止信号并等待 accept 任务退出，确保监听套接字已释放
    ///
    /// 不等任务退出就返回时，紧接着在同一端口重新启动会撞上 `EADDRINUSE`。
    pub async fn shutdown(mut self) {
        self.stop();
        if let Some(task) = self.task.take() {
            if tokio::time::timeout(INGRESS_SHUTDOWN_TIMEOUT, task)
                .await
                .is_err()
            {
                log::warn!(
                    "[Ingress] Port {} accept task did not exit within {}s",
                    self.port,
                    INGRESS_SHUTDOWN_TIMEOUT.as_secs()
                );
            }
        }
    }
}

impl Drop for IngressServerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 停止监听时等待 accept 任务退出的上限
const INGRESS_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// Ingress 监听管理器（全局多端口管理）
#[derive(Default)]
pub struct IngressManager {
    servers: Mutex<HashMap<u16, IngressServerHandle>>,
    op_lock: Mutex<()>,
}
impl IngressManager {
    /// 创建新的 Ingress 管理器
    pub fn new() -> Self {
        Self {
            servers: Mutex::new(HashMap::new()),
            op_lock: Mutex::new(()),
        }
    }

    /// 启动指定端口的 Ingress 监听服务。
    /// 若该端口已有其它流程在监听，自动平滑停止旧流程并接管端口（返回 Some(old_flow_name)），
    /// 杜绝端口互踩报错，实现焦点自动接管。
    pub async fn start_listener(
        &self,
        port: u16,
        config: IngressListenerConfig,
    ) -> Result<Option<String>, String> {
        let _guard = self.op_lock.lock().await;
        let (replaced_flow_name, old_handle) = {
            let mut servers = self.servers.lock().await;
            if let Some(existing) = servers.remove(&port) {
                (Some(existing.flow_name.clone()), Some(existing))
            } else {
                (None, None)
            }
        };

        if let Some(handle) = old_handle {
            log::info!(
                "[Ingress] Port {} was listening for flow '{:?}'. Shutting down old listener to take over...",
                port, replaced_flow_name
            );
            handle.shutdown().await;
        }

        let debug_enabled = config.debug_launcher.is_some();
        let flow_name = config.flow.name.clone();

        let handle = start_ingress_server(port, config).await?;
        {
            let mut servers = self.servers.lock().await;
            servers.insert(port, handle);
        }

        log::info!(
            "[Ingress] Started HTTP mock server on port {} for flow '{}' (debug={})",
            port,
            flow_name,
            debug_enabled
        );
        Ok(replaced_flow_name)
    }

    /// 启动指定端口的 Ingress 路由表监听服务（支持多 Flow 挂载）
    pub async fn start_router_listener(
        &self,
        port: u16,
        router: IngressRouter,
        registry: Arc<NodeRegistry>,
        debug_launcher: Option<Arc<dyn IngressDebugLauncher>>,
        request_timeout: Duration,
    ) -> Result<(), String> {
        let dummy_flow = router
            .routes
            .first()
            .map(|r| r.flow.clone())
            .unwrap_or_default();
        let config = IngressListenerConfig {
            flow: dummy_flow,
            router: Some(router),
            registry,
            debug_launcher,
            request_timeout,
        };
        self.start_listener(port, config).await.map(|_| ())
    }

    /// 停止指定端口的 Ingress 监听服务（等待监听套接字真正释放）
    pub async fn stop_listener(&self, port: u16) -> Result<(), String> {
        // 先从表中摘除句柄，再在锁外等待任务退出，避免长时间持锁
        let handle = {
            let mut servers = self.servers.lock().await;
            servers.remove(&port)
        };
        match handle {
            Some(handle) => {
                handle.shutdown().await;
                log::info!("[Ingress] Stopped HTTP mock server on port {}", port);
                Ok(())
            }
            None => Err(format!("Port {port} is not listening")),
        }
    }

    /// 检查指定端口是否处于监听状态
    pub async fn is_listening(&self, port: u16) -> bool {
        let servers = self.servers.lock().await;
        servers.contains_key(&port)
    }

    /// 获取所有活跃监听端口列表
    pub async fn get_active_ports(&self) -> Vec<u16> {
        let servers = self.servers.lock().await;
        let mut ports: Vec<u16> = servers.keys().copied().collect();
        ports.sort_unstable();
        ports
    }

    /// 停止所有监听服务并释放套接字
    pub async fn stop_all(&self) {
        let handles: Vec<IngressServerHandle> = {
            let mut servers = self.servers.lock().await;
            servers.drain().map(|(_, h)| h).collect()
        };
        for handle in handles {
            handle.shutdown().await;
        }
    }
}

/// 启动 Ingress TCP/HTTP 监听器
pub async fn start_ingress_server(
    port: u16,
    config: IngressListenerConfig,
) -> Result<IngressServerHandle, String> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Port {} is already in use: {}", port, e))?;

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    let flow_uuid = config.flow.uuid.clone();
    let flow_name = config.flow.name.clone();
    let ctx = Arc::new(config);
    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    log::info!("[Ingress] Shutdown signal received for port {}", port);
                    break;
                }
                accept_res = listener.accept() => {
                    match accept_res {
                        Ok((stream, client_addr)) => {
                            let ctx_clone = ctx.clone();
                            let conn_timeout = ctx_clone
                                .request_timeout
                                .saturating_add(Duration::from_secs(15));
                            tokio::spawn(async move {
                                match tokio::time::timeout(
                                    conn_timeout,
                                    handle_http_connection(stream, ctx_clone),
                                )
                                .await
                                {
                                    Ok(Err(e)) => {
                                        log::warn!("[Ingress {}] Error handling connection: {}", client_addr, e);
                                    }
                                    Err(_) => {
                                        log::warn!("[Ingress {}] Connection timed out after {}s", client_addr, conn_timeout.as_secs());
                                    }
                                    Ok(Ok(())) => {}
                                }
                            });
                        }
                        Err(e) => {
                            log::warn!("[Ingress] Accept error on port {}: {}", port, e);
                        }
                    }
                }
            }
        }
    });
    Ok(IngressServerHandle {
        port,
        flow_uuid,
        flow_name,
        shutdown_tx: Some(shutdown_tx),
        task: Some(task),
    })
}

/// 处理单条外部 HTTP 连接
async fn handle_http_connection(
    mut stream: TcpStream,
    ctx: Arc<IngressListenerConfig>,
) -> Result<(), String> {
    let mut buffer = Vec::with_capacity(4096);
    let mut temp_buf = [0u8; 2048];

    // 1. 读取头部直至 \r\n\r\n
    let header_end_pos = loop {
        let n = stream
            .read(&mut temp_buf)
            .await
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&temp_buf[..n]);

        if let Some(pos) = find_header_end(&buffer) {
            break pos;
        }

        if buffer.len() > 65536 {
            return Err("HTTP header too large".to_string());
        }
    };

    let header_bytes = &buffer[..header_end_pos];
    let header_str = String::from_utf8_lossy(header_bytes);
    let mut lines = header_str.lines();

    // 2. 解析请求行 (METHOD PATH HTTP/1.1)
    let request_line = lines.next().ok_or("Empty request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_uppercase();
    let full_path = parts.next().unwrap_or("/").to_string();

    let (path, query_map) = parse_path_and_query(&full_path);

    // 3. 解析请求头
    let mut headers_map = HashMap::new();
    let mut content_length: usize = 0;
    let mut chunked = false;
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_string();
            let val = v.trim().to_string();
            if key.eq_ignore_ascii_case("content-length") {
                content_length = val.parse::<usize>().unwrap_or(0);
            }
            if key.eq_ignore_ascii_case("transfer-encoding")
                && val.to_ascii_lowercase().contains("chunked")
            {
                chunked = true;
            }
            headers_map.insert(key, val);
        }
    }

    // 3a. 不支持的传输方式：分块编码会让请求体被静默丢弃，必须显式拒绝而不是当成空体
    if chunked {
        write_http_response(
            &mut stream,
            411,
            &[("Content-Type", "text/plain; charset=utf-8")],
            b"Chunked request bodies are not supported; use Content-Length",
        )
        .await;
        return Ok(());
    }

    // 3b. 请求体体积上限：超出直接拒绝，避免无界读取耗尽内存
    if content_length > MAX_REQUEST_BODY_BYTES {
        write_http_response(
            &mut stream,
            413,
            &[("Content-Type", "text/plain; charset=utf-8")],
            format!("Request body too large (limit {MAX_REQUEST_BODY_BYTES} bytes)").as_bytes(),
        )
        .await;
        return Ok(());
    }

    // 4. 读取请求体
    let body_start = header_end_pos + 4;
    let mut body_bytes = if buffer.len() > body_start {
        buffer[body_start..].to_vec()
    } else {
        Vec::new()
    };

    while body_bytes.len() < content_length {
        let needed = content_length - body_bytes.len();
        let read_size = needed.min(temp_buf.len());
        let n = stream
            .read(&mut temp_buf[..read_size])
            .await
            .map_err(|e| e.to_string())?;
        if n == 0 {
            // 连接在读完声明长度前关闭：不得把截断体当成完整请求继续执行
            write_http_response(
                &mut stream,
                400,
                &[("Content-Type", "text/plain; charset=utf-8")],
                b"Incomplete request body",
            )
            .await;
            return Ok(());
        }
        body_bytes.extend_from_slice(&temp_buf[..n]);
    }

    let body_json: serde_json::Value = if body_bytes.is_empty() {
        serde_json::Value::Null
    } else if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
        parsed
    } else {
        serde_json::Value::String(String::from_utf8_lossy(&body_bytes).to_string())
    };
    // 5. 校验 Ingress 路由配置 (path_pattern, methods, route_prefix)
    let (flow, relative_path) = match ctx.match_route(&path, &method) {
        RouteMatchResult::Found(f, rel_path) => (f.clone(), rel_path),
        RouteMatchResult::MethodNotAllowed => {
            write_http_response(
                &mut stream,
                405,
                &[("Content-Type", "text/plain; charset=utf-8")],
                b"Method Not Allowed",
            )
            .await;
            return Ok(());
        }
        RouteMatchResult::NotFound => {
            write_http_response(
                &mut stream,
                404,
                &[("Content-Type", "text/plain; charset=utf-8")],
                b"Not Found",
            )
            .await;
            return Ok(());
        }
    };

    // 6. 构造 ExecutionContext 并注入 request 变量与 response channel
    let (respond_tx, respond_rx) = oneshot::channel::<IngressResponsePayload>();
    let response_channel = Arc::new(IngressResponseChannel {
        tx: Arc::new(Mutex::new(Some(respond_tx))),
    });
    // 记录执行失败原因：通道关闭时据此区分“流程失败”与“流程未走 http:respond”
    let outcome = Arc::new(IngressOutcome::default());
    let request_vars = serde_json::json!({
        "request": {
            "method": method,
            "path": relative_path,
            "query": query_map,
            "headers": headers_map,
            "body": body_json.clone(),
        },
        "request.method": method,
        "request.path": relative_path,
        "request.query": query_map,
        "request.headers": headers_map,
        "request.body": body_json.clone(),
        "method": method,
        "path": relative_path,
        "body": body_json,
    });
    // 流程自身的初始变量/环境变量必须保留，request.* 覆盖同名键
    let mut initial_vars = flow
        .get_active_variables()
        .map(|v| serde_json::json!(v))
        .unwrap_or_else(|| serde_json::json!({}));
    if !initial_vars.is_object() {
        initial_vars = serde_json::json!({});
    }
    if let (Some(base), Some(req)) = (initial_vars.as_object_mut(), request_vars.as_object()) {
        for (k, v) in req {
            base.insert(k.clone(), v.clone());
        }
    }

    // 7. 分派执行：存在可用调试启动器时走可调试执行器（断点/单步生效，客户端连接
    //    保持挂起直到 http:respond 执行）；否则走普通执行器。
    let debug_launcher = ctx
        .debug_launcher
        .as_ref()
        .filter(|launcher| launcher.is_available())
        .cloned();

    if let Some(launcher) = debug_launcher {
        let breakpoints: Vec<String> = flow
            .nodes
            .iter()
            .filter(|n| n.has_breakpoint == Some(true))
            .map(|n| n.uuid.clone())
            .collect();
        let request = IngressRequest {
            flow: flow.clone(),
            variables: initial_vars,
            breakpoints,
            registry: ctx.registry.clone(),
            response_channel: response_channel.clone(),
        };
        let outcome = outcome.clone();
        tokio::spawn(async move {
            if let Err(e) = launcher.launch(request).await {
                log::warn!("[Ingress] Debug session failed: {}", e);
                outcome.set_failure(e);
            }
        });
    } else {
        if ctx.debug_launcher.is_some() {
            log::info!("[Ingress] Debug session busy, falling back to plain execution");
        }
        let flow_exec = flow.clone();
        let executor = crate::executor::FlowExecutor::new()
            .with_registry(ctx.registry.clone())
            .with_provider(INGRESS_RESPONSE_PROVIDER, response_channel);
        let outcome = outcome.clone();
        tokio::spawn(async move {
            let result = executor
                .execute_flow(&flow_exec, Some(initial_vars), None, None)
                .await;
            if !result.success {
                let message = result
                    .error
                    .unwrap_or_else(|| "flow execution failed".to_string());
                log::warn!(
                    "[Ingress] Flow execution failed for {}: {}",
                    flow_exec.uuid,
                    message
                );
                outcome.set_failure(message);
            }
        });
    }

    // 8. 等待流程中的 http:respond 节点完成（调试模式下等待人工单步，上限由监听配置给定）
    match tokio::time::timeout(ctx.request_timeout, respond_rx).await {
        Ok(Ok(payload)) => {
            let status_code = payload.status_code;
            let headers: Vec<(String, String)> = payload
                .headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            write_http_response_with_status(&mut stream, status_code, &headers, &payload.body)
                .await;
        }
        Ok(Err(_)) => {
            // 通道关闭：流程结束时没有执行到 http:respond。
            // 失败原因已知 → 500 并把原因回传；流程正常结束但未响应 → 同为 500 并说明原因，
            // 绝不返回成功的 200（否则调用方会把失败当成成功）。
            let (message, log_detail) = match outcome.failure() {
                Some(failure) => (format!("Flow execution failed: {failure}"), failure),
                None => (
                    "Flow finished without executing http:respond".to_string(),
                    "no http:respond executed".to_string(),
                ),
            };
            log::warn!(
                "[Ingress] Responding 500 for flow {} ({})",
                flow.uuid,
                log_detail
            );
            let body = serde_json::to_vec(&serde_json::json!({ "error": message }))
                .unwrap_or_else(|_| b"{\"error\":\"flow execution failed\"}".to_vec());
            write_http_response(
                &mut stream,
                500,
                &[("Content-Type", "application/json; charset=utf-8")],
                &body,
            )
            .await;
        }
        Err(_) => {
            // 超时（调试暂停时间过长或流程死锁）
            write_http_response(
                &mut stream,
                504,
                &[("Content-Type", "text/plain; charset=utf-8")],
                b"Gateway Timeout",
            )
            .await;
        }
    }

    Ok(())
}

/// 单次请求允许的请求体上限（8 MiB）
const MAX_REQUEST_BODY_BYTES: usize = 8 * 1024 * 1024;

/// 一次 Ingress 请求的执行结果，用于区分“未响应”的原因
#[derive(Default)]
struct IngressOutcome {
    /// 流程执行失败信息；`None` 表示流程正常结束
    failure: std::sync::Mutex<Option<String>>,
}

impl IngressOutcome {
    /// 记录执行失败原因
    fn set_failure(&self, message: String) {
        if let Ok(mut guard) = self.failure.lock() {
            *guard = Some(message);
        }
    }

    /// 读取执行失败原因
    fn failure(&self) -> Option<String> {
        self.failure.lock().ok().and_then(|guard| guard.clone())
    }
}

/// HTTP 状态码 → reason phrase
fn status_text(status_code: u16) -> &'static str {
    match status_code {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "OK",
    }
}

/// 写出一个响应（无自定义响应头），并显式声明 `Connection: close`
async fn write_http_response(
    stream: &mut TcpStream,
    status_code: u16,
    extra_headers: &[(&str, &str)],
    body: &[u8],
) {
    let headers: Vec<(String, String)> = extra_headers
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    write_http_response_with_status(stream, status_code, &headers, body).await;
}

/// 写出一个响应（可带自定义响应头）
///
/// 单次请求处理完毕后连接即被关闭，因此固定发送 `Connection: close`，
/// 避免 HTTP/1.1 默认 keep-alive 的客户端复用已关闭的连接。
/// 与框架生成的 `Content-Length`/`Connection` 冲突的用户头会被忽略。
async fn write_http_response_with_status(
    stream: &mut TcpStream,
    status_code: u16,
    headers: &[(String, String)],
    body: &[u8],
) {
    let mut resp_str = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        status_code,
        status_text(status_code),
        body.len()
    );
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("content-length") || k.eq_ignore_ascii_case("connection") {
            continue;
        }
        resp_str.push_str(&format!("{}: {}\r\n", k, v));
    }
    resp_str.push_str("\r\n");

    let mut resp_bytes = resp_str.into_bytes();
    resp_bytes.extend_from_slice(body);

    let _ = stream.write_all(&resp_bytes).await;
    let _ = stream.flush().await;
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_path_and_query(full_path: &str) -> (String, HashMap<String, String>) {
    if let Some((p, q)) = full_path.split_once('?') {
        let mut map = HashMap::new();
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                map.insert(k.to_string(), v.to_string());
            } else if !pair.is_empty() {
                map.insert(pair.to_string(), String::new());
            }
        }
        (p.to_string(), map)
    } else {
        (full_path.to_string(), HashMap::new())
    }
}

fn matches_path(pattern: &str, path: &str) -> bool {
    if pattern == "/*" || pattern == "*" || pattern.is_empty() {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/*") {
        return path.starts_with(prefix);
    }
    pattern == path
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::debug_types::{DebugCommand, DebugEvent};
    use tokio::sync::mpsc;

    #[test]
    fn test_path_pattern_matching() {
        assert!(matches_path("/*", "/webhook/stripe"));
        assert!(matches_path("/api/*", "/api/v1/users"));
        assert!(matches_path("/api/*", "/api/pay"));
        assert!(!matches_path("/api/*", "/other/path"));
        assert!(matches_path("/exact/path", "/exact/path"));
        assert!(!matches_path("/exact/path", "/exact/path2"));
    }

    #[test]
    fn test_parse_path_and_query() {
        let (path, query) = parse_path_and_query("/users?id=123&type=admin");
        assert_eq!(path, "/users");
        assert_eq!(query.get("id"), Some(&"123".to_string()));
        assert_eq!(query.get("type"), Some(&"admin".to_string()));
    }

    #[tokio::test]
    async fn test_ingress_server_mock_flow() {
        let registry = Arc::new(crate::default_registry::default_registry());
        let port = 19527;

        let ingress_node = flow_engine_core::node::Node {
            uuid: "node-in".to_string(),
            node_type: "http:ingress".to_string(),
            name: "Ingress".to_string(),
            position: Default::default(),
            data: flow_engine_core::node_config::NodeData::HttpIngress(
                flow_engine_core::node_config::HttpIngressConfig {
                    port,
                    path_pattern: "/webhook/pay".to_string(),
                    methods: vec!["POST".to_string()],
                },
            ),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let respond_node = flow_engine_core::node::Node {
            uuid: "node-out".to_string(),
            node_type: "http:respond".to_string(),
            name: "Respond".to_string(),
            position: Default::default(),
            data: flow_engine_core::node_config::NodeData::HttpRespond(
                flow_engine_core::node_config::HttpRespondConfig {
                    status_code: 200,
                    headers: HashMap::new(),
                    body_type: "json".to_string(),
                    body: r#"{"trade_no": "{{body.trade_no}}", "seed": "{{flow_seed}}", "received": true}"#.to_string(),
                },
            ),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let conn = flow_engine_core::connection::Connection {
            id: "c1".to_string(),
            source_node_uuid: "node-in".to_string(),
            target_node_uuid: "node-out".to_string(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        };

        let flow = Flow {
            uuid: "mock-flow-1".to_string(),
            name: "Mock Webhook Flow".to_string(),
            description: None,
            nodes: vec![ingress_node, respond_node],
            connections: vec![conn],
            // 流程自身的初始变量必须在 Ingress 执行时保留（request.* 只做覆盖）
            initial_variables: Some(HashMap::from([(
                "flow_seed".to_string(),
                serde_json::json!("seed-1"),
            )])),
            setup: None,
            teardown: None,
        };

        let manager = IngressManager::new();
        let start_res = manager
            .start_listener(
                port,
                IngressListenerConfig {
                    flow,
                    router: None,
                    registry,
                    debug_launcher: None,
                    request_timeout: Duration::from_secs(60),
                },
            )
            .await;
        assert!(
            start_res.is_ok(),
            "Failed to start listener: {:?}",
            start_res
        );
        assert!(manager.is_listening(port).await);

        // 发送真实 HTTP 请求
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .expect("Failed to connect to ingress server");

        let req = b"POST /webhook/pay HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: 20\r\n\r\n{\"trade_no\": \"T001\"}";
        stream
            .write_all(req)
            .await
            .expect("Failed to write request");
        stream.flush().await.expect("Failed to flush request");

        let mut resp_buf = vec![0u8; 1024];
        let n = stream
            .read(&mut resp_buf)
            .await
            .expect("Failed to read response");
        let resp_str = String::from_utf8_lossy(&resp_buf[..n]);

        assert!(
            resp_str.starts_with("HTTP/1.1 200 OK"),
            "Unexpected response: {}",
            resp_str
        );
        assert!(
            resp_str.contains("{\"trade_no\": \"T001\", \"seed\": \"seed-1\", \"received\": true}"),
            "流程初始变量必须在 Ingress 执行中保留且 request.* 生效: {}",
            resp_str
        );

        // 停止监听服务
        assert!(manager.stop_listener(port).await.is_ok());
        assert!(!manager.is_listening(port).await);
    }

    #[tokio::test]
    async fn test_ingress_server_with_coexisting_entry_node() {
        let registry = Arc::new(crate::default_registry::default_registry());
        let port = 19533;

        let entry_node = flow_engine_core::node::Node {
            uuid: "default-entry".to_string(),
            node_type: "entry".to_string(),
            name: "Start".to_string(),
            position: Default::default(),
            data: flow_engine_core::node_config::NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let ingress_node = flow_engine_core::node::Node {
            uuid: "node-in".to_string(),
            node_type: "http:ingress".to_string(),
            name: "Ingress".to_string(),
            position: Default::default(),
            data: flow_engine_core::node_config::NodeData::HttpIngress(
                flow_engine_core::node_config::HttpIngressConfig {
                    port,
                    path_pattern: "/webhook/*".to_string(),
                    methods: vec!["POST".to_string()],
                },
            ),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let respond_node = flow_engine_core::node::Node {
            uuid: "node-out".to_string(),
            node_type: "http:respond".to_string(),
            name: "Respond".to_string(),
            position: Default::default(),
            data: flow_engine_core::node_config::NodeData::HttpRespond(
                flow_engine_core::node_config::HttpRespondConfig {
                    status_code: 200,
                    headers: HashMap::new(),
                    body_type: "json".to_string(),
                    body: r#"{"ok": true, "user": "{{body.user}}"}"#.to_string(),
                },
            ),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let conn = flow_engine_core::connection::Connection {
            id: "c1".to_string(),
            source_node_uuid: "node-in".to_string(),
            target_node_uuid: "node-out".to_string(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        };

        // 画布中同时包含孤立的 entry 节点与 ingress -> respond 链路
        let flow = Flow {
            uuid: "coexist-flow".to_string(),
            name: "Coexist Flow".to_string(),
            description: None,
            nodes: vec![entry_node, ingress_node, respond_node],
            connections: vec![conn],
            initial_variables: None,
            setup: None,
            teardown: None,
        };

        let manager = IngressManager::new();
        manager
            .start_listener(
                port,
                IngressListenerConfig {
                    flow,
                    router: None,
                    registry,
                    debug_launcher: None,
                    request_timeout: Duration::from_secs(60),
                },
            )
            .await
            .expect("Failed to start listener");

        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .expect("Failed to connect");

        let req = b"POST /webhook/test HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: 16\r\n\r\n{\"user\": \"alice\"}";
        stream.write_all(req).await.expect("Failed to write");
        stream.flush().await.expect("Failed to flush");

        let mut resp_buf = vec![0u8; 1024];
        let n = stream.read(&mut resp_buf).await.expect("Failed to read");
        let resp_str = String::from_utf8_lossy(&resp_buf[..n]);

        assert!(
            resp_str.starts_with("HTTP/1.1 200 OK"),
            "Ingress 应优先以 http:ingress 为起点，忽略孤立的 entry 节点: {}",
            resp_str
        );
        assert!(resp_str.contains("{\"ok\": true, \"user\": \"alice\"}"));

        assert!(manager.stop_listener(port).await.is_ok());
    }

    #[tokio::test]
    async fn test_ingress_server_with_assertion_and_respond() {
        let registry = Arc::new(crate::default_registry::default_registry());
        let port = 19528;

        let ingress_node = flow_engine_core::node::Node {
            uuid: "in-node".to_string(),
            node_type: "http:ingress".to_string(),
            name: "Ingress".to_string(),
            position: Default::default(),
            data: flow_engine_core::node_config::NodeData::HttpIngress(
                flow_engine_core::node_config::HttpIngressConfig {
                    port,
                    path_pattern: "/pay/notify".to_string(),
                    methods: vec!["POST".to_string()],
                },
            ),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let assert_node = flow_engine_core::node::Node {
            uuid: "assert-node".to_string(),
            node_type: "assertion".to_string(),
            name: "Assert".to_string(),
            position: Default::default(),
            data: flow_engine_core::node_config::NodeData::Assertion(
                flow_engine_core::node_config::AssertionConfig {
                    assertions: vec![flow_engine_core::node_config::Assertion {
                        id: "a1".to_string(),
                        description: None,
                        target: "$.body.trade_no".to_string(),
                        operator: flow_engine_core::node_config::AssertionOperator::Eq,
                        expected: serde_json::json!("T001"),
                        enabled: true,
                    }],
                    ..Default::default()
                },
            ),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let respond_node = flow_engine_core::node::Node {
            uuid: "respond-node".to_string(),
            node_type: "http:respond".to_string(),
            name: "Respond".to_string(),
            position: Default::default(),
            data: flow_engine_core::node_config::NodeData::HttpRespond(
                flow_engine_core::node_config::HttpRespondConfig {
                    status_code: 200,
                    headers: HashMap::new(),
                    body_type: "json".to_string(),
                    body: r#"{"received": true}"#.to_string(),
                },
            ),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let conn1 = flow_engine_core::connection::Connection {
            id: "c1".to_string(),
            source_node_uuid: "in-node".to_string(),
            target_node_uuid: "assert-node".to_string(),
            source_port_id: "out".to_string(),
            target_port_id: "flow_in".to_string(),
        };

        let conn2 = flow_engine_core::connection::Connection {
            id: "c2".to_string(),
            source_node_uuid: "assert-node".to_string(),
            target_node_uuid: "respond-node".to_string(),
            source_port_id: "true".to_string(),
            target_port_id: "in".to_string(),
        };

        let flow = Flow {
            uuid: "mock-flow-2".to_string(),
            name: "Mock Flow with Assertion".to_string(),
            description: None,
            nodes: vec![ingress_node, assert_node, respond_node],
            connections: vec![conn1, conn2],
            initial_variables: None,
            setup: None,
            teardown: None,
        };

        let manager = IngressManager::new();
        let start_res = manager
            .start_listener(
                port,
                IngressListenerConfig {
                    flow,
                    router: None,
                    registry,
                    debug_launcher: None,
                    request_timeout: Duration::from_secs(60),
                },
            )
            .await;
        assert!(start_res.is_ok());

        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .expect("Failed to connect");

        let req = b"POST /pay/notify HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: 20\r\n\r\n{\"trade_no\": \"T001\"}";
        stream.write_all(req).await.expect("Write failed");
        stream.flush().await.expect("Flush failed");

        let mut resp_buf = vec![0u8; 1024];
        let n = stream.read(&mut resp_buf).await.expect("Read failed");
        let resp_str = String::from_utf8_lossy(&resp_buf[..n]);
        assert!(resp_str.starts_with("HTTP/1.1 200 OK"));
        assert!(resp_str.contains("{\"received\": true}"));

        assert!(manager.stop_listener(port).await.is_ok());
    }

    /// 测试用调试启动器：驱动真实的可调试执行器，并把事件/命令暴露给测试
    struct TestDebugLauncher {
        events: mpsc::UnboundedSender<DebugEvent>,
        commands: Mutex<Option<mpsc::UnboundedReceiver<DebugCommand>>>,
        launched: std::sync::atomic::AtomicUsize,
    }

    #[async_trait::async_trait]
    impl IngressDebugLauncher for TestDebugLauncher {
        fn is_available(&self) -> bool {
            true
        }

        async fn launch(&self, request: IngressRequest) -> Result<(), String> {
            self.launched
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let command_receiver = self
                .commands
                .lock()
                .await
                .take()
                .ok_or_else(|| "commands already consumed".to_string())?;

            let executor = crate::debug::DebuggableFlowExecutor::new()
                .with_registry(request.registry.clone())
                .with_provider(INGRESS_RESPONSE_PROVIDER, request.response_channel.clone());
            executor
                .start_debug(
                    &request.flow,
                    Some(request.variables.clone()),
                    request.breakpoints.clone(),
                    None,
                    self.events.clone(),
                    command_receiver,
                )
                .await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_ingress_debug_session_holds_client_until_continue() {
        let registry = Arc::new(crate::default_registry::default_registry());
        let port = 19529;

        let ingress_node = flow_engine_core::node::Node {
            uuid: "dbg-in".to_string(),
            node_type: "http:ingress".to_string(),
            name: "Ingress".to_string(),
            position: Default::default(),
            data: flow_engine_core::node_config::NodeData::HttpIngress(
                flow_engine_core::node_config::HttpIngressConfig {
                    port,
                    path_pattern: "/dbg/notify".to_string(),
                    methods: vec!["POST".to_string()],
                },
            ),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        // 断点落在 http:respond 上：流程暂停时客户端必须仍处于挂起状态
        let respond_node = flow_engine_core::node::Node {
            uuid: "dbg-out".to_string(),
            node_type: "http:respond".to_string(),
            name: "Respond".to_string(),
            position: Default::default(),
            data: flow_engine_core::node_config::NodeData::HttpRespond(
                flow_engine_core::node_config::HttpRespondConfig {
                    status_code: 201,
                    headers: HashMap::new(),
                    body_type: "json".to_string(),
                    body: r#"{"paused_then_responded": true}"#.to_string(),
                },
            ),
            ports: None,
            has_breakpoint: Some(true),
            on_error: None,
        };

        let conn = flow_engine_core::connection::Connection {
            id: "c1".to_string(),
            source_node_uuid: "dbg-in".to_string(),
            target_node_uuid: "dbg-out".to_string(),
            source_port_id: "out".to_string(),
            target_port_id: "in".to_string(),
        };

        let flow = Flow {
            uuid: "mock-flow-debug".to_string(),
            name: "Mock Flow Debug".to_string(),
            description: None,
            nodes: vec![ingress_node, respond_node],
            connections: vec![conn],
            initial_variables: None,
            setup: None,
            teardown: None,
        };

        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<DebugEvent>();
        let (command_tx, command_rx) = mpsc::unbounded_channel::<DebugCommand>();
        let launcher = Arc::new(TestDebugLauncher {
            events: event_tx,
            commands: Mutex::new(Some(command_rx)),
            launched: std::sync::atomic::AtomicUsize::new(0),
        });

        let manager = IngressManager::new();
        manager
            .start_listener(
                port,
                IngressListenerConfig {
                    flow,
                    router: None,
                    registry,
                    debug_launcher: Some(launcher),
                    request_timeout: Duration::from_secs(20),
                },
            )
            .await
            .expect("Failed to start listener");

        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .expect("Failed to connect");
        let req = b"POST /dbg/notify HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2\r\n\r\n{}";
        stream.write_all(req).await.expect("Write failed");
        stream.flush().await.expect("Flush failed");

        // 1. 会话必须在 respond 节点暂停
        let paused = loop {
            match tokio::time::timeout(Duration::from_secs(5), event_rx.recv()).await {
                Ok(Some(DebugEvent::PausedAtNode { node_uuid, .. })) => break node_uuid,
                Ok(Some(_)) => continue,
                other => panic!("expected PausedAtNode, got {:?}", other),
            }
        };
        assert_eq!(paused, "dbg-out", "断点应停在 http:respond 节点");

        // 2. 暂停期间客户端必须仍处于挂起：短时间内读不到任何响应
        let mut resp_buf = vec![0u8; 1024];
        let early = tokio::time::timeout(
            Duration::from_millis(300),
            tokio::io::AsyncReadExt::read(&mut stream, &mut resp_buf),
        )
        .await;
        assert!(early.is_err(), "暂停期间不应返回任何 HTTP 响应");

        // 3. 继续执行 → http:respond 发出响应 → 客户端拿到 201
        command_tx
            .send(DebugCommand::Continue)
            .expect("Failed to send Continue");
        let n = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::io::AsyncReadExt::read(&mut stream, &mut resp_buf),
        )
        .await
        .expect("Client should receive a response after continue")
        .expect("Read failed");

        let resp_str = String::from_utf8_lossy(&resp_buf[..n]);
        assert!(
            resp_str.starts_with("HTTP/1.1 201 Created"),
            "继续执行后应返回 http:respond 配置的状态码: {}",
            resp_str
        );
        assert!(resp_str.contains("{\"paused_then_responded\": true}"));

        assert!(manager.stop_listener(port).await.is_ok());
    }

    /// 构造 `http:ingress` -> `assertion`（必然失败）的流程，用于验证失败响应码
    fn failing_assertion_flow(port: u16) -> Flow {
        let ingress_node = flow_engine_core::node::Node {
            uuid: "fail-in".to_string(),
            node_type: "http:ingress".to_string(),
            name: "Ingress".to_string(),
            position: Default::default(),
            data: flow_engine_core::node_config::NodeData::HttpIngress(
                flow_engine_core::node_config::HttpIngressConfig {
                    port,
                    path_pattern: "/*".to_string(),
                    methods: vec!["POST".to_string()],
                },
            ),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let assert_node = flow_engine_core::node::Node {
            uuid: "fail-assert".to_string(),
            node_type: "assertion".to_string(),
            name: "Assert".to_string(),
            position: Default::default(),
            data: flow_engine_core::node_config::NodeData::Assertion(
                flow_engine_core::node_config::AssertionConfig {
                    assertions: vec![flow_engine_core::node_config::Assertion {
                        id: "a1".to_string(),
                        description: None,
                        target: "$.body.trade_no".to_string(),
                        operator: flow_engine_core::node_config::AssertionOperator::Eq,
                        expected: serde_json::json!("NEVER"),
                        enabled: true,
                    }],
                    ..Default::default()
                },
            ),
            ports: None,
            has_breakpoint: None,
            on_error: Some(flow_engine_core::node::ExecutionErrorStrategy::AbortFlow),
        };
        Flow {
            uuid: "mock-flow-fail".to_string(),
            name: "Failing Flow".to_string(),
            description: None,
            nodes: vec![ingress_node, assert_node],
            connections: vec![flow_engine_core::connection::Connection {
                id: "c1".to_string(),
                source_node_uuid: "fail-in".to_string(),
                target_node_uuid: "fail-assert".to_string(),
                source_port_id: "out".to_string(),
                target_port_id: "flow_in".to_string(),
            }],
            initial_variables: None,
            setup: None,
            teardown: None,
        }
    }

    /// 发送原始请求并读回完整响应
    async fn send_raw_request(port: u16, request: &[u8]) -> String {
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .expect("Failed to connect");
        stream.write_all(request).await.expect("Write failed");
        stream.flush().await.expect("Flush failed");

        let mut resp_buf = vec![0u8; 4096];
        let n = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::io::AsyncReadExt::read(&mut stream, &mut resp_buf),
        )
        .await
        .expect("Response timeout")
        .expect("Read failed");
        String::from_utf8_lossy(&resp_buf[..n]).to_string()
    }

    /// 流程失败必须回 500 而不是伪造的 200 成功回执
    #[tokio::test]
    async fn test_ingress_flow_failure_returns_500() {
        let port = 19530;
        let registry = Arc::new(crate::default_registry::default_registry());
        let manager = IngressManager::new();
        manager
            .start_listener(
                port,
                IngressListenerConfig {
                    flow: failing_assertion_flow(port),
                    router: None,
                    registry,
                    debug_launcher: None,
                    request_timeout: Duration::from_secs(30),
                },
            )
            .await
            .expect("Failed to start listener");

        let resp = send_raw_request(
            port,
            b"POST /any HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 20\r\n\r\n{\"trade_no\": \"T001\"}",
        )
        .await;

        assert!(
            resp.starts_with("HTTP/1.1 500 "),
            "流程失败必须返回 500，实际: {}",
            resp
        );
        assert!(
            resp.contains("\"error\""),
            "500 响应体应包含错误信息: {}",
            resp
        );
        assert!(manager.stop_listener(port).await.is_ok());
    }

    /// 分块传输与超限请求体必须被显式拒绝，不能被当成空体静默执行
    #[tokio::test]
    async fn test_ingress_rejects_chunked_and_oversized_bodies() {
        let port = 19531;
        let registry = Arc::new(crate::default_registry::default_registry());
        let manager = IngressManager::new();
        manager
            .start_listener(
                port,
                IngressListenerConfig {
                    flow: failing_assertion_flow(port),
                    router: None,
                    registry,
                    debug_launcher: None,
                    request_timeout: Duration::from_secs(30),
                },
            )
            .await
            .expect("Failed to start listener");

        let chunked = send_raw_request(
            port,
            b"POST /any HTTP/1.1\r\nHost: 127.0.0.1\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n",
        )
        .await;
        assert!(
            chunked.starts_with("HTTP/1.1 411 "),
            "chunked 请求应被显式拒绝: {}",
            chunked
        );

        let oversized = send_raw_request(
            port,
            format!(
                "POST /any HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n",
                MAX_REQUEST_BODY_BYTES + 1
            )
            .as_bytes(),
        )
        .await;
        assert!(
            oversized.starts_with("HTTP/1.1 413 "),
            "超限请求体应返回 413: {}",
            oversized
        );

        assert!(manager.stop_listener(port).await.is_ok());
    }

    /// 停止监听后必须立即可以在同一端口重启（accept 任务已退出）
    #[tokio::test]
    async fn test_ingress_listener_restarts_on_same_port_immediately() {
        let port = 19532;
        let registry = Arc::new(crate::default_registry::default_registry());
        let manager = IngressManager::new();

        for _ in 0..3 {
            manager
                .start_listener(
                    port,
                    IngressListenerConfig {
                        flow: failing_assertion_flow(port),
                        router: None,
                        registry: registry.clone(),
                        debug_launcher: None,
                        request_timeout: Duration::from_secs(10),
                    },
                )
                .await
                .expect("start_listener should not hit EADDRINUSE after stop");
            assert!(manager.stop_listener(port).await.is_ok());
        }
    }

    #[test]
    fn test_ingress_router_shared_gateway_matching() {
        let flow_a = Flow {
            uuid: "flow-a-uuid".to_string(),
            name: "Flow A".to_string(),
            ..Default::default()
        };
        let flow_b = Flow {
            uuid: "flow-b-uuid".to_string(),
            name: "Flow B".to_string(),
            ..Default::default()
        };

        let mut router = IngressRouter::new();
        router.add_route(IngressRoute {
            flow: flow_a,
            route_prefix: Some("flow-a".to_string()),
            alt_prefix: None,
            path_pattern: "/*".to_string(),
            methods: vec!["POST".to_string()],
        });
        router.add_route(IngressRoute {
            flow: flow_b,
            route_prefix: Some("flow-b".to_string()),
            alt_prefix: None,
            path_pattern: "/pay/*".to_string(),
            methods: vec!["GET".to_string(), "POST".to_string()],
        });

        // 1. 匹配 Flow A 前缀与相对路径
        match router.match_request("/flow-a/webhook", "POST") {
            RouteMatchResult::Found(f, rel) => {
                assert_eq!(f.name, "Flow A");
                assert_eq!(rel, "/webhook");
            }
            other => panic!("expected Found, got {:?}", other),
        }

        // 2. 匹配 Flow A 但方法不匹配
        match router.match_request("/flow-a/webhook", "GET") {
            RouteMatchResult::MethodNotAllowed => {}
            other => panic!("expected MethodNotAllowed, got {:?}", other),
        }

        // 3. 匹配 Flow B 前缀与子路径
        match router.match_request("/flow-b/pay/order-1", "GET") {
            RouteMatchResult::Found(f, rel) => {
                assert_eq!(f.name, "Flow B");
                assert_eq!(rel, "/pay/order-1");
            }
            other => panic!("expected Found, got {:?}", other),
        }

        // 4. 未注册路径返回 NotFound
        match router.match_request("/unknown/path", "GET") {
            RouteMatchResult::NotFound => {}
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_ingress_router_plan_from_flows() {
        fn make_flow(name: &str, uuid: &str, port: u16, path: &str) -> Flow {
            let node = flow_engine_core::node::Node {
                uuid: format!("{}-node", uuid),
                node_type: "http:ingress".to_string(),
                name: "Ingress".to_string(),
                position: Default::default(),
                data: NodeData::HttpIngress(flow_engine_core::node_config::HttpIngressConfig {
                    port,
                    path_pattern: path.to_string(),
                    methods: vec!["POST".to_string()],
                }),
                ports: None,
                has_breakpoint: None,
                on_error: None,
            };
            Flow {
                uuid: uuid.to_string(),
                name: name.to_string(),
                nodes: vec![node],
                ..Default::default()
            }
        }

        let f1 = make_flow("Order Pay", "uuid-111111", 9527, "/*");
        let f2 = make_flow("Order Pay", "uuid-222222", 9527, "/callback"); // slug 冲突
        let f3 = make_flow("Custom Flow", "uuid-333333", 8080, "/notify"); // 独立端口

        let plan = IngressRouter::plan_from_flows(&[f1, f2, f3], None).expect("plan routes");

        // 端口 9527 为共享网关，应包含两条路由，且 slug 被唯一隔离
        let gateway_router = plan.port_routers.get(&9527).expect("gateway router");
        assert_eq!(gateway_router.routes.len(), 2);
        let r1 = &gateway_router.routes[0];
        let r2 = &gateway_router.routes[1];
        assert_eq!(r1.route_prefix, Some("order-pay".to_string()));
        assert_eq!(r2.route_prefix, Some("order-pay-uuid-2".to_string()));

        // 端口 8080 为独立专属端口，无前缀
        let custom_router = plan.port_routers.get(&8080).expect("custom router");
        assert_eq!(custom_router.routes.len(), 1);
        assert_eq!(custom_router.routes[0].route_prefix, None);
    }

    #[test]
    fn test_ingress_router_custom_port_conflict() {
        fn make_flow(name: &str, uuid: &str, port: u16) -> Flow {
            let node = flow_engine_core::node::Node {
                uuid: format!("{}-node", uuid),
                node_type: "http:ingress".to_string(),
                name: "Ingress".to_string(),
                position: Default::default(),
                data: NodeData::HttpIngress(flow_engine_core::node_config::HttpIngressConfig {
                    port,
                    path_pattern: "/*".to_string(),
                    methods: vec![],
                }),
                ports: None,
                has_breakpoint: None,
                on_error: None,
            };
            Flow {
                uuid: uuid.to_string(),
                name: name.to_string(),
                nodes: vec![node],
                ..Default::default()
            }
        }

        let f1 = make_flow("Flow 1", "u1", 8080);
        let f2 = make_flow("Flow 2", "u2", 8080);
        let res = IngressRouter::plan_from_flows(&[f1, f2], None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Custom port 8080 conflict"));
    }

    #[test]
    fn test_ingress_user_scenario() {
        let node = flow_engine_core::node::Node {
            uuid: "771429e3-8323-46ce-be6e-fbd40bda0e28".to_string(),
            node_type: "http:ingress".to_string(),
            name: "HTTP Ingress 3".to_string(),
            position: Default::default(),
            data: NodeData::HttpIngress(flow_engine_core::node_config::HttpIngressConfig {
                port: 9527,
                path_pattern: "/a".to_string(),
                methods: vec!["GET".to_string(), "POST".to_string()],
            }),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };
        let flow = Flow {
            uuid: "demo-http-testing".to_string(),
            name: "HTTP Node Example Flow".to_string(),
            nodes: vec![node],
            ..Default::default()
        };

        let plan = IngressRouter::plan_from_flows(&[flow], Some(9527)).expect("plan ingress");
        let router = plan.port_routers.get(&9527).expect("router for 9527");

        // 1. UUID Slug 访问（http://127.0.0.1:9527/demo-http-testing/a）
        match router.match_request("/demo-http-testing/a", "GET") {
            RouteMatchResult::Found(f, rel) => {
                assert_eq!(f.uuid, "demo-http-testing");
                assert_eq!(rel, "/a");
            }
            other => panic!("expected Found for uuid slug, got {:?}", other),
        }

        // 2. Name Slug 访问（http://127.0.0.1:9527/http-node-example-flow/a）
        match router.match_request("/http-node-example-flow/a", "GET") {
            RouteMatchResult::Found(f, rel) => {
                assert_eq!(f.uuid, "demo-http-testing");
                assert_eq!(rel, "/a");
            }
            other => panic!("expected Found for name slug, got {:?}", other),
        }

        // 3. 直连根路径访问（http://127.0.0.1:9527/a）
        match router.match_request("/a", "GET") {
            RouteMatchResult::Found(f, rel) => {
                assert_eq!(f.uuid, "demo-http-testing");
                assert_eq!(rel, "/a");
            }
            other => panic!("expected Found for direct path, got {:?}", other),
        }

        // 4. 不存在的路径（/unknown）
        match router.match_request("/unknown", "GET") {
            RouteMatchResult::NotFound => {}
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_ingress_listener_auto_takeover_same_port() {
        let port = 19534;
        let registry = Arc::new(crate::default_registry::default_registry());
        let manager = IngressManager::new();

        let flow_a = Flow {
            uuid: "flow-a-uuid".to_string(),
            name: "Flow A".to_string(),
            nodes: vec![],
            ..Default::default()
        };
        let flow_b = Flow {
            uuid: "flow-b-uuid".to_string(),
            name: "Flow B".to_string(),
            nodes: vec![],
            ..Default::default()
        };

        // 1. 启动 Flow A，先前无监听，返回 None
        let res1 = manager
            .start_listener(
                port,
                IngressListenerConfig {
                    flow: flow_a,
                    router: None,
                    registry: registry.clone(),
                    debug_launcher: None,
                    request_timeout: Duration::from_secs(10),
                },
            )
            .await
            .expect("start Flow A");
        assert_eq!(res1, None);

        // 2. 启动 Flow B 占用相同端口，自动接管平滑停止 Flow A，返回 Some("Flow A")
        let res2 = manager
            .start_listener(
                port,
                IngressListenerConfig {
                    flow: flow_b,
                    router: None,
                    registry: registry.clone(),
                    debug_launcher: None,
                    request_timeout: Duration::from_secs(10),
                },
            )
            .await
            .expect("start Flow B takeover");
        assert_eq!(res2, Some("Flow A".to_string()));

        // 验证端口仍在监听
        assert!(manager.is_listening(port).await);
        assert!(manager.stop_listener(port).await.is_ok());
    }

    #[test]
    fn test_plan_from_flows_preserves_custom_port() {
        let flow = Flow {
            uuid: "flow-custom".to_string(),
            name: "Custom Flow".to_string(),
            nodes: vec![flow_engine_core::node::Node {
                uuid: "node-1".to_string(),
                node_type: "http:ingress".to_string(),
                name: "Ingress".to_string(),
                position: flow_engine_core::node::NodePosition { x: 0.0, y: 0.0 },
                data: NodeData::HttpIngress(flow_engine_core::node_config::HttpIngressConfig {
                    port: 8080,
                    path_pattern: "/api/test".to_string(),
                    methods: vec!["GET".to_string()],
                }),
                ports: None,
                has_breakpoint: None,
                on_error: None,
            }],
            ..Default::default()
        };

        let plan = IngressRouter::plan_from_flows(&[flow], Some(9527)).expect("plan router");
        assert!(plan.port_routers.contains_key(&8080));
        assert!(!plan.port_routers.contains_key(&9527));
        let router = &plan.port_routers[&8080];
        assert_eq!(router.routes[0].route_prefix, None);
    }

    #[test]
    fn test_plan_from_flows_detects_collision_for_same_named_flows() {
        let flow1 = Flow {
            uuid: "uuid-1".to_string(),
            name: "Duplicate Name".to_string(),
            nodes: vec![flow_engine_core::node::Node {
                uuid: "n1".to_string(),
                node_type: "http:ingress".to_string(),
                name: "Ingress".to_string(),
                position: flow_engine_core::node::NodePosition { x: 0.0, y: 0.0 },
                data: NodeData::HttpIngress(flow_engine_core::node_config::HttpIngressConfig {
                    port: 8080,
                    path_pattern: "/a".to_string(),
                    methods: vec![],
                }),
                ports: None,
                has_breakpoint: None,
                on_error: None,
            }],
            ..Default::default()
        };
        let flow2 = Flow {
            uuid: "uuid-2".to_string(),
            name: "Duplicate Name".to_string(),
            nodes: vec![flow_engine_core::node::Node {
                uuid: "n2".to_string(),
                node_type: "http:ingress".to_string(),
                name: "Ingress".to_string(),
                position: flow_engine_core::node::NodePosition { x: 0.0, y: 0.0 },
                data: NodeData::HttpIngress(flow_engine_core::node_config::HttpIngressConfig {
                    port: 8080,
                    path_pattern: "/b".to_string(),
                    methods: vec![],
                }),
                ports: None,
                has_breakpoint: None,
                on_error: None,
            }],
            ..Default::default()
        };

        let err = IngressRouter::plan_from_flows(&[flow1, flow2], None).unwrap_err();
        assert!(err.contains("Custom port 8080 conflict"));
    }

    #[test]
    fn test_extract_flow_slug_supports_36_char_uuid() {
        let flow = Flow {
            uuid: "12345678-1234-1234-1234-123456789abc".to_string(),
            name: "".to_string(),
            ..Default::default()
        };
        let (slug, alt) = extract_flow_slug(&flow);
        assert_eq!(slug, "12345678-1234-1234-1234-123456789abc");
        assert_eq!(alt, None);
    }

    #[test]
    fn test_match_request_wildcard_single_flow_fallback() {
        let flow = Flow {
            uuid: "flow-1".to_string(),
            name: "My Flow".to_string(),
            ..Default::default()
        };
        let router = IngressRouter {
            routes: vec![IngressRoute {
                flow: flow.clone(),
                route_prefix: Some("/my-flow".to_string()),
                alt_prefix: None,
                path_pattern: "/*".to_string(),
                methods: vec!["GET".to_string()],
            }],
        };

        // 1. 带前缀访问
        assert!(matches!(
            router.match_request("/my-flow/hello", "GET"),
            RouteMatchResult::Found(_, _)
        ));
        // 2. 单流程无前缀直连访问 /* 兜底成功
        assert!(matches!(
            router.match_request("/hello", "GET"),
            RouteMatchResult::Found(_, _)
        ));
    }
}
