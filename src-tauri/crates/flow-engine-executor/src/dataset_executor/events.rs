// 数据驱动执行事件与报告生成
//!
//! 定义 [`DatasetEvent`] 枚举，用于在执行过程中向前端发送实时事件。
//! 提供报告生成（含百分位统计）和报告持久化功能。

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use flow_engine_core::dataset::{DatasetExecutionReport, RowStatus, RowSummary};
use flow_engine_core::report::FlowExecutionReport;

// ============================================================================
// 事件定义
// ============================================================================

/// 数据驱动执行事件（从后端发送到前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DatasetEvent {
    /// 执行开始
    Started {
        /// 总行数
        total_rows: u32,
        /// 数据集名称（文件名）
        dataset_name: String,
    },
    /// 行execution complete（Success）
    RowCompleted {
        /// 行索引
        row_index: u32,
        /// 行执行状态
        status: RowStatus,
        /// 执行耗时（毫秒）
        duration_ms: u64,
    },
    /// 行执行Failed
    RowFailed {
        /// 行索引
        row_index: u32,
        /// Error信息
        error: String,
        /// 执行耗时（毫秒）
        duration_ms: u64,
    },
    /// 进度更新
    Progress {
        /// 已完成行数
        completed: u32,
        /// 总行数
        total: u32,
        /// Success数
        success_count: u32,
        /// Failed数（Fail 状态）
        fail_count: u32,
        /// Error数（Error 状态）
        error_count: u32,
        /// 已消耗时间（毫秒）
        elapsed_ms: u64,
        /// 预计剩余时间（毫秒）
        estimated_remaining_ms: Option<u64>,
    },
    /// 执行结束
    Ended {
        /// 最终报告
        report: Box<DatasetExecutionReport>,
    },
    /// 执行取消
    Cancelled {
        /// 部分报告（已执行的行）
        report: Box<DatasetExecutionReport>,
    },
}

// ============================================================================
// 发送事件辅助函数
// ============================================================================

/// 尝试发送事件，忽略发送Failed（接收端已关闭）
pub fn send_event(
    event_sender: &Option<tokio::sync::mpsc::UnboundedSender<DatasetEvent>>,
    event: DatasetEvent,
) {
    if let Some(sender) = event_sender {
        let _ = sender.send(event);
    }
}

// ============================================================================
// 报告生成
// ============================================================================

/// 从已排序的持续时间数据计算指定百分位数（线性插值）
///
/// 与 `stress_test::metrics::calculate_percentile_from_sorted` 算法一致。
fn calculate_percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let n = sorted.len();
    let rank = (p / 100.0) * (n - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper || upper >= n {
        return sorted[lower.min(n - 1)];
    }
    let fraction = rank - lower as f64;
    let lower_val = sorted[lower] as f64;
    let upper_val = sorted[upper] as f64;
    (lower_val + fraction * (upper_val - lower_val)).round() as u64
}

