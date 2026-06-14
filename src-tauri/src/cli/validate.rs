//! mpe validate 子命令

use std::path::PathBuf;

use super::output;
use super::output::CliOutput;

/// 验证流程
pub async fn execute(
    _format: &str,
    flow_path: &str,
) -> Result<(), super::CliError> {
    let path = PathBuf::from(flow_path);
    if !path.exists() {
        return Err(super::CliError::InvalidArgs(format!(
            "流程文件不存在: {}",
            flow_path
        )));
    }

    // 加载流程
    let flow = crate::load_flow(flow_path)
        .map_err(|e| super::CliError::Execution(format!("加载流程失败: {}", e)))?;

    // 验证流程
    let result = crate::validator::FlowValidator::new().validate(&flow);

    let output_data = serde_json::json!({
        "valid": result.valid,
        "errors": result.errors,
    });

    if result.valid {
        let cli_output = CliOutput::success(output_data);
        output::output_json(&cli_output);
        Ok(())
    } else {
        let cli_output = CliOutput {
            success: false,
            data: Some(output_data),
            error: Some(format!("Validation failed: {} error(s)", result.errors.len())),
        };
        output::output_json(&cli_output);
        Err(super::CliError::Execution("Flow validation failed".to_string()))
    }
}
