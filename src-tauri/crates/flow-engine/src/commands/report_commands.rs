//! 报告存储命令

use std::path::Path;

use crate::report::{FlowExecutionReport, ReportMetadata};
use crate::storage;

/// 保存流程执行报告
///
/// # 参数
/// - `report`: 报告对象
/// - `reports_dir`: 报告存储目录
pub fn save_flow_report(report: FlowExecutionReport, reports_dir: &str) -> Result<(), String> {
    let reports_path = Path::new(reports_dir);
    storage::save_report(&report, reports_path)
}

/// 加载流程执行报告
///
/// # 参数
/// - `uuid`: 报告 UUID
/// - `reports_dir`: 报告存储目录
pub fn load_flow_report(uuid: &str, reports_dir: &str) -> Result<FlowExecutionReport, String> {
    let reports_path = Path::new(reports_dir);
    storage::load_report(uuid, reports_path)
}

/// 列出所有报告元数据
///
/// # 参数
/// - `reports_dir`: 报告存储目录
pub fn list_flow_reports(reports_dir: &str) -> Result<Vec<ReportMetadata>, String> {
    let reports_path = Path::new(reports_dir);
    storage::list_reports(reports_path)
}

/// 删除流程执行报告
///
/// # 参数
/// - `uuid`: 报告 UUID
/// - `reports_dir`: 报告存储目录
pub fn delete_flow_report(uuid: &str, reports_dir: &str) -> Result<(), String> {
    let reports_path = Path::new(reports_dir);
    storage::delete_report(uuid, reports_path)
}
