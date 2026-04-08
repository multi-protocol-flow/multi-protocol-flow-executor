// 压力测试指标收集器
//
// 线程安全的指标收集器，用于收集压测过程中的各项指标：
// - 成功/失败次数（使用 AtomicU64）
// - 延迟记录（环形缓冲区，最多 10,000 条）
// - 错误分类统计（HashMap）

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use super::types::{ErrorSample, ErrorStatistics, ResponseTimeDistribution, StressTestStatistics};

/// 延迟记录的最大容量
const MAX_LATENCY_RECORDS: usize = 10_000;

/// 错误样本的最大保留数量（每种错误类型）
const MAX_ERROR_SAMPLES: usize = 10;

/// 压力测试指标收集器
///
/// 线程安全的指标收集器，用于在压测过程中收集和汇总各项指标。
/// 使用原子操作和互斥锁确保并发安全。
pub struct MetricsCollector {
    /// 成功次数计数器
    success_count: AtomicU64,
    /// 失败次数计数器
    failure_count: AtomicU64,
    /// 总迭代次数
    total_iterations: AtomicU64,
    /// 测试开始时间戳（毫秒）
    start_time_ms: AtomicU64,
    /// 延迟记录（环形缓冲区）
    /// 使用 (head, data) 元组存储，head 指向下一个写入位置
    latencies: Mutex<(usize, Vec<u64>)>,
    /// 最小延迟（毫秒）
    min_latency_ms: AtomicU64,
    /// 最大延迟（毫秒）
    max_latency_ms: AtomicU64,
    /// 延迟总和（毫秒），用于计算平均值
    total_latency_ms: AtomicU64,
    /// 错误分类统计
    /// key: error_kind, value: (count, samples)
    error_stats: Mutex<HashMap<String, (u64, Vec<ErrorSample>)>>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    /// 创建新的指标收集器
    pub fn new() -> Self {
        Self {
            success_count: AtomicU64::new(0),
            failure_count: AtomicU64::new(0),
            total_iterations: AtomicU64::new(0),
            start_time_ms: AtomicU64::new(0),
            latencies: Mutex::new((0, Vec::with_capacity(MAX_LATENCY_RECORDS))),
            min_latency_ms: AtomicU64::new(u64::MAX),
            max_latency_ms: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            error_stats: Mutex::new(HashMap::new()),
        }
    }

    /// 设置测试开始时间
    ///
    /// # 参数
    ///
    /// * `start_time_ms` - 开始时间戳（毫秒）
    pub fn set_start_time(&self, start_time_ms: u64) {
        self.start_time_ms.store(start_time_ms, Ordering::SeqCst);
    }

    /// 记录一次成功执行
    ///
    /// # 参数
    ///
    /// * `latency_ms` - 执行延迟（毫秒）
    pub fn record_success(&self, latency_ms: u64) {
        // 更新计数器
        self.success_count.fetch_add(1, Ordering::SeqCst);
        self.total_iterations.fetch_add(1, Ordering::SeqCst);

        // 更新延迟统计
        self.record_latency(latency_ms);
    }

    /// 记录一次失败执行
    ///
    /// # 参数
    ///
    /// * `error_kind` - 错误类型
    /// * `sample` - 错误样本
    pub fn record_failure(&self, error_kind: String, sample: ErrorSample) {
        // 更新计数器
        self.failure_count.fetch_add(1, Ordering::SeqCst);
        self.total_iterations.fetch_add(1, Ordering::SeqCst);

        // 记录错误统计
        if let Ok(mut stats) = self.error_stats.lock() {
            let entry = stats.entry(error_kind).or_insert((0, Vec::new()));
            entry.0 += 1;

            // 保留最新的 N 个错误样本
            if entry.1.len() >= MAX_ERROR_SAMPLES {
                entry.1.remove(0);
            }
            entry.1.push(sample);
        }
    }

