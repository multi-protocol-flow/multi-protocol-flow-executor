// 压力测试报告生成
//
// 从 MetricsCollector 获取统计数据，生成 StressTestReport 并持久化

use std::path::Path;
use std::time::Instant;

use uuid::Uuid;

use crate::storage::save_stress_report;
use flow_engine_core::flow::Flow;

use super::metrics::MetricsCollector;
use super::types::{
    ResponseTimeDistribution, StressTestConfig, StressTestReport, StressTestStatistics,
};

/// 生成压力测试报告
///
/// # 参数
/// - `flow`: 执行的流程
/// - `config`: 压力测试配置
/// - `metrics`: 指标收集器
/// - `start_time`: 测试开始时间
/// - `reports_dir`: 报告存储目录（如果为 None，则不保存文件）
///
/// # 返回
/// - `Ok(StressTestReport)`: 生成的报告
/// - `Err(String)`: 生成Failed
///
/// # 功能
/// - 从 `MetricsCollector` 获取统计数据
/// - 从 config 获取配置快照
/// - 计算汇总统计（总次数、Success/Failed数、Success率）
/// - 计算响应时间分布（平均值、最小值、最大值、P50/P90/P95/P99）
/// - 计算实际 RPS
/// - 提取Error分类统计
/// - 提取前 10 条Error样例（由 `MetricsCollector` 限制）
/// - 如果提供了 `reports_dir，调用` `save_stress_report()` 持久化报告
pub fn generate_report(
    flow: &Flow,
    config: &StressTestConfig,
    metrics: &MetricsCollector,
    start_time: Instant,
    reports_dir: Option<&Path>,
) -> Result<StressTestReport, String> {
    // 计算结束时间和持续时间
    let end_time = Instant::now();
    let duration_ms = end_time.duration_since(start_time).as_millis() as u64;

    // 计算开始时间戳（毫秒）
    let start_time_ms = flow_engine_core::utils::unix_timestamp_ms() - duration_ms;

    let end_time_ms = start_time_ms + duration_ms;

    // 从 MetricsCollector 获取统计数据
    let statistics = metrics.get_statistics();
    let response_time = metrics.get_response_time_distribution();
    let error_statistics = metrics.get_error_statistics();
    let node_statistics = metrics.get_node_level_statistics();
    let node_type_statistics = metrics.get_node_type_statistics();

    // 生成报告 UUID
    let report_uuid = Uuid::new_v4().to_string();

    // 构建报告
    let report = StressTestReport {
        uuid: report_uuid,
        flow_uuid: flow.uuid.clone(),
        flow_name: flow.name.clone(),
        start_time: start_time_ms,
        end_time: end_time_ms,
        duration_ms,
        config: config.clone(),
        statistics: StressTestStatistics {
            total_iterations: statistics.total_iterations,
            successful: statistics.successful,
            failed: statistics.failed,
            success_rate: statistics.success_rate,
            rps: statistics.rps,
        },
        response_time_distribution: ResponseTimeDistribution {
            avg_ms: response_time.avg_ms,
            min_ms: response_time.min_ms,
            max_ms: response_time.max_ms,
            p50_ms: response_time.p50_ms,
            p90_ms: response_time.p90_ms,
            p95_ms: response_time.p95_ms,
            p99_ms: response_time.p99_ms,
        },
        error_statistics,
        node_statistics,
        node_type_statistics,
        progress_history: vec![],
        flow_snapshot: Some(flow.clone()),
    };

    // 如果提供了存储目录，保存报告
    if let Some(dir) = reports_dir {
        save_stress_report(&report, dir)?;
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stress_test::types::ErrorSample;
    use tempfile::TempDir;

    /// 创建测试用的简单流程
    fn create_test_flow() -> Flow {
        use flow_engine_core::node::{Node, NodePosition};
        use flow_engine_core::node_config::NodeData;

        Flow { uuid: "test-flow-001".to_string(), name: "Test Flow".to_string(), description: None, nodes: vec![Node {
            uuid: "entry-001".to_string(),
            node_type: "entry".to_string(),
            name: "Entry".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        }], connections: vec![], initial_variables: None, environments: vec![], active_environment: None, hooks: None }
    }

    /// 测试生成空报告（无数据）
    #[test]
    fn test_generate_empty_report() {
        let flow = create_test_flow();
        let config = StressTestConfig::new(&flow.uuid);
        let metrics = MetricsCollector::new();
        let start_time = Instant::now();

        // 短暂等待以确保 duration_ms > 0
        std::thread::sleep(std::time::Duration::from_millis(10));

        let report = generate_report(&flow, &config, &metrics, start_time, None);

        assert!(report.is_ok());
        let report = report.unwrap();

        // 验证基本信息
        assert!(!report.uuid.is_empty());
        assert_eq!(report.flow_uuid, flow.uuid);
        assert_eq!(report.flow_name, flow.name);
        assert!(report.duration_ms > 0);

        // 验证统计信息（空数据）
        assert_eq!(report.statistics.total_iterations, 0);
        assert_eq!(report.statistics.successful, 0);
        assert_eq!(report.statistics.failed, 0);
        assert_eq!(report.statistics.success_rate, 0.0);

        // 验证响应时间分布（空数据）
        assert_eq!(report.response_time_distribution.avg_ms, 0.0);
        assert_eq!(report.response_time_distribution.min_ms, 0);
        assert_eq!(report.response_time_distribution.max_ms, 0);
        assert_eq!(report.response_time_distribution.p50_ms, 0);

        // 验证Error统计（空）
        assert!(report.error_statistics.is_empty());
    }

    /// 测试生成带Success数据的报告
    #[test]
    fn test_generate_report_with_success_data() {
        let flow = create_test_flow();
        let config = StressTestConfig::new(&flow.uuid);
        let metrics = MetricsCollector::new();

        // 设置开始时间
        let start_time_ms = flow_engine_core::utils::unix_timestamp_ms();
        metrics.set_start_time(start_time_ms);

        // 记录一些Success执行
        metrics.record_success(100);
        metrics.record_success(150);
        metrics.record_success(200);

        let start_time = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let report = generate_report(&flow, &config, &metrics, start_time, None).unwrap();

        // 验证统计信息
        assert_eq!(report.statistics.total_iterations, 3);
        assert_eq!(report.statistics.successful, 3);
        assert_eq!(report.statistics.failed, 0);
        assert!((report.statistics.success_rate - 1.0).abs() < 0.01);

        // 验证响应时间分布
        assert_eq!(report.response_time_distribution.min_ms, 100);
        assert_eq!(report.response_time_distribution.max_ms, 200);
        // 平均值: (100 + 150 + 200) / 3 = 150
        assert!(
            (report.response_time_distribution.avg_ms - 150.0).abs() < 0.1,
            "avg_ms 应该接近 150，实际为 {}",
            report.response_time_distribution.avg_ms
        );
    }

    /// 测试生成带Error数据的报告
    #[test]
    fn test_generate_report_with_errors() {
        let flow = create_test_flow();
        let config = StressTestConfig::new(&flow.uuid);
        let metrics = MetricsCollector::new();

        // 设置开始时间
        let start_time_ms = flow_engine_core::utils::unix_timestamp_ms();
        metrics.set_start_time(start_time_ms);

        // 记录Success和Failed
        metrics.record_success(100);
        metrics.record_success(100);

        let error_sample = ErrorSample {
            iteration: 1,
            node_uuid: "node-001".to_string(),
            node_name: "HTTP Request".to_string(),
            error_message: "Connection timeout".to_string(),
        };
        metrics.record_failure("Timeout".to_string(), error_sample);

        let start_time = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let report = generate_report(&flow, &config, &metrics, start_time, None).unwrap();

        // 验证统计信息
        assert_eq!(report.statistics.total_iterations, 3);
        assert_eq!(report.statistics.successful, 2);
        assert_eq!(report.statistics.failed, 1);
        // Success率: 2/3 ≈ 0.6667
        assert!(
            (report.statistics.success_rate - 0.6667).abs() < 0.01,
            "success_rate 应该接近 0.6667，实际为 {}",
            report.statistics.success_rate
        );

        // 验证Error统计
        assert_eq!(report.error_statistics.len(), 1);
        assert_eq!(report.error_statistics[0].error_kind, "Timeout");
        assert_eq!(report.error_statistics[0].count, 1);
        assert_eq!(report.error_statistics[0].samples.len(), 1);
    }

    /// 测试生成带多种Error类型的报告
    #[test]
    fn test_generate_report_with_multiple_error_kinds() {
        let flow = create_test_flow();
        let config = StressTestConfig::new(&flow.uuid);
        let metrics = MetricsCollector::new();

        // 设置开始时间
        let start_time_ms = flow_engine_core::utils::unix_timestamp_ms();
        metrics.set_start_time(start_time_ms);

        // 记录多种Error
        for i in 0..3 {
            let sample = ErrorSample {
                iteration: i,
                node_uuid: format!("node-{}", i),
                node_name: "HTTP".to_string(),
                error_message: format!("Timeout {}", i),
            };
            metrics.record_failure("Timeout".to_string(), sample);
        }

        for i in 0..2 {
            let sample = ErrorSample {
                iteration: i,
                node_uuid: format!("node-{}", i),
                node_name: "HTTP".to_string(),
                error_message: format!("Connection refused {}", i),
            };
            metrics.record_failure("ConnectionRefused".to_string(), sample);
        }

        let start_time = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let report = generate_report(&flow, &config, &metrics, start_time, None).unwrap();

        // 验证Error统计
        assert_eq!(report.error_statistics.len(), 2);

        let timeout_stat = report
            .error_statistics
            .iter()
            .find(|s| s.error_kind == "Timeout");
        assert!(timeout_stat.is_some());
        assert_eq!(timeout_stat.unwrap().count, 3);

        let conn_stat = report
            .error_statistics
            .iter()
            .find(|s| s.error_kind == "ConnectionRefused");
        assert!(conn_stat.is_some());
        assert_eq!(conn_stat.unwrap().count, 2);
    }

    /// 测试报告包含配置快照
    #[test]
    fn test_report_contains_config_snapshot() {
        let flow = create_test_flow();
        let config = StressTestConfig {
            concurrent_users: 20,
            ramp_up_seconds: 30,
            duration_seconds: 120,
            flow_uuid: flow.uuid.clone(),
            ..Default::default()
        };
        let metrics = MetricsCollector::new();

        let start_time = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let report = generate_report(&flow, &config, &metrics, start_time, None).unwrap();

        // 验证配置快照
        assert_eq!(report.config.concurrent_users, 20);
        assert_eq!(report.config.ramp_up_seconds, 30);
        assert_eq!(report.config.duration_seconds, 120);
        assert_eq!(report.config.flow_uuid, flow.uuid);
    }

    /// 测试报告包含响应时间分布
    #[test]
    fn test_report_contains_response_time_distribution() {
        let flow = create_test_flow();
        let config = StressTestConfig::new(&flow.uuid);
        let metrics = MetricsCollector::new();

        // 设置开始时间
        let start_time_ms = flow_engine_core::utils::unix_timestamp_ms();
        metrics.set_start_time(start_time_ms);

        // 记录 1-100 的延迟
        for i in 1..=100 {
            metrics.record_success(i);
        }

        let start_time = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let report = generate_report(&flow, &config, &metrics, start_time, None).unwrap();

        // 验证响应时间分布
        assert_eq!(report.response_time_distribution.min_ms, 1);
        assert_eq!(report.response_time_distribution.max_ms, 100);

        // P50 应该接近 50
        assert!(
            report.response_time_distribution.p50_ms >= 49
                && report.response_time_distribution.p50_ms <= 51,
            "P50 应该接近 50，实际为 {}",
            report.response_time_distribution.p50_ms
        );

        // P90 应该接近 90
        assert!(
            report.response_time_distribution.p90_ms >= 89
                && report.response_time_distribution.p90_ms <= 91,
            "P90 应该接近 90，实际为 {}",
            report.response_time_distribution.p90_ms
        );
    }

    /// 测试保存报告到文件
    #[test]
    fn test_save_report_to_file() {
        let flow = create_test_flow();
        let config = StressTestConfig::new(&flow.uuid);
        let metrics = MetricsCollector::new();

        // 设置开始时间
        let start_time_ms = flow_engine_core::utils::unix_timestamp_ms();
        metrics.set_start_time(start_time_ms);

        // 记录一些数据
        metrics.record_success(100);
        metrics.record_success(200);

        let start_time = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(10));

        // 创建临时目录
        let temp_dir = TempDir::new().expect("创建临时目录Failed");
        let reports_dir = temp_dir.path();

        let report = generate_report(&flow, &config, &metrics, start_time, Some(reports_dir));

        assert!(report.is_ok(), "生成报告应该Success");

        // 验证文件已创建
        let files: Vec<_> = std::fs::read_dir(reports_dir)
            .expect("读取目录Failed")
            .filter_map(|e| e.ok())
            .collect();

        assert_eq!(files.len(), 1, "应该创建一个报告文件");

        let filename = files[0].file_name();
        let filename_str = filename.to_string_lossy();
        assert!(
            filename_str.starts_with("performance_"),
            "文件名应该以 performance_ 开头"
        );
        assert!(filename_str.ends_with(".json"), "文件名应该以 .json 结尾");
    }

    /// 测试不保存报告（reports_dir 为 None）
    #[test]
    fn test_no_save_when_reports_dir_is_none() {
        let flow = create_test_flow();
        let config = StressTestConfig::new(&flow.uuid);
        let metrics = MetricsCollector::new();

        let start_time = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let report = generate_report(&flow, &config, &metrics, start_time, None);

        assert!(report.is_ok(), "生成报告应该Success");

        // 不应该有任何文件操作
        // 这里只验证报告生成Success，不涉及文件系统
    }

    /// 测试报告 UUID 唯一性
    #[test]
    fn test_report_uuid_uniqueness() {
        let flow = create_test_flow();
        let config = StressTestConfig::new(&flow.uuid);
        let metrics = MetricsCollector::new();

        let start_time = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let report1 = generate_report(&flow, &config, &metrics, start_time, None).unwrap();
        let report2 = generate_report(&flow, &config, &metrics, start_time, None).unwrap();

        // 每次生成的报告 UUID 应该不同
        assert_ne!(report1.uuid, report2.uuid);
    }

    /// 测试报告时间戳正确性
    #[test]
    fn test_report_timestamps() {
        let flow = create_test_flow();
        let config = StressTestConfig::new(&flow.uuid);
        let metrics = MetricsCollector::new();

        let start_time = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(100));

        let report = generate_report(&flow, &config, &metrics, start_time, None).unwrap();

        // 验证持续时间
        assert!(
            report.duration_ms >= 100,
            "持续时间应该至少 100ms，实际为 {}",
            report.duration_ms
        );

        // 验证时间戳逻辑
        assert_eq!(
            report.end_time,
            report.start_time + report.duration_ms,
            "end_time 应该等于 start_time + duration_ms"
        );
    }
}
