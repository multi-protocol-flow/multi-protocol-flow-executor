//! mpe flow 子命令 — 流程管理 (list/show/create/copy/delete/import/edit)

use std::path::Path;

use super::config::AppConfig;
use super::output::CliOutput;
use super::FlowCommands;
use super::CliError;

/// 执行 flow 管理子命令
pub async fn execute(
    _format: &str,
    command: FlowCommands,
) -> Result<(), CliError> {
    match command {
        FlowCommands::List { search, sort } => execute_list(search, &sort, _format).await,
        FlowCommands::Show { uuid_or_path } => execute_show(&uuid_or_path).await,
        FlowCommands::Create { name } => execute_create(&name).await,
        FlowCommands::Copy { uuid } => execute_copy(&uuid).await,
        FlowCommands::Delete { uuid } => execute_delete(&uuid).await,
        FlowCommands::Import { flow, name } => execute_import(&flow, &name).await,
        FlowCommands::Edit { command } => {
            // 委托给 flow_edit 模块
            super::flow_edit::execute(_format, command).await
        }
    }
}

/// 列出流程
async fn execute_list(
    search: Option<String>,
    sort: &str,
    format: &str,
) -> Result<(), CliError> {
    let config = AppConfig::load().map_err(CliError::Config)?;
    let flows_dir = &config.flows_dir;

    let mut list = crate::storage::list_flow_files(flows_dir)
        .map_err(CliError::Execution)?;

    // 搜索过滤
    if let Some(ref keyword) = search {
        let kw = keyword.to_lowercase();
        list.retain(|m| m.name.to_lowercase().contains(&kw));
    }

    // 排序
    match sort {
        "name" => list.sort_by(|a, b| a.name.cmp(&b.name)),
        _ => list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
    }

    if format == "table" {
        let headers = ["UUID", "名称", "更新时间"];
        let rows: Vec<Vec<String>> = list
            .iter()
            .map(|m| {
                let dt = chrono::DateTime::from_timestamp_millis(m.updated_at as i64)
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| m.updated_at.to_string());
                vec![
                    m.uuid.chars().take(8).collect(),
                    m.name.clone(),
                    dt,
                ]
            })
            .collect();
        print!("{}", super::output::format_table(&headers, &rows));
    } else {
        let output = CliOutput::success(list);
        super::output::output_json(&output);
    }
    Ok(())
}

/// 查看流程详情
async fn execute_show(uuid_or_path: &str) -> Result<(), CliError> {
    let config = AppConfig::load().map_err(CliError::Config)?;
    let flows_dir = &config.flows_dir;

    let flow = if is_file_path(uuid_or_path) {
        // 文件路径直接加载
        crate::storage::load_flow(uuid_or_path)
            .map_err(CliError::Execution)?
    } else {
        // UUID 加载
        crate::storage::load_flow_file(uuid_or_path, flows_dir)
            .map_err(CliError::Execution)?
    };

    let output = CliOutput::success(flow);
    super::output::output_json(&output);
    Ok(())
}

/// 创建流程
async fn execute_create(name: &str) -> Result<(), CliError> {
    let config = AppConfig::load().map_err(CliError::Config)?;
    let flows_dir = &config.flows_dir;

    // 确保目录存在
    std::fs::create_dir_all(flows_dir)
        .map_err(|e| CliError::Io(format!("创建流程目录失败: {}", e)))?;

    let input = crate::command::CreateFlowCommandInput {
        name: name.to_string(),
    };
    let output = crate::commands::create_flow(input);

    crate::storage::save_flow_file(&output.flow, flows_dir)
        .map_err(CliError::Execution)?;

    let cli_output = CliOutput::success(output.flow);
    super::output::output_json(&cli_output);
    Ok(())
}

/// 复制流程
async fn execute_copy(uuid: &str) -> Result<(), CliError> {
    let config = AppConfig::load().map_err(CliError::Config)?;
    let flows_dir = &config.flows_dir;

    let copied = crate::storage::copy_flow_file(uuid, flows_dir)
        .map_err(CliError::Execution)?;

    let output = CliOutput::success(copied);
    super::output::output_json(&output);
    Ok(())
}

/// 删除流程
async fn execute_delete(uuid: &str) -> Result<(), CliError> {
    let config = AppConfig::load().map_err(CliError::Config)?;
    let flows_dir = &config.flows_dir;

    crate::storage::delete_flow_file(uuid, flows_dir)
        .map_err(CliError::Execution)?;

    #[derive(serde::Serialize)]
    struct DeletedResult {
        deleted: String,
    }

    let output = CliOutput::success(DeletedResult {
        deleted: uuid.to_string(),
    });
    super::output::output_json(&output);
    Ok(())
}

/// 导入流程文件
async fn execute_import(flow_path: &str, name: &str) -> Result<(), CliError> {
    let config = AppConfig::load().map_err(CliError::Config)?;
    let flows_dir = &config.flows_dir;

    // 确保目录存在
    std::fs::create_dir_all(flows_dir)
        .map_err(|e| CliError::Io(format!("创建流程目录失败: {}", e)))?;

    // 加载外部流程文件
    let mut flow = crate::storage::load_flow(flow_path)
        .map_err(CliError::Execution)?;

    // 修改名称，生成新 UUID
    flow.name = name.to_string();
    flow.uuid = uuid::Uuid::new_v4().to_string();

    // 保存到 flows_dir
    crate::storage::save_flow_file(&flow, flows_dir)
        .map_err(CliError::Execution)?;

    let output = CliOutput::success(flow);
    super::output::output_json(&output);
    Ok(())
}

/// 判断字符串是否为文件路径（而非 UUID）
fn is_file_path(s: &str) -> bool {
    s.contains('/') || s.contains('\\') || s.ends_with(".mpf")
        || Path::new(s).extension().is_some()
}