/// 生成数据驱动执行报告
///
/// # 参数
/// - `summaries`: 所有已执行行的摘要
/// - `flow_uuid`: 流程 UUID
/// - `flow_name`: 流程名称
/// - `dataset_path`: 数据集文件路径
/// - `dataset_name`: 数据集名称（文件名）
/// - `encoding`: 编码
/// - `elapsed_ms`: 总执行时间（毫秒）
pub fn generate_report(
    summaries: Vec<RowSummary>,
    flow_uuid: &str,
    flow_name: &str,
    dataset_path: &str,
    dataset_name: &str,
    encoding: &str,
    elapsed_ms: u64,
) -> DatasetExecutionReport {
    let total_rows = summaries.len() as u32;
    let success_count = summaries
        .iter()
        .filter(|s| s.status == RowStatus::Success)
        .count() as u32;
    let fail_count = summaries
        .iter()
        .filter(|s| s.status == RowStatus::Fail)
        .count() as u32;
    let error_count = summaries
        .iter()
        .filter(|s| s.status == RowStatus::Error)
        .count() as u32;

    // 计算百分位数
    let mut durations: Vec<u64> = summaries.iter().map(|s| s.duration_ms).collect();
    durations.sort_unstable();
    let avg = if durations.is_empty() {
        0
    } else {
        durations.iter().sum::<u64>() / durations.len() as u64
    };
    let p50 = calculate_percentile(&durations, 50.0);
    let p95 = calculate_percentile(&durations, 95.0);
    let p99 = calculate_percentile(&durations, 99.0);

    // 执行时间戳
    let executed_at = flow_engine_core::utils::unix_timestamp_ms();

    DatasetExecutionReport {
        uuid: Uuid::new_v4().to_string(),
        flow_uuid: flow_uuid.to_string(),
        flow_name: flow_name.to_string(),
        dataset_path: dataset_path.to_string(),
        dataset_name: dataset_name.to_string(),
        encoding: encoding.to_string(),
        executed_at,
        total_duration_ms: elapsed_ms,
        total_rows,
        success_count,
        fail_count,
        error_count,
        avg_duration_ms: avg,
        p50_duration_ms: p50,
        p95_duration_ms: p95,
        p99_duration_ms: p99,
        row_summaries: summaries,
    }
}

// ============================================================================
// 报告存储
// ============================================================================

/// 保存数据驱动执行报告到磁盘
///
/// # 目录结构
/// ```text
/// {app_data_dir}/reports/dataset_{uuid}/
/// ├── summary.json                # 报告摘要
/// └── rows/
///     ├── row_0.json              # 行详细报告
///     ├── row_1.json
///     └── row_3.json
/// ```
///
/// # 参数
/// - `report`: 数据驱动执行报告
/// - `app_data_dir`: 应用数据目录
/// - `row_details`: 所有行的详细 FlowExecutionReport（key 为 [`row_index`]）
///
/// # 返回
/// 报告目录路径
pub fn save_report(
    report: &DatasetExecutionReport,
    app_data_dir: &Path,
    row_details: &HashMap<u32, FlowExecutionReport>,
) -> Result<String, String> {
    // 创建报告目录
    let report_dir = app_data_dir
        .join("reports")
        .join(format!("dataset_{}", report.uuid));
    std::fs::create_dir_all(&report_dir).map_err(|e| format!("创建报告目录Failed: {}", e))?;

    // 保存 summary.json（原子写入）
    let summary_path = report_dir.join("summary.json");
    let json =
        serde_json::to_string_pretty(report).map_err(|e| format!("序列化报告Failed: {}", e))?;

    let mut temp_file = tempfile::NamedTempFile::new_in(&report_dir)
        .map_err(|e| format!("创建临时文件Failed: {}", e))?;
    temp_file
        .write_all(json.as_bytes())
        .map_err(|e| format!("写入临时文件Failed: {}", e))?;
    temp_file
        .persist(&summary_path)
        .map_err(|e| format!("保存报告文件Failed: {}", e))?;

    // 保存行详情
    if !row_details.is_empty() {
        let rows_dir = report_dir.join("rows");
        std::fs::create_dir_all(&rows_dir).map_err(|e| format!("创建行详情目录Failed: {}", e))?;

        for (row_index, flow_report) in row_details {
            let row_path = rows_dir.join(format!("row_{}.json", row_index));
            let row_json = serde_json::to_string_pretty(flow_report)
                .map_err(|e| format!("序列化行报告Failed: {}", e))?;

            let mut row_temp = tempfile::NamedTempFile::new_in(&rows_dir)
                .map_err(|e| format!("创建行临时文件Failed: {}", e))?;
            row_temp
                .write_all(row_json.as_bytes())
                .map_err(|e| format!("写入行临时文件Failed: {}", e))?;
            row_temp
                .persist(&row_path)
                .map_err(|e| format!("保存行报告Failed: {}", e))?;
        }
    }

    Ok(report_dir.to_string_lossy().to_string())
}

