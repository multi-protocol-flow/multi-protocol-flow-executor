// 压力测试指标收集器
//
// 线程安全的指标收集器，用于收集压测过程中的各项指标：
// - Success/Failed次数（使用 AtomicU64）
// - 延迟记录（环形缓冲区，最多 10,000 条）
// - Error分类统计（HashMap）

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 从已排序的数据中计算指定分位数（线性插值）
///
/// # 参数
///
/// * `sorted` - 已排序的延迟数据
/// * `p` - 分位数百分比（0.0 - 100.0）
fn calculate_percentile_from_sorted(sorted: &[u64], p: f64) -> u64 {
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

use super::types::{
    ErrorSample, ErrorStatistics, NodeLevelStatistics, NodeMetrics, NodeTypeStatistics,
    ResponseTimeDistribution, StressTestStatistics,
};

/// 延迟记录的最大容量
const MAX_LATENCY_RECORDS: usize = 100_000;

/// Error样本的最大保留数量（每种Error类型）
const MAX_ERROR_SAMPLES: usize = 10;

/// 压力测试指标收集器
///
/// 线程安全的指标收集器，用于在压测过程中收集和汇总各项指标。
/// 使用原子操作和互斥锁确保并发安全。
pub struct MetricsCollector {
    /// Success次数计数器
    success_count: AtomicU64,
    /// Failed次数计数器
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
    /// Error分类统计
    /// key: `error_kind`, value: (count, samples)
    error_stats: Mutex<HashMap<String, (u64, Vec<ErrorSample>)>>,
    /// 节点级指标收集器
    node_metrics: Mutex<HashMap<String, Arc<NodeMetricsInner>>>,
}

/// 节点级指标内部数据结构
struct NodeMetricsInner {
    node_name: String,
    node_type: String,
    success_count: AtomicU64,
    failure_count: AtomicU64,
    latencies: Mutex<(usize, Vec<u64>)>,
    min_latency_ms: AtomicU64,
    max_latency_ms: AtomicU64,
    total_latency_ms: AtomicU64,
    error_stats: Mutex<HashMap<String, (u64, Vec<ErrorSample>)>>,
}

impl NodeMetricsInner {
    fn new(node_name: String, node_type: String) -> Self {
        Self {
            node_name,
            node_type,
            success_count: AtomicU64::new(0),
            failure_count: AtomicU64::new(0),
            latencies: Mutex::new((0, Vec::with_capacity(MAX_LATENCY_RECORDS))),
            min_latency_ms: AtomicU64::new(u64::MAX),
            max_latency_ms: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            error_stats: Mutex::new(HashMap::new()),
        }
    }

    fn record_success(&self, latency_ms: u64) {
        self.success_count.fetch_add(1, Ordering::SeqCst);
        self.total_latency_ms
            .fetch_add(latency_ms, Ordering::SeqCst);
        // 更新 min/max
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
        if let Ok(mut guard) = self.latencies.lock() {
            let (head, data) = &mut *guard;
            if data.len() < MAX_LATENCY_RECORDS {
                data.push(latency_ms);
            } else {
                data[*head] = latency_ms;
            }
            *head = (*head + 1) % MAX_LATENCY_RECORDS;
        }
    }

    fn record_failure(&self, error_kind: String, sample: ErrorSample) {
        self.failure_count.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut stats) = self.error_stats.lock() {
            let entry = stats.entry(error_kind).or_insert((0, Vec::new()));
            entry.0 += 1;
            if entry.1.len() >= MAX_ERROR_SAMPLES {
                entry.1.remove(0);
            }
            entry.1.push(sample);
        }
    }

    fn get_metrics(&self, node_uuid: &str) -> NodeMetrics {
        let success = self.success_count.load(Ordering::SeqCst);
        let failure = self.failure_count.load(Ordering::SeqCst);
        let total = success + failure;
        let success_rate = if total > 0 {
            success as f64 / total as f64
        } else {
            0.0
        };
        let total_latency = self.total_latency_ms.load(Ordering::SeqCst);
        let avg_latency_ms = if success > 0 {
            total_latency as f64 / success as f64
        } else {
            0.0
        };
        let min_ms = self.min_latency_ms.load(Ordering::SeqCst);
        let min_ms = if min_ms == u64::MAX { 0 } else { min_ms };

        NodeMetrics {
            node_uuid: node_uuid.to_string(),
            node_name: self.node_name.clone(),
            node_type: self.node_type.clone(),
            total_executions: total,
            successful_executions: success,
            failed_executions: failure,
            success_rate,
            avg_latency_ms,
            min_latency_ms: min_ms,
            max_latency_ms: self.max_latency_ms.load(Ordering::SeqCst),
            p50_latency_ms: self.calculate_percentile(50.0),
            p90_latency_ms: self.calculate_percentile(90.0),
            p95_latency_ms: self.calculate_percentile(95.0),
            p99_latency_ms: self.calculate_percentile(99.0),
        }
    }

    fn calculate_percentile(&self, p: f64) -> u64 {
        if let Ok(guard) = self.latencies.lock() {
            let data = &guard.1;
            if data.is_empty() {
                return 0;
            }
            let mut sorted: Vec<u64> = data.clone();
            sorted.sort_unstable();
            calculate_percentile_from_sorted(&sorted, p)
        } else {
            0
        }
    }

    fn get_response_time_distribution(&self) -> ResponseTimeDistribution {
        let total_latency = self.total_latency_ms.load(Ordering::SeqCst);
        let success = self.success_count.load(Ordering::SeqCst);
        let avg_ms = if success > 0 {
            total_latency as f64 / success as f64
        } else {
            0.0
        };
        let min_ms = self.min_latency_ms.load(Ordering::SeqCst);
        let min_ms = if min_ms == u64::MAX { 0 } else { min_ms };
        ResponseTimeDistribution {
            avg_ms,
            min_ms,
            max_ms: self.max_latency_ms.load(Ordering::SeqCst),
            p50_ms: self.calculate_percentile(50.0),
            p90_ms: self.calculate_percentile(90.0),
            p95_ms: self.calculate_percentile(95.0),
            p99_ms: self.calculate_percentile(99.0),
        }
    }

    fn get_error_statistics(&self) -> Vec<ErrorStatistics> {
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
            node_metrics: Mutex::new(HashMap::new()),
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

    /// 记录一次Success执行
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

    /// 记录一次Failed执行
    ///
    /// # 参数
    ///
    /// * `error_kind` - Error类型
    /// * `sample` - Error样本
    pub fn record_failure(&self, error_kind: String, sample: ErrorSample) {
        // 更新计数器
        self.failure_count.fetch_add(1, Ordering::SeqCst);
        self.total_iterations.fetch_add(1, Ordering::SeqCst);

        // 记录Error统计
        if let Ok(mut stats) = self.error_stats.lock() {
            let entry = stats.entry(error_kind).or_insert((0, Vec::new()));
            entry.0 += 1;

            // 保留最新的 N 个Error样本
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

            calculate_percentile_from_sorted(&sorted, p)
        } else {
            0
        }
    }

    /// 获取Error统计列表
    ///
    /// # 返回
    ///
    /// `按Error类型分组的Error统计`
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

        if let Ok(mut guard) = self.node_metrics.lock() {
            guard.clear();
        }
    }

    /// 记录节点级Success执行
    pub fn record_node_success(
        &self,
        node_uuid: &str,
        node_name: &str,
        node_type: &str,
        latency_ms: u64,
    ) {
        if let Ok(mut guard) = self.node_metrics.lock() {
            let inner = guard.entry(node_uuid.to_string()).or_insert_with(|| {
                Arc::new(NodeMetricsInner::new(
                    node_name.to_string(),
                    node_type.to_string(),
                ))
            });
            inner.record_success(latency_ms);
        }
    }

    /// 记录节点级Failed执行
    pub fn record_node_failure(
        &self,
        node_uuid: &str,
        node_name: &str,
        node_type: &str,
        error_kind: String,
        sample: ErrorSample,
    ) {
        if let Ok(mut guard) = self.node_metrics.lock() {
            let inner = guard.entry(node_uuid.to_string()).or_insert_with(|| {
                Arc::new(NodeMetricsInner::new(
                    node_name.to_string(),
                    node_type.to_string(),
                ))
            });
            inner.record_failure(error_kind, sample);
        }
    }

    /// 获取所有节点的实时指标
    pub fn get_node_metrics(&self) -> Vec<NodeMetrics> {
        if let Ok(guard) = self.node_metrics.lock() {
            guard
                .iter()
                .map(|(uuid, inner)| inner.get_metrics(uuid))
                .collect()
        } else {
            vec![]
        }
    }

    /// 获取节点级别统计（最终报告用）
    pub fn get_node_level_statistics(&self) -> Vec<NodeLevelStatistics> {
        if let Ok(guard) = self.node_metrics.lock() {
            guard
                .iter()
                .map(|(uuid, inner)| {
                    let success = inner.success_count.load(Ordering::SeqCst);
                    let failure = inner.failure_count.load(Ordering::SeqCst);
                    let total = success + failure;
                    let success_rate = if total > 0 {
                        success as f64 / total as f64
                    } else {
                        0.0
                    };
                    NodeLevelStatistics {
                        node_uuid: uuid.clone(),
                        node_name: inner.node_name.clone(),
                        node_type: inner.node_type.clone(),
                        total_executions: total,
                        successful_executions: success,
                        failed_executions: failure,
                        success_rate,
                        response_time_distribution: inner.get_response_time_distribution(),
                        error_statistics: inner.get_error_statistics(),
                    }
                })
                .collect()
        } else {
            vec![]
        }
    }

    /// 获取节点类型聚合统计
    pub fn get_node_type_statistics(&self) -> Vec<NodeTypeStatistics> {
        if let Ok(guard) = self.node_metrics.lock() {
            // 按节点类型聚合
            let mut type_map: HashMap<String, Vec<&Arc<NodeMetricsInner>>> = HashMap::new();
            for inner in guard.values() {
                type_map
                    .entry(inner.node_type.clone())
                    .or_default()
                    .push(inner);
            }
            type_map
                .into_iter()
                .map(|(node_type, inners)| {
                    let node_count = inners.len() as u32;
                    let mut total_executions = 0u64;
                    let mut successful = 0u64;
                    let mut failed = 0u64;
                    let mut all_latencies = Vec::new();
                    for inner in &inners {
                        let s = inner.success_count.load(Ordering::SeqCst);
                        let f = inner.failure_count.load(Ordering::SeqCst);
                        successful += s;
                        failed += f;
                        total_executions += s + f;
                        if let Ok(guard) = inner.latencies.lock() {
                            all_latencies.extend_from_slice(&guard.1);
                        }
                    }
                    let success_rate = if total_executions > 0 {
                        successful as f64 / total_executions as f64
                    } else {
                        0.0
                    };
                    let avg_ms = if successful > 0 {
                        let total_lat: u64 = inners
                            .iter()
                            .map(|i| i.total_latency_ms.load(Ordering::SeqCst))
                            .sum();
                        total_lat as f64 / successful as f64
                    } else {
                        0.0
                    };
                    all_latencies.sort_unstable();
                    let p50 = calculate_percentile_from_sorted(&all_latencies, 50.0);
                    let p90 = calculate_percentile_from_sorted(&all_latencies, 90.0);
                    let p95 = calculate_percentile_from_sorted(&all_latencies, 95.0);
                    let p99 = calculate_percentile_from_sorted(&all_latencies, 99.0);
                    let min_ms = all_latencies.first().copied().unwrap_or(0);
                    let max_ms = all_latencies.last().copied().unwrap_or(0);
                    NodeTypeStatistics {
                        node_type,
                        node_count,
                        total_executions,
                        successful_executions: successful,
                        failed_executions: failed,
                        success_rate,
                        response_time_distribution: ResponseTimeDistribution {
                            avg_ms,
                            min_ms,
                            max_ms,
                            p50_ms: p50,
                            p90_ms: p90,
                            p95_ms: p95,
                            p99_ms: p99,
                        },
                    }
                })
                .collect()
        } else {
            vec![]
        }
    }

    /// 获取当前时间戳（毫秒）
    fn current_time_ms() -> u64 {
        flow_engine_core::utils::unix_timestamp_ms()
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

        // 添加超过限制数量的Error样本
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
            (49..=51).contains(&p50),
            "P50 should be around 50, got {}",
            p50
        );

        let p90 = collector.calculate_percentile(90.0);
        assert!(
            (89..=91).contains(&p90),
            "P90 should be around 90, got {}",
            p90
        );

        let p95 = collector.calculate_percentile(95.0);
        assert!(
            (94..=96).contains(&p95),
            "P95 should be around 95, got {}",
            p95
        );

        let p99 = collector.calculate_percentile(99.0);
        assert!(
            (98..=100).contains(&p99),
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

        // 80% Success率
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

        // 并发记录Success
        for _ in 0..10 {
            let c = Arc::clone(&collector);
            handles.push(thread::spawn(move || {
                for i in 0..100 {
                    c.record_success(i + 1);
                }
            }));
        }

        // 并发记录Failed
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

        // 验证Success率
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
