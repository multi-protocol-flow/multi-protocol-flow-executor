//! Cross-platform process monitoring and termination.
#![allow(missing_docs)]

use std::fs;
use std::time::{Duration, Instant};

/// Check if a process is still alive, excluding zombie processes.
pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }

    if cfg!(target_os = "windows") {
        let output = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .output();
        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            return s.contains(&pid.to_string());
        }
        false
    } else {
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = fs::read_to_string(format!("/proc/{pid}/status")) {
                for line in status.lines() {
                    if line.starts_with("State:") && line.contains('Z') {
                        // Zombie process: already terminated
                        return false;
                    }
                }
            }
        }

        match std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
        {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
}

/// Terminate a process gracefully.
///
/// On Windows, invokes `taskkill /PID {pid} /T /F`.
/// On Unix, sends SIGTERM first; if the process does not terminate within
/// 5 seconds, falls back to SIGKILL.
pub fn terminate_process(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }

    if cfg!(target_os = "windows") {
        std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        // Send SIGTERM (-15)
        let term_status = std::process::Command::new("kill")
            .args(["-15", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if !term_status {
            return false;
        }

        // Wait up to 5 seconds for clean exit
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if !is_process_alive(pid) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        // Fallback to SIGKILL (-9) if still alive
        if is_process_alive(pid) {
            let _ = std::process::Command::new("kill")
                .args(["-9", &pid.to_string()])
                .status();
        }

        !is_process_alive(pid)
    }
}
