// 数据驱动执行器
//!
//! 逐行读取 CSV 数据，将每行作为 `runtime_variables` 注入流程执行。
//! 支持顺序执行和并发执行（Semaphore），以及 Continue/StopOnFirst 失败策略。

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use serde_json::json;
use tokio::sync::{watch, Semaphore};

use crate::csv_parser;
use crate::executor::FlowExecutor;
use flow_engine_core::dataset::{RowStatus, RowSummary};
use flow_engine_core::flow::Flow;

use super::events::{self, DatasetEvent};
use super::types::{DatasetExecutionConfig, FailureStrategy};

/// 数据驱动执行器
///
/// 从 CSV 文件逐行读取数据，每行注入为 `runtime_variables` 执行流程，
/// 收集每行的执行结果并返回摘要列表。
pub struct DatasetExecutor {
    /// 节点插件注册表（预留用于未来的自定义执行器构建）
    #[allow(dead_code)]
    registry: Arc<flow_engine_core::registry::NodeRegistry>,
}

impl DatasetExecutor {
    /// 创建新的数据驱动执行器
    pub fn new(registry: Arc<flow_engine_core::registry::NodeRegistry>) -> Self {
        Self { registry }
    }

    /// 执行数据驱动流程
    ///
    /// # 参数
    /// - `config`: 执行配置（CSV 路径、并发数、失败策略等）
    /// - `flow`: 要执行的流程定义
    /// - `cancel_rx`: 取消信号接收器（None 表示不可取消）
    /// - `event_sender`: 事件发送器（None 表示不发送事件）
    ///
    /// # 返回
    /// 每行的执行摘要，按 `row_index` 排序
    pub async fn execute(
        &self,
        config: &DatasetExecutionConfig,
        flow: &Flow,
        cancel_rx: Option<watch::Receiver<bool>>,
        event_sender: Option<tokio::sync::mpsc::UnboundedSender<DatasetEvent>>,
    ) -> Result<Vec<RowSummary>, String> {
        // 从文件路径提取数据集名称
        let dataset_name = Path::new(&config.dataset_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // 先扫描获取总行数（用于注入 _row_count 内置变量）
        let info = csv_parser::validate(
            &config.dataset_path,
            config.dataset_encoding.as_deref(),
            config.delimiter,
        )?;
        let total_rows = info.row_count;

        if total_rows == 0 {
            return Ok(Vec::new());
        }

        // 发送 Started 事件
        events::send_event(
            &event_sender,
            DatasetEvent::Started {
                total_rows,
                dataset_name: dataset_name.clone(),
            },
        );

        // 根据并发数选择执行模式
        let start = Instant::now();
        let result = if config.concurrency <= 1 {
            self.execute_sequential(
                config,
                flow,
                cancel_rx,
                &dataset_name,
                total_rows,
                &event_sender,
                start,
            )
            .await
        } else {
            self.execute_concurrent(
                config,
                flow,
                cancel_rx,
                &dataset_name,
                total_rows,
                &event_sender,
            )
            .await
        };

        match result {
            Ok((summaries, cancelled)) => {
                let elapsed_ms = start.elapsed().as_millis() as u64;
                let encoding = config
                    .dataset_encoding
                    .as_deref()
                    .unwrap_or("utf-8")
                    .to_string();

                let report = events::generate_report(
                    summaries.clone(),
                    &flow.uuid,
                    &flow.name,
                    &config.dataset_path,
                    &dataset_name,
                    &encoding,
                    elapsed_ms,
                );

                if cancelled {
                    events::send_event(
                        &event_sender,
                        DatasetEvent::Cancelled {
                            report: Box::new(report),
                        },
                    );
                } else {
                    events::send_event(
                        &event_sender,
                        DatasetEvent::Ended {
                            report: Box::new(report),
                        },
                    );
                }

                Ok(summaries)
            }
            Err(e) => Err(e),
        }
    }

    /// 顺序执行（concurrency = 1）
    ///
    /// 返回 (summaries, `was_cancelled`)
    async fn execute_sequential(
        &self,
        config: &DatasetExecutionConfig,
        flow: &Flow,
        mut cancel_rx: Option<watch::Receiver<bool>>,
        dataset_name: &str,
        total_rows: u32,
        event_sender: &Option<tokio::sync::mpsc::UnboundedSender<DatasetEvent>>,
        start_time: Instant,
    ) -> Result<(Vec<RowSummary>, bool), String> {
        let mut csv_reader = csv_parser::parse_csv(
            &config.dataset_path,
            config.dataset_encoding.as_deref(),
            config.delimiter,
        )?;

        let mut results: Vec<RowSummary> = Vec::new();
        let mut row_index: u32 = 0;
        let executor = FlowExecutor::new();
        let mut cancelled = false;

        while let Some(row_result) = csv_reader.next_row() {
            // 检查取消信号
            if let Some(ref mut rx) = cancel_rx {
                if *rx.borrow_and_update() {
                    cancelled = true;
                    break;
                }
            }

            let csv_row = row_result?;
            let row_vars =
                Self::build_runtime_variables(csv_row.clone(), row_index, total_rows, dataset_name);

            let row_start = Instant::now();
            let execution_result = executor
                .execute_flow(flow, None, Some(row_vars), None)
                .await;
            let duration_ms = row_start.elapsed().as_millis() as u64;

            let (status, error_message) = if execution_result.success {
                (RowStatus::Success, None)
            } else if execution_result.error.is_some() {
                (RowStatus::Error, execution_result.error.clone())
            } else {
                (RowStatus::Fail, execution_result.error.clone())
            };

            // 发送行完成/失败事件
            match status {
                RowStatus::Success => {
                    events::send_event(
                        event_sender,
                        DatasetEvent::RowCompleted {
                            row_index,
                            status: RowStatus::Success,
                            duration_ms,
                        },
                    );
                }
                RowStatus::Fail | RowStatus::Error => {
                    events::send_event(
                        event_sender,
                        DatasetEvent::RowFailed {
                            row_index,
                            error: error_message.clone().unwrap_or_default(),
                            duration_ms,
                        },
                    );
                }
            }

            let summary = RowSummary {
                row_index,
                input_data: csv_row,
                status: status.clone(),
                duration_ms,
                error_message,
                has_detail_report: false,
            };

            results.push(summary);

            // 发送进度事件
            let elapsed_ms = start_time.elapsed().as_millis() as u64;
            let completed = results.len() as u32;
            let success_count = results
                .iter()
                .filter(|r| r.status == RowStatus::Success)
                .count() as u32;
            let fail_count = results
                .iter()
                .filter(|r| r.status == RowStatus::Fail)
                .count() as u32;
            let error_count = results
                .iter()
                .filter(|r| r.status == RowStatus::Error)
                .count() as u32;

            // 估算剩余时间
            let estimated_remaining_ms = if completed > 0 {
                let avg_per_row = elapsed_ms / completed as u64;
                let remaining = total_rows.saturating_sub(completed);
                Some(avg_per_row * remaining as u64)
            } else {
                None
            };

            events::send_event(
                event_sender,
                DatasetEvent::Progress {
                    completed,
                    total: total_rows,
                    success_count,
                    fail_count,
                    error_count,
                    elapsed_ms,
                    estimated_remaining_ms,
                },
            );

            // StopOnFirst 策略：遇到非成功行即停止
            if config.failure_strategy == FailureStrategy::StopOnFirst && status != RowStatus::Success
            {
                break;
            }

            row_index += 1;
        }

        Ok((results, cancelled))
    }

    /// 并发执行（concurrency > 1，使用 Semaphore 控制并发数）
    ///
    /// 返回 (summaries, `was_cancelled`)
    async fn execute_concurrent(
        &self,
        config: &DatasetExecutionConfig,
        flow: &Flow,
        cancel_rx: Option<watch::Receiver<bool>>,
        dataset_name: &str,
        total_rows: u32,
        event_sender: &Option<tokio::sync::mpsc::UnboundedSender<DatasetEvent>>,
    ) -> Result<(Vec<RowSummary>, bool), String> {
        let mut csv_reader = csv_parser::parse_csv(
            &config.dataset_path,
            config.dataset_encoding.as_deref(),
            config.delimiter,
        )?;

        // 收集所有行数据
        let mut all_rows: Vec<(u32, serde_json::Value)> = Vec::new();
        let mut idx: u32 = 0;
        while let Some(row_result) = csv_reader.next_row() {
            all_rows.push((idx, row_result?));
            idx += 1;
        }

        let semaphore = Arc::new(Semaphore::new(config.concurrency as usize));
        let failure_strategy = config.failure_strategy.clone();

        // 用于收集结果的通道
        let (tx, mut rx) = tokio::sync::mpsc::channel::<RowSummary>(all_rows.len());

        // 用于通知 StopOnFirst 停止的通道
        let (stop_tx, _) = tokio::sync::broadcast::channel::<()>(1);
        let stop_tx_check = stop_tx.clone();

        let flow = Arc::new(flow.clone());
        let dataset_name = dataset_name.to_string();

        // 进度跟踪用的原子计数器
        let completed_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let success_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let fail_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let error_count = Arc::new(std::sync::atomic::AtomicU32::new(0));

        // 共享开始时间戳（毫秒），用于计算进度事件的 elapsed_ms
        let start_time_ms = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        ));

