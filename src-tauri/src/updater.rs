//! 应用更新检测、下载与安装模块
//!
//! 与官网 changelog (`https://download.mpe.run/changelog.json`) 对齐，
//! 支持基于 SemVer 的版本检测、流式分块下载带进度反馈以及跨平台安装执行。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
    std::env::consts::OS
}
/// 获取 Linux 环境下的下载包候选 key 列表（按当前运行与安装形态优先级排序）
pub fn get_linux_download_keys() -> Vec<&'static str> {
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("APPIMAGE").is_some() {
            vec!["linux_appimage", "linux_deb", "linux"]
        } else if std::path::Path::new("/etc/debian_version").exists()
            || std::env::current_exe()
                .map(|p| p.starts_with("/usr"))
                .unwrap_or(false)
        {
            vec!["linux_deb", "linux_appimage", "linux"]
        } else {
            vec!["linux_appimage", "linux_deb", "linux"]
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        vec!["linux_deb", "linux_appimage", "linux"]
    }
}

/// 根据平台类型解析最匹配的下载 URL
pub fn resolve_download_url(downloads: &HashMap<String, String>, platform: &str) -> Option<String> {
    if platform == "linux" {
        for key in get_linux_download_keys() {
            if let Some(url) = downloads.get(key) {
                return Some(url.clone());
            }
        }
    }
    downloads.get(platform).cloned()
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
        let download_url = resolve_download_url(&entry.downloads, platform);
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
        current_version.contains('-')
            || current_version.contains("beta")
            || current_version.contains("alpha")
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
        return Err(format!(
            "Update server returned HTTP status {}",
            response.status()
        ));
    }

    let entries: Vec<ChangelogEntry> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse changelog response: {e}"))?;

    find_latest_release(&entries, current_version, include_pre, platform)
        .ok_or_else(|| "No compatible release found for current platform".to_string())
}

/// 读取 XDG 配置中声明的下载目录（`~/.config/user-dirs.dirs` 的 `XDG_DOWNLOAD_DIR`）
#[cfg(target_os = "linux")]
fn xdg_download_dir(home: &Path) -> Option<PathBuf> {
    let config = home.join(".config").join("user-dirs.dirs");
    let content = std::fs::read_to_string(config).ok()?;

    for line in content.lines() {
        let Some(raw) = line.trim().strip_prefix("XDG_DOWNLOAD_DIR=") else {
            continue;
        };

        let value = raw.trim().trim_matches('"');
        if value.is_empty() {
            continue;
        }

        let path = PathBuf::from(value.replace("$HOME", &home.to_string_lossy()));
        if path.is_absolute() {
            return Some(path);
        }
    }

    None
}

/// 获取更新包保存目录
///
/// Linux 下依次尝试 XDG 下载目录、`~/Downloads`、`~/.cache/mpe/updates`：
/// 前两者位于用户主目录下的非隐藏路径，沙箱化的软件中心（Ubuntu App Center / snap-store）
/// 才能读取；直接存放于 `/tmp` 会被 Snap 的私有 `/tmp` 命名空间隔离而无法打开。
#[cfg(target_os = "linux")]
pub fn get_update_download_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        if let Some(dir) = xdg_download_dir(&home) {
            if std::fs::create_dir_all(&dir).is_ok() {
                return dir;
            }
        }

        let downloads = home.join("Downloads");
        if downloads.is_dir() {
            return downloads;
        }

        let cache = home.join(".cache").join("mpe").join("updates");
        if std::fs::create_dir_all(&cache).is_ok() {
            return cache;
        }
    }

    std::env::temp_dir()
}

/// 获取更新包保存目录（非 Linux 平台直接使用系统临时目录）
#[cfg(not(target_os = "linux"))]
pub fn get_update_download_dir() -> PathBuf {
    std::env::temp_dir()
}

