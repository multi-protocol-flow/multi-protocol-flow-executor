//! Daemon process spawning, I/O redirection, and early crash detection.
#![allow(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// Spawn a background daemon child process with protected stdio.
///
/// Directs stdin to `Stdio::null()`. Directs stdout and stderr to `{log_dir}/{id}.log`.
/// If log file creation fails in restricted environments, falls back to `Stdio::null()`
/// so process creation is not blocked.
pub fn spawn_daemon(
    cmd: &mut Command,
    id: &str,
    log_dir: &Path,
) -> Result<(Child, PathBuf), String> {
    let _ = fs::create_dir_all(log_dir);
    let log_path = log_dir.join(format!("{id}.log"));

    let log_file_res = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path);

    cmd.stdin(Stdio::null());

    match log_file_res {
        Ok(log_file) => match log_file.try_clone() {
            Ok(err_file) => {
                cmd.stdout(Stdio::from(log_file));
                cmd.stderr(Stdio::from(err_file));
            }
            Err(_) => {
                cmd.stdout(Stdio::from(log_file));
                cmd.stderr(Stdio::null());
            }
        },
        Err(e) => {
            log::warn!(
                "[Supervisor Daemon] Failed to open log file {}: {e}. Falling back to null stdio.",
                log_path.display()
            );
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
        }
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn daemon process: {e}"))?;

    Ok((child, log_path))
}

/// Check if child has crashed prematurely, extracting log diagnostics if available.
pub fn check_early_crash(child: &mut Child, log_path: &Path) -> Result<(), String> {
    if let Ok(Some(status)) = child.try_wait() {
        let err_msg = fs::read_to_string(log_path).unwrap_or_default();
        let reason = if err_msg.trim().is_empty() {
            format!("exited with code {status}")
        } else {
            err_msg.trim().to_string()
        };
        return Err(format!("Process failed to start: {reason}"));
    }
    Ok(())
}
