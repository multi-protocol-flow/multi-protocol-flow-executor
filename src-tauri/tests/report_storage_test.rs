//! 报告存储功能测试

use tempfile::TempDir;

/// 创建测试用的流程执行报告
fn create_test_report(uuid: &str, flow_name: &str) -> flow_engine::report::FlowExecutionReport {
    flow_engine::report::FlowExecutionReport {
        uuid: uuid.to_string(),
        flow_uuid: "test-flow-uuid".to_string(),
        flow_name: flow_name.to_string(),
        start_time: 1000,
        end_time: 2000,
        duration_ms: 1000,
        total_nodes: 1,
        executed_count: 1,
        status: flow_engine::report::ExecutionStatus::Success,
        node_reports: vec![],
        statistics: flow_engine::statistics::FlowStatistics {
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
        execution_mode: flow_engine::report::ExecutionMode::Normal,
        // 新增字段
        initial_variables: None,
        final_variables: None,
        execution_path: vec![],
        skipped_nodes: vec![],
        ws_streams: vec![],
        flow_snapshot: None,
        end_node_reached: None,
        flow_warning: None,
        plugin_streams: vec![],
    }
}

#[test]
fn test_save_report_success() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let reports_dir = temp_dir.path();

    let report = create_test_report("test-uuid-1", "测试流程");

    let result = flow_engine::storage::save_report(&report, reports_dir);
    assert!(result.is_ok(), "save report failed: {:?}", result.err());

    // 验证文件存在
    let file_path = reports_dir.join("test-uuid-1.json");
    assert!(file_path.exists(), "report file not created");
}

#[test]
fn test_load_report_success() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let reports_dir = temp_dir.path();

    let report = create_test_report("test-uuid-2", "测试流程");
    flow_engine::storage::save_report(&report, reports_dir).expect("failed to save report");

    let loaded = flow_engine::storage::load_report("test-uuid-2", reports_dir);
    assert!(loaded.is_ok(), "load report failed: {:?}", loaded.err());

    let loaded_report = loaded.expect("should load saved report successfully");
    assert_eq!(loaded_report.uuid, "test-uuid-2");
    assert_eq!(loaded_report.flow_name, "测试流程");
}

#[test]
fn test_load_report_not_found() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let reports_dir = temp_dir.path();

    let err = flow_engine::storage::load_report("non-existent-uuid", reports_dir)
        .expect_err("loading non-existent report should return error");
    assert!(
        err.contains("does not exist") || err.contains("not found"),
        "unexpected error: {}",
        err
    );
}

#[test]
fn test_list_reports_sorted() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let reports_dir = temp_dir.path();

    // 创建多个报告，不同时间戳
    let mut report1 = create_test_report("uuid-1", "流程1");
    report1.start_time = 1000;
    flow_engine::storage::save_report(&report1, reports_dir).expect("save failed");

    let mut report2 = create_test_report("uuid-2", "流程2");
    report2.start_time = 3000;
    flow_engine::storage::save_report(&report2, reports_dir).expect("save failed");

    let mut report3 = create_test_report("uuid-3", "流程3");
    report3.start_time = 2000;
    flow_engine::storage::save_report(&report3, reports_dir).expect("save failed");

    let result = flow_engine::storage::list_reports(reports_dir);
    assert!(result.is_ok(), "list reports failed: {:?}", result.err());

    let reports = result.expect("should list reports successfully");
    assert_eq!(reports.len(), 3);

    // 验证按时间戳倒序排序
    assert_eq!(reports[0].uuid, "uuid-2"); // timestamp 3000
    assert_eq!(reports[1].uuid, "uuid-3"); // timestamp 2000
    assert_eq!(reports[2].uuid, "uuid-1"); // timestamp 1000
}

#[test]
fn test_delete_report_success() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let reports_dir = temp_dir.path();

    let report = create_test_report("test-uuid-delete", "测试流程");
    flow_engine::storage::save_report(&report, reports_dir).expect("save failed");

    let result = flow_engine::storage::delete_report("test-uuid-delete", reports_dir);
    assert!(result.is_ok(), "delete report failed: {:?}", result.err());

    // 验证文件不存在
    let file_path = reports_dir.join("test-uuid-delete.json");
    assert!(!file_path.exists(), "report file should have been deleted");
}

#[test]
fn test_delete_report_not_found() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let reports_dir = temp_dir.path();

    let err = flow_engine::storage::delete_report("non-existent-uuid", reports_dir)
        .expect_err("deleting non-existent report should return error");
    assert!(
        err.contains("does not exist") || err.contains("not found"),
        "unexpected error: {}",
        err
    );
}

#[test]
fn test_list_reports_empty_directory() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let reports_dir = temp_dir.path();

    let result = flow_engine::storage::list_reports(reports_dir);
    assert!(result.is_ok(), "listing empty directory should succeed");
    assert_eq!(
        result
            .expect("listing empty directory should succeed")
            .len(),
        0
    );
}
