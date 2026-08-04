//! Child process spawning hygiene.
//!
//! Responsibilities:
//! - **stderr draining** — a spawned task reads the plugin's stderr in
//!   blocks so the 64 KB pipe buffer never fills and stalls the plugin;
//! - **Windows Job Object** — `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` so the
//!   host's death kills the plugin and its grandchildren (the only module
//!   that uses `unsafe`; the crate-level override is confined here);
//! - **handle-inheritance hygiene** — `CREATE_NO_WINDOW` (Windows) plus
//!   std's default CLOEXEC semantics so grandchildren do not inherit the
//!   stdout pipe (which would prevent EOF);
//! - **interpreter env** — `PYTHONUTF8=1` / `PYTHONIOENCODING=utf-8` so
//!   Python plugins speak UTF-8 over a binary pipe.
//!
//! Known limitation (documented, accepted): a plugin compiled with
//! `windows_subsystem = "windows"` has no usable stdio handles; such plugins
//! cannot use this transport.

use std::collections::HashMap;

use mpe_protocol::manifest::PluginEntry;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};

use crate::error::PluginError;

/// Interpreter env forcing UTF-8 I/O (Python).
pub const PYTHONUTF8: &str = "PYTHONUTF8";
/// Interpreter env forcing UTF-8 stdio encoding (Python).
pub const PYTHONIOENCODING: &str = "PYTHONIOENCODING";

/// A spawned plugin process with its lifetime hygiene applied.
pub struct PluginProcess {
    child: Child,
    /// Windows Job Object handle kept alive for the process's lifetime;
    /// dropped (on `into_parts`) the OS kills the job's processes.
    #[cfg(windows)]
    job: Option<crate::process::win::JobObject>,
}

/// Keeps the plugin's Windows Job Object alive after the child has been
/// handed to the transport. Dropped when the connection is done with the
/// process — closing the last job handle triggers the kill-on-close.
pub struct PluginProcessGuard {
    /// RAII job handle; never read, only kept alive for its Drop semantics.
    #[cfg(windows)]
    #[allow(dead_code)]
    job: Option<crate::process::win::JobObject>,
}

impl PluginProcess {
    /// Splits into the child (for the transport) and a guard keeping the
    /// Job Object alive.
    pub fn into_parts(self) -> (Child, PluginProcessGuard) {
        (
            self.child,
            PluginProcessGuard {
                #[cfg(windows)]
                job: self.job,
            },
        )
    }

    /// The child's OS process id, if spawned.
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }
}

/// Spawns a plugin process with spawn hygiene applied.
///
/// - stdin/stdout/stderr are piped; stderr is drained by a spawned task.
/// - `PYTHONUTF8=1` and `PYTHONIOENCODING=utf-8` are injected, then
///   `env_overrides` (e.g. the manifest's `env` map) on top.
/// - On Windows: `CREATE_NO_WINDOW` and, best-effort, a kill-on-close Job
///   Object (assigning can fail if the process is already in a job — that is
///   logged, not fatal).
pub fn spawn_plugin(
    entry: &PluginEntry,
    env_overrides: &HashMap<String, String>,
) -> Result<PluginProcess, PluginError> {
    let mut command = Command::new(&entry.command);
    command
        .args(&entry.args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    // Interpreter UTF-8 env (harmless for non-interpreters).
    command.env(PYTHONUTF8, "1");
    command.env(PYTHONIOENCODING, "utf-8");
    for (key, value) in env_overrides {
        command.env(key, value);
    }

    // Handle-inheritance hygiene: no console window, stdio pipes stay
    // non-inheritable for grandchildren (std marks them CLOEXEC on unix;
    // on Windows std only makes the piped stdio handles inheritable).
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .map_err(|e| PluginError::SpawnFailed(e.to_string()))?;

    // Drain stderr from the moment of spawn: a 64 KB pipe buffer stalls the
    // plugin forever if nobody reads it.
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(stderr_drain_task(stderr));
    }

    // Windows Job Object: zombie prevention + kill the whole tree with the
    // host. Best-effort — some environments already hold the process in a
    // job and refuse the assignment.
    #[cfg(windows)]
    let job = match child.id() {
        Some(pid) => match crate::process::win::assign_job(pid) {
            Ok(job) => Some(job),
            Err(e) => {
                log::warn!("could not assign plugin to kill-on-close job: {e}");
                None
            }
        },
        None => None,
    };

    Ok(PluginProcess {
        child,
        #[cfg(windows)]
        job,
    })
}

