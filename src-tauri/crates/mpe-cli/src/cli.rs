//! mpe - Multi-Protocol Executor CLI Module
//!
//! Flow engine command-line tool, invoked by the unified `main.rs` entry point.

pub mod config;
pub mod output;
pub mod run;
pub mod validate;
pub mod flow;
pub mod flow_edit;
pub mod report;
pub mod stress;
pub mod debug_cmd;
pub mod run_node;

use clap::{Parser, Subcommand};

// ── CLI 错误类型 ──────────────────────────────────────────────────

/// CLI error type
#[derive(Debug)]
pub enum CliError {
    /// Configuration error
    Config(String),
    /// Execution error
    Execution(String),
    /// Invalid arguments error
    InvalidArgs(String),
    /// I/O error
    Io(String),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Config(msg) => write!(f, "Configuration error: {}", msg),
            CliError::Execution(msg) => write!(f, "Execution error: {}", msg),
            CliError::InvalidArgs(msg) => write!(f, "Invalid arguments: {}", msg),
            CliError::Io(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e.to_string())
    }
}

/// Parse --var key=value pair into (String, String) tuple
fn parse_var(s: &str) -> Result<(String, String), String> {
    let (key, val) = s.split_once('=').ok_or_else(|| {
        format!("--var format error (expected key=value): {}", s)
    })?;
    Ok((key.to_string(), val.to_string()))
}

// ── Clap 定义 ─────────────────────────────────────────────────────

/// Multi-Protocol Executor — AI-ready flow engine CLI
#[derive(Parser)]
#[command(name = "mpe", version, about = "Multi-Protocol Flow Orchestration & Execution Engine")]
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
        /// Flow file path (.mpe)
        #[arg(short, long)]
        flow: String,
        /// Variable overrides (key=value)
        #[arg(long, value_name = "KEY=VALUE")]
        var: Vec<String>,
        /// Enable NDJSON streaming output
        #[arg(long)]
        stream: bool,
        /// Do not save execution report
        #[arg(long)]
        no_report: bool,
    },
    /// Validate a flow
    Validate {
        /// Flow file path (.mpe)
        #[arg(short, long)]
        flow: String,
    },
    /// Debug a flow (NDJSON bidirectional protocol)
    Debug {
        /// Flow file path (.mpe)
        #[arg(short, long)]
        flow: String,
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
    /// Report management
    Report {
        /// Report management subcommand
        #[command(subcommand)]
        command: ReportCommands,
    },
    /// Execute a single protocol node for connectivity verification
    RunNode {
        /// JSON configuration for the node
        /// (e.g. '{"type":"redis_connect","host":"127.0.0.1","port":6379}')
        node_json: String,

        /// Flow variables (key=value or key=json)
        #[arg(short, long = "var", value_parser = parse_var)]
        var: Vec<(String, String)>,

        /// Output format (json)
        #[arg(short, long, default_value = "json")]
        format: String,
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
    /// Set a variable
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
    /// Remove a variable
    VariableRemove {
        /// Flow UUID
        #[arg(long)]
        flow: String,
        /// Variable name
        #[arg(long)]
        key: String,
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

    // 尝试创建文件输出，失败则仅输出到控制台
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

    // 文件输出可选，失败不影响 CLI 执行
    if let Some(file) = file_logger {
        dispatch = dispatch.chain(file);
    }

    let result = dispatch.apply();

    // 日志初始化失败不影响 CLI 执行
    if result.is_ok() {
        log::info!("Log initialized: {:?}", log_file);
    }
}

/// CLI asynchronous main logic
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

    match cli.command {
        Commands::Run {
            flow,
            var,
            stream,
            no_report,
        } => run::execute(&cli.format, &flow, &var, stream, no_report, cancel_rx).await,
        Commands::Validate { flow } => validate::execute(&cli.format, &flow).await,
        Commands::Debug {
            flow,
            r#break,
            var,
        } => debug_cmd::execute(&cli.format, &flow, &r#break, &var, cancel_rx).await,
        Commands::Stress { command } => stress::execute(&cli.format, command, cancel_rx).await,
        Commands::Flow { command } => flow::execute(&cli.format, command).await,
        Commands::RunNode { node_json, var, format } => {
            run_node::execute(&format, &node_json, &var).await
        },
        Commands::Report { command } => report::execute(&cli.format, command).await,
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

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    let result = rt.block_on(async_main());

    match result {
        Ok(()) => output::EXIT_SUCCESS,
        Err(CliError::InvalidArgs(_)) => {
            output::output_error(result.unwrap_err().to_string());
            output::EXIT_INVALID_ARGS
        }
        Err(e) => {
            output::output_error(e.to_string());
            output::EXIT_FAILURE
        }
    }
}