        let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
        let mut cancelled = false;

        for (row_index, csv_row) in all_rows {
            // 检查取消信号
            if let Some(ref rx_cancel) = cancel_rx {
                if *rx_cancel.borrow() {
                    cancelled = true;
                    break;
                }
            }

            // 检查 StopOnFirst 停止信号（非阻塞尝试接收）
            if stop_tx_check.subscribe().try_recv().is_ok() {
                cancelled = true;
                break;
            }

            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|e| format!("获取信号量失败: {}", e))?;

            let tx = tx.clone();
            let flow = Arc::clone(&flow);
            let dataset_name = dataset_name.clone();
            let mut stop_rx = stop_tx_check.subscribe();
            let stop_tx_task = stop_tx_check.clone();
            let failure_strategy = failure_strategy.clone();
            let event_sender = event_sender.clone();
            let completed_count = Arc::clone(&completed_count);
            let success_count = Arc::clone(&success_count);
            let fail_count = Arc::clone(&fail_count);
            let error_count = Arc::clone(&error_count);
            let start_time_ms = Arc::clone(&start_time_ms);

            let handle = tokio::spawn(async move {
                let _permit = permit; // 持有信号量直到任务完成

                // 检查是否已收到停止信号
                if stop_rx.try_recv().is_ok() {
                    return;
                }

                let row_vars = Self::build_runtime_variables(
                    csv_row.clone(),
                    row_index,
                    total_rows,
                    &dataset_name,
                );

                let executor = FlowExecutor::new();
                let row_start = Instant::now();
                let execution_result = executor
                    .execute_flow(&flow, None, Some(row_vars), None)
                    .await;
                let duration_ms = row_start.elapsed().as_millis() as u64;

                let (status, error_message) = if execution_result.success {
                    (RowStatus::Success, None)
                } else if execution_result.error.is_some() {
                    (RowStatus::Error, execution_result.error.clone())
                } else {
                    (RowStatus::Fail, execution_result.error.clone())
                };

                // 发送行事件
                match status {
                    RowStatus::Success => {
                        events::send_event(
                            &event_sender,
                            DatasetEvent::RowCompleted {
                                row_index,
                                status: RowStatus::Success,
                                duration_ms,
                            },
                        );
                    }
                    RowStatus::Fail | RowStatus::Error => {
                        events::send_event(
                            &event_sender,
                            DatasetEvent::RowFailed {
                                row_index,
                                error: error_message.clone().unwrap_or_default(),
                                duration_ms,
                            },
                        );
                    }
                }

                let summary = RowSummary {
                    row_index,
                    input_data: csv_row,
                    status: status.clone(),
                    duration_ms,
                    error_message,
                    has_detail_report: false,
                };

                let _ = tx.send(summary).await;

                // 更新进度计数器并发送进度事件
                let completed = completed_count
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    + 1;
                match status {
                    RowStatus::Success => {
                        success_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                    RowStatus::Fail => {
                        fail_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                    RowStatus::Error => {
                        error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                }

                let current_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let elapsed_ms = current_ms.saturating_sub(start_time_ms.load(std::sync::atomic::Ordering::SeqCst));
                let sc = success_count.load(std::sync::atomic::Ordering::SeqCst);
                let fc = fail_count.load(std::sync::atomic::Ordering::SeqCst);
                let ec = error_count.load(std::sync::atomic::Ordering::SeqCst);
                let estimated_remaining_ms = if completed > 0 {
                    let avg_per_row = elapsed_ms / completed as u64;
                    let remaining = total_rows.saturating_sub(completed);
                    Some(avg_per_row * remaining as u64)
                } else {
                    None
                };

                events::send_event(
                    &event_sender,
                    DatasetEvent::Progress {
                        completed,
                        total: total_rows,
                        success_count: sc,
                        fail_count: fc,
                        error_count: ec,
                        elapsed_ms,
                        estimated_remaining_ms,
                    },
                );

                // StopOnFirst：发送停止信号
                if failure_strategy == FailureStrategy::StopOnFirst && status != RowStatus::Success {
                    let _ = stop_tx_task.send(());
                }
            });

            handles.push(handle);
        }

        // 关闭发送端，以便接收端能在所有任务完成后退出
        drop(tx);

        // 等待所有任务完成
        for handle in handles {
            let _ = handle.await;
        }

        // 收集所有结果
        let mut results: Vec<RowSummary> = Vec::new();
        while let Some(summary) = rx.recv().await {
            results.push(summary);
        }

        // 按 row_index 排序以保证顺序
        results.sort_by_key(|r| r.row_index);

        Ok((results, cancelled))
    }

    /// 构建运行时变量：CSV 行数据 + 内置变量
    ///
    /// 注入以下内置变量：
    /// - `_row_index`: 当前行索引（从 0 开始）
    /// - `_row_count`: 总行数
    /// - `_dataset_name`: 数据集文件名
    fn build_runtime_variables(
        csv_row: serde_json::Value,
        row_index: u32,
        row_count: u32,
        dataset_name: &str,
    ) -> serde_json::Value {
        let mut row_vars = csv_row;
        if let Some(obj) = row_vars.as_object_mut() {
            obj.insert("_row_index".to_string(), json!(row_index));
            obj.insert("_row_count".to_string(), json!(row_count));
            obj.insert("_dataset_name".to_string(), json!(dataset_name));
        }
        row_vars
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node::{Node, NodePosition};
    use flow_engine_core::node_config::NodeData;
    use std::io::Write;

    /// 创建最小化的测试流程（仅含 entry 节点）
    fn create_test_flow() -> Flow {
        Flow { uuid: "test-dataset-flow".to_string(), name: "Dataset Test Flow".to_string(), initial_variables: None, environments: vec![], active_environment: None, nodes: vec![Node {
            uuid: "entry-001".to_string(),
            node_type: "entry".to_string(),
            name: "Entry".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        }], connections: vec![], hooks: None }
    }

    /// 创建临时 CSV 文件
    fn create_temp_csv(content: &[u8]) -> tempfile::NamedTempFile {
        let mut file = tempfile::Builder::new()
            .suffix(".csv")
            .tempfile()
            .expect("创建临时文件失败");
        file.write_all(content).expect("写入临时文件失败");
        file
    }

    /// 创建 DatasetExecutor（使用默认注册表）
    fn create_executor() -> DatasetExecutor {
        let registry = Arc::new(crate::default_registry::default_registry());
        DatasetExecutor::new(registry)
    }

    /// 测试顺序执行（concurrency=1）3 行数据
    #[tokio::test]
    async fn test_sequential_execution() {
        let csv = create_temp_csv(b"name,age\nAlice,30\nBob,25\nCharlie,35\n");
        let path = csv.path().to_str().expect("路径转换失败");

        let flow = create_test_flow();
        let executor = create_executor();
        let config = DatasetExecutionConfig {
            flow_uuid: flow.uuid.clone(),
            dataset_path: path.to_string(),
            dataset_encoding: None,
            delimiter: None,
            concurrency: 1,
            failure_strategy: FailureStrategy::Continue,
        };

        let results = executor.execute(&config, &flow, None, None).await.expect("执行失败");

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].row_index, 0);
        assert_eq!(results[1].row_index, 1);
        assert_eq!(results[2].row_index, 2);

        // 验证 CSV 原始数据保留在 input_data 中
        assert_eq!(results[0].input_data["name"], json!("Alice"));
        assert_eq!(results[0].input_data["age"], json!("30"));

        // 全部应成功（entry 节点总是成功）
        for r in &results {
            assert_eq!(r.status, RowStatus::Success);
        }
    }

