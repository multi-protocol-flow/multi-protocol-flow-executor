//! mpe - Multi-Protocol Executor CLI
//!
//! AI 可托管的流程引擎命令行工具。

#[path = "mpe/config.rs"]
mod config;
#[path = "mpe/output.rs"]
mod output;
#[path = "mpe/run.rs"]
mod run;
#[path = "mpe/validate.rs"]
mod validate;
#[path = "mpe/flow.rs"]
mod flow;
#[path = "mpe/flow_edit.rs"]
mod flow_edit;
#[path = "mpe/report.rs"]
mod report;
#[path = "mpe/stress.rs"]
mod stress;
#[path = "mpe/debug_cmd.rs"]
mod debug_cmd;

use std::process;
use clap::{Parser, Subcommand};

// ── CLI 错误类型 ──────────────────────────────────────────────────

/// CLI 错误类型
#[derive(Debug)]
enum CliError {
    /// 配置错误
    Config(String),
    /// 执行错误
    Execution(String),
    /// 参数错误
    InvalidArgs(String),
    /// IO 错误
    Io(String),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Config(msg) => write!(f, "配置错误: {}", msg),
            CliError::Execution(msg) => write!(f, "执行错误: {}", msg),
            CliError::InvalidArgs(msg) => write!(f, "参数错误: {}", msg),
            CliError::Io(msg) => write!(f, "IO 错误: {}", msg),
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e.to_string())
    }
}

// ── Clap 定义 ─────────────────────────────────────────────────────

/// Multi-Protocol Executor — AI 可托管的流程引擎 CLI
#[derive(Parser)]
#[command(name = "mpe", version, about = "多协议流程编排与执行引擎")]
struct Cli {
    /// 输出格式 (json 或 table)
    #[arg(long, global = true, default_value = "json")]
    format: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 执行流程
    Run {
        /// 流程文件路径 (.mpe)
        #[arg(short, long)]
        flow: String,
        /// 变量覆盖 (key=value)
        #[arg(long, value_name = "KEY=VALUE")]
        var: Vec<String>,
        /// 启用 NDJSON 流式输出
        #[arg(long)]
        stream: bool,
        /// 不保存执行报告
        #[arg(long)]
        no_report: bool,
    },
    /// 验证流程
    Validate {
        /// 流程文件路径 (.mpe)
        #[arg(short, long)]
        flow: String,
    },
    /// 调试流程 (NDJSON 双向协议)
    Debug {
        /// 流程文件路径 (.mpe)
        #[arg(short, long)]
        flow: String,
        /// 断点节点 UUID
        #[arg(long)]
        r#break: Vec<String>,
        /// 变量覆盖 (key=value)
        #[arg(long, value_name = "KEY=VALUE")]
        var: Vec<String>,
    },
    /// 压力测试
    Stress {
        #[command(subcommand)]
        command: StressCommands,
    },
    /// 流程管理
    Flow {
        #[command(subcommand)]
        command: FlowCommands,
    },
    /// 报告管理
    Report {
        #[command(subcommand)]
        command: ReportCommands,
    },
}

#[derive(Subcommand)]
enum StressCommands {
    /// 执行压力测试
    Run {
        /// 流程文件路径 (.mpe)
        #[arg(short, long)]
        flow: String,
        /// 并发用户数
        #[arg(short, long, default_value_t = 50)]
        concurrent_users: u32,
        /// 预热时间/秒
        #[arg(short, long, default_value_t = 10)]
        ramp_up: u32,
        /// 持续时间/秒
        #[arg(short, long, default_value_t = 60)]
        duration: u32,
        /// 输出报告文件路径
        #[arg(short, long)]
        output: Option<String>,
    },
    /// 压测报告管理
    Report {
        #[command(subcommand)]
        command: StressReportCommands,
    },
}

#[derive(Subcommand)]
enum StressReportCommands {
    /// 列出压测报告
    List,
    /// 查看压测报告
    Show {
        /// 报告文件路径
        file_path: String,
    },
    /// 删除压测报告
    Delete {
        /// 报告文件路径
        file_path: String,
    },
}

