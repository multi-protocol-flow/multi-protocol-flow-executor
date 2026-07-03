// 压力测试执行器

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rand::Rng;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::csv_parser;
use crate::executor::FlowExecutor;
use flow_engine_core::dataset::DataPickStrategy;
use flow_engine_core::flow::Flow;

use super::command::StressTestEvent;
use super::metrics::MetricsCollector;
use super::types::{
    ErrorSample, ResponseTimeDistribution, StressTestConfig, StressTestProgress, StressTestReport,
    StressTestStatistics, StressTestStatus,
};

/// 进度报告间隔（毫秒）
const PROGRESS_INTERVAL_MS: u64 = 1000;

/// 计算当前预热阶段的并发用户数
///
/// 使用线性预热算法，并发数随时间线性增长。
///
/// # 公式
/// `并发用户数 = floor(目标并发数 × 已过时间 / 预热时长)`
///
/// # 参数
/// - `target_concurrency`: 目标并发用户数
/// - `ramp_up_seconds`: 预热时长（秒）
/// - `elapsed_seconds`: 已经过时间（秒）
///
/// # 边界情况
/// - `ramp_up_seconds = 0`: 立即达到目标并发
/// - `elapsed_seconds >= ramp_up_seconds`: 维持目标并发
///
/// # 示例
/// ```
/// use flow_engine_executor::stress_test::calculate_current_concurrency;
/// // 目标 100 用户，预热 10 秒
/// assert_eq!(calculate_current_concurrency(100, 10, 0), 0);   // 0 秒
/// assert_eq!(calculate_current_concurrency(100, 10, 5), 50);  // 5 秒
/// assert_eq!(calculate_current_concurrency(100, 10, 10), 100); // 10 秒
/// assert_eq!(calculate_current_concurrency(100, 10, 15), 100); // 15 秒
/// ```
pub fn calculate_current_concurrency(
    target_concurrency: u32,
    ramp_up_seconds: u32,
    elapsed_seconds: u64,
) -> u32 {
    // 边界情况：预热时长为 0，立即达到目标并发
    if ramp_up_seconds == 0 {
        return target_concurrency;
    }

    // 边界情况：已经超过预热时长，维持目标并发
    if elapsed_seconds >= ramp_up_seconds as u64 {
        return target_concurrency;
    }

    // 线性计算：floor(target * elapsed / ramp_up)
    // 使用 u64 避免乘法溢出
    let target = target_concurrency as u64;
    let elapsed = elapsed_seconds;
    let ramp_up = ramp_up_seconds as u64;

    ((target * elapsed) / ramp_up) as u32
}

/// 压力测试执行器内部状态
struct StressTestState {
    /// 当前状态
    status: StressTestStatus,
    /// 测试开始时间
    start_time: Option<Instant>,
    /// 指标收集器
    metrics: Arc<MetricsCollector>,
    /// 停止信号
    stop_flag: Arc<AtomicBool>,
    /// 当前活跃的虚拟用户任务句柄
    active_tasks: Vec<tokio::task::JoinHandle<()>>,
    /// 最终报告（压测结束后存储）
    report: Option<StressTestReport>,
}

impl Default for StressTestState {
    fn default() -> Self {
        Self {
            status: StressTestStatus::Idle,
            start_time: None,
            metrics: Arc::new(MetricsCollector::new()),
            stop_flag: Arc::new(AtomicBool::new(false)),
            active_tasks: Vec::new(),
            report: None,
        }
    }
}

/// 压力测试执行器
///
/// 负责管理压力测试的生命周期，包括：
/// - 预热阶段：按时间线性增加并发用户
/// - 稳定阶段：维持目标并发数，持续执行
/// - 指标收集：实时收集并聚合指标
/// - 事件发送：向前端发送进度和完成事件
pub struct StressTestExecutor {
    /// 流程定义
    flow: Flow,
    /// 压测配置
    config: StressTestConfig,
    /// 事件发送器
    event_sender: mpsc::UnboundedSender<StressTestEvent>,
    /// 内部状态（线程安全）
    state: Arc<RwLock<StressTestState>>,
}

