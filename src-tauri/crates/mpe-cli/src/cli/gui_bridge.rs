//! CLI 与 GUI 本地实例通信桥梁
//!
//! 检测正在运行的 MPE GUI 实例（通过 `gui-instance.json` 与 Local Loopback HTTP 探针），
//! 并通过 HTTP POST 派发指令唤醒 GUI，或在未运行时自动启动 GUI。

use crate::cli::CliError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// GUI 实例运行时信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuiInstanceInfo {
    /// 进程 PID
    pub pid: u32,
    /// 监听端口
    pub port: u16,
    /// 认证 Token
    pub auth_token: String,
    /// 应用版本
    pub version: String,
}

/// 获取 GUI 实例元数据文件路径
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

/// 探测当前是否有存活运行的 GUI 实例
///
/// 读取元数据文件并通过 HTTP `/health` 端点发送带 Token 的探针验证。
/// 若进程已退出或端口无响应，自动清理过期的元数据文件并返回 `None`。
pub async fn find_running_gui() -> Option<GuiInstanceInfo> {
    let path = get_gui_instance_path();
    let content = std::fs::read_to_string(&path).ok()?;
    let info: GuiInstanceInfo = serde_json::from_str(&content).ok()?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(800))
        .build()
        .ok()?;

    let health_url = format!("http://127.0.0.1:{}/health", info.port);
    let res = client
        .get(&health_url)
        .header("Authorization", format!("Bearer {}", info.auth_token))
        .send()
        .await;

    match res {
        Ok(resp) if resp.status().is_success() => Some(info),
        _ => {
            // 实例无响应，清理过期文件
            let _ = std::fs::remove_file(&path);
            None
        }
    }
}

/// 向正在运行的 GUI 实例发送控制指令
pub async fn send_gui_command(
    info: &GuiInstanceInfo,
    endpoint: &str,
    body: &serde_json::Value,
) -> Result<(), CliError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| CliError::execution(format!("Failed to build HTTP client: {e}")))?;

    let url = format!("http://127.0.0.1:{}{}", info.port, endpoint);
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", info.auth_token))
        .json(body)
        .send()
        .await
        .map_err(|e| CliError::execution(format!("Failed to send command to GUI: {e}")))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        Err(CliError::execution(format!(
            "GUI returned error status {status}: {text}"
        )))
    }
}

/// 在未检测到 GUI 实例时，拉起桌面应用主程序
pub fn launch_gui() -> Result<(), CliError> {
    let current_exe = std::env::current_exe()
        .map_err(|e| CliError::execution(format!("Failed to determine executable path: {e}")))?;

    std::process::Command::new(current_exe)
        .spawn()
        .map_err(|e| CliError::execution(format!("Failed to launch MPE GUI application: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gui_instance_info_serde() {
        let info = GuiInstanceInfo {
            pid: 12345,
            port: 54321,
            auth_token: "test-token-uuid".to_string(),
            version: "0.1.0".to_string(),
        };

        let json_str = match serde_json::to_string(&info) {
            Ok(s) => s,
            Err(e) => panic!("serialize should succeed: {e}"),
        };
        let deserialized: GuiInstanceInfo = match serde_json::from_str(&json_str) {
            Ok(d) => d,
            Err(e) => panic!("deserialize should succeed: {e}"),
        };

        assert_eq!(info, deserialized);
    }

    #[test]
    fn test_get_gui_instance_path() {
        let path = get_gui_instance_path();
        assert!(path.to_string_lossy().ends_with("gui-instance.json"));
    }

    #[tokio::test]
    async fn test_send_gui_command_mock_server() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;
        let _ = rustls::crypto::ring::default_provider().install_default();

        let listener = match TcpListener::bind("127.0.0.1:0").await {
            Ok(l) => l,
            Err(e) => panic!("bind failed: {e}"),
        };
        let port = match listener.local_addr() {
            Ok(a) => a.port(),
            Err(e) => panic!("addr failed: {e}"),
        };

        let token = "secret-token-123".to_string();
        let expected_token = token.clone();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                if let Ok(n) = socket.read(&mut buf).await {
                    let req = String::from_utf8_lossy(&buf[..n]);
                    if req.contains(&format!("Bearer {}", expected_token)) {
                        let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}";
                        let _ = socket.write_all(resp).await;
                    }
                }
            }
        });

        let info = GuiInstanceInfo {
            pid: std::process::id(),
            port,
            auth_token: token,
            version: "0.1.0".to_string(),
        };

        let res = send_gui_command(
            &info,
            "/open",
            &serde_json::json!({ "path": "/test/path" }),
        )
        .await;
        assert!(res.is_ok());
    }
}