    /// 测试并发执行（concurrency=2）5 行数据
    #[tokio::test]
    async fn test_concurrent_execution() {
        let csv = create_temp_csv(
            b"name,age\nA,1\nB,2\nC,3\nD,4\nE,5\n",
        );
        let path = csv.path().to_str().expect("路径转换失败");

        let flow = create_test_flow();
        let executor = create_executor();
        let config = DatasetExecutionConfig {
            flow_uuid: flow.uuid.clone(),
            dataset_path: path.to_string(),
            dataset_encoding: None,
            delimiter: None,
            concurrency: 2,
            failure_strategy: FailureStrategy::Continue,
        };

        let results = executor.execute(&config, &flow, None, None).await.expect("执行失败");

        assert_eq!(results.len(), 5);

        // 并发执行后结果应按 row_index 排序
        for (i, r) in results.iter().enumerate() {
            assert_eq!(r.row_index, i as u32);
        }

        // 验证每行 CSV 原始数据
        assert_eq!(results[0].input_data["name"], json!("A"));
        assert_eq!(results[4].input_data["name"], json!("E"));
    }

    /// 测试 Continue 失败策略 — 即使失败也继续执行所有行
    #[tokio::test]
    async fn test_continue_failure_strategy() {
        // 创建含空行的 CSV（空行不会导致失败，但我们可以验证 continue 行为）
        let csv = create_temp_csv(b"name\nAlice\nBob\nCharlie\n");
        let path = csv.path().to_str().expect("路径转换失败");

        let flow = create_test_flow();
        let executor = create_executor();
        let config = DatasetExecutionConfig {
            flow_uuid: flow.uuid.clone(),
            dataset_path: path.to_string(),
            dataset_encoding: None,
            delimiter: None,
            concurrency: 1,
            failure_strategy: FailureStrategy::Continue,
        };

        let results = executor.execute(&config, &flow, None, None).await.expect("执行失败");
        assert_eq!(results.len(), 3, "Continue 策略应执行全部行");
    }

