//! CLI 入口点 - 流程执行命令行工具

use std::env;
use std::path::PathBuf;
use std::process;
use std::time::Instant;

// 导入类型和函数
use app_lib::{load_flow, report, ExecutionResult, FlowExecutor};
use app_lib::save_report;
use report::{NodeExecutionReport, FlowExecutionReport};

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
    println!("  cli --execute --flow <path>  执行指定路径的流程");
    println!("  cli -e -f <path>             执行指定路径的流程（短格式）");
    println!("  cli --help                   显示帮助信息");
    println!("  cli -h                       显示帮助信息（短格式）");
    println!();
    println!("参数:");
    println!("  -e, --execute                执行流程");
    println!("  -f, --flow <path>            流程文件路径");
    println!("  -h, --help                   显示帮助信息");
    println!();
    println!("示例:");
    println!("  cli --execute --flow my-flow.json");
    println!("  cli -e -f /path/to/flow.json");
}

fn parse_args(args: Vec<String>) -> Result<Option<String>, String> {
    let mut execute = false;
    let mut flow_path = None;
    let mut help = false;

    let mut iter = args.iter().peekable();

    // 跳过程序名
    iter.next();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-e" | "--execute" => {
                execute = true;
            }
            "-f" | "--flow" => {
                if let Some(path) = iter.next() {
                    flow_path = Some(path.clone());
                } else {
                    return Err("--flow 参数需要指定路径".to_string());
                }
            }
            "-h" | "--help" => {
                help = true;
            }
            arg if arg.starts_with('-') => {
                return Err(format!("未知选项: {}", arg));
            }
            _ => {
                return Err(format!("未知参数: {}", arg));
            }
        }
    }

    // 如果请求帮助，返回 Ok(None) 表示只显示帮助
    if help {
        return Ok(None);
    }

    // 验证必选参数
    if execute && flow_path.is_none() {
        return Err("--execute 需要 --flow 参数".to_string());
    }

    if !execute && flow_path.is_some() {
        return Err("--flow 需要 --execute 参数".to_string());
    }

    Ok(flow_path)
}

async fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    // 解析命令行参数
    let flow_path = match parse_args(args)? {
        Some(path) => path,
        None => {
            // 显示帮助信息并退出成功
            print_help();
            return Ok(());
        }
    };

    // 开始计时
    let start_time = Instant::now();

    // 加载流程
    let flow = load_flow(&flow_path).map_err(|e| format!("加载流程失败: {}", e))?;

    // 执行流程（使用 FlowExecutor，不重新实现）
    let executor = FlowExecutor::new()
        .with_flow_file_path(PathBuf::from(&flow_path));
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
        
        println!("报告已保存到: reports/{}.json", flow_report.uuid);
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

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

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
