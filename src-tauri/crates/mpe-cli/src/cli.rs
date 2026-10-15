//! mpe - Multi-Protocol Executor CLI Module
//!
//! Flow engine command-line tool, invoked by the unified `main.rs` entry point.

pub mod config;
pub mod debug_cmd;
pub mod env;
pub mod flow;
pub mod flow_edit;
pub mod gui_bridge;
pub mod junit;
pub mod output;
#[cfg(feature = "plugin-manager")]
pub mod plugin;
pub mod report;
pub mod run;
pub mod run_node;
pub mod stress;
pub mod validate;
pub mod variables;
pub mod summary;
pub mod serve;

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};

/// 环境目录覆盖是进程级全局状态；任何依赖它的测试必须持这把锁串行执行，
/// 否则会与其他测试模块写入的覆盖目录互相踩踏。
#[cfg(test)]
pub(crate) static ENV_DIR_OVERRIDE_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

// ── CLI Error类型 ──────────────────────────────────────────────────

/// CLI error type
#[derive(Debug)]
pub enum CliError {
    /// Configuration error
    Config(String),
    /// Execution error
    Execution {
        /// Error message
        message: String,
        /// Optional structured error data
        data: Option<serde_json::Value>,
    },
    /// Invalid arguments error
    InvalidArgs(String),
    /// I/O error
    Io(String),
    /// Infrastructure timeout / unreachable error (retryable)
    InfraTimeout(String),
}

impl CliError {
    /// Create an execution error without structured data
    pub fn execution(message: impl Into<String>) -> Self {
        Self::Execution {
            message: message.into(),
            data: None,
        }
    }

    /// Create an execution error with structured data
    pub fn execution_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self::Execution {
            message: message.into(),
            data: Some(data),
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Config(msg) => write!(f, "Configuration error: {}", msg),
            CliError::Execution { message, .. } => write!(f, "Execution error: {}", message),
            CliError::InvalidArgs(msg) => write!(f, "Invalid arguments: {}", msg),
            CliError::Io(msg) => write!(f, "I/O error: {}", msg),
            CliError::InfraTimeout(msg) => write!(f, "Infrastructure timeout / unreachable: {}", msg),
        }
    }
}
impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e.to_string())
    }
}

/// Detect if error message corresponds to an infrastructure timeout / unreachable error (retryable)
pub(crate) fn is_infra_error(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    lower.contains("econnrefused")
        || lower.contains("connection refused")
        || lower.contains("dns error")
        || lower.contains("dns resolution failed")
        || lower.contains("broker unreachable")
        || lower.contains("timed out")
        || lower.contains("connection timed out")
        || lower.contains("connect timeout")
        || lower.contains("timeout expired")
        || lower.contains("network unreachable")
        || lower.contains("host unreachable")
}

/// Parse --var key=value pair into (String, String) tuple
fn parse_var(s: &str) -> Result<(String, String), String> {
    let (key, val) = s
        .split_once('=')
        .ok_or_else(|| format!("--var format error (expected key=value): {}", s))?;
    Ok((key.to_string(), val.to_string()))
}

// ── Clap 定义 ─────────────────────────────────────────────────────

/// Multi-Protocol Executor — AI-ready flow engine CLI
#[derive(Parser)]
#[command(
    name = "mpe",
    version,
    about = "Multi-Protocol Flow Orchestration & Execution Engine"
)]
pub struct Cli {
    /// Output format (json or table)
    #[arg(long, global = true, default_value = "json")]
    format: String,

    /// CLI subcommands
    #[command(subcommand)]
    pub command: Commands,
}