/// 流式分块下载安装包，并定期调用进度回调
pub async fn download_update_package<F>(url: &str, on_progress: F) -> Result<PathBuf, String>
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
        return Err(format!(
            "Download failed with HTTP status {}",
            response.status()
        ));
    }

    let total = response.content_length();

    // 从 URL 提取文件名
    let file_name =
        url.rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or(match std::env::consts::OS {
                "windows" => "mpe-update-setup.exe",
                "macos" => "mpe-update-universal.dmg",
                _ => "mpe-update-linux.tar.gz",
            });

    let target_dir = get_update_download_dir();
    if let Err(e) = tokio::fs::create_dir_all(&target_dir).await {
        log::warn!(
            "[Updater] Failed to create update directory {}: {e}",
            target_dir.display()
        );
    }
    let final_path = target_dir.join(file_name);
    let part_path = target_dir.join(format!("{file_name}.part"));

    let mut file = tokio::fs::File::create(&part_path).await.map_err(|e| {
        format!(
            "Failed to create temporary file {}: {e}",
            part_path.display()
        )
    })?;

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

    log::info!(
        "[Updater] Download completed successfully: {}",
        final_path.display()
    );
    Ok(final_path)
}

/// 判断待安装包版本是否高于当前版本
///
/// 任一版本号无法解析时返回 `true`（不拦截），避免因解析差异阻断正常更新。
#[cfg(target_os = "linux")]
#[allow(dead_code)]
fn is_package_newer(package_version: &str, current_version: &str) -> bool {
    match (
        flow_engine::semver::parse(package_version),
        flow_engine::semver::parse(current_version),
    ) {
        (Ok(pkg), Ok(cur)) => pkg > cur,
        _ => true,
    }
}

/// 判断待安装包版本是否严格低于当前版本
#[cfg(target_os = "linux")]
fn is_package_older(package_version: &str, current_version: &str) -> bool {
    match (
        flow_engine::semver::parse(package_version),
        flow_engine::semver::parse(current_version),
    ) {
        (Ok(pkg), Ok(cur)) => pkg < cur,
        _ => false,
    }
}

/// 从执行路径中提取 macOS 的 `.app` 应用程序包路径
pub fn find_app_bundle_in_path(path: &Path) -> Option<PathBuf> {
    for ancestor in path.ancestors() {
        if ancestor.extension().and_then(|s| s.to_str()) == Some("app") {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

/// 安全转义 POSIX Shell 单引号字符串，杜绝任何参数扩展、命令替换或反引号注入
#[inline]
pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r#"'\''"#))
}

/// 读取 `.deb` 包 control 中的 Version 字段（`dpkg-deb` 不可用时返回 `None`）
#[cfg(target_os = "linux")]
fn read_deb_version(file_path: &Path) -> Option<String> {
    let output = std::process::Command::new("dpkg-deb")
        .arg("-f")
        .arg(file_path)
        .arg("Version")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version.is_empty() {
        None
    } else {
        Some(version)
    }
}

/// 将安装包复制到全局可遍历的暂存目录，返回暂存后的路径
///
/// `apt` 会先以内部低权限用户 `_apt` 读取安装包，而用户家目录（通常是 `0750`）对其不可遍历，
/// 触发提示 `由于文件 ... 无法被用户 '_apt' 访问，已脱离沙盒并提权为根用户`。
/// 复制到 `0755` 的随机暂存目录后，`apt` 能以 `_apt` 身份正常读取，安装输出保持干净。
///
/// 暂存副本不主动删除：`apt` 可能在本进程退出后继续读取，`/var/tmp` 由系统定期清理。
#[cfg(target_os = "linux")]
#[allow(dead_code)]
fn stage_package_for_root(file_path: &Path) -> std::io::Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let file_name = file_path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "installer path has no file name",
        )
    })?;

    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let staging_dir = Path::new("/var/tmp").join(format!("mpe-update-{unique}"));
    std::fs::create_dir_all(&staging_dir)?;
    std::fs::set_permissions(&staging_dir, std::fs::Permissions::from_mode(0o755))?;

    let staged = staging_dir.join(file_name);
    std::fs::copy(file_path, &staged)?;
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o644))?;
    Ok(staged)
}

