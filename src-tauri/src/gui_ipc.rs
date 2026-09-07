//! GUI 实例本地 IPC 监听器 (Local Loopback HTTP Bridge)
//!
//! 在 Tauri 应用启动时绑定 127.0.0.1:0 随机空闲端口，
//! 并将 `{ pid, port, auth_token, version }` 写入运行时元数据文件。
//! 接收来自 CLI (`mpe open` / `mpe ai`) 的远程调用，激活前台窗口并广播对应操作事件。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use uuid::Uuid;

/// GUI 实例运行时元数据
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuiInstanceInfo {
    /// 运行进程 PID
    pub pid: u32,
    /// 监听的 Loopback 端口号
    pub port: u16,
    /// 随机生成的认证 Token
    pub auth_token: String,
    /// 应用版本号
    pub version: String,
}

/// 打开工作区请求体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWorkspacePayload {
    /// 要打开或关联的项目路径
    pub path: String,
}

/// 触发 AI 会话请求体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerAiPayload {
    /// 工作区路径
    pub workspace: String,
    /// 用户指令提示词
    pub prompt: String,
}

/// 获取 GUI 运行时实例元数据文件存储路径
pub fn get_gui_instance_path() -> PathBuf {
    if let Ok(dir) = std::env::var("MPE_HOME") {
        return PathBuf::from(dir).join("gui-instance.json");
    }
    if cfg!(target_os = "windows") {
        if let Ok(app_data) = std::env::var("APPDATA") {
            return PathBuf::from(app_data).join("mpe").join("gui-instance.json");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".mpe").join("gui-instance.json");
    }
    PathBuf::from(".mpe").join("gui-instance.json")
}

/// 清理当前实例写入的元数据文件
pub fn cleanup_gui_instance_file() {
    let path = get_gui_instance_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(info) = serde_json::from_str::<GuiInstanceInfo>(&content) {
            if info.pid == std::process::id() {
                let _ = std::fs::remove_file(&path);
                log::info!("[GUI IPC] Cleaned up instance file at {}", path.display());
            }
        }
    }
}

/// 在后台异步任务中启动 Local Loopback HTTP 服务
pub fn start_gui_ipc_server(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_server_loop(app).await {
            log::error!("[GUI IPC] Server loop failed: {e}");
        }
    });
}

async fn run_server_loop(app: tauri::AppHandle) -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind local loopback TCP: {e}"))?;

    let local_addr = listener
        .local_addr()
        .map_err(|e| format!("Failed to get local addr: {e}"))?;
    let port = local_addr.port();
    let auth_token = Uuid::new_v4().to_string();
    let pid = std::process::id();
    let version = env!("CARGO_PKG_VERSION").to_string();

    let info = GuiInstanceInfo {
        pid,
        port,
        auth_token: auth_token.clone(),
        version,
    };

    let instance_path = get_gui_instance_path();
    if let Some(parent) = instance_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            log::warn!("[GUI IPC] Failed to create dir {}: {e}", parent.display());
        }
    }

    let json_bytes = serde_json::to_vec_pretty(&info)
        .map_err(|e| format!("Failed to serialize instance info: {e}"))?;
    std::fs::write(&instance_path, json_bytes)
        .map_err(|e| format!("Failed to write instance info to {}: {e}", instance_path.display()))?;

    log::info!(
        "[GUI IPC] Loopback bridge listening on 127.0.0.1:{}, metadata written to {}",
        port,
        instance_path.display()
    );

    let auth_token = Arc::new(auth_token);

    loop {
        let (mut socket, _) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                log::warn!("[GUI IPC] Accept error: {e}");
                continue;
            }
        };

        let app_handle = app.clone();
        let expected_token = Arc::clone(&auth_token);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 8192];
            let mut total_read = 0;

            // 读取 HTTP 请求头
            let (method, path, headers_end, content_length) = loop {
                let n = match socket.read(&mut buf[total_read..]).await {
                    Ok(0) => return,
                    Ok(n) => n,
                    Err(_) => return,
                };
                total_read += n;

                if let Some(pos) = find_subsequence(&buf[..total_read], b"\r\n\r\n") {
                    let header_str = String::from_utf8_lossy(&buf[..pos]);
                    let mut lines = header_str.lines();
                    let request_line = lines.next().unwrap_or("");
                    let mut parts = request_line.split_whitespace();
                    let method = parts.next().unwrap_or("").to_uppercase();
                    let path = parts.next().unwrap_or("").to_string();

                    let mut content_len = 0usize;
                    let mut has_valid_auth = false;

                    for line in lines {
                        let lower = line.to_lowercase();
                        if lower.starts_with("content-length:") {
                            if let Some(val) = line.split(':').nth(1) {
                                content_len = val.trim().parse::<usize>().unwrap_or(0);
                            }
                        }
                        if lower.starts_with("authorization:") {
                            if let Some(val) = line.split(':').nth(1) {
                                let token = val.trim().trim_start_matches("Bearer").trim();
                                if token == expected_token.as_str() {
                                    has_valid_auth = true;
                                }
                            }
                        }
                        if lower.starts_with("x-auth-token:") {
                            if let Some(val) = line.split(':').nth(1) {
                                if val.trim() == expected_token.as_str() {
                                    has_valid_auth = true;
                                }
                            }
                        }
                    }

                    if !has_valid_auth {
                        let resp = b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 12\r\nConnection: close\r\n\r\nUnauthorized";
                        let _ = socket.write_all(resp).await;
                        return;
                    }

                    break (method, path, pos + 4, content_len);
                }

                if total_read >= buf.len() {
                    let resp = b"HTTP/1.1 413 Payload Too Large\r\nContent-Length: 16\r\nConnection: close\r\n\r\nPayload Too Large";
                    let _ = socket.write_all(resp).await;
                    return;
                }
            };

            // 读取 Body
            let body_already_read = total_read - headers_end;
            let mut body = buf[headers_end..total_read].to_vec();
            if body_already_read < content_length {
                let remaining = content_length - body_already_read;
                let mut rest = vec![0u8; remaining];
                if socket.read_exact(&mut rest).await.is_ok() {
                    body.extend_from_slice(&rest);
                }
            }

            // 处理路由
            match (method.as_str(), path.as_str()) {
                ("GET", "/health") => {
                    let payload = serde_json::json!({
                        "status": "ok",
                        "pid": std::process::id(),
                    });
                    let body_str = payload.to_string();
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body_str.len(),
                        body_str
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                }
                ("POST", "/open") => {
                    if let Ok(req) = serde_json::from_slice::<OpenWorkspacePayload>(&body) {
                        let _ = app_handle.emit("mpe://open-workspace", req);
                        activate_window(&app_handle);

                        let resp = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}";
                        let _ = socket.write_all(resp).await;
                    } else {
                        let resp = b"HTTP/1.1 400 Bad Request\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"error\":\"bad\"}";
                        let _ = socket.write_all(resp).await;
                    }
                }
                ("POST", "/ai") => {
                    if let Ok(req) = serde_json::from_slice::<TriggerAiPayload>(&body) {
                        let _ = app_handle.emit("mpe://trigger-ai", req);
                        activate_window(&app_handle);

                        let resp = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}";
                        let _ = socket.write_all(resp).await;
                    } else {
                        let resp = b"HTTP/1.1 400 Bad Request\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"error\":\"bad\"}";
                        let _ = socket.write_all(resp).await;
                    }
                }
                _ => {
                    let resp = b"HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nNot Found";
                    let _ = socket.write_all(resp).await;
                }
            }
        });
    }
}

fn activate_window(app_handle: &tauri::AppHandle) {
    if let Some(win) = app_handle.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
