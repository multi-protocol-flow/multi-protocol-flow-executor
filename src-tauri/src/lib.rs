//! 多协议流程执行引擎库
//!
//! 提供 Tauri 应用入口点和 IPC 命令。
//! 业务逻辑委托给 flow-engine crate。

use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;

// CLI 模块（从 mpe-cli crate 导入，供统一入口 main.rs 调用）
pub use mpe_cli::cli;

// Tauri 特有模块
pub mod gui_ipc;
mod plugin_protocol;
mod tauri_adapters;
mod tauri_commands;
pub mod updater;

// 导出 flow-engine 的所有公开项
pub use flow_engine::*;

// 额外导出 CLI 需要的具体类型（flow-engine 未 re-export 的模块内容）
pub use flow_engine::executor::{ExecutionResult, FlowExecutor};
pub use flow_engine::report::{FlowExecutionReport, NodeExecutionReport};
pub use flow_engine::storage::{
    delete_report, ensure_flows_dir, get_data_dir, get_flows_dir, get_reports_dir, list_reports,
    load_flow, load_report, save_flow, save_report,
};

// 导出 Tauri 命令供 main.rs 使用
pub use tauri_commands::{
    ai_chat_stream,
    ai_execute_command,
    ai_fetch_models,
    ai_inspect_api_spec,
    ai_read_code_slice,
    ai_scan_workspace,
    ai_search_code,
    ai_test_connection,
    cancel_dataset_execution,
    // 执行会话管理
    cancel_flow,
    check_app_update,
    copy_flow_file,
    create_flow,
    create_folder,
    delete_dataset_report,
    delete_flow_file,
    delete_flow_report,
    delete_folder,
    delete_stress_test_report,
    download_update_package,
    end_debug,
    execute_flow,
    execute_flow_with_dataset,
    execute_single_node,
    force_end_debug,
    get_app_data_dir,
    // 应用更新命令
    get_app_version,
    get_debug_status,
    get_node_config_schema,
    get_node_execution_detail,
    // 节点注册表命令
    get_registered_node_types,
    get_stress_test_status,
    install_update_package,
    list_dataset_reports,
    // 流程文件存储命令
    list_flow_files,
    list_flow_reports,
    // 文件夹管理命令
    list_folders,
    list_stress_test_reports,
    load_dataset_report,
    load_dataset_row_detail,
    load_flow_file,
    load_flow_report,
    load_stress_test_report,
    move_flow_to_folder,
    move_folder,
    open_directory,
    rename_flow_file,
    rename_folder,
    save_csv_template,
    save_dataset_report,
    save_flow_file,
    save_flow_report,
    save_stress_test_report,
    send_debug_command,
    start_debug,
    // Ingress Mock 服务命令
    start_ingress_listener,
    stop_ingress_listener,
    get_active_ingress_ports,
    // 压力测试命令
    start_stress_test,
    stop_stress_test,
    // 数据集执行命令
    validate_dataset_csv,
    validate_flow,
    DatasetExecutionSessionManager,
    // 会话管理
    DebugSessionManager,
    ExecutionSessionManager,
};

// 插件配置面板 uiCall 转发命令
#[cfg(feature = "plugin-manager")]
pub use tauri_commands::plugin_ui_call;

// 数据源插件命令（列表 / browse / get_item / get_templates / validate_config）
#[cfg(feature = "plugin-manager")]
pub use tauri_commands::{
    datasource_browse, datasource_get_item, datasource_get_templates, datasource_validate_config,
    list_datasources,
};

// 插件设置命令（目录信息/热重载/已安装列表/市场列表/市场安装）
#[cfg(feature = "plugin-manager")]
pub use tauri_commands::{
    get_plugin_dir_info, install_marketplace_plugin, list_installed_plugins,
    list_marketplace_plugins, reload_plugins,
};

// SSE 连接测试命令
#[cfg(feature = "protocol-sse")]
pub use tauri_commands::sse_test_connection;