impl StressTestExecutor {
    /// 创建新的压力测试执行器
    ///
    /// # 参数
    /// - `flow`: 要执行的流程
    /// - `config`: 压力测试配置
    /// - `event_sender`: 事件发送器，用于向前端发送事件
    ///
    /// # 返回
    /// 新的压力测试执行器实例
    pub fn new(
        flow: Flow,
        config: StressTestConfig,
        event_sender: mpsc::UnboundedSender<StressTestEvent>,
    ) -> Self {
        Self {
            flow,
            config,
            event_sender,
            state: Arc::new(RwLock::new(StressTestState::default())),
        }
    }

    /// 启动压力测试
    ///
    /// # 返回
    /// - `Ok(())`: 启动Success
    /// - `Err(msg)`: 启动Failed（配置验证Failed或测试已在运行）
    pub async fn start(&self) -> Result<(), String> {
        // 验证配置
        self.config.validate()?;

        // 获取读锁检查状态
        {
            let state = self.state.read().await;
            if state.status != StressTestStatus::Idle {
                return Err("压力测试已在运行中".to_string());
            }
        }

        // 获取写锁更新状态
        let (metrics, stop_flag) = {
            let mut state = self.state.write().await;

            // 重置状态
            state.metrics = Arc::new(MetricsCollector::new());
            state.stop_flag = Arc::new(AtomicBool::new(false));
            state.active_tasks.clear();
            state.start_time = Some(Instant::now());
            state.status = if self.config.ramp_up_seconds > 0 {
                StressTestStatus::RampUp
            } else {
                StressTestStatus::Running
            };

            // 设置指标收集器的开始时间
            let start_time_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            state.metrics.set_start_time(start_time_ms);

            (Arc::clone(&state.metrics), Arc::clone(&state.stop_flag))
        };

        // 发送启动事件
        let _ = self.event_sender.send(StressTestEvent::Started {
            flow_uuid: self.flow.uuid.clone(),
            config: self.config.clone(),
        });

        // 克隆必要的数据用于异步任务
        let flow = self.flow.clone();
        let config = self.config.clone();
        let event_sender = self.event_sender.clone();
        let state = Arc::clone(&self.state);

        // 启动主控任务
        tokio::spawn(async move {
            Self::run_stress_test(flow, config, metrics, stop_flag, event_sender, state).await;
        });

        log::info!(
            "[压测] 启动压力测试: flow={}, concurrent={}, ramp_up={}s, duration={}s",
            self.flow.uuid,
            self.config.concurrent_users,
            self.config.ramp_up_seconds,
            self.config.duration_seconds
        );

        Ok(())
    }

    /// 停止压力测试
    ///
    /// # 返回
    /// - `Ok(())`: 停止Success
    /// - `Err(msg)`: 停止Failed（测试未在运行）
    pub async fn stop(&self) -> Result<(), String> {
        let state = self.state.read().await;

        if state.status == StressTestStatus::Idle {
            return Err("压力测试未在运行".to_string());
        }

        // 设置停止标志
        state.stop_flag.store(true, Ordering::SeqCst);

        log::info!("[压测] 停止压力测试: flow={}", self.flow.uuid);

        Ok(())
    }

    /// 获取当前进度
    ///
    /// # 返回
    /// 当前的压力测试进度信息
    pub async fn get_progress(&self) -> StressTestProgress {
        let state = self.state.read().await;

        let elapsed_seconds = if let Some(start_time) = state.start_time {
            start_time.elapsed().as_secs()
        } else {
            0
        };

        let statistics = state.metrics.get_statistics();
        let response_time = state.metrics.get_response_time_distribution();
        let node_metrics = state.metrics.get_node_metrics();

        StressTestProgress {
            status: state.status.clone(),
            elapsed_seconds,
            total_iterations: statistics.total_iterations,
            successful_iterations: statistics.successful,
            failed_iterations: statistics.failed,
            current_rps: statistics.rps,
            avg_latency_ms: response_time.avg_ms,
            node_metrics,
        }
    }

    /// 获取最终报告
    ///
    /// # 返回
    /// - `Some(report)`: 压测已完成，返回最终报告
    /// - `None`: 压测未完成或未启动
    pub async fn get_report(&self) -> Option<StressTestReport> {
        let state = self.state.read().await;
        state.report.clone()
    }

