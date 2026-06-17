//! mpe-cli — Multi-Protocol Executor CLI
//!
//! Standalone CLI binary without Tauri/GUI dependencies.
//! Built for Linux servers, CI/CD, and Docker environments.

fn main() {
    let exit_code = mpe_cli::run_cli();
    std::process::exit(exit_code);
}