    /// 测试空 CSV（仅有表头）返回空结果
    #[tokio::test]
    async fn test_empty_csv() {
        let csv = create_temp_csv(b"name,age\n");
        let path = csv.path().to_str().expect("路径转换失败");

        let flow = create_test_flow();
        let executor = create_executor();
        let config = DatasetExecutionConfig {
            flow_uuid: flow.uuid.clone(),
            dataset_path: path.to_string(),
            dataset_encoding: None,
            delimiter: None,
            concurrency: 1,
            failure_strategy: FailureStrategy::Continue,
        };

        let results = executor.execute(&config, &flow, None, None).await.expect("执行失败");
        assert!(results.is_empty(), "空 CSV 应返回空结果");
    }

    /// 测试取消信号停止执行
    #[tokio::test]
    async fn test_cancel_signal() {
        let csv = create_temp_csv(b"name\nA\nB\nC\nD\nE\n");
        let path = csv.path().to_str().expect("路径转换失败");

        let flow = create_test_flow();
        let executor = create_executor();
        let config = DatasetExecutionConfig {
            flow_uuid: flow.uuid.clone(),
            dataset_path: path.to_string(),
            dataset_encoding: None,
            delimiter: None,
            concurrency: 1,
            failure_strategy: FailureStrategy::Continue,
        };

        // 创建已触发的取消信号
        let (tx, rx) = watch::channel(true);

        let results = executor.execute(&config, &flow, Some(rx), None).await.expect("执行失败");
        // 取消信号已触发，不应执行任何行
        assert!(results.is_empty(), "已取消时应返回空结果");

        drop(tx);
    }