    /// 运行压力测试主循环
    async fn run_stress_test(
        flow: Flow,
        config: StressTestConfig,
        metrics: Arc<MetricsCollector>,
        stop_flag: Arc<AtomicBool>,
        event_sender: mpsc::UnboundedSender<StressTestEvent>,
        state: Arc<RwLock<StressTestState>>,
    ) {
        let start_time = Instant::now();
        let duration = Duration::from_secs(config.duration_seconds as u64);
        let mut last_progress_time = Instant::now();
        // 累积进度历史，用于持久化到报告中供历史查看时渲染图表
        let mut progress_history: Vec<StressTestProgress> = Vec::new();

        // 加载 CSV 数据源（如果配置了）
        let csv_rows: Arc<Vec<serde_json::Value>> = if let Some(ref path) = config.dataset_path {
            match csv_parser::parse_csv(path, config.dataset_encoding.as_deref(), None) {
                Ok(reader) => match reader.collect_all() {
                    Ok(rows) => {
                        log::info!("[压测] 加载 CSV 数据源: path={}, rows={}", path, rows.len());
                        Arc::new(rows)
                    }
                    Err(e) => {
                        log::error!("[压测] 解析 CSV 数据Failed: {}", e);
                        Arc::new(Vec::new())
                    }
                },
                Err(e) => {
                    log::error!("[压测] 打开 CSV 文件Failed: {}", e);
                    Arc::new(Vec::new())
                }
            }
        } else {
            Arc::new(Vec::new())
        };
        let dataset_strategy = config.dataset_strategy.clone().unwrap_or_default();

        // 用于管理虚拟用户任务
        let (task_stop_tx, _task_stop_rx) = broadcast::channel::<()>(1);

        // 主循环
        loop {
            let elapsed = start_time.elapsed();

            // 检查是否超时
            if config.duration_seconds > 0 && elapsed >= duration {
                break;
            }

            // 检查是否收到停止信号
            if stop_flag.load(Ordering::SeqCst) {
                break;
            }

            // 计算当前并发数
            let current_concurrency = calculate_current_concurrency(
                config.concurrent_users,
                config.ramp_up_seconds,
                elapsed.as_secs(),
            );

            // 更新状态（从 RampUp 到 Running）
            {
                let mut state_guard = state.write().await;
                if elapsed.as_secs() >= config.ramp_up_seconds as u64
                    && state_guard.status == StressTestStatus::RampUp
                {
                    state_guard.status = StressTestStatus::Running;
                }
            }

            // 确保有足够的虚拟用户
            {
                let mut state_guard = state.write().await;

                // 清理已完成的任务
                state_guard
                    .active_tasks
                    .retain(|handle| !handle.is_finished());

                // 计算需要启动的新任务数
                let current_active = state_guard.active_tasks.len() as u32;
                let needed = current_concurrency.saturating_sub(current_active);

                // 启动新的虚拟用户
                for user_id in 0..needed {
                    let flow_clone = flow.clone();
                    let metrics_clone = Arc::clone(&metrics);
                    let stop_flag_clone = Arc::clone(&stop_flag);
                    let stop_rx = task_stop_tx.subscribe();
                    let csv_rows_clone = Arc::clone(&csv_rows);
                    let strategy = dataset_strategy.clone();
                    let virtual_user_id = current_active + user_id;

                    let handle = tokio::spawn(async move {
                        Self::run_virtual_user(
                            flow_clone,
                            metrics_clone,
                            stop_flag_clone,
                            stop_rx,
                            csv_rows_clone,
                            strategy,
                            virtual_user_id,
                        )
                        .await;
                    });

                    state_guard.active_tasks.push(handle);
                }
            }

            // 每秒发送进度事件
            if last_progress_time.elapsed().as_millis() >= PROGRESS_INTERVAL_MS as u128 {
                let progress = {
                    let state_guard = state.read().await;
                    let elapsed_secs = start_time.elapsed().as_secs();
                    let statistics = state_guard.metrics.get_statistics();
                    let response_time = state_guard.metrics.get_response_time_distribution();

                    StressTestProgress {
                        status: state_guard.status.clone(),
                        elapsed_seconds: elapsed_secs,
                        total_iterations: statistics.total_iterations,
                        successful_iterations: statistics.successful,
                        failed_iterations: statistics.failed,
                        current_rps: statistics.rps,
                        avg_latency_ms: response_time.avg_ms,
                        node_metrics: state_guard.metrics.get_node_metrics(),
                    }
                };

                let _ = event_sender.send(StressTestEvent::Progress { progress: progress.clone() });
                progress_history.push(progress);
                last_progress_time = Instant::now();
            }

            // 短暂休眠，避免忙等待
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // 停止所有虚拟用户
        let _ = task_stop_tx.send(());

        // 等待所有任务完成（最多等待 5 秒）
        {
            let mut state_guard = state.write().await;
            let timeout = tokio::time::sleep(Duration::from_secs(5));
            tokio::pin!(timeout);

            while !state_guard.active_tasks.is_empty() {
                tokio::select! {
                    _ = &mut timeout => {
                        log::warn!("[压测] 等待虚拟用户超时，强制终止");
                        break;
                    }
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {
                        state_guard.active_tasks.retain(|handle| !handle.is_finished());
                    }
                }
            }
            state_guard.active_tasks.clear();
        }

        // 生成最终报告
        let report = Self::generate_report(&flow, &config, &metrics, start_time, &progress_history);

        // 更新最终状态并存储报告
        {
            let mut state_guard = state.write().await;
            state_guard.status = StressTestStatus::Stopped;
            state_guard.report = Some(report.clone());
        }

        // 发送结束事件
        let _ = event_sender.send(StressTestEvent::Ended { report: Box::new(report) });

        log::info!("[压测] 压力测试完成: flow={}", flow.uuid);
    }

    /// 运行单个虚拟用户
    async fn run_virtual_user(
        flow: Flow,
        metrics: Arc<MetricsCollector>,
        stop_flag: Arc<AtomicBool>,
        mut stop_rx: broadcast::Receiver<()>,
        csv_rows: Arc<Vec<serde_json::Value>>,
        dataset_strategy: DataPickStrategy,
        virtual_user_id: u32,
    ) {
        let executor = FlowExecutor::new();
        let mut iteration_index: u64 = 0;

        loop {
            // 检查停止信号
            if stop_flag.load(Ordering::SeqCst) {
                break;
            }

            // 检查广播停止信号
            if stop_rx.try_recv().is_ok() {
                break;
            }

            // 记录开始时间
            let iteration_start = Instant::now();

            // 根据 CSV 数据策略计算运行时变量
            let runtime_vars = if !csv_rows.is_empty() {
                let row = match dataset_strategy {
                    DataPickStrategy::RoundRobin => {
                        &csv_rows[iteration_index as usize % csv_rows.len()]
                    }
                    DataPickStrategy::Random => {
                        let idx = rand::thread_rng().gen_range(0..csv_rows.len());
                        &csv_rows[idx]
                    }
                    DataPickStrategy::Unique => {
                        &csv_rows[virtual_user_id as usize % csv_rows.len()]
                    }
                };
                Some(row.clone())
            } else {
                None
            };

            // 执行流程
            let execution_result = executor.execute_flow(&flow, None, runtime_vars, None).await;

            // 计算延迟
            let latency_ms = iteration_start.elapsed().as_millis() as u64;

            // 记录结果
            if execution_result.success {
                metrics.record_success(latency_ms);
            } else {
                // execution complete但Failed
                let error_msg = execution_result
                    .error
                    .clone()
                    .unwrap_or_else(|| "未知Error".to_string());
                let sample = ErrorSample {
                    iteration: metrics.get_statistics().total_iterations,
                    node_uuid: String::new(),
                    node_name: String::new(),
                    error_message: error_msg.clone(),
                };
                metrics.record_failure("ExecutionFailed".to_string(), sample);
            }

            // 记录每个节点的执行结果
            for node_report in &execution_result.node_reports {
                let node_latency = node_report.duration_ms;
                let node_status = &node_report.status;
                let status_str = format!("{:?}", node_status);
                if status_str.contains("Success") {
                    metrics.record_node_success(
                        &node_report.node_uuid,
                        &node_report.node_name,
                        &node_report.node_type,
                        node_latency,
                    );
                } else {
                    let node_error = node_report.error.as_ref()
                        .map(|e| format!("{:?}", e))
                        .unwrap_or_else(|| "未知节点Error".to_string());
                    let node_sample = ErrorSample {
                        iteration: metrics.get_statistics().total_iterations,
                        node_uuid: node_report.node_uuid.clone(),
                        node_name: node_report.node_name.clone(),
                        error_message: node_error.clone(),
                    };
                    metrics.record_node_failure(
                        &node_report.node_uuid,
                        &node_report.node_name,
                        &node_report.node_type,
                        "NodeFailed".to_string(),
                        node_sample,
                    );
                }
            }

            // 短暂休眠，避免过于紧密的循环
            tokio::time::sleep(Duration::from_millis(10)).await;
            iteration_index += 1;
        }
    }

    /// 生成最终报告
    fn generate_report(
        flow: &Flow,
        config: &StressTestConfig,
        metrics: &MetricsCollector,
        start_time: Instant,
        progress_history: &[StressTestProgress],
    ) -> StressTestReport {
        let end_time = Instant::now();
        let duration_ms = end_time.duration_since(start_time).as_millis() as u64;

        let start_time_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64 - duration_ms)
            .unwrap_or(0);

        let end_time_ms = start_time_ms + duration_ms;

        let statistics = metrics.get_statistics();
        let response_time = metrics.get_response_time_distribution();
        let error_statistics = metrics.get_error_statistics();
        let node_statistics = metrics.get_node_level_statistics();
        let node_type_statistics = metrics.get_node_type_statistics();

        StressTestReport {
            uuid: Uuid::new_v4().to_string(),
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
            progress_history: progress_history.to_vec(),
            flow_snapshot: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试预热算法的基本计算
    #[test]
    fn test_ramp_up_basic_calculation() {
        // 目标 100 用户，预热 10 秒
        assert_eq!(calculate_current_concurrency(100, 10, 0), 0); // 0%
        assert_eq!(calculate_current_concurrency(100, 10, 1), 10); // 10%
        assert_eq!(calculate_current_concurrency(100, 10, 2), 20); // 20%
        assert_eq!(calculate_current_concurrency(100, 10, 5), 50); // 50%
        assert_eq!(calculate_current_concurrency(100, 10, 10), 100); // 100%
    }

    /// 测试超过预热时长后的行为
    #[test]
    fn test_ramp_up_exceeded() {
        // 超过预热时长，维持目标并发
        assert_eq!(calculate_current_concurrency(100, 10, 10), 100);
        assert_eq!(calculate_current_concurrency(100, 10, 15), 100);
        assert_eq!(calculate_current_concurrency(100, 10, 100), 100);
    }

    /// 测试预热时长为 0 的边界情况
    #[test]
    fn test_ramp_up_zero_duration() {
        // 预热时长为 0，立即达到目标并发
        assert_eq!(calculate_current_concurrency(50, 0, 0), 50);
        assert_eq!(calculate_current_concurrency(50, 0, 1), 50);
        assert_eq!(calculate_current_concurrency(50, 0, 100), 50);
    }

    /// 测试非整除情况的取整
    #[test]
    fn test_ramp_up_floor_division() {
        // 目标 100 用户，预热 3 秒
        // 1 秒: 100 * 1 / 3 = 33.33 -> 33
        assert_eq!(calculate_current_concurrency(100, 3, 1), 33);
        // 2 秒: 100 * 2 / 3 = 66.66 -> 66
        assert_eq!(calculate_current_concurrency(100, 3, 2), 66);
        // 3 秒: 100 * 3 / 3 = 100
        assert_eq!(calculate_current_concurrency(100, 3, 3), 100);
    }

    /// 测试目标并发为 1 的边界情况
    #[test]
    fn test_ramp_up_target_one() {
        // 目标 1 用户，预热 10 秒
        assert_eq!(calculate_current_concurrency(1, 10, 0), 0);
        assert_eq!(calculate_current_concurrency(1, 10, 5), 0); // 0.5 取整为 0
        assert_eq!(calculate_current_concurrency(1, 10, 9), 0); // 0.9 取整为 0
        assert_eq!(calculate_current_concurrency(1, 10, 10), 1);
    }

    /// 测试高并发场景
    #[test]
    fn test_ramp_up_high_concurrency() {
        // 目标 10000 用户，预热 60 秒
        assert_eq!(calculate_current_concurrency(10000, 60, 0), 0);
        assert_eq!(calculate_current_concurrency(10000, 60, 30), 5000); // 50%
        assert_eq!(calculate_current_concurrency(10000, 60, 45), 7500); // 75%
        assert_eq!(calculate_current_concurrency(10000, 60, 60), 10000);
        assert_eq!(calculate_current_concurrency(10000, 60, 120), 10000);
    }

    /// 测试长时间预热
    #[test]
    fn test_ramp_up_long_duration() {
        // 目标 100 用户，预热 300 秒
        assert_eq!(calculate_current_concurrency(100, 300, 0), 0);
        assert_eq!(calculate_current_concurrency(100, 300, 150), 50); // 50%
        assert_eq!(calculate_current_concurrency(100, 300, 300), 100);
    }

    /// 测试预热时长为 1 秒的边界情况
    #[test]
    fn test_ramp_up_one_second() {
        // 预热时长为 1 秒
        assert_eq!(calculate_current_concurrency(100, 1, 0), 0);
        assert_eq!(calculate_current_concurrency(100, 1, 1), 100);
        assert_eq!(calculate_current_concurrency(100, 1, 2), 100);
    }

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

    /// 测试 StressTestExecutor 创建
    #[tokio::test]
    async fn test_executor_creation() {
        let flow = create_test_flow();
        let config = StressTestConfig::new(&flow.uuid);
        let (tx, _rx) = mpsc::unbounded_channel();

        let executor = StressTestExecutor::new(flow, config, tx);

        // 验证初始状态
        let progress = executor.get_progress().await;
        assert_eq!(progress.status, StressTestStatus::Idle);
        assert_eq!(progress.elapsed_seconds, 0);
        assert_eq!(progress.total_iterations, 0);
    }

    /// 测试启动和停止压力测试
    #[tokio::test]
    async fn test_start_and_stop() {
        let flow = create_test_flow();
        let config = StressTestConfig {
            concurrent_users: 2,
            ramp_up_seconds: 1,
            duration_seconds: 5,
            flow_uuid: flow.uuid.clone(),
            ..Default::default()
        };
        let (tx, mut rx) = mpsc::unbounded_channel();

        let executor = StressTestExecutor::new(flow, config, tx);

        // 启动压测
        let result = executor.start().await;
        assert!(result.is_ok(), "启动应该Success: {:?}", result);

        // 验证状态已改变
        let progress = executor.get_progress().await;
        assert_ne!(progress.status, StressTestStatus::Idle);

        // 接收启动事件
        if let Some(event) = rx.recv().await {
            match event {
                StressTestEvent::Started { flow_uuid, .. } => {
                    assert_eq!(flow_uuid, "test-flow-001");
                }
                _ => panic!("期望 Started 事件"),
            }
        }

        // 停止压测
        let result = executor.stop().await;
        assert!(result.is_ok(), "停止应该Success: {:?}", result);

        // 再次停止应该Failed
        // 等待一小段时间让状态更新
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    /// 测试重复启动应该Failed
    #[tokio::test]
    async fn test_double_start_fails() {
        let flow = create_test_flow();
        let config = StressTestConfig {
            concurrent_users: 1,
            ramp_up_seconds: 0,
            duration_seconds: 10,
            flow_uuid: flow.uuid.clone(),
            ..Default::default()
        };
        let (tx, _rx) = mpsc::unbounded_channel();

        let executor = StressTestExecutor::new(flow, config, tx);

        // 第一次启动应该Success
        let result = executor.start().await;
        assert!(result.is_ok());

        // 第二次启动应该Failed
        let result = executor.start().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("已在运行"));

        // 清理：停止压测
        let _ = executor.stop().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    /// 测试无效配置启动Failed
    #[tokio::test]
    async fn test_invalid_config_fails() {
        let flow = create_test_flow();
        let config = StressTestConfig {
            concurrent_users: 0, // 无效：并发数为 0
            ramp_up_seconds: 0,
            duration_seconds: 10,
            flow_uuid: flow.uuid.clone(),
            ..Default::default()
        };
        let (tx, _rx) = mpsc::unbounded_channel();

        let executor = StressTestExecutor::new(flow, config, tx);

        // 启动应该Failed
        let result = executor.start().await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.find("并发用户数").is_some());
    }

    /// 测试停止未运行的压测Failed
    #[tokio::test]
    async fn test_stop_not_running_fails() {
        let flow = create_test_flow();
        let config = StressTestConfig::default();
        let (tx, _rx) = mpsc::unbounded_channel();

        let executor = StressTestExecutor::new(flow, config, tx);

        // 停止未运行的压测应该Failed
        let result = executor.stop().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未在运行"));
    }

    /// 测试进度报告
    #[tokio::test]
    async fn test_progress_reporting() {
        let flow = create_test_flow();
        let config = StressTestConfig {
            concurrent_users: 1,
            ramp_up_seconds: 0,
            duration_seconds: 3,
            flow_uuid: flow.uuid.clone(),
            ..Default::default()
        };
        let (tx, mut rx) = mpsc::unbounded_channel();

        let executor = StressTestExecutor::new(flow, config, tx);

        // 启动压测
        let _ = executor.start().await;

        // 等待一些进度事件
        let mut received_progress = false;
        let start = Instant::now();

        while start.elapsed() < Duration::from_secs(2) {
            if let Some(event) = rx.recv().await {
                if matches!(event, StressTestEvent::Progress { .. }) {
                    received_progress = true;
                    break;
                }
            }
        }

        assert!(received_progress, "应该收到进度事件");

        // 清理
        let _ = executor.stop().await;
    }

    /// 测试最终报告生成
    #[tokio::test]
    async fn test_final_report() {
        let flow = create_test_flow();
        let config = StressTestConfig {
            concurrent_users: 1,
            ramp_up_seconds: 0,
            duration_seconds: 2,
            flow_uuid: flow.uuid.clone(),
            ..Default::default()
        };
        let (tx, mut rx) = mpsc::unbounded_channel();

        let executor = StressTestExecutor::new(flow.clone(), config.clone(), tx);

        // 启动压测
        let _ = executor.start().await;

        // 等待结束事件
        let mut received_report = false;
        let start = Instant::now();

        while start.elapsed() < Duration::from_secs(5) {
            match rx.recv().await {
                Some(StressTestEvent::Ended { report }) => {
                    received_report = true;
                    assert_eq!(report.flow_uuid, flow.uuid);
                    assert_eq!(report.flow_name, flow.name);
                    assert_eq!(report.config.concurrent_users, config.concurrent_users);
                    assert!(report.duration_ms > 0);
                    break;
                }
                _ => continue,
            }
        }

        assert!(received_report, "应该收到结束事件和报告");

        // 验证最终状态
        let progress = executor.get_progress().await;
        assert_eq!(progress.status, StressTestStatus::Stopped);
    }

    // ========================================================================
    // CSV 数据源策略测试
    // ========================================================================

    /// 辅助函数：创建临时 CSV 文件
    fn create_temp_csv(content: &[u8]) -> tempfile::NamedTempFile {
        use std::io::Write;
        let mut file = tempfile::Builder::new()
            .suffix(".csv")
            .tempfile()
            .expect("创建临时文件Failed");
        file.write_all(content).expect("写入临时文件Failed");
        file
    }

    /// 测试 RoundRobin 策略：3 行数据，6 次迭代，模式应为 0,1,2,0,1,2
    #[test]
    fn test_round_robin_strategy_pattern() {
        use flow_engine_core::dataset::DataPickStrategy;

        let csv_rows: Arc<Vec<serde_json::Value>> = Arc::new(vec![
            serde_json::json!({"name": "row0"}),
            serde_json::json!({"name": "row1"}),
            serde_json::json!({"name": "row2"}),
        ]);

        // 模拟 6 次迭代的 RoundRobin 选择
        let strategy = DataPickStrategy::RoundRobin;

        // 模拟 6 次迭代的 RoundRobin 选择
        let mut results = Vec::new();
        for iteration in 0..6u64 {
            let row = match strategy {
                DataPickStrategy::RoundRobin => &csv_rows[iteration as usize % csv_rows.len()],
                _ => panic!("不应到达此分支"),
            };
            results.push(row["name"].as_str().unwrap().to_string());
        }

        assert_eq!(results, vec!["row0", "row1", "row2", "row0", "row1", "row2"]);
    }

    /// 测试 Unique 策略：2 个用户，3 行数据
    #[test]
    fn test_unique_strategy_pattern() {
        use flow_engine_core::dataset::DataPickStrategy;

        let csv_rows: Arc<Vec<serde_json::Value>> = Arc::new(vec![
            serde_json::json!({"name": "row0"}),
            serde_json::json!({"name": "row1"}),
            serde_json::json!({"name": "row2"}),
        ]);

        let _strategy = DataPickStrategy::Unique;

        // 用户 0 → row0, 用户 1 → row1
        let row0 = &csv_rows[0u32 as usize % csv_rows.len()];
        let row1 = &csv_rows[1u32 as usize % csv_rows.len()];
        assert_eq!(row0["name"].as_str().unwrap(), "row0");
        assert_eq!(row1["name"].as_str().unwrap(), "row1");
    }

    /// 测试无 CSV 时行为不变（runtime_variables 为 None）
    #[test]
    fn test_no_csv_runtime_vars_is_none() {
        let csv_rows: Arc<Vec<serde_json::Value>> = Arc::new(Vec::new());

        // 空 csv_rows → runtime_vars 应为 None
        let runtime_vars = if !csv_rows.is_empty() {
            Some(csv_rows[0].clone())
        } else {
            None
        };

        assert!(runtime_vars.is_none(), "无 CSV 数据时 runtime_variables 应为 None");
    }

    /// 测试 StressTestConfig 新字段的序列化/反序列化（向后兼容）
    #[test]
    fn test_config_new_fields_serialization() {
        use flow_engine_core::dataset::DataPickStrategy;

        // 包含新字段的配置
        let config = StressTestConfig {
            concurrent_users: 5,
            ramp_up_seconds: 5,
            duration_seconds: 30,
            flow_uuid: "flow-001".to_string(),
            dataset_path: Some("/data/test.csv".to_string()),
            dataset_encoding: Some("utf-8".to_string()),
            dataset_strategy: Some(DataPickStrategy::RoundRobin),
        };

        let json = serde_json::to_string(&config).expect("序列化Failed");
        assert!(json.contains("datasetPath"));
        assert!(json.contains("datasetEncoding"));
        assert!(json.contains("datasetStrategy"));
        assert!(json.contains("round_robin"));

        // 反序列化往返
        let deserialized: StressTestConfig = serde_json::from_str(&json).expect("反序列化Failed");
        assert_eq!(deserialized.dataset_path, Some("/data/test.csv".to_string()));
        assert_eq!(deserialized.dataset_encoding, Some("utf-8".to_string()));
        assert_eq!(deserialized.dataset_strategy, Some(DataPickStrategy::RoundRobin));
    }

    /// 测试旧 JSON（缺少新字段）反序列化时的默认值
    #[test]
    fn test_config_backward_compat_deserialization() {
        let json = r#"{
            "concurrentUsers": 10,
            "rampUpSeconds": 10,
            "durationSeconds": 60,
            "flowUuid": "flow-001"
        }"#;

        let config: StressTestConfig = serde_json::from_str(json).expect("反序列化Failed");
        assert_eq!(config.concurrent_users, 10);
        assert_eq!(config.dataset_path, None);
        assert_eq!(config.dataset_encoding, None);
        assert_eq!(config.dataset_strategy, None);
    }

    /// 测试带 CSV 数据源的压力测试完整流程
    #[tokio::test]
    async fn test_stress_test_with_csv_dataset() {
        let csv_file = create_temp_csv(b"name,value\nAlice,1\nBob,2\nCharlie,3\n");
        let csv_path = csv_file.path().to_str().expect("路径转换Failed").to_string();

        let flow = create_test_flow();
        let config = StressTestConfig {
            concurrent_users: 1,
            ramp_up_seconds: 0,
            duration_seconds: 2,
            flow_uuid: flow.uuid.clone(),
            dataset_path: Some(csv_path),
            dataset_encoding: Some("utf-8".to_string()),
            dataset_strategy: Some(flow_engine_core::dataset::DataPickStrategy::RoundRobin),
            ..Default::default()
        };
        let (tx, mut rx) = mpsc::unbounded_channel();

        let executor = StressTestExecutor::new(flow.clone(), config.clone(), tx);
        let result = executor.start().await;
        assert!(result.is_ok(), "启动应该Success: {:?}", result);

        // 等待结束事件
        let mut received_report = false;
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            match rx.recv().await {
                Some(StressTestEvent::Ended { report }) => {
                    received_report = true;
                    // 验证报告中包含 dataset 配置
                    assert!(report.config.dataset_path.is_some());
                    assert_eq!(
                        report.config.dataset_strategy,
                        Some(flow_engine_core::dataset::DataPickStrategy::RoundRobin)
                    );
                    break;
                }
                _ => continue,
            }
        }
        assert!(received_report, "应该收到结束事件");
    }
}
