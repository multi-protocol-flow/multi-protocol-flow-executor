//! mpe flow subcommand — flow management (list/show/create/copy/delete/import/edit)

use std::path::Path;

use super::config::AppConfig;
use super::output::CliOutput;
use super::CliError;
use super::FlowCommands;

/// Execute flow management subcommand
pub async fn execute(_format: &str, command: FlowCommands) -> Result<(), CliError> {
    match command {
        FlowCommands::List { search, sort } => execute_list(search, &sort, _format).await,
        FlowCommands::Show { uuid_or_path } => execute_show(&uuid_or_path).await,
        FlowCommands::Create { name } => execute_create(&name).await,
        FlowCommands::Copy { uuid } => execute_copy(&uuid).await,
        FlowCommands::Delete { uuid } => execute_delete(&uuid).await,
        FlowCommands::Import { flow, name } => execute_import(&flow, &name).await,
        FlowCommands::Edit { command } => {
            // Delegate to flow_edit module
            super::flow_edit::execute(_format, command).await
        }
        FlowCommands::Fmt {
            path_or_uuid,
            to,
            all,
        } => execute_fmt(path_or_uuid.as_deref(), &to, all, _format).await,
    }
}

/// Format or convert flow files (.mpf) to YAML or JSON
async fn execute_fmt(
    path_or_uuid: Option<&str>,
    to_format: &str,
    all: bool,
    format_output: &str,
) -> Result<(), CliError> {
    let target_format = match to_format.to_lowercase().as_str() {
        "yaml" | "yml" => flow_engine::storage::FlowFormat::Yaml,
        "json" => flow_engine::storage::FlowFormat::Json,
        other => {
            return Err(CliError::execution(format!(
                "unsupported target format: '{}', expected 'yaml' or 'json'",
                other
            )))
        }
    };
    // 目标路径/目录判定
    let (target_dir, single_file_target) = match path_or_uuid {
        Some(target) => {
            let p = Path::new(target);
            if p.is_dir() {
                // 用户指定了目录（例如 mpe flow fmt /path/to/dir/）
                (Some(p.to_path_buf()), None)
            } else if p.is_file() {
                // 用户指定了具体文件
                (None, Some(p.to_path_buf()))
            } else if all {
                let config = AppConfig::load().map_err(CliError::Config)?;
                (Some(config.flows_dir), None)
            } else {
                let config = AppConfig::load().map_err(CliError::Config)?;
                let (_, target_path) =
                    flow_engine::storage::load_flow_file_with_path(target, &config.flows_dir)
                        .map_err(CliError::execution)?;
                (None, Some(target_path))
            }
        }
        None => {
            if all {
                let config = AppConfig::load().map_err(CliError::Config)?;
                (Some(config.flows_dir), None)
            } else {
                return Err(CliError::execution(
                    "please specify a flow file, directory, UUID, or use --all",
                ));
            }
        }
    };

    #[derive(serde::Serialize)]
    struct FmtResult {
        formatted: usize,
        format: String,
        files: Vec<String>,
    }

    let mut formatted_files = Vec::new();

    if let Some(dir) = target_dir {
        let list =
            flow_engine::storage::list_flow_files(&dir).map_err(CliError::execution)?;
        for meta in list {
            let path = std::path::PathBuf::from(&meta.file_path);
            if path.is_file() {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| CliError::Io(format!("failed to read {}: {}", path.display(), e)))?;
                let flow_file = flow_engine::storage::deserialize_flow_file(&content)
                    .map_err(CliError::execution)?;
                let output_content =
                    flow_engine::storage::serialize_flow_file(&flow_file, target_format)
                        .map_err(CliError::execution)?;
                std::fs::write(&path, output_content)
                    .map_err(|e| CliError::Io(format!("failed to write {}: {}", path.display(), e)))?;
                formatted_files.push(meta.file_path);
            }
        }
    } else if let Some(target_path) = single_file_target {
        let content = std::fs::read_to_string(&target_path)
            .map_err(|e| CliError::Io(format!("failed to read {}: {}", target_path.display(), e)))?;
        let flow_file = flow_engine::storage::deserialize_flow_file(&content)
            .map_err(CliError::execution)?;
        let output_content =
            flow_engine::storage::serialize_flow_file(&flow_file, target_format)
                .map_err(CliError::execution)?;
        std::fs::write(&target_path, output_content).map_err(|e| {
            CliError::Io(format!("failed to write {}: {}", target_path.display(), e))
        })?;
        formatted_files.push(target_path.to_string_lossy().to_string());
    }
    let result = FmtResult {
        formatted: formatted_files.len(),
        format: to_format.to_lowercase(),
        files: formatted_files,
    };

    if format_output == "table" {
        println!(
            "Successfully formatted {} flow(s) to {}",
            result.formatted, result.format
        );
        for f in &result.files {
            println!("  {}", f);
        }
    } else {
        let output = CliOutput::success(result);
        super::output::output_json(&output);
    }
    Ok(())
}