#[derive(Subcommand)]
enum FlowCommands {
    /// 列出流程
    List {
        /// 搜索关键词
        #[arg(long)]
        search: Option<String>,
        /// 排序方式 (name/date)
        #[arg(long, default_value = "date")]
        sort: String,
    },
    /// 查看流程详情
    Show {
        /// 流程 UUID 或文件路径
        uuid_or_path: String,
    },
    /// 创建流程
    Create {
        /// 流程名称
        #[arg(long)]
        name: String,
    },
    /// 复制流程
    Copy {
        /// 流程 UUID
        uuid: String,
    },
    /// 删除流程
    Delete {
        /// 流程 UUID
        uuid: String,
    },
    /// 导入流程
    Import {
        /// 流程文件路径
        #[arg(short, long)]
        flow: String,
        /// 流程名称
        #[arg(long)]
        name: String,
    },
    /// 编辑流程
    Edit {
        #[command(subcommand)]
        command: FlowEditCommands,
    },
}

#[derive(Subcommand)]
enum FlowEditCommands {
    /// 添加节点
    NodeAdd {
        /// 流程 UUID
        #[arg(long)]
        flow: String,
        /// 节点类型
        #[arg(long)]
        r#type: String,
        /// 节点名称
        #[arg(long)]
        name: String,
        /// 节点配置 (JSON 字符串)
        #[arg(long)]
        config: Option<String>,
    },
    /// 移除节点
    NodeRemove {
        /// 流程 UUID
        #[arg(long)]
        flow: String,
        /// 节点 UUID
        #[arg(long)]
        node: String,
    },
    /// 更新节点
    NodeUpdate {
        /// 流程 UUID
        #[arg(long)]
        flow: String,
        /// 节点 UUID
        #[arg(long)]
        node: String,
        /// 节点配置 (JSON 字符串)
        #[arg(long)]
        config: Option<String>,
    },
    /// 添加连接
    ConnectionAdd {
        /// 流程 UUID
        #[arg(long)]
        flow: String,
        /// 源节点 UUID
        #[arg(long)]
        source: String,
        /// 目标节点 UUID
        #[arg(long)]
        target: String,
        /// 源端口
        #[arg(long, default_value = "output")]
        source_port: String,
        /// 目标端口
        #[arg(long, default_value = "input")]
        target_port: String,
    },
    /// 移除连接
    ConnectionRemove {
        /// 流程 UUID
        #[arg(long)]
        flow: String,
        /// 连接 ID
        #[arg(long)]
        connection: String,
    },
    /// 设置变量
    VariableSet {
        /// 流程 UUID
        #[arg(long)]
        flow: String,
        /// 变量名
        #[arg(long)]
        key: String,
        /// 变量值
        #[arg(long)]
        value: String,
    },
    /// 移除变量
    VariableRemove {
        /// 流程 UUID
        #[arg(long)]
        flow: String,
        /// 变量名
        #[arg(long)]
        key: String,
    },
}

#[derive(Subcommand)]
enum ReportCommands {
    /// 列出报告
    List,
    /// 查看报告
    Show {
        /// 报告 UUID
        uuid: String,
    },
    /// 删除报告
    Delete {
        /// 报告 UUID
        uuid: String,
    },
}

// ── 入口 ──────────────────────────────────────────────────────────

/// 配置日志：同时输出到控制台和文件
fn setup_logging() {
    // 获取日志目录
    let log_dir = app_lib::get_data_dir()
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
        log::info!("日志已初始化: {:?}", log_file);
    }
}

fn main() {
    setup_logging();

    // 安装 rustls CryptoProvider（HTTP/gRPC TLS 需要）
    // 必须在任何 TLS 连接创建之前调用
    let _ = rustls::crypto::ring::default_provider().install_default();

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    let result = rt.block_on(async_main());

    let exit_code = match result {
        Ok(()) => output::EXIT_SUCCESS,
        Err(CliError::InvalidArgs(_)) => {
            output::output_error(result.unwrap_err().to_string());
            output::EXIT_INVALID_ARGS
        }
        Err(e) => {
            output::output_error(e.to_string());
            output::EXIT_FAILURE
        }
    };

    process::exit(exit_code);
}

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
        Commands::Report { command } => report::execute(&cli.format, command).await,
    }
}