// TCP 协议模板命令
#[cfg(feature = "protocol-tcp")]
pub use tauri_commands::{tcp_get_template, tcp_list_templates};

// GraphQL 内省命令
#[cfg(feature = "protocol-graphql")]
pub use tauri_commands::graphql_introspect_schema;

// 导出适配器（供测试使用）
pub use tauri_adapters::{
    create_log_sender, TauriAppDirProvider, TauriDebugEventSender, TauriDirectoryOpener,
    TauriUnderlyingLogSender,
};
/// 获取默认日志目录
fn default_logs_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .map(|p| p.join("logs"))
        .unwrap_or_else(|_| PathBuf::from("logs"))
}

/// 共享节点注册表状态（Tauri State）
///
/// 应用启动时构建一次，所有执行/验证/注册表查询命令共享同一个注册表，
/// 使插件节点（`SidecarPlugin`）的插件进程在多次执行之间复用。
///
/// 插件目录热重载（`reload_plugins` 命令）通过替换整个快照实现：命令短临界区
/// 内克隆 Arc（不跨 await 持锁），旧快照的插件管理器在锁释放后统一 shutdown，
/// 因此重载瞬间正在执行的插件节点可能报错（预期行为）。
pub struct NodeRegistryState {
    /// 当前生效的注册表快照（registry + 插件管理器 + 插件目录）
    pub inner: std::sync::RwLock<RegistrySnapshot>,
}