/// Reads the plugin's stderr in blocks and forwards it to the host log so
/// the pipe never fills (a full pipe stalls the plugin on its next write).
async fn stderr_drain_task(mut stderr: tokio::process::ChildStderr) {
    let mut buf = [0u8; 8192];
    loop {
        match stderr.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => log::debug!(
                "[plugin-stderr] {}",
                String::from_utf8_lossy(&buf[..n]).trim_end()
            ),
            Err(e) => {
                log::warn!("plugin stderr read failed: {e}");
                break;
            }
        }
    }
}

/// Windows Job Object FFI (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`).
///
/// This module is the sole user of `unsafe` in the crate; the crate-level
/// `unsafe_code = "allow"` override exists for it.
#[cfg(windows)]
pub(crate) mod win {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
        JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };

    /// RAII handle to a Windows Job Object; closing the last handle to the
    /// job terminates every process assigned to it (kill-on-job-close).
    pub struct JobObject(HANDLE);

    // SAFETY: HANDLE is an opaque pointer; moving it between threads and
    // dropping it on any thread is safe (`CloseHandle` is thread-safe and the
    // handle has no thread affinity).
    unsafe impl Send for JobObject {}
    // SAFETY: as for Send — the handle may be shared and closed from any
    // thread.
    unsafe impl Sync for JobObject {}

    impl Drop for JobObject {
        fn drop(&mut self) {
            // SAFETY: CloseHandle on a valid handle returned by the OS.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    /// Creates a kill-on-close job and assigns the process with the given
    /// pid to it.
    ///
    /// # Safety
    ///
    /// Uses the Windows Job Objects API; all raw handles are managed by RAII
    /// wrappers and closed exactly once.
    pub fn assign_job(pid: u32) -> Result<JobObject, String> {
        // SAFETY: CreateJobObjectW with null security attributes and name.
        let job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(|e| e.to_string())?;
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = Default::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let info_ptr =
            (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast::<core::ffi::c_void>();
        let info_len = std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32;
        // SAFETY: SetInformationJobObject with a valid, correctly-sized info
        // struct for the ExtendedLimitInformation class.
        unsafe {
            SetInformationJobObject(job, JobObjectExtendedLimitInformation, info_ptr, info_len)
                .map_err(|e| {
                    let _ = CloseHandle(job);
                    format!("set job limits: {e}")
                })?;
        }
        let process_handle = unsafe {
            OpenProcess(
                PROCESS_SET_QUOTA | PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                pid,
            )
        }
        .map_err(|e| {
            // SAFETY: close the job handle on the error path.
            unsafe {
                let _ = CloseHandle(job);
            }
            format!("open plugin process: {e}")
        })?;
        // SAFETY: AssignProcessToJobObject with a job and a process handle;
        // the process handle is closed below.
        unsafe { AssignProcessToJobObject(job, process_handle) }.map_err(|e| {
            // SAFETY: close both handles on the error path.
            unsafe {
                let _ = CloseHandle(job);
                let _ = CloseHandle(process_handle);
            }
            format!("assign plugin to job: {e}")
        })?;
        // SAFETY: the process handle is no longer needed; the job keeps the
        // process alive.
        unsafe {
            let _ = CloseHandle(process_handle);
        }
        Ok(JobObject(job))
    }
}
