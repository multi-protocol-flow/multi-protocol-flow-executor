//! 应用更新检测、下载与安装模块
//!
//! 与官网 changelog (`https://download.mpe.run/changelog.json`) 对齐，
//! 支持基于 SemVer 的版本检测、流式分块下载带进度反馈以及跨平台安装执行。

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

/// 官方更新信息源地址
pub const CHANGELOG_URL: &str = "https://download.mpe.run/changelog.json";

/// 默认请求 User-Agent
pub const DEFAULT_USER_AGENT: &str = concat!("mpe-app/", env!("CARGO_PKG_VERSION"));

/// 官方 changelog.json 中单条版本记录结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    /// 版本号字符串，如 "0.1.1-beta.2" 或 "0.1.2"
    pub version: String,
    /// Git Tag 名称，如 "v0.1.1-beta.2"
    #[serde(default)]
    pub tag: Option<String>,
    /// 发布时间（ISO 8601 格式）
    #[serde(default)]
    pub date: Option<String>,
    /// 更新说明（支持 Markdown）
    #[serde(default)]
    pub notes: Option<String>,
    /// 是否为预发布版本
    #[serde(default)]
    pub prerelease: Option<bool>,
    /// 各平台直链：key 为 "windows" | "macos" | "linux"
    #[serde(default)]
    pub downloads: HashMap<String, String>,
}

/// 检查更新返回的结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateCheckResult {
    /// 当前应用版本
    pub current_version: String,
    /// 匹配到的最新版本
    pub latest_version: String,
    /// 是否存在可用更新（latest > current）
    pub has_update: bool,
    /// 最新版本发布日期
    pub release_date: Option<String>,
    /// 最新版本更新日志
    pub release_notes: Option<String>,
    /// 最新版本是否为预发布版
    pub is_prerelease: bool,
    /// 当前平台对应的下载链接
    pub download_url: Option<String>,
    /// 当前操作系统平台识别符 ("windows" | "macos" | "linux")
    pub platform: String,
}

/// 下载进度事件载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// 已接收字节数
    pub received: u64,
    /// 总字节数（若服务端未返回 Content-Length 则为 None）
    pub total: Option<u64>,
    /// 下载百分比 (0.0 ~ 100.0)
    pub percent: f32,
    /// 当前下载速率（字节/秒）
    pub bytes_per_sec: f64,
}

/// 获取当前操作系统平台识别符
#[inline]
pub fn get_current_platform() -> &'static str {
    match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        "linux" => "linux",
        other => other,
    }
}

/// 从 changelog 列表中根据当前版本和策略计算匹配的最新版本
pub fn find_latest_release(
    entries: &[ChangelogEntry],
    current_version_str: &str,
    include_prerelease: bool,
    platform: &str,
) -> Option<UpdateCheckResult> {
    let clean_current = current_version_str.trim().trim_start_matches('v');
    let current_ver = flow_engine::semver::parse(clean_current).ok()?;

    let mut candidate: Option<(ChangelogEntry, flow_engine::semver::Version)> = None;

    for entry in entries {
        let is_entry_prerelease = entry.prerelease.unwrap_or(false);
        if !include_prerelease && is_entry_prerelease {
            continue;
        }

        let clean_entry_ver = entry.version.trim().trim_start_matches('v');
        let parsed_ver = match flow_engine::semver::parse(clean_entry_ver) {
            Ok(v) => v,
            Err(e) => {
                log::debug!("[Updater] Skip invalid semver '{}': {e}", entry.version);
                continue;
            }
        };

        if !include_prerelease && !parsed_ver.prerelease.is_empty() {
            continue;
        }

        // 判断是否有更优候选
        let is_better = match &candidate {
            None => true,
            Some((_, best_ver)) => &parsed_ver > best_ver,
        };

        if is_better {
            candidate = Some((entry.clone(), parsed_ver));
        }
    }

    candidate.map(|(entry, best_ver)| {
        let has_update = best_ver > current_ver;
        let download_url = entry.downloads.get(platform).cloned();
        let is_prerelease = entry.prerelease.unwrap_or(!best_ver.prerelease.is_empty());

        UpdateCheckResult {
            current_version: clean_current.to_string(),
            latest_version: entry.version.clone(),
            has_update,
            release_date: entry.date.clone(),
            release_notes: entry.notes.clone(),
            is_prerelease,
            download_url,
            platform: platform.to_string(),
        }
    })
}

/// 请求远程更新接口并检查更新
pub async fn check_for_update(
    current_version: &str,
    include_prerelease: Option<bool>,
) -> Result<UpdateCheckResult, String> {
    let platform = get_current_platform();

    // 如果未显式指定 include_prerelease，当前版本若带 pre-release 标签则默认包含预发布更新
    let include_pre = include_prerelease.unwrap_or_else(|| {
        current_version.contains('-') || current_version.contains("beta") || current_version.contains("alpha")
    });

    log::info!(
        "[Updater] Checking updates for version={}, platform={}, include_prerelease={}",
        current_version,
        platform,
        include_pre
    );

    let client = reqwest::Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let response = client
        .get(CHANGELOG_URL)
        .send()
        .await
        .map_err(|e| format!("Network request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Update server returned HTTP status {}", response.status()));
    }

    let entries: Vec<ChangelogEntry> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse changelog response: {e}"))?;

    find_latest_release(&entries, current_version, include_pre, platform)
        .ok_or_else(|| "No compatible release found for current platform".to_string())
}

