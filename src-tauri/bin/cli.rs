//! CLI 入口点 - 流程执行命令行工具

use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::time::Instant;

use tokio::sync::mpsc;

// 导入类型和函数
use app_lib::{load_flow, ExecutionResult, FlowExecutor, NodeExecutionReport, FlowExecutionReport, save_report};
use app_lib::flow::Flow;
use app_lib::stress_test::{StressTestConfig, StressTestExecutor, StressTestEvent};

/// CLI 输出结果结构（与期望的 JSON 格式匹配）
#[derive(serde::Serialize)]
struct CliOutput {
    success: bool,
    data: Option<serde_json::Value>,
    error: Option<String>,
    execution_time: u64,
    node_reports: Vec<NodeExecutionReport>,
    flow_report: Option<FlowExecutionReport>,
}

fn print_help() {
    println!("多协议流程执行器 - CLI 工具");
    println!();
    println!("用法:");
    println!("  cli --execute --flow <path>           执行指定路径的流程");
    println!("  cli -e -f <path>                      执行指定路径的流程（短格式）");
    println!("  cli stress-test -f <path> [options]   执行压力测试");
    println!("  cli --help                            显示帮助信息");
    println!("  cli -h                                显示帮助信息（短格式）");
    println!();
    println!("参数:");
    println!("  -e, --execute                         执行流程");
    println!("  -f, --flow <path>                     流程文件路径");
    println!("  -h, --help                            显示帮助信息");
    println!();
    println!("压力测试参数:");
    println!("  stress-test                           执行压力测试");
    println!("  -c, --concurrent-users <num>          并发用户数 (默认: 50)");
    println!("  -r, --ramp-up <seconds>               预热时间/秒 (默认: 10)");
    println!("  -d, --duration <seconds>              持续时间/秒 (默认: 60)");
    println!("  -o, --output <path>                   输出报告文件路径");
    println!();
    println!("示例:");
    println!("  cli --execute --flow my-flow.json");
    println!("  cli -e -f /path/to/flow.json");
    println!("  cli stress-test -f flow.json -c 100 -r 20 -d 120");
}

/// CLI 命令
enum CliCommand {
    /// 显示帮助
    Help,
    /// 执行流程
    Execute { flow: PathBuf },
    /// 压力测试
    StressTest {
        flow: PathBuf,
        concurrent_users: u32,
        ramp_up_seconds: u32,
        duration_seconds: u32,
        output: Option<PathBuf>,
    },
}

fn parse_args(args: Vec<String>) -> Result<CliCommand, String> {
    let mut iter = args.iter().peekable();

    // 跳过程序名
    iter.next();

    // 检查第一个参数
    let first_arg = iter.peek().map(|s| s.as_str());

    match first_arg {
        Some("-h") | Some("--help") => {
            Ok(CliCommand::Help)
        }
        Some("stress-test") => {
            // 消费 stress-test
            iter.next();

            let mut flow_path: Option<PathBuf> = None;
            let mut concurrent_users: u32 = 50;
            let mut ramp_up_seconds: u32 = 10;
            let mut duration_seconds: u32 = 60;
            let mut output: Option<PathBuf> = None;

            while let Some(arg) = iter.next() {
                match arg.as_str() {
                    "-h" | "--help" => {
                        return Ok(CliCommand::Help);
                    }
                    "-f" | "--flow" => {
                        if let Some(path) = iter.next() {
                            flow_path = Some(PathBuf::from(path));
                        } else {
                            return Err("--flow 参数需要指定路径".to_string());
                        }
                    }
                    "-c" | "--concurrent-users" => {
                        if let Some(val) = iter.next() {
                            concurrent_users = val.parse().map_err(|_| {
                                format!("无效的并发用户数: {}", val)
                            })?;
                        } else {
                            return Err("--concurrent-users 参数需要指定数值".to_string());
                        }
                    }
                    "-r" | "--ramp-up" => {
                        if let Some(val) = iter.next() {
                            ramp_up_seconds = val.parse().map_err(|_| {
                                format!("无效的预热时间: {}", val)
                            })?;
                        } else {
                            return Err("--ramp-up 参数需要指定数值".to_string());
                        }
                    }
                    "-d" | "--duration" => {
                        if let Some(val) = iter.next() {
                            duration_seconds = val.parse().map_err(|_| {
                                format!("无效的持续时间: {}", val)
                            })?;
                        } else {
                            return Err("--duration 参数需要指定数值".to_string());
                        }
                    }
                    "-o" | "--output" => {
                        if let Some(path) = iter.next() {
                            output = Some(PathBuf::from(path));
                        } else {
                            return Err("--output 参数需要指定路径".to_string());
                        }
                    }
                    arg if arg.starts_with('-') => {
                        return Err(format!("未知选项: {}", arg));
                    }
                    _ => {
                        return Err(format!("未知参数: {}", arg));
                    }
                }
            }

            let flow = flow_path.ok_or_else(|| "--flow 参数是必需的".to_string())?;

            Ok(CliCommand::StressTest {
                flow,
                concurrent_users,
                ramp_up_seconds,
                duration_seconds,
                output,
            })
        }
        Some("-e") | Some("--execute") => {
            // 消费 -e/--execute
            iter.next();

            let mut flow_path = None;

            while let Some(arg) = iter.next() {
                match arg.as_str() {
                    "-f" | "--flow" => {
                        if let Some(path) = iter.next() {
                            flow_path = Some(PathBuf::from(path));
                        } else {
                            return Err("--flow 参数需要指定路径".to_string());
                        }
                    }
                    "-h" | "--help" => {
                        return Ok(CliCommand::Help);
                    }
                    arg if arg.starts_with('-') => {
                        return Err(format!("未知选项: {}", arg));
                    }
                    _ => {
                        return Err(format!("未知参数: {}", arg));
                    }
                }
            }

            let flow = flow_path.ok_or_else(|| "--flow 参数是必需的".to_string())?;
            Ok(CliCommand::Execute { flow })
        }
        None => {
            Ok(CliCommand::Help)
        }
        Some(arg) => {
            Err(format!("未知命令或选项: {}", arg))
        }
    }
}

