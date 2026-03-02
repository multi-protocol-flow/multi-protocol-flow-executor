//! 报告存储功能测试

use tempfile::TempDir;

/// 创建测试用的流程执行报告
fn create_test_report(uuid: &str, flow_name: &str) -> app_lib::report::FlowExecutionReport {
    app_lib::report::FlowExecutionReport {
        uuid: uuid.to_string(),
        flow_uuid: "test-flow-uuid".to_string(),
        flow_name: flow_name.to_string(),
        start_time: 1000,
        end_time: 2000,
        duration_ms: 1000,
        total_nodes: 1,
        executed_count: 1,
        status: app_lib::report::ExecutionStatus::Success,
        node_reports: vec![],
        statistics: app_lib::statistics::FlowStatistics {
            success_count: 1,
            failed_count: 0,
            avg_node_duration_ms: Some(1000),
            max_node_duration_ms: Some(1000),
            min_node_duration_ms: Some(1000),
            total_input_size_bytes: Some(0),
            total_output_size_bytes: Some(0),
            error_rate: Some(0.0),
            completion_rate: Some(1.0),
        },
        flow_error: None,
    }
}

#[test]
fn test_save_report_success() {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let reports_dir = temp_dir.path();

    let report = create_test_report("test-uuid-1", "测试流程");

    let result = app_lib::save_report(&report, reports_dir);
    assert!(result.is_ok(), "保存报告失败: {:?}", result.err());

    // 验证文件存在
    let file_path = reports_dir.join("test-uuid-1.json");
    assert!(file_path.exists(), "报告文件未创建");
}

#[test]
fn test_load_report_success() {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let reports_dir = temp_dir.path();

    let report = create_test_report("test-uuid-2", "测试流程");
    app_lib::save_report(&report, reports_dir).expect("保存报告失败");

    let loaded = app_lib::load_report("test-uuid-2", reports_dir);
    assert!(loaded.is_ok(), "加载报告失败: {:?}", loaded.err());

    let loaded_report = loaded.unwrap();
    assert_eq!(loaded_report.uuid, "test-uuid-2");
    assert_eq!(loaded_report.flow_name, "测试流程");
}

#[test]
fn test_load_report_not_found() {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let reports_dir = temp_dir.path();

    let result = app_lib::load_report("non-existent-uuid", reports_dir);
    assert!(result.is_err(), "加载不存在的报告应该返回错误");
    assert!(result.unwrap_err().contains("不存在"));
}

#[test]
fn test_list_reports_sorted() {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let reports_dir = temp_dir.path();

    // 创建多个报告，不同时间戳
    let mut report1 = create_test_report("uuid-1", "流程1");
    report1.start_time = 1000;
    app_lib::save_report(&report1, reports_dir).expect("保存失败");

    let mut report2 = create_test_report("uuid-2", "流程2");
    report2.start_time = 3000;
    app_lib::save_report(&report2, reports_dir).expect("保存失败");

    let mut report3 = create_test_report("uuid-3", "流程3");
    report3.start_time = 2000;
    app_lib::save_report(&report3, reports_dir).expect("保存失败");

    let result = app_lib::list_reports(reports_dir);
    assert!(result.is_ok(), "列出报告失败: {:?}", result.err());

    let reports = result.unwrap();
    assert_eq!(reports.len(), 3);

    // 验证按时间戳倒序排序
    assert_eq!(reports[0].uuid, "uuid-2"); // timestamp 3000
    assert_eq!(reports[1].uuid, "uuid-3"); // timestamp 2000
    assert_eq!(reports[2].uuid, "uuid-1"); // timestamp 1000
}

#[test]
fn test_delete_report_success() {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let reports_dir = temp_dir.path();

    let report = create_test_report("test-uuid-delete", "测试流程");
    app_lib::save_report(&report, reports_dir).expect("保存失败");

    let result = app_lib::delete_report("test-uuid-delete", reports_dir);
    assert!(result.is_ok(), "删除报告失败: {:?}", result.err());

    // 验证文件不存在
    let file_path = reports_dir.join("test-uuid-delete.json");
    assert!(!file_path.exists(), "报告文件应该被删除");
}

#[test]
fn test_delete_report_not_found() {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let reports_dir = temp_dir.path();

    let result = app_lib::delete_report("non-existent-uuid", reports_dir);
    assert!(result.is_err(), "删除不存在的报告应该返回错误");
    assert!(result.unwrap_err().contains("不存在"));
}

#[test]
fn test_list_reports_empty_directory() {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let reports_dir = temp_dir.path();

    let result = app_lib::list_reports(reports_dir);
    assert!(result.is_ok(), "列出空目录应该成功");
    assert_eq!(result.unwrap().len(), 0);
}