/// 流式分块下载安装包，并定期调用进度回调
pub async fn download_update_package<F>(
    url: &str,
    on_progress: F,
) -> Result<PathBuf, String>
where
    F: Fn(DownloadProgress) + Send + Sync + 'static,
{
    log::info!("[Updater] Starting download from '{url}'");

    let client = reqwest::Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to download URL: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with HTTP status {}", response.status()));
    }

    let total = response.content_length();

    // 从 URL 提取文件名
    let file_name = url
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            match std::env::consts::OS {
                "windows" => "mpe-update-setup.exe",
                "macos" => "mpe-update-universal.dmg",
                _ => "mpe-update-linux.tar.gz",
            }
        });

    let temp_dir = std::env::temp_dir();
    let final_path = temp_dir.join(file_name);
    let part_path = temp_dir.join(format!("{file_name}.part"));

    let mut file = tokio::fs::File::create(&part_path)
        .await
        .map_err(|e| format!("Failed to create temporary file {}: {e}", part_path.display()))?;

    let mut received: u64 = 0;
    let start_time = std::time::Instant::now();
    let mut last_emit_time = std::time::Instant::now();

    // 初始进度通知
    on_progress(DownloadProgress {
        received: 0,
        total,
        percent: 0.0,
        bytes_per_sec: 0.0,
    });

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Error downloading chunk: {e}"))?
    {
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed writing chunk to disk: {e}"))?;

        received += chunk.len() as u64;

        let now = std::time::Instant::now();
        if now.duration_since(last_emit_time).as_millis() >= 150 || received == total.unwrap_or(0) {
            let elapsed_secs = start_time.elapsed().as_secs_f64();
            let bytes_per_sec = if elapsed_secs > 0.0 {
                received as f64 / elapsed_secs
            } else {
                0.0
            };
            let percent = match total {
                Some(t) if t > 0 => ((received as f64 / t as f64) * 100.0) as f32,
                _ => 0.0,
            };

            on_progress(DownloadProgress {
                received,
                total,
                percent,
                bytes_per_sec,
            });
            last_emit_time = now;
        }
    }

    file.flush()
        .await
        .map_err(|e| format!("Failed flushing downloaded file: {e}"))?;
    drop(file);

    // 重命名 .part 文件为目标文件
    tokio::fs::rename(&part_path, &final_path)
        .await
        .map_err(|e| format!("Failed to finalize downloaded package: {e}"))?;

    log::info!("[Updater] Download completed successfully: {}", final_path.display());
    Ok(final_path)
}

/// 执行安装包并视平台退出当前程序
pub fn launch_installer(file_path: &Path, app: &tauri::AppHandle) -> Result<(), String> {
    if !file_path.exists() {
        return Err(format!("Installer file not found at {}", file_path.display()));
    }

    log::info!("[Updater] Launching installer: {}", file_path.display());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

        std::process::Command::new(file_path)
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .spawn()
            .map_err(|e| format!("Failed to run Windows installer: {e}"))?;

        // Windows 上正常退出当前程序以释放文件锁定
        app.exit(0);
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        let _ = app;
        // macOS 上使用 open 命令挂载 DMG 镜像
        std::process::Command::new("open")
            .arg(file_path)
            .spawn()
            .map_err(|e| format!("Failed to open DMG file: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        let _ = app;
        // Linux 上尝试打开所在文件夹
        if let Some(parent) = file_path.parent() {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = app;
        Err("Unsupported operating system for installer execution".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_entry(version: &str, prerelease: bool, win_url: &str) -> ChangelogEntry {
        let mut downloads = HashMap::new();
        downloads.insert("windows".to_string(), win_url.to_string());
        downloads.insert("macos".to_string(), "https://example.com/mac.dmg".to_string());
        downloads.insert("linux".to_string(), "https://example.com/linux.tar.gz".to_string());

        ChangelogEntry {
            version: version.to_string(),
            tag: Some(format!("v{version}")),
            date: Some("2026-08-20T00:00:00Z".to_string()),
            notes: Some("Test release notes".to_string()),
            prerelease: Some(prerelease),
            downloads,
        }
    }

    #[test]
    fn test_find_latest_release_newer_exists() {
        let entries = vec![
            make_test_entry("0.1.1-beta.2", true, "https://example.com/0.1.1-beta.2.exe"),
            make_test_entry("0.1.1-beta.1", true, "https://example.com/0.1.1-beta.1.exe"),
        ];

        let result = find_latest_release(&entries, "0.1.1-beta.1", true, "windows").expect("Candidate found");
        assert!(result.has_update);
        assert_eq!(result.latest_version, "0.1.1-beta.2");
        assert_eq!(result.current_version, "0.1.1-beta.1");
        assert_eq!(result.download_url.as_deref(), Some("https://example.com/0.1.1-beta.2.exe"));
    }

    #[test]
    fn test_find_latest_release_up_to_date() {
        let entries = vec![
            make_test_entry("0.1.1-beta.2", true, "https://example.com/0.1.1-beta.2.exe"),
            make_test_entry("0.1.1-beta.1", true, "https://example.com/0.1.1-beta.1.exe"),
        ];

        let result = find_latest_release(&entries, "0.1.1-beta.2", true, "windows").expect("Candidate found");
        assert!(!result.has_update);
        assert_eq!(result.latest_version, "0.1.1-beta.2");
    }

    #[test]
    fn test_find_latest_release_exclude_prerelease() {
        let entries = vec![
            make_test_entry("0.2.0-beta.1", true, "https://example.com/0.2.0-beta.1.exe"),
            make_test_entry("0.1.0", false, "https://example.com/0.1.0.exe"),
        ];

        let result = find_latest_release(&entries, "0.1.0", false, "windows").expect("Candidate found");
        assert!(!result.has_update);
        assert_eq!(result.latest_version, "0.1.0");
    }
}