impl NodeRegistryState {
    /// 克隆当前生效的快照（短临界区读锁，`into_inner` 处理中毒锁）
    pub fn snapshot(&self) -> RegistrySnapshot {
        self.inner
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// 注册表快照：一次插件扫描/describe 的完整结果，原子替换。
#[derive(Clone)]
pub struct RegistrySnapshot {
    /// 共享节点注册表（内置节点 + 已登记的插件节点）
    pub registry: Arc<flow_engine::registry::NodeRegistry>,
    /// 本次快照对应的插件目录
    #[cfg(feature = "plugin-manager")]
    pub plugins_dir: PathBuf,
    /// 插件管理器（持有插件进程生命周期；`plugin-manager` 特性关闭时不存在）
    #[cfg(feature = "plugin-manager")]
    pub manager: Arc<flow_engine::PluginManager>,
}

/// 解析插件目录：`MPE_PLUGIN_DIR` 环境变量优先，否则设置文件 `pluginDir`，
/// 否则默认数据目录下 `plugins`，最后兜底相对路径 `plugins`。
///
/// 返回解析结果与来源（前端据此展示来源徽标并在 env 指定时禁用编辑）。
fn resolve_plugin_dir(app: &tauri::AppHandle) -> (PathBuf, crate::tauri_commands::PluginDirSource) {
    use crate::tauri_commands::PluginDirSource;
    if let Some(dir) = std::env::var_os("MPE_PLUGIN_DIR") {
        return (PathBuf::from(dir), PluginDirSource::Env);
    }
    if let Ok(store) = app.store("store/settings.bin") {
        if let Some(dir) = store
            .get("pluginDir")
            .and_then(|v| v.as_str().map(PathBuf::from))
        {
            return (dir, PluginDirSource::Config);
        }
    }
    let dir = flow_engine::storage::get_data_dir()
        .ok()
        .map(|d| d.join("plugins"))
        .unwrap_or_else(|| PathBuf::from("plugins"));
    (dir, PluginDirSource::Default)
}
/// 解析流程存储目录：设置文件 `flowsDir`，缺省为应用数据目录下 `flows`
///
/// 与前端 `flowStorage.getFlowsDir()` 使用完全相同的来源与回退：
/// 设置项 → `get_app_data_dir()/flows`。注意应用数据目录由 Tauri 依据 identifier
/// 生成（`.../com.multi-protocol-flow-executor`），与引擎内硬编码的默认数据目录
/// 不同，因此这里必须显式回退到应用的 app_data_dir，否则会出现
/// 「前端列表可见、执行期按 ID 找不到子流程」。
pub(crate) fn resolve_flows_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    let configured = app.store("store/settings.bin").ok().and_then(|store| {
        store
            .get("flowsDir")
            .and_then(|v| v.as_str().map(String::from))
    });
    resolve_flows_dir_from(configured.as_deref(), app.path().app_data_dir().ok())
}

/// 流程目录解析的纯逻辑：设置项（去空白后非空）优先，否则 `app_data_dir/flows`
///
/// 单独抽出以便单测覆盖"默认目录与引擎内置默认值不同"这一情形。
pub(crate) fn resolve_flows_dir_from(
    configured: Option<&str>,
    app_data_dir: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(dir) = configured.map(str::trim).filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    app_data_dir.map(|dir| dir.join("flows"))
}

/// 将当前生效的流程目录登记到引擎存储层
///
/// 在任何执行入口调用一次，使执行期的子流程解析与前端列表使用同一目录。
pub(crate) fn sync_flows_dir_override(app: &tauri::AppHandle) {
    flow_engine::storage::set_flows_dir_override(resolve_flows_dir(app));
}

/// 解析默认执行超时时间：`MPE_DEFAULT_TIMEOUT_MS` 环境变量优先，否则设置文件 `defaultExecutionTimeoutMs`，
/// 缺省回退到 30 秒。
pub(crate) fn resolve_default_execution_timeout(app: &tauri::AppHandle) -> std::time::Duration {
    std::env::var("MPE_DEFAULT_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&ms| ms > 0)
        .or_else(|| {
            app.store("store/settings.bin")
                .ok()
                .and_then(|store| {
                    store
                        .get("defaultExecutionTimeoutMs")
                        .and_then(|v| v.as_u64())
                })
                .filter(|&ms| ms > 0)
        })
        .map(std::time::Duration::from_millis)
        .unwrap_or(std::time::Duration::from_secs(30))
}

/// 解析插件连接配置（并发上限、队列深度、默认超时时间）
pub(crate) fn resolve_plugin_connection_config(
    app: &tauri::AppHandle,
) -> flow_engine::ConnectionConfig {
    let mut config = flow_engine::ConnectionConfig::default();

    if let Some(concurrency) = std::env::var("MPE_PLUGIN_MAX_CONCURRENCY")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&c| c > 0)
        .or_else(|| {
            app.store("store/settings.bin")
                .ok()
                .and_then(|store| {
                    store
                        .get("pluginMaxConcurrency")
                        .and_then(|v| v.as_u64().map(|n| n as usize))
                })
                .filter(|&c| c > 0)
        })
    {
        config.max_concurrent_requests = concurrency;
    }

    if let Some(queue_cap) = std::env::var("MPE_PLUGIN_QUEUE_CAPACITY")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&q| q > 0)
        .or_else(|| {
            app.store("store/settings.bin")
                .ok()
                .and_then(|store| {
                    store
                        .get("pluginQueueCapacity")
                        .and_then(|v| v.as_u64().map(|n| n as usize))
                })
                .filter(|&q| q > 0)
        })
    {
        config.queue_capacity = queue_cap;
    }

    let default_timeout = resolve_default_execution_timeout(app);
    if config.request_timeout < default_timeout {
        config.request_timeout = default_timeout;
    }

    config
}