// 使用 write_all 需要 Write trait
use std::io::Write;

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::dataset::RowSummary;
    use serde_json::json;
    use tempfile::TempDir;

    // ===== DatasetEvent 序列化测试 =====

    #[test]
    fn test_event_started_serde() {
        let event = DatasetEvent::Started {
            total_rows: 100,
            dataset_name: "test.csv".to_string(),
        };
        let json = serde_json::to_string(&event).expect("序列化Failed");
        assert!(json.contains("started"));
        assert!(json.contains("total_rows"));
        assert!(json.contains("test.csv"));

        let de: DatasetEvent = serde_json::from_str(&json).expect("反序列化Failed");
        if let DatasetEvent::Started {
            total_rows,
            dataset_name,
        } = de
        {
            assert_eq!(total_rows, 100);
            assert_eq!(dataset_name, "test.csv");
        } else {
            panic!("期望 Started 事件");
        }
    }

    #[test]
    fn test_event_row_completed_serde() {
        let event = DatasetEvent::RowCompleted {
            row_index: 5,
            status: RowStatus::Success,
            duration_ms: 120,
        };
        let json = serde_json::to_string(&event).expect("序列化Failed");
        assert!(json.contains("row_completed"));

        let de: DatasetEvent = serde_json::from_str(&json).expect("反序列化Failed");
        if let DatasetEvent::RowCompleted {
            row_index,
            status,
            duration_ms,
        } = de
        {
            assert_eq!(row_index, 5);
            assert_eq!(status, RowStatus::Success);
            assert_eq!(duration_ms, 120);
        } else {
            panic!("期望 RowCompleted 事件");
        }
    }

    #[test]
    fn test_event_row_failed_serde() {
        let event = DatasetEvent::RowFailed {
            row_index: 2,
            error: "连接超时".to_string(),
            duration_ms: 5000,
        };
        let json = serde_json::to_string(&event).expect("序列化Failed");
        assert!(json.contains("row_failed"));
        assert!(json.contains("连接超时"));

        let de: DatasetEvent = serde_json::from_str(&json).expect("反序列化Failed");
        if let DatasetEvent::RowFailed {
            row_index,
            error,
            duration_ms,
        } = de
        {
            assert_eq!(row_index, 2);
            assert_eq!(error, "连接超时");
            assert_eq!(duration_ms, 5000);
        } else {
            panic!("期望 RowFailed 事件");
        }
    }

    #[test]
    fn test_event_progress_serde() {
        let event = DatasetEvent::Progress {
            completed: 10,
            total: 100,
            success_count: 8,
            fail_count: 1,
            error_count: 1,
            elapsed_ms: 5000,
            estimated_remaining_ms: Some(45000),
        };
        let json = serde_json::to_string(&event).expect("序列化Failed");
        assert!(json.contains("progress"));
        assert!(json.contains("estimated_remaining_ms"));

        let de: DatasetEvent = serde_json::from_str(&json).expect("反序列化Failed");
        if let DatasetEvent::Progress {
            completed,
            total,
            estimated_remaining_ms,
            ..
        } = de
        {
            assert_eq!(completed, 10);
            assert_eq!(total, 100);
            assert_eq!(estimated_remaining_ms, Some(45000));
        } else {
            panic!("期望 Progress 事件");
        }
    }

    #[test]
    fn test_event_ended_serde() {
        let report = create_test_report(3, 2, 1, 0);
        let event = DatasetEvent::Ended {
            report: Box::new(report),
        };
        let json = serde_json::to_string(&event).expect("序列化Failed");
        assert!(json.contains("ended"));

        let de: DatasetEvent = serde_json::from_str(&json).expect("反序列化Failed");
        if let DatasetEvent::Ended { report } = de {
            assert_eq!(report.total_rows, 3);
        } else {
            panic!("期望 Ended 事件");
        }
    }

    #[test]
    fn test_event_cancelled_serde() {
        let report = create_test_report(5, 3, 1, 1);
        let event = DatasetEvent::Cancelled {
            report: Box::new(report),
        };
        let json = serde_json::to_string(&event).expect("序列化Failed");
        assert!(json.contains("cancelled"));

        let de: DatasetEvent = serde_json::from_str(&json).expect("反序列化Failed");
        if let DatasetEvent::Cancelled { report } = de {
            assert_eq!(report.total_rows, 5);
        } else {
            panic!("期望 Cancelled 事件");
        }
    }

    #[test]
    fn test_all_event_variants_roundtrip() {
        let events_json = vec![
            serde_json::to_string(&DatasetEvent::Started {
                total_rows: 10,
                dataset_name: "data.csv".to_string(),
            })
            .expect("序列化Failed"),
            serde_json::to_string(&DatasetEvent::RowCompleted {
                row_index: 0,
                status: RowStatus::Success,
                duration_ms: 50,
            })
            .expect("序列化Failed"),
            serde_json::to_string(&DatasetEvent::RowFailed {
                row_index: 1,
                error: "err".to_string(),
                duration_ms: 100,
            })
            .expect("序列化Failed"),
            serde_json::to_string(&DatasetEvent::Progress {
                completed: 2,
                total: 10,
                success_count: 1,
                fail_count: 0,
                error_count: 1,
                elapsed_ms: 200,
                estimated_remaining_ms: Some(800),
            })
            .expect("序列化Failed"),
            serde_json::to_string(&DatasetEvent::Ended {
                report: Box::new(create_test_report(2, 1, 0, 1)),
            })
            .expect("序列化Failed"),
            serde_json::to_string(&DatasetEvent::Cancelled {
                report: Box::new(create_test_report(1, 1, 0, 0)),
            })
            .expect("序列化Failed"),
        ];

        for json in events_json {
            let _: DatasetEvent = serde_json::from_str(&json).expect("反序列化Failed");
        }
    }

    // ===== 百分位数计算测试 =====

    #[test]
    fn test_percentile_empty() {
        assert_eq!(calculate_percentile(&[], 50.0), 0);
    }

    #[test]
    fn test_percentile_single() {
        assert_eq!(calculate_percentile(&[100], 50.0), 100);
        assert_eq!(calculate_percentile(&[100], 99.0), 100);
    }

    #[test]
    fn test_percentile_range() {
        // 1..=100 已排序
        let data: Vec<u64> = (1..=100).collect();
        let p50 = calculate_percentile(&data, 50.0);
        assert!((49..=51).contains(&p50), "P50 应接近 50，实际 {}", p50);

        let p95 = calculate_percentile(&data, 95.0);
        assert!((94..=96).contains(&p95), "P95 应接近 95，实际 {}", p95);

        let p99 = calculate_percentile(&data, 99.0);
        assert!((98..=100).contains(&p99), "P99 应接近 99，实际 {}", p99);
    }

    // ===== 报告生成测试 =====

    #[test]
    fn test_generate_report_empty() {
        let report = generate_report(
            vec![],
            "flow-001",
            "Test Flow",
            "/data/test.csv",
            "test.csv",
            "utf-8",
            0,
        );

        assert!(!report.uuid.is_empty());
        assert_eq!(report.flow_uuid, "flow-001");
        assert_eq!(report.total_rows, 0);
        assert_eq!(report.success_count, 0);
        assert_eq!(report.avg_duration_ms, 0);
        assert_eq!(report.p50_duration_ms, 0);
    }

    #[test]
    fn test_generate_report_with_data() {
        let summaries = vec![
            create_summary(0, RowStatus::Success, 100),
            create_summary(1, RowStatus::Success, 200),
            create_summary(2, RowStatus::Fail, 300),
            create_summary(3, RowStatus::Error, 400),
        ];

        let report = generate_report(
            summaries,
            "flow-001",
            "Test Flow",
            "/data/test.csv",
            "test.csv",
            "utf-8",
            1000,
        );

        assert_eq!(report.total_rows, 4);
        assert_eq!(report.success_count, 2);
        assert_eq!(report.fail_count, 1);
        assert_eq!(report.error_count, 1);
        assert_eq!(report.total_duration_ms, 1000);

        // 平均值: (100+200+300+400)/4 = 250
        assert_eq!(report.avg_duration_ms, 250);
    }

    #[test]
    fn test_generate_report_uuid_unique() {
        let r1 = generate_report(vec![], "f1", "F", "", "", "", 0);
        let r2 = generate_report(vec![], "f1", "F", "", "", "", 0);
        assert_ne!(r1.uuid, r2.uuid);
    }

    // ===== 报告存储测试 =====

    #[test]
    fn test_save_report_creates_files() {
        let temp_dir = TempDir::new().expect("创建临时目录Failed");

        let report = create_test_report(3, 2, 1, 0);
        let row_details = HashMap::new();

        let result = save_report(&report, temp_dir.path(), &row_details);
        assert!(result.is_ok(), "保存报告应Success: {:?}", result.err());

        // 验证 summary.json 存在
        let report_dir = temp_dir
            .path()
            .join("reports")
            .join(format!("dataset_{}", report.uuid));
        assert!(report_dir.exists(), "报告目录应存在");
        assert!(
            report_dir.join("summary.json").exists(),
            "summary.json 应存在"
        );

        // 验证 summary.json 可反序列化
        let content = std::fs::read_to_string(report_dir.join("summary.json")).expect("读取Failed");
        let loaded: DatasetExecutionReport =
            serde_json::from_str(&content).expect("反序列化Failed");
        assert_eq!(loaded.uuid, report.uuid);
        assert_eq!(loaded.total_rows, 3);

        // 无 rows 目录（空 map）
        assert!(
            !report_dir.join("rows").exists(),
            "无行详情时不应创建 rows 目录"
        );
    }

    #[test]
    fn test_save_report_with_failed_rows() {
        let temp_dir = TempDir::new().expect("创建临时目录Failed");

        let report = create_test_report(5, 3, 1, 1);

        // 构造行详情
        let mut row_details = HashMap::new();
        row_details.insert(2, create_flow_report("flow-001", "Test Flow"));
        row_details.insert(4, create_flow_report("flow-001", "Test Flow"));

        let result = save_report(&report, temp_dir.path(), &row_details);
        assert!(result.is_ok(), "保存报告应Success");

        let report_dir = temp_dir
            .path()
            .join("reports")
            .join(format!("dataset_{}", report.uuid));

        // 验证 rows 目录和文件存在
        let rows_dir = report_dir.join("rows");
        assert!(rows_dir.exists(), "rows 目录应存在");
        assert!(rows_dir.join("row_2.json").exists(), "row_2.json 应存在");
        assert!(rows_dir.join("row_4.json").exists(), "row_4.json 应存在");

        // 验证行详情文件可反序列化
        let content = std::fs::read_to_string(rows_dir.join("row_2.json")).expect("读取Failed");
        let flow_report: FlowExecutionReport =
            serde_json::from_str(&content).expect("反序列化Failed");
        assert_eq!(flow_report.flow_uuid, "flow-001");
    }

    #[test]
    fn test_save_report_no_rows_dir_when_empty_map() {
        let temp_dir = TempDir::new().expect("创建临时目录Failed");

        let report = create_test_report(3, 3, 0, 0);
        let row_details = HashMap::new();

        save_report(&report, temp_dir.path(), &row_details).expect("保存Failed");

        let report_dir = temp_dir
            .path()
            .join("reports")
            .join(format!("dataset_{}", report.uuid));
        assert!(!report_dir.join("rows").exists());
    }

    // ===== 事件序列测试 =====

    #[test]
    fn test_expected_event_sequence_for_successful_run() {
        // 模拟一个完整的Success执行过程应该产生的事件序列
        let total_rows = 3u32;
        let dataset_name = "test.csv".to_string();

        let events = vec![
            DatasetEvent::Started {
                total_rows,
                dataset_name: dataset_name.clone(),
            },
            DatasetEvent::RowCompleted {
                row_index: 0,
                status: RowStatus::Success,
                duration_ms: 100,
            },
            DatasetEvent::Progress {
                completed: 1,
                total: total_rows,
                success_count: 1,
                fail_count: 0,
                error_count: 0,
                elapsed_ms: 100,
                estimated_remaining_ms: Some(200),
            },
            DatasetEvent::RowCompleted {
                row_index: 1,
                status: RowStatus::Success,
                duration_ms: 100,
            },
            DatasetEvent::Progress {
                completed: 2,
                total: total_rows,
                success_count: 2,
                fail_count: 0,
                error_count: 0,
                elapsed_ms: 200,
                estimated_remaining_ms: Some(100),
            },
            DatasetEvent::RowCompleted {
                row_index: 2,
                status: RowStatus::Success,
                duration_ms: 100,
            },
            DatasetEvent::Ended {
                report: Box::new(create_test_report(3, 3, 0, 0)),
            },
        ];

        // 验证事件序列可序列化/反序列化
        for event in &events {
            let json = serde_json::to_string(event).expect("序列化Failed");
            let _: DatasetEvent = serde_json::from_str(&json).expect("反序列化Failed");
        }

        // 验证事件顺序: Started → Row* → Ended
        assert!(matches!(events[0], DatasetEvent::Started { .. }));
        assert!(matches!(
            events[events.len() - 1],
            DatasetEvent::Ended { .. }
        ));
    }

    #[test]
    fn test_cancelled_event_with_partial_report() {
        // 取消时生成部分报告
        let partial_summaries = vec![
            create_summary(0, RowStatus::Success, 100),
            create_summary(1, RowStatus::Success, 150),
        ];
        let report = generate_report(
            partial_summaries,
            "flow-001",
            "Test",
            "/data.csv",
            "data.csv",
            "utf-8",
            300,
        );

        let event = DatasetEvent::Cancelled {
            report: Box::new(report),
        };

        let json = serde_json::to_string(&event).expect("序列化Failed");
        let de: DatasetEvent = serde_json::from_str(&json).expect("反序列化Failed");

        if let DatasetEvent::Cancelled { report } = de {
            assert_eq!(report.total_rows, 2);
            assert_eq!(report.success_count, 2);
        } else {
            panic!("期望 Cancelled 事件");
        }
    }

    // ===== 辅助函数 =====

    fn create_summary(row_index: u32, status: RowStatus, duration_ms: u64) -> RowSummary {
        let is_error = status == RowStatus::Error;
        RowSummary {
            row_index,
            input_data: json!({"col": format!("val_{}", row_index)}),
            status,
            duration_ms,
            error_message: if is_error {
                Some("测试Error".to_string())
            } else {
                None
            },
            has_detail_report: !is_error,
        }
    }

    fn create_test_report(
        total: u32,
        success: u32,
        fail: u32,
        _error: u32,
    ) -> DatasetExecutionReport {
        let mut summaries = Vec::new();
        for i in 0..total {
            let status = if i < success {
                RowStatus::Success
            } else if i < success + fail {
                RowStatus::Fail
            } else {
                RowStatus::Error
            };
            summaries.push(create_summary(i, status, 100 + i as u64 * 10));
        }

        generate_report(
            summaries,
            "flow-001",
            "Test Flow",
            "/data/test.csv",
            "test.csv",
            "utf-8",
            total as u64 * 100,
        )
    }

    fn create_flow_report(flow_uuid: &str, flow_name: &str) -> FlowExecutionReport {
        use flow_engine_core::report::ExecutionStatus;
        use flow_engine_core::statistics::FlowStatistics;

        FlowExecutionReport {
            uuid: Uuid::new_v4().to_string(),
            flow_uuid: flow_uuid.to_string(),
            flow_name: flow_name.to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            total_nodes: 1,
            executed_count: 1,
            status: ExecutionStatus::Failed,
            node_reports: vec![],
            statistics: FlowStatistics::default(),
            flow_error: None,
            execution_mode: flow_engine_core::report::ExecutionMode::Normal,
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
}