    /// 记录延迟数据
    ///
    /// 使用环形缓冲区存储延迟数据，超过容量时覆盖旧数据
    fn record_latency(&self, latency_ms: u64) {
        // 更新最小/最大延迟
        self.update_min_max_latency(latency_ms);

        // 更新延迟总和
        self.total_latency_ms
            .fetch_add(latency_ms, Ordering::SeqCst);

        // 添加到环形缓冲区
        if let Ok(mut guard) = self.latencies.lock() {
            let (head, data) = &mut *guard;

            if data.len() < MAX_LATENCY_RECORDS {
                data.push(latency_ms);
            } else {
                // 环形覆盖
                data[*head] = latency_ms;
            }

            *head = (*head + 1) % MAX_LATENCY_RECORDS;
        }
    }

    /// 更新最小/最大延迟（使用 CAS 循环）
    fn update_min_max_latency(&self, latency_ms: u64) {
        // 更新最小值
        loop {
            let current = self.min_latency_ms.load(Ordering::SeqCst);
            if latency_ms >= current {
                break;
            }
            if self
                .min_latency_ms
                .compare_exchange(current, latency_ms, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }

        // 更新最大值
        loop {
            let current = self.max_latency_ms.load(Ordering::SeqCst);
            if latency_ms <= current {
                break;
            }
            if self
                .max_latency_ms
                .compare_exchange(current, latency_ms, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }
    }

    /// 获取当前统计信息
    ///
    /// # 返回
    ///
    /// 当前的压力测试统计信息
    pub fn get_statistics(&self) -> StressTestStatistics {
        let total = self.total_iterations.load(Ordering::SeqCst);
        let successful = self.success_count.load(Ordering::SeqCst);
        let failed = self.failure_count.load(Ordering::SeqCst);

        let success_rate = if total > 0 {
            successful as f64 / total as f64
        } else {
            0.0
        };

        // 计算 RPS
        let start_time = self.start_time_ms.load(Ordering::SeqCst);
        let current_time = Self::current_time_ms();
        let rps = if start_time > 0 && current_time > start_time {
            let elapsed_seconds = (current_time - start_time) as f64 / 1000.0;
            total as f64 / elapsed_seconds
        } else {
            0.0
        };

        StressTestStatistics {
            total_iterations: total,
            successful,
            failed,
            success_rate,
            rps,
        }
    }

    /// 获取响应时间分布
    ///
    /// # 返回
    ///
    /// 响应时间分布统计
    pub fn get_response_time_distribution(&self) -> ResponseTimeDistribution {
        let total_latency = self.total_latency_ms.load(Ordering::SeqCst);
        let total_count = self.success_count.load(Ordering::SeqCst);

        let avg_ms = if total_count > 0 {
            total_latency as f64 / total_count as f64
        } else {
            0.0
        };

        let min_ms = self.min_latency_ms.load(Ordering::SeqCst);
        let max_ms = self.max_latency_ms.load(Ordering::SeqCst);

        // 如果没有数据，min_ms 会是 u64::MAX，需要处理
        let min_ms = if min_ms == u64::MAX { 0 } else { min_ms };

        ResponseTimeDistribution {
            avg_ms,
            min_ms,
            max_ms,
            p50_ms: self.calculate_percentile(50.0),
            p90_ms: self.calculate_percentile(90.0),
            p95_ms: self.calculate_percentile(95.0),
            p99_ms: self.calculate_percentile(99.0),
        }
    }

    /// 计算指定分位数的延迟
    ///
    /// 使用线性插值计算分位数
    ///
    /// # 参数
    ///
    /// * `p` - 分位数百分比（0.0 - 100.0）
    ///
    /// # 返回
    ///
    /// 对应分位数的延迟值（毫秒）
    pub fn calculate_percentile(&self, p: f64) -> u64 {
        if let Ok(guard) = self.latencies.lock() {
            let data = &guard.1;

            if data.is_empty() {
                return 0;
            }

            // 复制并排序
            let mut sorted: Vec<u64> = data.clone();
            sorted.sort_unstable();

            let n = sorted.len();
            let rank = (p / 100.0) * (n - 1) as f64;
            let lower = rank.floor() as usize;
            let upper = rank.ceil() as usize;

            if lower == upper || upper >= n {
                return sorted[lower.min(n - 1)];
            }

            // 线性插值
            let fraction = rank - lower as f64;
            let lower_val = sorted[lower] as f64;
            let upper_val = sorted[upper] as f64;

            (lower_val + fraction * (upper_val - lower_val)).round() as u64
        } else {
            0
        }
    }

    /// 获取错误统计列表
    ///
    /// # 返回
    ///
    /// 按错误类型分组的错误统计
    pub fn get_error_statistics(&self) -> Vec<ErrorStatistics> {
        if let Ok(guard) = self.error_stats.lock() {
            guard
                .iter()
                .map(|(kind, (count, samples))| ErrorStatistics {
                    error_kind: kind.clone(),
                    count: *count,
                    samples: samples.clone(),
                })
                .collect()
        } else {
            vec![]
        }
    }

    /// 重置所有指标
    ///
    /// 用于重新开始压测时清空之前的数据
    pub fn reset(&self) {
        self.success_count.store(0, Ordering::SeqCst);
        self.failure_count.store(0, Ordering::SeqCst);
        self.total_iterations.store(0, Ordering::SeqCst);
        self.start_time_ms.store(0, Ordering::SeqCst);
        self.min_latency_ms.store(u64::MAX, Ordering::SeqCst);
        self.max_latency_ms.store(0, Ordering::SeqCst);
        self.total_latency_ms.store(0, Ordering::SeqCst);

        if let Ok(mut guard) = self.latencies.lock() {
            guard.0 = 0;
            guard.1.clear();
        }

        if let Ok(mut guard) = self.error_stats.lock() {
            guard.clear();
        }
    }

    /// 获取当前时间戳（毫秒）
    fn current_time_ms() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_metrics_collector_new() {
        let collector = MetricsCollector::new();

        let stats = collector.get_statistics();
        assert_eq!(stats.total_iterations, 0);
        assert_eq!(stats.successful, 0);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[test]
    fn test_record_success() {
        let collector = MetricsCollector::new();

        collector.record_success(100);
        collector.record_success(200);
        collector.record_success(300);

        let stats = collector.get_statistics();
        assert_eq!(stats.successful, 3);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.total_iterations, 3);

        let dist = collector.get_response_time_distribution();
        assert_eq!(dist.min_ms, 100);
        assert_eq!(dist.max_ms, 300);
        // 平均值: (100 + 200 + 300) / 3 = 200
        assert!((dist.avg_ms - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_record_failure() {
        let collector = MetricsCollector::new();

        let sample = ErrorSample {
            iteration: 1,
            node_uuid: "node-001".to_string(),
            node_name: "HTTP Request".to_string(),
            error_message: "Connection timeout".to_string(),
        };

        collector.record_failure("Timeout".to_string(), sample);

        let stats = collector.get_statistics();
        assert_eq!(stats.successful, 0);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.total_iterations, 1);

        let error_stats = collector.get_error_statistics();
        assert_eq!(error_stats.len(), 1);
        assert_eq!(error_stats[0].error_kind, "Timeout");
        assert_eq!(error_stats[0].count, 1);
        assert_eq!(error_stats[0].samples.len(), 1);
    }

    #[test]
    fn test_record_multiple_error_kinds() {
        let collector = MetricsCollector::new();

        let sample1 = ErrorSample {
            iteration: 1,
            node_uuid: "node-001".to_string(),
            node_name: "HTTP".to_string(),
            error_message: "Timeout".to_string(),
        };

        let sample2 = ErrorSample {
            iteration: 2,
            node_uuid: "node-002".to_string(),
            node_name: "HTTP".to_string(),
            error_message: "Connection refused".to_string(),
        };

        collector.record_failure("Timeout".to_string(), sample1.clone());
        collector.record_failure("ConnectionError".to_string(), sample2);
        collector.record_failure("Timeout".to_string(), sample1);

        let error_stats = collector.get_error_statistics();
        assert_eq!(error_stats.len(), 2);

        let timeout_stat = error_stats.iter().find(|s| s.error_kind == "Timeout");
        assert!(timeout_stat.is_some());
        assert_eq!(timeout_stat.unwrap().count, 2);

        let conn_stat = error_stats
            .iter()
            .find(|s| s.error_kind == "ConnectionError");
        assert!(conn_stat.is_some());
        assert_eq!(conn_stat.unwrap().count, 1);
    }

    #[test]
    fn test_error_samples_limit() {
        let collector = MetricsCollector::new();

        // 添加超过限制数量的错误样本
        for i in 0..(MAX_ERROR_SAMPLES + 5) {
            let sample = ErrorSample {
                iteration: i as u64,
                node_uuid: format!("node-{:03}", i),
                node_name: "HTTP".to_string(),
                error_message: format!("Error {}", i),
            };
            collector.record_failure("Timeout".to_string(), sample);
        }

        let error_stats = collector.get_error_statistics();
        assert_eq!(error_stats[0].samples.len(), MAX_ERROR_SAMPLES);
        // 应该保留最新的样本
        assert_eq!(error_stats[0].samples[0].iteration, 5);
        assert_eq!(error_stats[0].samples[9].iteration, 14);
    }

    #[test]
    fn test_calculate_percentile_empty() {
        let collector = MetricsCollector::new();

        assert_eq!(collector.calculate_percentile(50.0), 0);
        assert_eq!(collector.calculate_percentile(90.0), 0);
        assert_eq!(collector.calculate_percentile(99.0), 0);
    }

    #[test]
    fn test_calculate_percentile_single() {
        let collector = MetricsCollector::new();
        collector.record_success(100);

        assert_eq!(collector.calculate_percentile(50.0), 100);
        assert_eq!(collector.calculate_percentile(90.0), 100);
        assert_eq!(collector.calculate_percentile(99.0), 100);
    }

    #[test]
    fn test_calculate_percentile() {
        let collector = MetricsCollector::new();

        // 添加 1-100 的延迟
        for i in 1..=100 {
            collector.record_success(i);
        }

        let p50 = collector.calculate_percentile(50.0);
        assert!(
            p50 >= 49 && p50 <= 51,
            "P50 should be around 50, got {}",
            p50
        );

        let p90 = collector.calculate_percentile(90.0);
        assert!(
            p90 >= 89 && p90 <= 91,
            "P90 should be around 90, got {}",
            p90
        );

        let p95 = collector.calculate_percentile(95.0);
        assert!(
            p95 >= 94 && p95 <= 96,
            "P95 should be around 95, got {}",
            p95
        );

        let p99 = collector.calculate_percentile(99.0);
        assert!(
            p99 >= 98 && p99 <= 100,
            "P99 should be around 99, got {}",
            p99
        );
    }

    #[test]
    fn test_circular_buffer() {
        let collector = MetricsCollector::new();

        // 填满缓冲区
        for i in 1..=MAX_LATENCY_RECORDS {
            collector.record_success(i as u64);
        }

        // 继续添加，应该覆盖旧数据
        for i in (MAX_LATENCY_RECORDS + 1)..=(MAX_LATENCY_RECORDS + 100) {
            collector.record_success(i as u64);
        }

        // 验证 P99 应该接近最大值
        let p99 = collector.calculate_percentile(99.0);
        assert!(p99 > 9000, "P99 should be high, got {}", p99);
    }

    #[test]
    fn test_success_rate() {
        let collector = MetricsCollector::new();

        // 80% 成功率
        for _ in 0..80 {
            collector.record_success(100);
        }

        let sample = ErrorSample {
            iteration: 0,
            node_uuid: "node".to_string(),
            node_name: "HTTP".to_string(),
            error_message: "Error".to_string(),
        };

        for _ in 0..20 {
            collector.record_failure("Error".to_string(), sample.clone());
        }

        let stats = collector.get_statistics();
        assert!((stats.success_rate - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_reset() {
        let collector = MetricsCollector::new();

        collector.record_success(100);
        collector.record_success(200);

        let sample = ErrorSample {
            iteration: 0,
            node_uuid: "node".to_string(),
            node_name: "HTTP".to_string(),
            error_message: "Error".to_string(),
        };
        collector.record_failure("Error".to_string(), sample);

        collector.reset();

        let stats = collector.get_statistics();
        assert_eq!(stats.total_iterations, 0);
        assert_eq!(stats.successful, 0);
        assert_eq!(stats.failed, 0);

        let dist = collector.get_response_time_distribution();
        assert_eq!(dist.min_ms, 0);
        assert_eq!(dist.max_ms, 0);

        assert!(collector.get_error_statistics().is_empty());
    }

    #[test]
    fn test_concurrent_recording() {
        let collector = Arc::new(MetricsCollector::new());
        let mut handles = vec![];

        // 并发记录成功
        for _ in 0..10 {
            let c = Arc::clone(&collector);
            handles.push(thread::spawn(move || {
                for i in 0..100 {
                    c.record_success(i + 1);
                }
            }));
        }

        // 并发记录失败
        for _ in 0..5 {
            let c = Arc::clone(&collector);
            handles.push(thread::spawn(move || {
                let sample = ErrorSample {
                    iteration: 0,
                    node_uuid: "node".to_string(),
                    node_name: "HTTP".to_string(),
                    error_message: "Error".to_string(),
                };
                for _ in 0..20 {
                    c.record_failure("Timeout".to_string(), sample.clone());
                }
            }));
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        let stats = collector.get_statistics();
        assert_eq!(stats.successful, 1000); // 10 threads * 100
        assert_eq!(stats.failed, 100); // 5 threads * 20
        assert_eq!(stats.total_iterations, 1100);

        // 验证成功率
        let expected_rate = 1000.0 / 1100.0;
        assert!((stats.success_rate - expected_rate).abs() < 0.01);
    }

    #[test]
    fn test_concurrent_percentile_calculation() {
        let collector = Arc::new(MetricsCollector::new());
        let mut handles = vec![];

        // 并发写入延迟数据
        for t in 0..4 {
            let c = Arc::clone(&collector);
            handles.push(thread::spawn(move || {
                for i in 0..250 {
                    c.record_success((t * 250 + i + 1) as u64);
                }
            }));
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // 验证数据完整性
        let stats = collector.get_statistics();
        assert_eq!(stats.successful, 1000);

        // 验证延迟数据已正确记录
        if let Ok(guard) = collector.latencies.lock() {
            assert_eq!(guard.1.len(), 1000);
        };
    }

    #[test]
    fn test_response_time_distribution() {
        let collector = MetricsCollector::new();

        // 添加一些延迟数据
        for i in 1..=100 {
            collector.record_success(i);
        }

        let dist = collector.get_response_time_distribution();

        assert_eq!(dist.min_ms, 1);
        assert_eq!(dist.max_ms, 100);
        // 平均值: (1+100)*100/2/100 = 50.5
        assert!((dist.avg_ms - 50.5).abs() < 0.1);

        // P50 应该接近 50
        assert!(dist.p50_ms >= 49 && dist.p50_ms <= 51);
    }

    #[test]
    fn test_min_latency_initialization() {
        let collector = MetricsCollector::new();

        // 在没有数据时，min_ms 应该是 0
        let dist = collector.get_response_time_distribution();
        assert_eq!(dist.min_ms, 0);

        // 添加第一个数据后
        collector.record_success(100);
        let dist = collector.get_response_time_distribution();
        assert_eq!(dist.min_ms, 100);
    }
}