/// 执行安装包并视平台退出当前程序
pub fn launch_installer(file_path: &Path, app: &tauri::AppHandle) -> Result<(), String> {
    if !file_path.exists() {
        return Err(format!(
            "Installer file not found at {}",
            file_path.display()
        ));
    }

    log::info!("[Updater] Launching installer: {}", file_path.display());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;

        #[link(name = "shell32")]
        extern "system" {
            fn ShellExecuteW(
                hwnd: *mut std::ffi::c_void,
                lpOperation: *const u16,
                lpFile: *const u16,
                lpParameters: *const u16,
                lpDirectory: *const u16,
                nShowCmd: i32,
            ) -> isize;
        }

        let path_wide: Vec<u16> = file_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let verb_wide: Vec<u16> = "open\0".encode_utf16().collect();

        const SW_SHOWNORMAL: i32 = 1;

        let ret = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb_wide.as_ptr(),
                path_wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };

        if ret <= 32 {
            let err_desc = match ret {
                5 => " (Access denied / UAC elevation rejected)",
                2 => " (File not found)",
                _ => "",
            };
            return Err(format!(
                "Failed to run Windows installer via ShellExecute: code {ret}{err_desc}"
            ));
        }

        // Windows 上正常退出当前程序以释放文件锁定
        app.exit(0);
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        let app_bundle = std::env::current_exe().ok().and_then(|p| find_app_bundle_in_path(&p));
        let pid = std::process::id();
        let dmg_quoted = shell_quote(&file_path.to_string_lossy());
        let target_quoted = shell_quote(
            &app_bundle
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
        );

        // 构造后台自动化升级与替换脚本
        // 脚本执行流程：
        //   1. 等待当前应用 PID 退出（避免二进制文件占用与写保护）
        //   2. 使用 trap 在脚本退出时彻底清理临时挂载点与脚本自身
        //   3. 使用 hdiutil 静默挂载 DMG 到临时目录
        //   4. 在挂载卷中寻找新版 .app 目录
        //   5. 备份原版本后使用 ditto 覆盖替换，失败时安全回滚原应用，避免文件丢失
        //   6. 若自动替换成功，移除隔离属性并重新拉起新版；若失败则 fallback 激活 Finder 打开 DMG
        let script = format!(
            r#"#!/bin/sh
PID="{pid}"
DMG={dmg_quoted}
TARGET={target_quoted}

MOUNT_DIR=$(mktemp -d /tmp/mpe_dmg_XXXXXX)
trap 'hdiutil detach "$MOUNT_DIR" -quiet 2>/dev/null; rm -rf "$MOUNT_DIR" "$0" 2>/dev/null' EXIT INT TERM

# 等待主应用完全退出
while kill -0 "$PID" 2>/dev/null; do
    sleep 0.2
done

if hdiutil attach "$DMG" -mountpoint "$MOUNT_DIR" -nobrowse -quiet; then
    SRC_APP=$(find "$MOUNT_DIR" -maxdepth 1 -name "*.app" | head -n 1)
    if [ -n "$SRC_APP" ] && [ -n "$TARGET" ] && [ -d "$TARGET" ]; then
        BACKUP="${{TARGET}}.old.$$"
        mv "$TARGET" "$BACKUP" 2>/dev/null
        if ditto "$SRC_APP" "$TARGET" 2>/dev/null; then
            rm -rf "$BACKUP" 2>/dev/null
            xattr -rd com.apple.quarantine "$TARGET" 2>/dev/null
            open "$TARGET"
            exit 0
        else
            # 覆盖失败安全回滚
            mv "$BACKUP" "$TARGET" 2>/dev/null
        fi
    fi
fi

# Fallback：若无法自动覆盖或非应用包运行，通过 Finder 打开 DMG 并置顶激活
open "$DMG"
osascript -e 'tell application "Finder" to activate' 2>/dev/null || true
"#
        );

        let script_path = std::env::temp_dir().join(format!("mpe_update_{pid}.sh"));
        std::fs::write(&script_path, &script)
            .map_err(|e| format!("Failed to write macOS updater script: {e}"))?;

        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755));

        // 脱离当前进程独立执行脚本
        std::process::Command::new("/bin/sh")
            .arg(&script_path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn macOS updater script: {e}"))?;

        // 退出当前应用以释放资源并让升级脚本顺利执行
        app.exit(0);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        if file_name.ends_with(".deb") {
            log::info!(
                "[Updater] Launching .deb package installer for {}",
                file_path.display()
            );

            // 拒绝安装严格低于当前版本的安装包（允许相等版本用于测试与重装）
            if let Some(package_version) = read_deb_version(file_path) {
                let current_version = env!("CARGO_PKG_VERSION");
                if is_package_older(&package_version, current_version) {
                    return Err(format!(
                        "Downloaded package version {package_version} is older than the current version {current_version}; the release manifest may point to a stale artifact"
                    ));
                }
            }

            // 1. 优先尝试轻量级专用 GUI 安装器 gdebi-gtk（专为单个 deb 设计）
            if std::process::Command::new("gdebi-gtk")
                .arg(file_path)
                .spawn()
                .is_ok()
            {
                log::info!("[Updater] Launched gdebi-gtk successfully");
                return Ok(());
            }

            // 2. 尝试使用 gnome-software 打开 deb
            if std::process::Command::new("gnome-software")
                .arg("--local-filename")
                .arg(file_path)
                .spawn()
                .is_ok()
            {
                log::info!("[Updater] Launched gnome-software successfully");
                return Ok(());
            }

            // 3. 尝试使用系统默认 mime 处理程序 (xdg-open) 唤起系统应用中心
            if std::process::Command::new("xdg-open")
                .arg(file_path)
                .spawn()
                .is_ok()
            {
                log::info!("[Updater] Launched xdg-open successfully");
                return Ok(());
            }

            // 4. 若上述 GUI 安装器均未启动，尝试在常见终端模拟器中弹出提权安装命令
            // 通过位置参数 $1 传入文件路径，杜绝 shell 注入风险
            let bash_inline_script = "sudo apt install -y \"$1\"; echo; read -r -p 'Press Enter to exit...' _";
            let terminals: [(&str, Vec<&str>); 5] = [
                ("gnome-terminal", vec!["--", "bash", "-c", bash_inline_script, "_"]),
                ("x-terminal-emulator", vec!["-e", "bash", "-c", bash_inline_script, "_"]),
                ("konsole", vec!["-e", "bash", "-c", bash_inline_script, "_"]),
                ("xfce4-terminal", vec!["-x", "bash", "-c", bash_inline_script, "_"]),
                ("xterm", vec!["-e", "bash", "-c", bash_inline_script, "_"]),
            ];

            let file_os = file_path.as_os_str();
            for (term, args) in terminals {
                let mut cmd = std::process::Command::new(term);
                for arg in args {
                    cmd.arg(arg);
                }
                cmd.arg(file_os);
                if cmd.spawn().is_ok() {
                    log::info!("[Updater] Launched terminal installer via {term}");
                    return Ok(());
                }
            }

            Err(format!(
                "Failed to launch .deb installer: no supported package manager (gdebi, gnome-software, xdg-open, terminal) could be started for {}",
                file_path.display()
            ))
        } else if file_name.ends_with(".appimage") {
            log::info!(
                "[Updater] Setting up .AppImage execution for {}",
                file_path.display()
            );
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(file_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(file_path, perms);
            }

            // 启动新版本 AppImage 并退出当前应用
            std::process::Command::new(file_path)
                .spawn()
                .map_err(|e| format!("Failed to launch updated AppImage: {e}"))?;

            app.exit(0);
            Ok(())
        } else {
            log::info!(
                "[Updater] Opening archive/folder for {}",
                file_path.display()
            );
            // 对于 .tar.gz 等归档文件，使用 xdg-open 打开文件本身（归档管理器），失败则打开所在目录
            let opened = std::process::Command::new("xdg-open")
                .arg(file_path)
                .spawn()
                .is_ok();

            if !opened {
                if let Some(parent) = file_path.parent() {
                    let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
                }
            }
            Ok(())
        }
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
        downloads.insert(
            "macos".to_string(),
            "https://example.com/mac.dmg".to_string(),
        );
        downloads.insert(
            "linux".to_string(),
            "https://example.com/linux.tar.gz".to_string(),
        );

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

        let result = find_latest_release(&entries, "0.1.1-beta.1", true, "windows")
            .expect("Candidate found");
        assert!(result.has_update);
        assert_eq!(result.latest_version, "0.1.1-beta.2");
        assert_eq!(result.current_version, "0.1.1-beta.1");
        assert_eq!(
            result.download_url.as_deref(),
            Some("https://example.com/0.1.1-beta.2.exe")
        );
    }

    #[test]
    fn test_find_latest_release_up_to_date() {
        let entries = vec![
            make_test_entry("0.1.1-beta.2", true, "https://example.com/0.1.1-beta.2.exe"),
            make_test_entry("0.1.1-beta.1", true, "https://example.com/0.1.1-beta.1.exe"),
        ];

        let result = find_latest_release(&entries, "0.1.1-beta.2", true, "windows")
            .expect("Candidate found");
        assert!(!result.has_update);
        assert_eq!(result.latest_version, "0.1.1-beta.2");
    }

    #[test]
    fn test_find_latest_release_exclude_prerelease() {
        let entries = vec![
            make_test_entry("0.2.0-beta.1", true, "https://example.com/0.2.0-beta.1.exe"),
            make_test_entry("0.1.0", false, "https://example.com/0.1.0.exe"),
        ];

        let result =
            find_latest_release(&entries, "0.1.0", false, "windows").expect("Candidate found");
        assert!(!result.has_update);
        assert_eq!(result.latest_version, "0.1.0");
    }

    #[test]
    fn test_find_latest_release_linux_deb_preferred() {
        let mut downloads = HashMap::new();
        downloads.insert(
            "windows".to_string(),
            "https://example.com/win.exe".to_string(),
        );
        downloads.insert(
            "linux".to_string(),
            "https://example.com/linux.tar.gz".to_string(),
        );
        downloads.insert(
            "linux_deb".to_string(),
            "https://example.com/linux.deb".to_string(),
        );
        downloads.insert(
            "linux_appimage".to_string(),
            "https://example.com/linux.AppImage".to_string(),
        );

        let entry = ChangelogEntry {
            version: "0.2.0".to_string(),
            tag: Some("v0.2.0".to_string()),
            date: Some("2026-09-14T00:00:00Z".to_string()),
            notes: Some("Notes".to_string()),
            prerelease: Some(false),
            downloads,
        };

        let result =
            find_latest_release(&[entry], "0.1.0", false, "linux").expect("Candidate found");
        assert!(result.has_update);
        assert_eq!(
            result.download_url.as_deref(),
            Some("https://example.com/linux.deb")
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_xdg_download_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path();

        // 未配置时返回 None
        assert_eq!(xdg_download_dir(home), None);

        let config_dir = home.join(".config");
        std::fs::create_dir_all(&config_dir).expect("create .config");
        std::fs::write(
            config_dir.join("user-dirs.dirs"),
            "# comment\nXDG_DESKTOP_DIR=\"$HOME/Desktop\"\nXDG_DOWNLOAD_DIR=\"$HOME/下载\"\n",
        )
        .expect("write user-dirs.dirs");

        assert_eq!(xdg_download_dir(home), Some(home.join("下载")));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_is_package_newer() {
        assert!(is_package_newer("0.2.0-beta.2", "0.2.0-beta.1"));
        assert!(is_package_newer("0.2.0", "0.2.0-beta.2"));
        assert!(!is_package_newer("0.2.0-beta.1", "0.2.0-beta.1"));
        assert!(!is_package_newer("0.1.0", "0.2.0-beta.1"));
        // 无法解析的版本号不拦截
        assert!(is_package_newer("not-a-version", "0.2.0"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_is_package_older() {
        assert!(is_package_older("0.1.0", "0.2.0-beta.1"));
        assert!(is_package_older("0.2.0-beta.1", "0.2.0-beta.2"));
        assert!(!is_package_older("0.2.0-beta.1", "0.2.0-beta.1"));
        assert!(!is_package_older("0.2.0-beta.2", "0.2.0-beta.1"));
    }

    #[test]
    fn test_find_app_bundle_in_path() {
        let p1 = Path::new("/Applications/MPE.app/Contents/MacOS/mpe");
        assert_eq!(find_app_bundle_in_path(p1), Some(PathBuf::from("/Applications/MPE.app")));

        let p2 = Path::new("/Users/dev/Downloads/MPE.app");
        assert_eq!(find_app_bundle_in_path(p2), Some(PathBuf::from("/Users/dev/Downloads/MPE.app")));

        let p3 = Path::new("/usr/local/bin/mpe");
        assert_eq!(find_app_bundle_in_path(p3), None);
    }

    #[test]
    fn test_shell_quote() {
        assert_eq!(shell_quote("/simple/path"), "'/simple/path'");
        assert_eq!(shell_quote("path with spaces"), "'path with spaces'");
        assert_eq!(shell_quote("evil'path\"with$var`cmd`"), "'evil'\\''path\"with$var`cmd`'");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_read_deb_version_and_stage_package() {
        use std::os::unix::fs::PermissionsExt;

        if std::process::Command::new("dpkg-deb")
            .arg("--version")
            .output()
            .is_err()
        {
            return; // 环境缺少 dpkg-deb，跳过
        }

        let tmp = tempfile::tempdir().expect("tempdir");
        let pkg_root = tmp.path().join("pkg");
        std::fs::create_dir_all(pkg_root.join("DEBIAN")).expect("create DEBIAN");
        std::fs::write(
            pkg_root.join("DEBIAN/control"),
            "Package: mpe-test\nVersion: 9.9.9\nArchitecture: all\nMaintainer: test\nDescription: test\n",
        )
        .expect("write control");

        let deb_path = tmp.path().join("mpe-test_9.9.9_all.deb");
        let built = std::process::Command::new("dpkg-deb")
            .arg("-b")
            .arg(&pkg_root)
            .arg(&deb_path)
            .status()
            .expect("run dpkg-deb -b");
        assert!(built.success(), "dpkg-deb -b 构建测试包失败");

        assert_eq!(read_deb_version(&deb_path).as_deref(), Some("9.9.9"));

        let staged = stage_package_for_root(&deb_path).expect("stage package");
        assert_eq!(
            staged.file_name().and_then(|n| n.to_str()),
            Some("mpe-test_9.9.9_all.deb")
        );
        let mode = std::fs::metadata(&staged)
            .expect("staged metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            mode, 0o644,
            "暂存副本必须全局可读，apt 才能以 _apt 身份读取"
        );
        assert_eq!(
            std::fs::metadata(staged.parent().expect("staging dir"))
                .expect("staging dir metadata")
                .permissions()
                .mode()
                & 0o777,
            0o755
        );

        if let Some(dir) = staged.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}
