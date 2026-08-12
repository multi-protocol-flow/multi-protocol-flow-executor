//! mpe validate subcommand

use std::path::PathBuf;

use super::output;
use super::output::CliOutput;

/// Validate flow
pub async fn execute(
    _format: &str,
    flow_path: &str,
    registry: std::sync::Arc<flow_engine::registry::NodeRegistry>,
) -> Result<(), super::CliError> {
    let path = PathBuf::from(flow_path);
    if !path.exists() {
        return Err(super::CliError::InvalidArgs(format!(
            "Flow file not found: {}",
            flow_path
        )));
    }

    // Load flow
    let flow = flow_engine::storage::load_flow(flow_path)
        .map_err(|e| super::CliError::Execution(format!("Failed to load flow: {}", e)))?;

    // Validate flow with registry (structural + plugin-level validation).
    // 共享 registry（内置 + 插件）来自 build_shared_registry —— 插件节点
    // （如 db:connect）在 validate 时同样按已注册插件校验。
    let result =
        flow_engine::validator::FlowValidator::new().validate_with_registry(&flow, &registry);

    let output_data = serde_json::json!({
        "valid": result.valid,
        "errors": result.errors,
    });

    if result.valid {
        let cli_output = CliOutput::success(output_data);
        output::output_json(&cli_output);
        Ok(())
    } else {
        // 插件节点（如 db:*）校验依赖插件已安装（MPE_PLUGIN_DIR /
        // data_dir/plugins / ./plugins）；未知类型错误给出发现提示。
        let hint = if result
            .errors
            .iter()
            .any(|e| e.message.contains("unknown type"))
        {
            "; hint: plugin nodes (e.g. db:*) validate only when the plugin is installed (set MPE_PLUGIN_DIR or place it under ./plugins / data_dir/plugins)"
        } else {
            ""
        };
        let cli_output = CliOutput {
            success: false,
            data: Some(output_data),
            error: Some(format!(
                "Validation failed: {} error(s){}",
                result.errors.len(),
                hint
            )),
        };
        output::output_json(&cli_output);
        Err(super::CliError::Execution(
            "Flow validation failed".to_string(),
        ))
    }
}