    /// 测试文件不存在时返回错误
    #[tokio::test]
    async fn test_file_not_found() {
        let flow = create_test_flow();
        let executor = create_executor();
        let config = DatasetExecutionConfig {
            flow_uuid: flow.uuid.clone(),
            dataset_path: "/nonexistent/file.csv".to_string(),
            dataset_encoding: None,
            delimiter: None,
            concurrency: 1,
            failure_strategy: FailureStrategy::Continue,
        };

        let result = executor.execute(&config, &flow, None, None).await;
        assert!(result.is_err(), "文件不存在应返回错误");
        assert!(
            result.unwrap_err().contains("无法打开文件"),
            "错误信息应包含文件相关提示"
        );
    }

    /// 测试自定义分隔符
    #[tokio::test]
    async fn test_custom_delimiter() {
        let csv = create_temp_csv(b"name;age\nAlice;30\nBob;25\n");
        let path = csv.path().to_str().expect("路径转换失败");

        let flow = create_test_flow();
        let executor = create_executor();
        let config = DatasetExecutionConfig {
            flow_uuid: flow.uuid.clone(),
            dataset_path: path.to_string(),
            dataset_encoding: None,
            delimiter: Some(';'),
            concurrency: 1,
            failure_strategy: FailureStrategy::Continue,
        };

        let results = executor.execute(&config, &flow, None, None).await.expect("执行失败");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].input_data["name"], json!("Alice"));
        assert_eq!(results[0].input_data["age"], json!("30"));
    }

    /// 测试 runtime_variables 构建（CSV 数据 + 内置变量）
    #[test]
    fn test_runtime_variables_injection() {
        let csv_row = json!({"host": "localhost", "port": "8080"});
        let vars = DatasetExecutor::build_runtime_variables(
            csv_row.clone(),
            0,
            1,
            "test.csv",
        );

        // CSV 原始数据保留
        assert_eq!(vars["host"], json!("localhost"));
        assert_eq!(vars["port"], json!("8080"));
        // 内置变量注入
        assert_eq!(vars["_row_index"], json!(0));
        assert_eq!(vars["_row_count"], json!(1));
        assert_eq!(vars["_dataset_name"], json!("test.csv"));
    }

    /// 测试事件序列：Started → RowCompleted×3 → Progress×3 → Ended
    #[tokio::test]
    async fn test_event_sequence_sequential() {
        let csv = create_temp_csv(b"name\nA\nB\nC\n");
        let path = csv.path().to_str().expect("路径转换失败");

        let flow = create_test_flow();
        let executor = create_executor();
        let config = DatasetExecutionConfig {
            flow_uuid: flow.uuid.clone(),
            dataset_path: path.to_string(),
            dataset_encoding: None,
            delimiter: None,
            concurrency: 1,
            failure_strategy: FailureStrategy::Continue,
        };

        let (event_sender, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<super::events::DatasetEvent>();

        let results = executor.execute(&config, &flow, None, Some(event_sender)).await.expect("执行失败");
        assert_eq!(results.len(), 3);

        // 收集所有事件
        let mut collected_events: Vec<super::events::DatasetEvent> = Vec::new();
        while let Ok(event) = event_rx.try_recv() {
            collected_events.push(event);
        }

        // 验证事件序列: Started → (RowCompleted|Progress) ×3 → Ended
        assert!(
            matches!(collected_events.first(), Some(super::events::DatasetEvent::Started { total_rows: 3, .. })),
            "第一个事件应为 Started"
        );
        assert!(
            matches!(collected_events.last(), Some(super::events::DatasetEvent::Ended { .. })),
            "最后一个事件应为 Ended"
        );

        // 统计 RowCompleted 事件数
        let row_completed_count = collected_events
            .iter()
            .filter(|e| matches!(e, super::events::DatasetEvent::RowCompleted { .. }))
            .count();
        assert_eq!(row_completed_count, 3, "应有 3 个 RowCompleted 事件");

        // 统计 Progress 事件数
        let progress_count = collected_events
            .iter()
            .filter(|e| matches!(e, super::events::DatasetEvent::Progress { .. }))
            .count();
        assert_eq!(progress_count, 3, "应有 3 个 Progress 事件");
    }

    /// 测试取消时发送 Cancelled 事件（含部分报告）
    #[tokio::test]
    async fn test_cancelled_event_with_partial_report() {
        let csv = create_temp_csv(b"name\nA\nB\nC\nD\nE\n");
        let path = csv.path().to_str().expect("路径转换失败");

        let flow = create_test_flow();
        let executor = create_executor();
        let config = DatasetExecutionConfig {
            flow_uuid: flow.uuid.clone(),
            dataset_path: path.to_string(),
            dataset_encoding: None,
            delimiter: None,
            concurrency: 1,
            failure_strategy: FailureStrategy::Continue,
        };

        let (tx, rx) = watch::channel(true); // 已触发取消
        let (event_sender, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<super::events::DatasetEvent>();

        let _results = executor.execute(&config, &flow, Some(rx), Some(event_sender)).await.expect("执行失败");

        // 收集事件
        let mut collected_events: Vec<super::events::DatasetEvent> = Vec::new();
        while let Ok(event) = event_rx.try_recv() {
            collected_events.push(event);
        }

        // 应有 Cancelled 事件
        let cancelled_events: Vec<_> = collected_events
            .iter()
            .filter(|e| matches!(e, super::events::DatasetEvent::Cancelled { .. }))
            .collect();
        assert_eq!(cancelled_events.len(), 1, "应有 1 个 Cancelled 事件");

        drop(tx);
    }
}
