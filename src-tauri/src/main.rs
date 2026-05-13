//! Multi-Protocol Flow Executor 统一入口
//!
//! **无参数启动** → GUI 模式（Tauri 桌面应用）
//! **有参数启动** → CLI 模式（mpe 命令行工具）
//!
//! 用法:
//!   mpe.exe                  # 打开桌面 GUI
//!   mpe.exe run -f flow.mpe  # CLI 执行流程
//!   mpe.exe --help           # 查看 CLI 帮助

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        // 有命令行参数 → CLI 模式
        let exit_code = app_lib::cli::run_cli();
        std::process::exit(exit_code);
    } else {
        // 无参数 → GUI 模式（Tauri 桌面应用）
        // Windows 上分离控制台，使终端关闭不影响应用
        #[cfg(target_os = "windows")]
        unsafe {
            windows::Win32::System::Console::FreeConsole();
        }
        app_lib::run();
    }
}