/// 构建共享节点注册表状态
///
/// 插件扫描与 describe 在启动时同步阻塞执行（describe 会拉起插件进程，
/// 但数量有限、耗时毫秒级，可接受）；扫描失败仅告警，不阻塞启动。
/// 关闭 `plugin-manager` 特性时仅构建内置节点注册表。
///
/// `host_locale` 是宿主当前 UI 语言（BCP-47，如 `en-US`）：插件进程在
/// spawn 时经 `MPE_LOCALE` 环境变量注入（匹配插件 `locales` 则用宿主语言，
/// 否则用插件 `default_locale`），语言在插件进程生命周期内固定。
fn build_node_registry_state(
    app: &tauri::AppHandle,
    host_locale: Option<String>,
) -> NodeRegistryState {
    #[cfg(feature = "plugin-manager")]
    {
        let (plugins_dir, _source) = resolve_plugin_dir(app);
        let default_timeout = resolve_default_execution_timeout(app);
        let conn_config = resolve_plugin_connection_config(app);
        let mut manager = flow_engine::PluginManager::new(plugins_dir.clone())
            .with_host_locale(host_locale)
            .with_connection_config(conn_config)
            .with_default_execution_timeout(default_timeout);
        let _ = tauri::async_runtime::block_on(manager.scan_and_describe());
        let manager = Arc::new(manager);
        let registry = Arc::new(flow_engine::default_registry_with_plugins(&manager));
        NodeRegistryState {
            inner: std::sync::RwLock::new(RegistrySnapshot {
                registry,
                plugins_dir,
                manager,
            }),
        }
    }
    #[cfg(not(feature = "plugin-manager"))]
    {
        let _ = host_locale;
        NodeRegistryState {
            inner: std::sync::RwLock::new(RegistrySnapshot {
                registry: Arc::new(flow_engine::default_registry::default_registry()),
            }),
        }
    }
}