/// CLI subcommand enumeration
#[derive(Subcommand)]
pub enum Commands {
    /// Execute a flow
    Run {
        /// Flow file path (.mpf)
        #[arg(short, long)]
        flow: String,
        /// Active environment name (loads from environments dir)
        #[arg(short = 'e', long = "env")]
        env: Option<String>,
        /// Variable overrides (key=value)
        #[arg(long, value_name = "KEY=VALUE")]
        var: Vec<String>,
        /// Enable NDJSON streaming output
        #[arg(long)]
        stream: bool,
        /// Do not save execution report
        #[arg(long)]
        no_report: bool,
        /// Report output format (json | junit)
        #[arg(long, default_value = "json")]
        report_format: String,
        /// Export execution report to this file path
        #[arg(long)]
        report_file: Option<std::path::PathBuf>,
        /// Export GitHub Actions Step Summary markdown to this file path
        #[arg(long)]
        summary: Option<std::path::PathBuf>,
    },
    /// Start long-running HTTP Ingress mock server
    Serve {
        /// Flow file path (.mpf or .yaml) or directory containing flows
        #[arg(short, long)]
        flow: String,
        /// Override listening port
        #[arg(short, long)]
        port: Option<u16>,
        /// Environment name
        #[arg(short = 'e', long = "env")]
        env: Option<String>,
        /// Variable overrides (KEY=VALUE)
        #[arg(long, value_name = "KEY=VALUE")]
        var: Vec<String>,
        /// Graceful shutdown timeout in seconds
        #[arg(long, default_value = "15")]
        graceful_timeout: u64,
    },
    /// Validate a flow
    Validate {
        /// Flow file path (.mpe)
        #[arg(short, long)]
        flow: String,
    },
    /// Debug a flow (NDJSON bidirectional protocol)
    Debug {
        /// Flow file path (.mpf)
        #[arg(short, long)]
        flow: String,
        /// Active environment name (loads from environments dir)
        #[arg(short = 'e', long = "env")]
        env: Option<String>,
        /// Breakpoint node UUID
        #[arg(long)]
        r#break: Vec<String>,
        /// Variable overrides (key=value)
        #[arg(long, value_name = "KEY=VALUE")]
        var: Vec<String>,
    },
    /// Stress testing
    Stress {
        /// Stress testing subcommand
        #[command(subcommand)]
        command: StressCommands,
    },
    /// Flow management
    Flow {
        /// Flow management subcommand
        #[command(subcommand)]
        command: FlowCommands,
    },
    /// Global environment file management
    Env {
        /// Environment subcommand
        #[command(subcommand)]
        command: EnvCommands,
    },
    /// Report management
    Report {
        /// Report management subcommand
        #[command(subcommand)]
        command: ReportCommands,
    },
    /// Execute a single protocol node for connectivity verification
    RunNode {
        /// JSON configuration for the node
        /// (e.g. '{"type":"redis:connect","host":"127.0.0.1","port":6379}')
        node_json: String,

        /// Flow variables (key=value or key=json)
        #[arg(short, long = "var", value_parser = parse_var)]
        var: Vec<(String, String)>,

        /// Output format (json)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Plugin management
    #[cfg(feature = "plugin-manager")]
    Plugin {
        /// Plugin management subcommand
        #[command(subcommand)]
        command: PluginCommands,
    },
    /// Activate and analyze the current project or execute AI tasks in MPE GUI
    Ai {
        /// Prompt for the AI (e.g. "Analyze APIs and generate login flow")
        prompt: String,
        /// Target workspace directory (defaults to current working directory)
        #[arg(short, long)]
        workspace: Option<PathBuf>,
    },
    /// Open or mount workspace/flow directory in MPE GUI
    Open {
        /// Project path or flow file path (defaults to current directory ".")
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

/// Plugin management subcommand enumeration
#[cfg(feature = "plugin-manager")]
#[derive(Subcommand)]
pub enum PluginCommands {
    /// Show the effective plugins directory
    Dir,
    /// List installed plugins
    List,
    /// Install a plugin from a marketplace
    Install {
        /// Plugin name
        name: String,
        /// Exact plugin version (defaults to the first listing match)
        #[arg(long)]
        version: Option<String>,
        /// Install directory (defaults to the effective plugins directory)
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Marketplace base URL (or `MPE_PLUGIN_REGISTRY` env var)
        #[arg(long)]
        registry: Option<String>,
        /// Re-install over an existing plugin
        #[arg(long)]
        force: bool,
    },
}

/// Stress testing subcommand enumeration
#[derive(Subcommand)]
pub enum StressCommands {
    /// Run stress test
    Run {
        /// Flow file path (.mpe)
        #[arg(short, long)]
        flow: String,
        /// Active environment name (loads from environments dir)
        #[arg(short = 'e', long = "env")]
        env: Option<String>,
        /// Variable overrides (key=value)
        #[arg(long, value_name = "KEY=VALUE")]
        var: Vec<String>,
        /// Number of concurrent users
        #[arg(short, long, default_value_t = 50)]
        concurrent_users: u32,
        /// Ramp-up time (seconds)
        #[arg(short, long, default_value_t = 10)]
        ramp_up: u32,
        /// Duration (seconds)
        #[arg(short, long, default_value_t = 60)]
        duration: u32,
        /// Output report file path
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Stress report management
    Report {
        /// Stress report subcommand
        #[command(subcommand)]
        command: StressReportCommands,
    },
}

/// Stress report subcommand enumeration
#[derive(Subcommand)]
pub enum StressReportCommands {
    /// List stress reports
    List,
    /// View stress report
    Show {
        /// Report file path
        file_path: String,
    },
    /// Delete stress report
    Delete {
        /// Report file path
        file_path: String,
    },
}

/// Flow management subcommand enumeration
#[derive(Subcommand)]
pub enum FlowCommands {
    /// List flows
    List {
        /// Search keyword
        #[arg(long)]
        search: Option<String>,
        /// Sort order (name/date)
        #[arg(long, default_value = "date")]
        sort: String,
    },
    /// Show flow details
    Show {
        /// Flow UUID or file path
        uuid_or_path: String,
    },
    /// Create a flow
    Create {
        /// Flow name
        #[arg(long)]
        name: String,
    },
    /// Copy a flow
    Copy {
        /// Flow UUID
        uuid: String,
    },
    /// Delete a flow
    Delete {
        /// Flow UUID
        uuid: String,
    },
    /// Import a flow
    Import {
        /// Flow file path
        #[arg(short, long)]
        flow: String,
        /// Flow name
        #[arg(long)]
        name: String,
    },
    /// Edit a flow
    Edit {
        /// Flow editing subcommand
        #[command(subcommand)]
        command: FlowEditCommands,
    },
    /// Format or convert flow files (.mpf) to YAML or JSON
    Fmt {
        /// Flow file path or UUID (optional if --all is used)
        path_or_uuid: Option<String>,
        /// Target format (yaml or json)
        #[arg(long, default_value = "yaml")]
        to: String,
        /// Format/convert all flow files in flows directory
        #[arg(long)]
        all: bool,
    },
    /// Convert between standard Flow (.mpf) and declarative YAML DSL
    Convert {
        /// Input file path (.mpf or .yaml)
        input: String,
        /// Output file path (.yaml or .mpf)
        output: String,
    },
}

/// Flow editing subcommand enumeration
#[derive(Subcommand)]
pub enum FlowEditCommands {
    /// Add a node
    NodeAdd {
        /// Flow UUID
        #[arg(long)]
        flow: String,
        /// Node type
        #[arg(long)]
        r#type: String,
        /// Node name
        #[arg(long)]
        name: String,
        /// Node configuration (JSON string)
        #[arg(long)]
        config: Option<String>,
    },
    /// Remove a node
    NodeRemove {
        /// Flow UUID
        #[arg(long)]
        flow: String,
        /// Node UUID
        #[arg(long)]
        node: String,
    },
    /// Update a node
    NodeUpdate {
        /// Flow UUID
        #[arg(long)]
        flow: String,
        /// Node UUID
        #[arg(long)]
        node: String,
        /// Node configuration (JSON string)
        #[arg(long)]
        config: Option<String>,
    },
    /// Add a connection
    ConnectionAdd {
        /// Flow UUID
        #[arg(long)]
        flow: String,
        /// Source node UUID
        #[arg(long)]
        source: String,
        /// Target node UUID
        #[arg(long)]
        target: String,
        /// Source port
        #[arg(long, default_value = "out")]
        source_port: String,
        /// Target port (fixed to "in")
        #[arg(long, default_value = "in")]
        target_port: String,
    },
    /// Remove a connection
    ConnectionRemove {
        /// Flow UUID
        #[arg(long)]
        flow: String,
        /// Connection ID
        #[arg(long)]
        connection: String,
    },
    /// Set a flow fallback variable (flow.initial_variables; environment files are managed separately)
    VariableSet {
        /// Flow UUID
        #[arg(long)]
        flow: String,
        /// Variable name
        #[arg(long)]
        key: String,
        /// Variable value
        #[arg(long)]
        value: String,
    },
    /// Remove a flow fallback variable (flow.initial_variables)
    VariableRemove {
        /// Flow UUID
        #[arg(long)]
        flow: String,
        /// Variable name
        #[arg(long)]
        key: String,
    },
}

/// Global environment file management subcommand enumeration
///
/// 环境是流程之外的外挂 YAML 文件：`<name>.yaml` 为共享定义（会进 git），
/// `<name>.local.yaml` 为机器私有覆盖（可进 gitignore 保存密钥）。
#[derive(Subcommand)]
pub enum EnvCommands {
    /// List all environments (name, variable count, local override flag, path)
    List,
    /// Show an environment's variables
    Show {
        /// Environment name
        name: String,
        /// Layer to print: merged (base + local override), base, or local
        #[arg(long, default_value = "merged", value_name = "LAYER")]
        source: String,
    },
    /// Add or update variables (KEY=VALUE; values are parsed as JSON when possible)
    Set {
        /// Environment name
        name: String,
        /// Variables to write (e.g. API_URL=https://dev.api TIMEOUT=1000)
        #[arg(required = true, value_name = "KEY=VALUE")]
        vars: Vec<String>,
        /// Write to the machine-local override file (<name>.local.yaml) instead of the shared file
        #[arg(long)]
        local: bool,
    },
    /// Delete an environment (shared file plus its local override)
    Delete {
        /// Environment name
        name: String,
        /// Delete only the machine-local override file, keeping the shared definition
        #[arg(long)]
        local: bool,
    },
}

/// Report management subcommand enumeration
#[derive(Subcommand)]
pub enum ReportCommands {
    /// List reports
    List,
    /// View report
    Show {
        /// Report UUID
        uuid: String,
    },
    /// Delete report
    Delete {
        /// Report UUID
        uuid: String,
    },
}

// ── CLI 入口 ──────────────────────────────────────────────────────

/// Configure logging: output to both console and file
fn setup_logging() {
    // 获取日志目录
    let log_dir = flow_engine::storage::get_data_dir()
        .ok()
        .map(|d| d.join("logs"))
        .unwrap_or_default();

    // 确保日志目录存在
    let _ = std::fs::create_dir_all(&log_dir);

    // 生成日志文件名：mpe_YYYY-MM-DD_HH-MM-SS.log
    let log_file = log_dir.join(format!(
        "mpe_{}.log",
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
    ));

    // 尝试创建文件输出，Failed则仅输出到控制台
    let file_logger = fern::log_file(&log_file).ok();

    // 配置 fern 日志分发器
    let mut dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stderr());

    // 文件输出可选，Failed不影响 CLI 执行
    if let Some(file) = file_logger {
        dispatch = dispatch.chain(file);
    }

    let result = dispatch.apply();

    // 日志初始化Failed不影响 CLI 执行
    if result.is_ok() {
        log::info!("Log initialized: {:?}", log_file);
    }
}

/// CLI 异步主逻辑
async fn async_main() -> Result<(), CliError> {
    let cli = Cli::parse();

    // 创建取消信号通道（Ctrl+C 时触发）
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    // 启动 Ctrl+C 监听任务
    let ctrlc_cancel_tx = cancel_tx.clone();
    let _ctrlc_handle = tokio::spawn(async move {
        // 忽略第二次 Ctrl+C，防止强制中断清理过程
        let _ = tokio::signal::ctrl_c().await;
        let _ = ctrlc_cancel_tx.send(true);
    });

    // 按需构建共享节点注册表（内置节点 + 插件节点）：只有真正执行/验证/调试/
    // 压测/单节点命令需要；flow/report/plugin 管理命令不拉起插件进程。
    match cli.command {
        Commands::Run {
            flow,
            env,
            var,
            stream,
            no_report,
            report_format,
            report_file,
            summary,
        } => {
            let (registry, plugin_manager) = build_shared_registry().await;
            run::execute(
                &cli.format,
                &flow,
                env.as_deref(),
                &var,
                stream,
                no_report,
                &report_format,
                report_file.as_deref(),
                summary.as_deref(),
                registry,
                plugin_manager,
                cancel_rx,
            )
            .await
        }
        Commands::Serve {
            flow,
            port,
            env,
            var,
            graceful_timeout,
        } => {
            let (registry, _plugin_manager) = build_shared_registry().await;
            serve::execute(
                &cli.format,
                &flow,
                port,
                env,
                &var,
                graceful_timeout,
                registry,
                cancel_rx,
            )
            .await
        }
        Commands::Validate { flow } => {
            let (registry, _plugin_manager) = build_shared_registry().await;
            validate::execute(&cli.format, &flow, registry).await
        }
        Commands::Debug {
            flow,
            env,
            r#break,
            var,
        } => {
            let (registry, _plugin_manager) = build_shared_registry().await;
            debug_cmd::execute(
                &cli.format,
                &flow,
                env.as_deref(),
                &r#break,
                &var,
                registry,
                cancel_rx,
            )
            .await
        }
        Commands::Stress { command } => {
            let (registry, _plugin_manager) = build_shared_registry().await;
            stress::execute(&cli.format, command, registry, cancel_rx).await
        }
        Commands::Flow { command } => flow::execute(&cli.format, command).await,
        Commands::Env { command } => env::execute(&cli.format, command).await,
        Commands::RunNode {
            node_json,
            var,
            format,
        } => {
            let (registry, _plugin_manager) = build_shared_registry().await;
            run_node::execute(&format, &node_json, &var, registry).await
        }
        Commands::Report { command } => report::execute(&cli.format, command).await,
        #[cfg(feature = "plugin-manager")]
        Commands::Plugin { command } => plugin::execute(command).await,
        Commands::Open { path } => {
            let abs_path = std::fs::canonicalize(&path).unwrap_or_else(|_| {
                if path.is_absolute() {
                    path.clone()
                } else {
                    std::env::current_dir()
                        .map(|cwd| cwd.join(&path))
                        .unwrap_or(path.clone())
                }
            });
            let path_str = abs_path.to_string_lossy().to_string();

            if let Some(gui) = gui_bridge::find_running_gui().await {
                gui_bridge::send_gui_command(
                    &gui,
                    "/open",
                    &serde_json::json!({ "path": path_str }),
                )
                .await?;
                println!("[MPE] Command sent successfully. Waking up MPE desktop application.");
                Ok(())
            } else {
                println!(
                    "[MPE] No running GUI instance detected. Launching MPE desktop application..."
                );
                gui_bridge::launch_gui()?;
                Ok(())
            }
        }
        Commands::Ai { prompt, workspace } => {
            let target_ws = workspace.unwrap_or_else(|| PathBuf::from("."));
            let abs_ws = std::fs::canonicalize(&target_ws).unwrap_or_else(|_| {
                if target_ws.is_absolute() {
                    target_ws.clone()
                } else {
                    std::env::current_dir()
                        .map(|cwd| cwd.join(&target_ws))
                        .unwrap_or(target_ws.clone())
                }
            });
            let ws_str = abs_ws.to_string_lossy().to_string();

            if let Some(gui) = gui_bridge::find_running_gui().await {
                gui_bridge::send_gui_command(
                    &gui,
                    "/ai",
                    &serde_json::json!({ "workspace": ws_str, "prompt": prompt }),
                )
                .await?;
                println!("[MPE] Command sent successfully. Waking up MPE desktop application.");
                Ok(())
            } else {
                println!(
                    "[MPE] No running GUI instance detected. Launching MPE desktop application..."
                );
                gui_bridge::launch_gui()?;
                Ok(())
            }
        }
    }
}

/// 解析插件目录：`MPE_PLUGIN_DIR` 环境变量 > config.toml `[storage].plugins_dir`
/// > 默认数据目录 `plugins` > 兜底相对路径 `plugins`。
///
/// `AppConfig` 解析失败时降级为旧的 env → 默认目录解析。
#[cfg(feature = "plugin-manager")]
fn plugins_dir() -> PathBuf {
    crate::cli::config::AppConfig::load()
        .map(|config| config.plugins_dir)
        .unwrap_or_else(|_| {
            std::env::var_os("MPE_PLUGIN_DIR")
                .map(PathBuf::from)
                .or_else(|| {
                    flow_engine::storage::get_data_dir()
                        .ok()
                        .map(|d| d.join("plugins"))
                })
                .unwrap_or_else(|| PathBuf::from("plugins"))
        })
}

/// 插件管理器句柄；`plugin-manager` 特性关闭时退化为 `()`（最小 CLI 构建）。
#[cfg(feature = "plugin-manager")]
pub(crate) type PluginManagerHandle = Option<Arc<flow_engine::PluginManager>>;
#[cfg(not(feature = "plugin-manager"))]
pub(crate) type PluginManagerHandle = ();

/// 构建 CLI 共享节点注册表
///
/// 启用 `plugin-manager` 特性时扫描插件目录并登记插件节点（describe 拉起
/// 插件进程），同时返回插件管理器（用于 flowEnded 等 fire-and-forget 通知，
/// 插件进程可跨多次执行复用）；关闭时仅注册内置节点，管理器为 `None`。
/// 扫描失败仅告警，不影响启动。
async fn build_shared_registry() -> (
    Arc<flow_engine::registry::NodeRegistry>,
    PluginManagerHandle,
) {
    #[cfg(feature = "plugin-manager")]
    {
        use flow_engine::default_registry_with_plugins;
        use flow_engine::PluginManager;

        let default_timeout = std::env::var("MPE_DEFAULT_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&ms| ms > 0)
            .map(std::time::Duration::from_millis);
        let app_config = crate::cli::config::AppConfig::load().ok();
        let conn_config = app_config
            .map(|c| c.plugin_connection_config)
            .unwrap_or_default();
        let mut manager = PluginManager::new(plugins_dir()).with_connection_config(conn_config);
        if let Some(timeout) = default_timeout {
            manager = manager.with_default_execution_timeout(timeout);
        }
        manager.scan_and_describe().await;
        let manager = Arc::new(manager);
        let registry = Arc::new(default_registry_with_plugins(&manager));
        (registry, Some(manager))
    }
    #[cfg(not(feature = "plugin-manager"))]
    {
        (
            Arc::new(flow_engine::default_registry::default_registry()),
            (),
        )
    }
}

/// CLI main entry point (synchronous, returns exit code)
///
/// Called by the unified `main.rs` entry point. Returns process exit code:
/// - 0: Success
/// - 1: Execution error
/// - 2: Invalid arguments
pub fn run_cli() -> i32 {
    setup_logging();

    // 安装 rustls CryptoProvider（HTTP/gRPC TLS 需要）
    // 必须在任何 TLS 连接创建之前调用
    let _ = rustls::crypto::ring::default_provider().install_default();

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            output::output_error(format!("Failed to create Tokio runtime: {e}"));
            return output::EXIT_FAILURE;
        }
    };

    let result = rt.block_on(async_main());

    match result {
        Ok(()) => output::EXIT_SUCCESS,
        Err(CliError::InvalidArgs(msg)) => {
            output::output_error(msg);
            output::EXIT_INVALID_ARGS
        }
        Err(CliError::InfraTimeout(msg)) => {
            output::output_error(msg);
            output::EXIT_INFRA_TIMEOUT
        }
        Err(CliError::Config(msg)) => {
            output::output_error(format!("Configuration error: {}", msg));
            output::EXIT_INVALID_ARGS
        }
        Err(CliError::Io(msg)) => {
            output::output_error(format!("I/O error: {}", msg));
            output::EXIT_INVALID_ARGS
        }
        Err(CliError::Execution { message, data }) => {
            if message.contains("Failed to load environment")
                || message.contains("Failed to load flow")
                || message.contains("Environment file not found")
            {
                output::output_error_with_data(message, data);
                output::EXIT_INVALID_ARGS
            } else if is_infra_error(&message) {
                output::output_error_with_data(message, data);
                output::EXIT_INFRA_TIMEOUT
            } else {
                output::output_error_with_data(message, data);
                output::EXIT_FAILURE
            }
        }
    }
}