/// List flows
async fn execute_list(search: Option<String>, sort: &str, format: &str) -> Result<(), CliError> {
    let config = AppConfig::load().map_err(CliError::Config)?;
    let flows_dir = &config.flows_dir;

    let mut list = flow_engine::storage::list_flow_files(flows_dir).map_err(CliError::execution)?;

    // Search filter
    if let Some(ref keyword) = search {
        let kw = keyword.to_lowercase();
        list.retain(|m| m.name.to_lowercase().contains(&kw));
    }

    // Sort
    match sort {
        "name" => list.sort_by(|a, b| a.name.cmp(&b.name)),
        _ => list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
    }

    if format == "table" {
        let headers = ["UUID", "Name", "Updated"];
        let rows: Vec<Vec<String>> = list
            .iter()
            .map(|m| {
                let dt = chrono::DateTime::from_timestamp_millis(m.updated_at as i64)
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| m.updated_at.to_string());
                vec![m.uuid.chars().take(8).collect(), m.name.clone(), dt]
            })
            .collect();
        print!("{}", super::output::format_table(&headers, &rows));
    } else {
        let output = CliOutput::success(list);
        super::output::output_json(&output);
    }
    Ok(())
}

/// Show flow details
async fn execute_show(uuid_or_path: &str) -> Result<(), CliError> {
    let config = AppConfig::load().map_err(CliError::Config)?;
    let flows_dir = &config.flows_dir;

    let flow = if is_file_path(uuid_or_path) {
        // Direct load from file path
        flow_engine::storage::load_flow(uuid_or_path).map_err(CliError::execution)?
    } else {
        // Load by UUID
        flow_engine::storage::load_flow_file(uuid_or_path, flows_dir)
            .map_err(CliError::execution)?
    };

    let flow_file = flow_engine::storage::FlowFile {
        _version: 1,
        flow,
        _meta: None,
    };
    let output = CliOutput::success(flow_file);
    super::output::output_json(&output);
    Ok(())
}

/// Create flow
async fn execute_create(name: &str) -> Result<(), CliError> {
    let config = AppConfig::load().map_err(CliError::Config)?;
    let flows_dir = &config.flows_dir;

    // Ensure directory exists
    std::fs::create_dir_all(flows_dir)
        .map_err(|e| CliError::Io(format!("failed to create flow directory: {}", e)))?;

    let input = flow_engine::command::CreateFlowCommandInput {
        name: name.to_string(),
    };
    let output = flow_engine::commands::create_flow(input);

    flow_engine::storage::save_flow_file(&output.flow, flows_dir).map_err(CliError::execution)?;

    let flow_file = flow_engine::storage::FlowFile {
        _version: 1,
        flow: output.flow,
        _meta: None,
    };
    let cli_output = CliOutput::success(flow_file);
    super::output::output_json(&cli_output);
    Ok(())
}

/// Copy flow
async fn execute_copy(uuid: &str) -> Result<(), CliError> {
    let config = AppConfig::load().map_err(CliError::Config)?;
    let flows_dir = &config.flows_dir;

    let copied =
        flow_engine::storage::copy_flow_file(uuid, flows_dir).map_err(CliError::execution)?;

    let flow_file = flow_engine::storage::FlowFile {
        _version: 1,
        flow: copied,
        _meta: None,
    };
    let output = CliOutput::success(flow_file);
    super::output::output_json(&output);
    Ok(())
}

/// Delete flow
async fn execute_delete(uuid: &str) -> Result<(), CliError> {
    let config = AppConfig::load().map_err(CliError::Config)?;
    let flows_dir = &config.flows_dir;

    flow_engine::storage::delete_flow_file(uuid, flows_dir).map_err(CliError::execution)?;

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

/// Import flow file
async fn execute_import(flow_path: &str, name: &str) -> Result<(), CliError> {
    let config = AppConfig::load().map_err(CliError::Config)?;
    let flows_dir = &config.flows_dir;

    // Ensure directory exists
    std::fs::create_dir_all(flows_dir)
        .map_err(|e| CliError::Io(format!("failed to create flow directory: {}", e)))?;

    // Load external flow file
    let mut flow = flow_engine::storage::load_flow(flow_path).map_err(CliError::execution)?;

    // Update name, generate new UUID
    flow.name = name.to_string();
    flow.uuid = uuid::Uuid::new_v4().to_string();

    // Save to flows_dir
    flow_engine::storage::save_flow_file(&flow, flows_dir).map_err(CliError::execution)?;

    let flow_file = flow_engine::storage::FlowFile {
        _version: 1,
        flow,
        _meta: None,
    };
    let output = CliOutput::success(flow_file);
    super::output::output_json(&output);
    Ok(())
}

/// Check if string is a file path (not UUID)
fn is_file_path(s: &str) -> bool {
    s.contains('/') || s.contains('\\') || s.ends_with(".mpf") || Path::new(s).extension().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_execute_fmt_roundtrip() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let json_content = r#"{
  "_version": 1,
  "flow": {
    "uuid": "test-fmt-001",
    "name": "Fmt Test Flow",
    "nodes": [],
    "connections": []
  }
}"#;
        temp_file.write_all(json_content.as_bytes()).unwrap();
        let path = temp_file.path().to_str().unwrap();

        // Convert to YAML
        execute_fmt(Some(path), "yaml", false, "json").await.unwrap();

        let yaml_content = std::fs::read_to_string(path).unwrap();
        assert_eq!(
            flow_engine::storage::detect_flow_format(&yaml_content),
            flow_engine::storage::FlowFormat::Yaml
        );
        assert!(yaml_content.contains("test-fmt-001"));

        // Convert back to JSON
        execute_fmt(Some(path), "json", false, "json").await.unwrap();

        let converted_json = std::fs::read_to_string(path).unwrap();
        assert_eq!(
            flow_engine::storage::detect_flow_format(&converted_json),
            flow_engine::storage::FlowFormat::Json
        );
        assert!(converted_json.contains("test-fmt-001"));
    }
}
