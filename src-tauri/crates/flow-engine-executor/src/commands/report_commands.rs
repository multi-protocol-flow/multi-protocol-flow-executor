//! 报告存储命令

use std::path::Path;

use crate::storage;
use flow_engine_core::report::{FlowExecutionReport, ReportMetadata};

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

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::report::{
        ExecutionMode, ExecutionStatus, FlowExecutionReport, NodeExecutionReport,
    };
    use flow_engine_core::statistics::FlowStatistics;
    use std::fs;
    use tempfile::TempDir;

    /// 创建测试用的流程执行报告
    fn create_test_report(uuid: &str, status: ExecutionStatus) -> FlowExecutionReport {
        FlowExecutionReport {
            uuid: uuid.to_string(),
            flow_uuid: "test-flow-uuid".to_string(),
            flow_name: "测试流程".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            total_nodes: 2,
            executed_count: 2,
            status,
            node_reports: vec![
                NodeExecutionReport {
                    node_uuid: "node-1".to_string(),
                    node_type: "entry".to_string(),
                    node_name: "入口节点".to_string(),
                    start_time: 1000,
                    end_time: 1100,
                    duration_ms: 100,
                    status: ExecutionStatus::Success,
                    input_data: None,
                    output_data: Some(serde_json::json!({"result": "ok"})),
                    used_port_id: Some("out".to_string()),
                    error: None,
                    resolved_config: None,
                    variable_changes: None,
                    skip_reason: None,
                    wait_duration_ms: None,
                },
                NodeExecutionReport {
                    node_uuid: "node-2".to_string(),
                    node_type: "http".to_string(),
                    node_name: "HTTP请求".to_string(),
                    start_time: 1100,
                    end_time: 2000,
                    duration_ms: 900,
                    status: ExecutionStatus::Success,
                    input_data: Some(serde_json::json!({"url": "https://example.com"})),
                    output_data: Some(serde_json::json!({"status": 200})),
                    used_port_id: Some("true".to_string()),
                    error: None,
                    resolved_config: None,
                    variable_changes: None,
                    skip_reason: None,
                    wait_duration_ms: None,
                },
            ],
            statistics: FlowStatistics::default(),
            flow_error: None,
            execution_mode: ExecutionMode::Normal,
            initial_variables: Some(serde_json::json!({"api_key": "test"})),
            final_variables: Some(serde_json::json!({"api_key": "test", "response": "ok"})),
            execution_path: vec!["node-1".to_string(), "node-2".to_string()],
            skipped_nodes: vec![],
            ws_streams: vec![],
            flow_snapshot: None,
            end_node_reached: None,
            flow_warning: None,
        }
    }

    /// 测试保存流程执行报告
    #[test]
    fn test_save_flow_report() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path().to_string_lossy().to_string();

        let report = create_test_report("test-uuid-001", ExecutionStatus::Success);
        let result = save_flow_report(report, &reports_dir);

        assert!(result.is_ok(), "保存报告应该成功");

        // 验证文件已创建
        let report_path = temp_dir.path().join("test-uuid-001.json");
        assert!(report_path.exists(), "报告文件应该存在");
    }

    /// 测试保存调试模式的报告
    #[test]
    fn test_save_flow_report_debug_mode() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path().to_string_lossy().to_string();

        let mut report = create_test_report("debug-uuid-001", ExecutionStatus::Success);
        report.execution_mode = ExecutionMode::Debug;

        let result = save_flow_report(report, &reports_dir);

        assert!(result.is_ok(), "保存调试报告应该成功");

        // 验证调试报告文件名格式
        let report_path = temp_dir.path().join("debug_debug-uuid-001.json");
        assert!(report_path.exists(), "调试报告文件应该存在");
    }

    /// 测试加载流程执行报告
    #[test]
    fn test_load_flow_report() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path().to_string_lossy().to_string();

        // 先保存报告
        let original_report = create_test_report("test-uuid-002", ExecutionStatus::Success);
        save_flow_report(original_report.clone(), &reports_dir).expect("保存报告失败");

        // 加载报告
        let result = load_flow_report("test-uuid-002", &reports_dir);

        assert!(result.is_ok(), "加载报告应该成功");
        let loaded_report = result.unwrap();

        // 验证报告内容
        assert_eq!(loaded_report.uuid, "test-uuid-002");
        assert_eq!(loaded_report.flow_uuid, "test-flow-uuid");
        assert_eq!(loaded_report.flow_name, "测试流程");
        assert_eq!(loaded_report.total_nodes, 2);
        assert_eq!(loaded_report.node_reports.len(), 2);
    }

    /// 测试加载不存在的报告
    #[test]
    fn test_load_flow_report_not_found() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path().to_string_lossy().to_string();

        let result = load_flow_report("non-existent-uuid", &reports_dir);

        assert!(result.is_err(), "加载不存在的报告应该失败");
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("不存在"), "错误消息应包含'不存在'");
    }

    /// 测试列出报告元数据
    #[test]
    fn test_list_flow_reports() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path().to_string_lossy().to_string();

        // 保存多个报告
        let report1 = create_test_report("list-test-001", ExecutionStatus::Success);
        let report2 = create_test_report("list-test-002", ExecutionStatus::Failed);

        save_flow_report(report1, &reports_dir).expect("保存报告1失败");
        save_flow_report(report2, &reports_dir).expect("保存报告2失败");

        // 列出报告
        let result = list_flow_reports(&reports_dir);

        assert!(result.is_ok(), "列出报告应该成功");
        let reports = result.unwrap();
        assert_eq!(reports.len(), 2, "应该有2个报告");

        // 验证报告元数据包含正确的字段
        for metadata in &reports {
            assert!(!metadata.uuid.is_empty(), "UUID不应为空");
            assert!(!metadata.flow_name.is_empty(), "流程名称不应为空");
            assert!(matches!(metadata.status.as_str(), "success" | "failed"));
        }
    }

    /// 测试列出空目录的报告
    #[test]
    fn test_list_flow_reports_empty_directory() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path().to_string_lossy().to_string();

        let result = list_flow_reports(&reports_dir);

        assert!(result.is_ok(), "列出空目录报告应该成功");
        let reports = result.unwrap();
        assert!(reports.is_empty(), "空目录应该返回空列表");
    }

    /// 测试删除流程执行报告
    #[test]
    fn test_delete_flow_report() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path().to_string_lossy().to_string();

        // 先保存报告
        let report = create_test_report("delete-test-001", ExecutionStatus::Success);
        save_flow_report(report, &reports_dir).expect("保存报告失败");

        // 确认文件存在
        let report_path = temp_dir.path().join("delete-test-001.json");
        assert!(report_path.exists(), "报告文件应该存在");

        // 删除报告
        let result = delete_flow_report("delete-test-001", &reports_dir);

        assert!(result.is_ok(), "删除报告应该成功");
        assert!(!report_path.exists(), "删除后文件不应该存在");
    }

    /// 测试删除不存在的报告
    #[test]
    fn test_delete_flow_report_not_found() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path().to_string_lossy().to_string();

        let result = delete_flow_report("non-existent-delete", &reports_dir);

        assert!(result.is_err(), "删除不存在的报告应该失败");
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("不存在"), "错误消息应包含'不存在'");
    }

    /// 测试报告保存和加载的完整周期
    #[test]
    fn test_report_save_load_roundtrip() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path().to_string_lossy().to_string();

        // 创建包含完整数据的报告
        let original_report = create_test_report("roundtrip-001", ExecutionStatus::Success);

        // 保存
        save_flow_report(original_report.clone(), &reports_dir).expect("保存报告失败");

        // 加载
        let loaded_report = load_flow_report("roundtrip-001", &reports_dir).expect("加载报告失败");

        // 验证所有字段保持一致
        assert_eq!(loaded_report.uuid, original_report.uuid);
        assert_eq!(loaded_report.flow_uuid, original_report.flow_uuid);
        assert_eq!(loaded_report.flow_name, original_report.flow_name);
        assert_eq!(loaded_report.start_time, original_report.start_time);
        assert_eq!(loaded_report.end_time, original_report.end_time);
        assert_eq!(loaded_report.duration_ms, original_report.duration_ms);
        assert_eq!(loaded_report.total_nodes, original_report.total_nodes);
        assert_eq!(loaded_report.executed_count, original_report.executed_count);
        assert_eq!(loaded_report.status, original_report.status);
        assert_eq!(
            loaded_report.node_reports.len(),
            original_report.node_reports.len()
        );
        assert_eq!(loaded_report.execution_mode, original_report.execution_mode);
        assert_eq!(loaded_report.execution_path, original_report.execution_path);
    }

    /// 测试报告格式验证 - 验证序列化/反序列化正确性
    #[test]
    fn test_report_format_validation() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path().to_string_lossy().to_string();

        // 创建报告
        let report = create_test_report("format-test-001", ExecutionStatus::Success);
        save_flow_report(report, &reports_dir).expect("保存报告失败");

        // 直接读取文件验证 JSON 格式
        let report_path = temp_dir.path().join("format-test-001.json");
        let content = fs::read_to_string(&report_path).expect("读取文件失败");

        // 验证是有效的 JSON
        let json_value: serde_json::Value =
            serde_json::from_str(&content).expect("文件内容应该是有效的 JSON");

        // 验证关键字段存在
        assert!(json_value.get("uuid").is_some(), "应包含 uuid 字段");
        assert!(
            json_value.get("flow_uuid").is_some(),
            "应包含 flow_uuid 字段"
        );
        assert!(
            json_value.get("flow_name").is_some(),
            "应包含 flow_name 字段"
        );
        assert!(json_value.get("status").is_some(), "应包含 status 字段");
        assert!(
            json_value.get("node_reports").is_some(),
            "应包含 node_reports 字段"
        );
        assert!(
            json_value.get("statistics").is_some(),
            "应包含 statistics 字段"
        );
    }

    /// 测试报告存储到自定义目录
    #[test]
    fn test_report_storage_custom_directory() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let custom_dir = temp_dir.path().join("custom_reports");
        let reports_dir = custom_dir.to_string_lossy().to_string();

        // 自定义目录不存在，应该自动创建
        let report = create_test_report("custom-dir-001", ExecutionStatus::Success);
        let result = save_flow_report(report, &reports_dir);

        assert!(result.is_ok(), "保存到自定义目录应该成功");
        assert!(custom_dir.exists(), "自定义目录应该被创建");
    }

    /// 测试失败状态报告的处理
    #[test]
    fn test_failed_status_report() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let reports_dir = temp_dir.path().to_string_lossy().to_string();

        let report = create_test_report("failed-001", ExecutionStatus::Failed);
        save_flow_report(report, &reports_dir).expect("保存失败状态报告失败");

        let loaded = load_flow_report("failed-001", &reports_dir).expect("加载报告失败");
        assert_eq!(loaded.status, ExecutionStatus::Failed);

        // 验证列表中的状态
        let reports = list_flow_reports(&reports_dir).expect("列出报告失败");
        assert_eq!(reports[0].status, "failed");
    }
}