/// 启动 Tauri 应用
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 安装 rustls CryptoProvider（HTTP/gRPC TLS 需要）
    // rustls 0.23+ 要求显式选择加密后端
    let _ = rustls::crypto::ring::default_provider().install_default();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_process::init());

    // 注册 `mpe-plugin` 自定义协议（PR-1）：为插件配置面板 iframe 提供前端 HTML。
    // 数据只来自内存 PluginManager 注册表缓存（D6），绝不触碰文件系统；
    // 每个响应带 D8 self-CSP 头。handler 在请求时经 state 取管理器，
    // 因此状态由 .setup 中的 `app.manage` 注入即可（setup 早于任何窗口导航）。
    // 注意：Tauri 2 的异步协议注册方法只在 `Builder` 上（App/AppHandle 没有），
    // 故在构建链上注册；handler 闭包按请求时态访问 state，无捕获、天然 'static。
    #[cfg(feature = "plugin-manager")]
    let builder = builder.register_asynchronous_uri_scheme_protocol(
        plugin_protocol::SCHEME,
        |ctx, request, responder| {
            let state = ctx.app_handle().state::<NodeRegistryState>();
            let manager = Arc::clone(&state.snapshot().manager);
            let host = request.uri().host().unwrap_or_default().to_owned();
            let page = request.uri().path().trim_start_matches('/').to_owned();
            let response = plugin_protocol::plugin_iframe_response(
                |plugin_id, page| {
                    let resolved = manager
                        .registrations()
                        .iter()
                        .find(|registration| registration.name == plugin_id)
                        .and_then(|registration| {
                            registration.nodes.iter().find_map(|node| {
                                let frontend = if page == plugin_protocol::VIEWER_PAGE {
                                    node.viewer.as_ref()
                                } else {
                                    node.frontend.as_ref()
                                };
                                frontend.map(|f| (f.kind.as_str(), f.content.as_deref()))
                            })
                        });
                    // 面板 iframe 加载诊断：每次 scheme 请求记录命中情况到
                    // 日志（GUI 无控制台时 logs/app.log 可查）。
                    log::info!(
                        "[plugin-iframe] {} {} -> {}",
                        host,
                        page,
                        if resolved.is_some() { "hit" } else { "miss" }
                    );
                    resolved
                },
                &host,
                &page,
            );
            responder.respond(response);
        },
    );

    builder
        .setup(|app| {
            // 读取宿主当前 UI 语言（store/settings.bin 的 locale 键），注入插件进程。
            // 语言在插件 spawn 时固定；宿主切换语言后需重启或重新加载插件才生效。
            let host_locale = match app.store("store/settings.bin") {
                Ok(store) => store
                    .get("locale")
                    .and_then(|v| v.as_str().map(String::from)),
                Err(_) => None,
            };

            // 注册共享节点注册表状态（内置节点 + 插件节点，全局构建一次）
            let node_registry_state = build_node_registry_state(app.handle(), host_locale);
            let stress_registry = Arc::clone(&node_registry_state.snapshot().registry);
            app.manage(node_registry_state);

            // 注册调试会话管理器状态
            app.manage(Arc::new(Mutex::new(DebugSessionManager::default())));

            // 注册压力测试会话管理器状态（共享注册表，使插件进程跨虚拟用户复用）
            app.manage(Arc::new(
                flow_engine::stress_test::StressTestSessionManager::new()
                    .with_registry(stress_registry),
            ));

            // 注册执行会话管理器状态（用于取消流程执行）
            app.manage(ExecutionSessionManager::new());

            // 注册数据驱动执行会话管理器状态（用于取消数据驱动执行）
            app.manage(DatasetExecutionSessionManager::new());


            // 注册 Ingress 流程级 Mock 服务监听管理器
            app.manage(Arc::new(flow_engine::ingress::IngressManager::new()));
            // 从 store 读取日志目录配置
            let log_dir = match app.store("store/settings.bin") {
                Ok(store) => store
                    .get("logDir")
                    .and_then(|v| v.as_str().map(PathBuf::from))
                    .unwrap_or_else(|| default_logs_dir(app.handle())),
                Err(_) => default_logs_dir(app.handle()),
            };

            // 确保目录存在
            let _ = std::fs::create_dir_all(&log_dir);

            // 设置日志级别（调试模式用 Debug，生产模式用 Info）
            let level = if cfg!(debug_assertions) {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            };

            // 注册 log 插件（生产模式也启用）
            let _ = app.handle().plugin(
                tauri_plugin_log::Builder::new()
                    .level(level)
                    // 文件 target（生产诊断）
                    .target(tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::Folder {
                            path: log_dir,
                            file_name: Some("app".into()),
                        },
                    ))
                    // Stdout target：插件加载失败等 warn/error 同时打到终端
                    // 控制台（开发诊断可见；无控制台的 GUI 环境自动 no-op）。
                    .target(tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::Stdout,
                    ))
                    .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                    .build(),
            );
            gui_ipc::start_gui_ipc_server(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_flow,
            execute_flow,
            execute_single_node,
            get_node_execution_detail,
            save_flow_report,
            load_flow_report,
            list_flow_reports,
            delete_flow_report,
            get_app_data_dir,
            open_directory,
            start_debug,
            send_debug_command,
            end_debug,
            get_debug_status,
            force_end_debug,
            // Ingress Mock 监听服务命令
            start_ingress_listener,
            stop_ingress_listener,
            get_active_ingress_ports,
            // 压力测试命令
            start_stress_test,
            stop_stress_test,
            get_stress_test_status,
            save_stress_test_report,
            load_stress_test_report,
            list_stress_test_reports,
            delete_stress_test_report,
            // SSE 连接测试命令
            #[cfg(feature = "protocol-sse")]
            sse_test_connection,
            // TCP 协议模板命令
            #[cfg(feature = "protocol-tcp")]
            tcp_list_templates,
            #[cfg(feature = "protocol-tcp")]
            tcp_get_template,
            // GraphQL 内省命令
            #[cfg(feature = "protocol-graphql")]
            graphql_introspect_schema,
            // 执行会话管理
            cancel_flow,
            // 流程验证命令
            validate_flow,
            // 流程文件存储命令
            list_flow_files,
            load_flow_file,
            save_flow_file,
            delete_flow_file,
            copy_flow_file,
            rename_flow_file,
            // 文件夹管理命令
            list_folders,
            create_folder,
            delete_folder,
            rename_folder,
            move_flow_to_folder,
            move_folder,
            // 节点注册表命令
            get_registered_node_types,
            get_node_config_schema,
            #[cfg(feature = "plugin-manager")]
            plugin_ui_call,
            // 数据源插件命令（列表 / browse / get_item / get_templates / validate_config）
            #[cfg(feature = "plugin-manager")]
            list_datasources,
            #[cfg(feature = "plugin-manager")]
            datasource_browse,
            #[cfg(feature = "plugin-manager")]
            datasource_get_item,
            #[cfg(feature = "plugin-manager")]
            datasource_get_templates,
            #[cfg(feature = "plugin-manager")]
            datasource_validate_config,
            // 插件设置（目录热重载 / 已安装列表 / 市场列表）
            #[cfg(feature = "plugin-manager")]
            get_plugin_dir_info,
            #[cfg(feature = "plugin-manager")]
            reload_plugins,
            #[cfg(feature = "plugin-manager")]
            list_installed_plugins,
            #[cfg(feature = "plugin-manager")]
            list_marketplace_plugins,
            #[cfg(feature = "plugin-manager")]
            install_marketplace_plugin,
            // CSV 模板命令
            save_csv_template,
            // 数据集执行命令
            validate_dataset_csv,
            execute_flow_with_dataset,
            cancel_dataset_execution,
            save_dataset_report,
            list_dataset_reports,
            load_dataset_report,
            load_dataset_row_detail,
            delete_dataset_report,
            // AI Copilot 命令
            ai_test_connection,
            ai_fetch_models,
            ai_chat_stream,
            ai_scan_workspace,
            ai_search_code,
            ai_read_code_slice,
            ai_inspect_api_spec,
            ai_execute_command,
            // 应用更新命令
            get_app_version,
            check_app_update,
            download_update_package,
            install_update_package,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                gui_ipc::cleanup_gui_instance_file();
            }
        });
}