/// 执行压力测试
async fn stress_test(
    flow_path: PathBuf,
    concurrent_users: u32,
    ramp_up_seconds: u32,
    duration_seconds: u32,
    output: Option<PathBuf>,
) -> Result<(), String> {
    // 1. 加载流程文件
    let flow_json = std::fs::read_to_string(&flow_path)
        .map_err(|e| format!("读取流程文件失败: {}", e))?;
    let flow: Flow = serde_json::from_str(&flow_json)
        .map_err(|e| format!("解析流程文件失败: {}", e))?;

    log::info!(
        "[压测] 启动压力测试: flow={}, concurrent={}, ramp_up={}s, duration={}s",
        flow.uuid,
        concurrent_users,
        ramp_up_seconds,
        duration_seconds
    );

    // 2. 创建配置
    let config = StressTestConfig {
        concurrent_users,
        ramp_up_seconds,
        duration_seconds,
        flow_uuid: flow.uuid.clone(),
    };

    // 3. 创建事件通道
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    // 4. 创建执行器
    let executor = StressTestExecutor::new(flow.clone(), config.clone(), event_tx);

    // 5. 启动执行器
    executor.start().await.map_err(|e| format!("启动压力测试失败: {}", e))?;

    // 6. 监听事件并输出 NDJSON
    let mut final_report: Option<app_lib::stress_test::StressTestReport> = None;

    while let Some(event) = event_rx.recv().await {
        let json = serde_json::to_string(&event)
            .map_err(|e| format!("序列化事件失败: {}", e))?;
        println!("{}", json);

        // 刷新 stdout 以确保立即输出
        let _ = std::io::stdout().flush();

        // 捕获最终报告
        if let StressTestEvent::Ended { report } = &event {
            final_report = Some(report.clone());
        }
    }

    // 7. 如果指定了输出路径，保存报告
    if let Some(output_path) = output {
        if let Some(report) = final_report {
            let report_json = serde_json::to_string_pretty(&report)
                .map_err(|e| format!("序列化报告失败: {}", e))?;
            std::fs::write(&output_path, &report_json)
                .map_err(|e| format!("写入报告文件失败: {}", e))?;
            eprintln!("报告已保存到: {}", output_path.display());
        }
    }

    Ok(())
}

async fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    // 解析命令行参数
    let command = parse_args(args)?;

    match command {
        CliCommand::Help => {
            print_help();
            Ok(())
        }
        CliCommand::Execute { flow } => {
            execute_flow_command(flow).await
        }
        CliCommand::StressTest {
            flow,
            concurrent_users,
            ramp_up_seconds,
            duration_seconds,
            output,
        } => {
            stress_test(flow, concurrent_users, ramp_up_seconds, duration_seconds, output).await
        }
    }
}

/// 执行流程命令
async fn execute_flow_command(flow_path: PathBuf) -> Result<(), String> {
    // 开始计时
    let start_time = Instant::now();

    // 加载流程
    let flow = load_flow(flow_path.to_str().unwrap_or(""))
        .map_err(|e| format!("加载流程失败: {}", e))?;

    // 执行流程（使用 FlowExecutor，不重新实现）
    let executor = FlowExecutor::new()
        .with_flow_file_path(flow_path.clone());
    let result: ExecutionResult = executor.execute_flow(&flow, None, None).await;

    // 计算总执行时间
    let execution_time = start_time.elapsed().as_millis() as u64;

    // 将 ExecutionResult 转换为 CLI 输出格式
    let cli_output = CliOutput {
        success: result.success,
        data: result.data,
        error: result.error,
        execution_time,
        node_reports: result.node_reports,
        flow_report: result.flow_report,
    };

    // 序列化为 JSON 并输出到 stdout
    let json_output =
        serde_json::to_string_pretty(&cli_output).map_err(|e| format!("序列化输出失败: {}", e))?;

    println!("{}", json_output);

    // 保存执行报告到 reports 目录
    if let Some(ref flow_report) = cli_output.flow_report {
        let reports_dir = std::env::current_dir()
            .map_err(|e| format!("获取当前目录失败: {}", e))?
            .join("reports");
        
        save_report(flow_report, &reports_dir)
            .map_err(|e| format!("保存报告失败: {}", e))?;
        
        eprintln!("报告已保存到: reports/{}.json", flow_report.uuid);
    }

    // 如果执行失败，返回错误
    if !cli_output.success {
        // 输出错误信息到 stderr
        if let Some(error) = &cli_output.error {
            eprintln!("错误: {}", error);
        }
        process::exit(1);
    }

    Ok(())
}

/// 配置日志：同时输出到控制台和文件
fn setup_logging() {
    // 获取日志目录
    let log_dir = app_lib::get_data_dir()
        .ok()
        .map(|d| d.join("logs"))
        .unwrap_or_default();
    
    // 确保日志目录存在
    let _ = std::fs::create_dir_all(&log_dir);
    
    // 生成日志文件名：cli_YYYY-MM-DD_HH-MM-SS.log
    let log_file = log_dir.join(format!(
        "cli_{}.log",
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

    // 创建 Tokio 运行时
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    // 在运行时中运行 async main 逻辑
    if let Err(error) = rt.block_on(run()) {
        // 输出错误信息到 stderr
        eprintln!("错误: {}", error);
        // 使用非零退出码表示失败
        process::exit(1);
    }
}