#[cfg(test)]
mod flows_dir_tests {
    use super::resolve_flows_dir_from;
    use std::path::PathBuf;

    /// 设置项非空时优先使用设置项（工作区目录）
    #[test]
    fn resolve_flows_dir_prefers_configured_dir() {
        let resolved = resolve_flows_dir_from(
            Some("/workspace/mpe-flows"),
            Some(PathBuf::from(
                "/home/u/.local/share/com.multi-protocol-flow-executor",
            )),
        );
        assert_eq!(resolved, Some(PathBuf::from("/workspace/mpe-flows")));
    }

    /// 未配置时回退到「应用数据目录/flows」——它不是引擎内置的默认目录
    /// （`<data_dir>/multi-protocol-flow-executor/flows`），二者不一致会导致
    /// 前端列表可见而执行期按 ID 找不到子流程。
    #[test]
    fn resolve_flows_dir_falls_back_to_app_data_dir() {
        let resolved = resolve_flows_dir_from(
            None,
            Some(PathBuf::from(
                "/home/u/.local/share/com.multi-protocol-flow-executor",
            )),
        );
        assert_eq!(
            resolved,
            Some(PathBuf::from(
                "/home/u/.local/share/com.multi-protocol-flow-executor/flows"
            ))
        );
    }

    /// 空白设置值按未配置处理
    #[test]
    fn resolve_flows_dir_ignores_blank_setting() {
        let app_data = Some(PathBuf::from("/data/app"));
        assert_eq!(
            resolve_flows_dir_from(Some("   "), app_data.clone()),
            Some(PathBuf::from("/data/app/flows"))
        );
        assert_eq!(
            resolve_flows_dir_from(Some(""), app_data),
            Some(PathBuf::from("/data/app/flows"))
        );
    }

    /// 应用数据目录不可用时返回 None（调用方清除覆盖，回退引擎默认目录）
    #[test]
    fn resolve_flows_dir_none_when_unresolvable() {
        assert_eq!(resolve_flows_dir_from(None, None), None);
    }
}
