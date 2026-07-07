// 压力测试 IPC 命令类型定义

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

use flow_engine_core::flow::Flow;

use super::executor::StressTestExecutor;
use super::{StressTestConfig, StressTestProgress, StressTestReport, StressTestStatus};

/// 开始压力测试命令的输入参数
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StartStressTestInput {
    /// 流程数据
    pub flow: Flow,
    /// 压力测试配置
    pub config: StressTestConfig,
}

/// 停止压力测试命令的输入参数
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StopStressTestInput {}

/// 获取压力测试状态命令的输出结果
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GetStressTestStatusOutput {
    /// 当前状态
    pub status: StressTestStatus,
    /// 进度信息（运行中时有值）
    pub progress: Option<StressTestProgress>,
}

/// 压力测试事件（从后端发送到前端）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StressTestEvent {
    /// 压力测试已启动
    Started {
        /// 流程 UUID
        flow_uuid: String,
        /// 测试配置
        config: StressTestConfig,
    },
    /// 进度更新
    Progress {
        /// 当前进度
        progress: StressTestProgress,
    },
    /// 压力测试已结束
    Ended {
        /// 最终报告
        report: Box<StressTestReport>,
    },
}

// ============================================================================
// 会话管理器
// ============================================================================

/// 压力测试会话管理器
///
/// 管理当前压测会话，确保同时只能有一个压测会话运行。
/// 使用 `Arc<RwLock>` 实现线程安全的会话管理。
pub struct StressTestSessionManager {
    /// 当前活跃的压测会话
    /// - `Some(executor)` - 有正在运行的会话
    /// - `None` - 没有运行中的会话
    session: RwLock<Option<Arc<StressTestExecutor>>>,
}

impl Default for StressTestSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StressTestSessionManager {
    /// 创建新的会话管理器
    pub fn new() -> Self {
        Self {
            session: RwLock::new(None),
        }
    }

    /// 启动新的压测会话
    ///
    /// # 参数
    /// - `flow`: 要执行的流程
    /// - `config`: 压测配置
    /// - `event_sender`: 事件发送器
    ///
    /// # 返回
    /// - `Ok(())` - 启动Success
    /// - `Err(String)` - 启动Failed（如已有会话运行）
    pub async fn start(
        &self,
        flow: Flow,
        config: StressTestConfig,
        event_sender: mpsc::UnboundedSender<StressTestEvent>,
    ) -> Result<(), String> {
        // 验证配置（在获取锁之前进行，避免持有锁时做耗时操作）
        config.validate()?;

        // 获取写锁，确保检查和设置是原子操作，避免 TOCTOU 竞态条件
        let mut session = self.session.write().await;

        // 检查是否已有会话
        if let Some(ref executor) = *session {
            let progress = executor.get_progress().await;
            if progress.status != StressTestStatus::Stopped {
                return Err("A stress test session is already running, please stop it first".to_string());
            }
            // 会话已停止，清理它以允许重新启动
            *session = None;
        }

        // 创建执行器（传入 event_sender）
        let executor = Arc::new(StressTestExecutor::new(flow, config, event_sender.clone()));

        // 启动执行器（在持有锁的情况下启动，确保状态一致性）
        executor.start().await?;

        // 保存会话
        *session = Some(executor);

        Ok(())
    }

    /// 停止当前压测会话
    ///
    /// # 返回
    /// - `Ok(())` - 停止Success
    /// - `Err(String)` - 停止Failed（如没有运行中的会话）
    pub async fn stop(&self) -> Result<(), String> {
        // 获取并清除会话
        let session = {
            let mut guard = self.session.write().await;
            guard.take()
        };

        match session {
            Some(executor) => {
                // 停止执行器
                executor.stop().await?;

                log::info!("[StressTestSessionManager] Session stopped");
                Ok(())
            }
            None => Err("No running stress test session".to_string()),
        }
    }

    /// 获取当前压测状态
    ///
    /// # 返回
    /// - `GetStressTestStatusOutput` - 当前状态和进度
    pub async fn get_status(&self) -> Result<GetStressTestStatusOutput, String> {
        let session = self.session.read().await;

        match session.as_ref() {
            Some(executor) => {
                let progress = executor.get_progress().await;

                Ok(GetStressTestStatusOutput {
                    status: progress.status.clone(),
                    progress: Some(progress),
                })
            }
            None => Ok(GetStressTestStatusOutput {
                status: StressTestStatus::Idle,
                progress: None,
            }),
        }
    }

    /// 检查是否有运行中的会话
    pub async fn is_running(&self) -> bool {
        self.session.read().await.is_some()
    }
}

// ============================================================================
// 命令处理函数
// ============================================================================

/// 处理启动压力测试命令
///
/// # 参数
/// - `input`: 启动参数（流程和配置）
/// - `event_sender`: 事件发送器，用于向前端发送压测事件
/// - `state`: 会话管理器（共享状态）
///
/// # 返回
/// - `Ok(())` - 启动Success
/// - `Err(String)` - 启动Failed
pub async fn handle_start_stress_test(
    input: StartStressTestInput,
    event_sender: mpsc::UnboundedSender<StressTestEvent>,
    state: Arc<StressTestSessionManager>,
) -> Result<(), String> {
    log::info!(
        "[StressTestCommand] Start stress test: flow={}, concurrent={}, duration={}s",
        input.flow.uuid,
        input.config.concurrent_users,
        input.config.duration_seconds
    );

    state.start(input.flow, input.config, event_sender).await
}

/// 处理停止压力测试命令
///
/// # 参数
/// - `state`: 会话管理器（共享状态）
///
/// # 返回
/// - `Ok(())` - 停止Success
/// - `Err(String)` - 停止Failed
pub async fn handle_stop_stress_test(state: Arc<StressTestSessionManager>) -> Result<(), String> {
    log::info!("[StressTestCommand] Stop stress test");
    state.stop().await
}

/// 处理获取压力测试状态命令
///
/// # 参数
/// - `state`: 会话管理器（共享状态）
///
/// # 返回
/// - `Ok(GetStressTestStatusOutput)` - 当前状态和进度
/// - `Err(String)` - 获取状态Failed
pub async fn handle_get_stress_test_status(
    state: Arc<StressTestSessionManager>,
) -> Result<GetStressTestStatusOutput, String> {
    state.get_status().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stress_test::{ResponseTimeDistribution, StressTestStatistics};
    use flow_engine_core::node::{Node, NodePosition, Port, PortType};
    use flow_engine_core::node_config::NodeData;

    fn create_test_flow() -> Flow {
        Flow {
            uuid: "flow-001".to_string(),
            name: "Test Flow".to_string(),
            description: None,
            nodes: vec![Node {
                uuid: "node-001".to_string(),
                node_type: "entry".to_string(),
                name: "Entry".to_string(),
                position: NodePosition { x: 0.0, y: 0.0 },
                data: NodeData::entry(),
                ports: Some(vec![Port {
                    id: "out".to_string(),
                    port_type: PortType::Output,
                    label: Some("Output".to_string()),
                }]),
                has_breakpoint: None,
                on_error: None,
            }],
            connections: vec![],
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            hooks: None,
        }
    }

    #[test]
    fn test_start_stress_test_input_serialization() {
        let flow = create_test_flow();
        let config = StressTestConfig {
            concurrent_users: 5,
            ramp_up_seconds: 5,
            duration_seconds: 30,
            flow_uuid: "flow-001".to_string(),
            ..Default::default()
        };
        let input = StartStressTestInput { flow, config };

        let json = serde_json::to_string(&input).expect("序列化Failed");
        assert!(json.contains("flow"));
        assert!(json.contains("config"));
        assert!(json.contains("concurrentUsers"));
        assert!(json.contains("rampUpSeconds"));
        assert!(json.contains("durationSeconds"));

        let deserialized: StartStressTestInput =
            serde_json::from_str(&json).expect("反序列化Failed");
        assert_eq!(deserialized.config.concurrent_users, 5);
    }

    #[test]
    fn test_stop_stress_test_input_serialization() {
        let input = StopStressTestInput {};

        let json = serde_json::to_string(&input).expect("序列化Failed");
        assert_eq!(json, "{}");

        let deserialized: StopStressTestInput =
            serde_json::from_str(&json).expect("反序列化Failed");
        // 空结构体，验证反序列化Success即可
        let _ = deserialized;
    }

    #[test]
    fn test_get_stress_test_status_output_serialization() {
        let output = GetStressTestStatusOutput {
            status: StressTestStatus::Running,
            progress: Some(StressTestProgress {
                status: StressTestStatus::Running,
                elapsed_seconds: 10,
                total_iterations: 100,
                successful_iterations: 95,
                failed_iterations: 5,
                current_rps: 10.0,
                avg_latency_ms: 150.0,
                node_metrics: vec![],
            }),
        };

        let json = serde_json::to_string(&output).expect("序列化Failed");
        assert!(json.contains("status"));
        assert!(json.contains("progress"));
        assert!(json.contains("elapsedSeconds"));
        assert!(json.contains("totalIterations"));
        assert!(json.contains("successfulIterations"));
        assert!(json.contains("failedIterations"));
        assert!(json.contains("currentRps"));
        assert!(json.contains("avgLatencyMs"));

        let deserialized: GetStressTestStatusOutput =
            serde_json::from_str(&json).expect("反序列化Failed");
        assert_eq!(deserialized.status, StressTestStatus::Running);
        assert!(deserialized.progress.is_some());
    }

    #[test]
    fn test_get_stress_test_status_output_without_progress() {
        let output = GetStressTestStatusOutput {
            status: StressTestStatus::Idle,
            progress: None,
        };

        let json = serde_json::to_string(&output).expect("序列化Failed");
        assert!(json.contains("status"));
        assert!(json.contains("null"));

        let deserialized: GetStressTestStatusOutput =
            serde_json::from_str(&json).expect("反序列化Failed");
        assert_eq!(deserialized.status, StressTestStatus::Idle);
        assert!(deserialized.progress.is_none());
    }

    #[test]
    fn test_stress_test_event_started() {
        let config = StressTestConfig {
            concurrent_users: 10,
            ramp_up_seconds: 10,
            duration_seconds: 60,
            flow_uuid: "flow-001".to_string(),
            ..Default::default()
        };
        let event = StressTestEvent::Started {
            flow_uuid: "flow-001".to_string(),
            config,
        };

        let json = serde_json::to_string(&event).expect("序列化Failed");
        assert!(json.contains("started"));
        assert!(json.contains("flowUuid"));
        assert!(json.contains("config"));

        let deserialized: StressTestEvent = serde_json::from_str(&json).expect("反序列化Failed");
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_stress_test_event_progress() {
        let event = StressTestEvent::Progress {
            progress: StressTestProgress {
                status: StressTestStatus::Running,
                elapsed_seconds: 30,
                total_iterations: 300,
                successful_iterations: 290,
                failed_iterations: 10,
                current_rps: 10.5,
                avg_latency_ms: 120.5,
                node_metrics: vec![],
            },
        };

        let json = serde_json::to_string(&event).expect("序列化Failed");
        assert!(json.contains("progress"));
        assert!(json.contains("elapsedSeconds"));
        assert!(json.contains("currentRps"));

        let deserialized: StressTestEvent = serde_json::from_str(&json).expect("反序列化Failed");
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_stress_test_event_ended() {
        let report = StressTestReport {
            uuid: "report-001".to_string(),
            flow_uuid: "flow-001".to_string(),
            flow_name: "Test Flow".to_string(),
            start_time: 1000,
            end_time: 2000,
            duration_ms: 1000,
            config: StressTestConfig::default(),
            statistics: StressTestStatistics {
                total_iterations: 100,
                successful: 95,
                failed: 5,
                success_rate: 0.95,
                rps: 10.0,
            },
            response_time_distribution: ResponseTimeDistribution {
                avg_ms: 150.0,
                min_ms: 50,
                max_ms: 500,
                p50_ms: 140,
                p90_ms: 300,
                p95_ms: 400,
                p99_ms: 450,
            },
            error_statistics: vec![],
            node_statistics: vec![],
            node_type_statistics: vec![],
            progress_history: vec![],
            flow_snapshot: None,
        };
        let event = StressTestEvent::Ended {
            report: Box::new(report),
        };

        let json = serde_json::to_string(&event).expect("序列化Failed");
        assert!(json.contains("ended"));
        assert!(json.contains("report"));
        assert!(json.contains("report-001"));

        let deserialized: StressTestEvent = serde_json::from_str(&json).expect("反序列化Failed");
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_all_stress_test_event_variants() {
        let events = vec![
            StressTestEvent::Started {
                flow_uuid: "flow-001".to_string(),
                config: StressTestConfig::default(),
            },
            StressTestEvent::Progress {
                progress: StressTestProgress::default(),
            },
            StressTestEvent::Ended {
                report: Box::new(StressTestReport::default()),
            },
        ];

        for event in events {
            let json = serde_json::to_string(&event).expect("序列化Failed");
            let deserialized: StressTestEvent =
                serde_json::from_str(&json).expect("反序列化Failed");
            assert_eq!(event, deserialized);
        }
    }

    // ============================================================================
    // StressTestSessionManager 测试
    // ============================================================================

    #[tokio::test]
    async fn test_session_manager_new() {
        let manager = StressTestSessionManager::new();
        assert!(!manager.is_running().await);
    }

    #[tokio::test]
    async fn test_session_manager_default() {
        let manager = StressTestSessionManager::default();
        assert!(!manager.is_running().await);
    }

    #[tokio::test]
    async fn test_session_manager_get_status_idle() {
        let manager = StressTestSessionManager::new();
        let result = manager.get_status().await.expect("获取状态Failed");

        assert_eq!(result.status, StressTestStatus::Idle);
        assert!(result.progress.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_stop_without_session() {
        let manager = StressTestSessionManager::new();
        let result = manager.stop().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No running stress test session"));
    }

    #[tokio::test]
    async fn test_session_manager_start_success() {
        let manager = Arc::new(StressTestSessionManager::new());
        let (event_sender, _event_receiver) = mpsc::unbounded_channel();

        let flow = create_test_flow();
        let config = StressTestConfig {
            concurrent_users: 5,
            ramp_up_seconds: 5,
            duration_seconds: 30,
            flow_uuid: flow.uuid.clone(),
            ..Default::default()
        };

        let input = StartStressTestInput { flow, config };

        let result = handle_start_stress_test(input, event_sender, manager.clone()).await;
        assert!(result.is_ok(), "启动应该Failed: {:?}", result.err());
        assert!(manager.is_running().await);

        // 清理：停止会话
        let stop_result = handle_stop_stress_test(manager).await;
        assert!(stop_result.is_ok());
    }

    #[tokio::test]
    async fn test_session_manager_start_twice_fails() {
        let manager = Arc::new(StressTestSessionManager::new());
        let (event_sender1, _event_receiver1) = mpsc::unbounded_channel();
        let (event_sender2, _event_receiver2) = mpsc::unbounded_channel();

        // 第一次启动
        let flow1 = create_test_flow();
        let config1 = StressTestConfig {
            concurrent_users: 5,
            ramp_up_seconds: 5,
            duration_seconds: 30,
            flow_uuid: flow1.uuid.clone(),
            ..Default::default()
        };
        let input1 = StartStressTestInput {
            flow: flow1,
            config: config1,
        };

        let result1 = handle_start_stress_test(input1, event_sender1, manager.clone()).await;
        assert!(result1.is_ok(), "第一次启动应该Success");

        // 第二次启动应该Failed
        let flow2 = Flow {
            uuid: "flow-002".to_string(),
            name: "Test Flow 2".to_string(),
            description: None,
            nodes: vec![Node {
                uuid: "node-002".to_string(),
                node_type: "entry".to_string(),
                name: "Entry".to_string(),
                position: NodePosition { x: 0.0, y: 0.0 },
                data: NodeData::entry(),
                ports: None,
                has_breakpoint: None,
                on_error: None,
            }],
            connections: vec![],
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            hooks: None,
        };
        let config2 = StressTestConfig {
            concurrent_users: 10,
            ramp_up_seconds: 10,
            duration_seconds: 60,
            flow_uuid: flow2.uuid.clone(),
            ..Default::default()
        };
        let input2 = StartStressTestInput {
            flow: flow2,
            config: config2,
        };

        let result2 = handle_start_stress_test(input2, event_sender2, manager.clone()).await;
        assert!(result2.is_err(), "第二次启动应该Failed");
        let error = result2.unwrap_err();
        assert!(error.contains("A stress test session is already running"));

        // 清理
        let _ = handle_stop_stress_test(manager).await;
    }

    #[tokio::test]
    async fn test_session_manager_start_invalid_config() {
        let manager = Arc::new(StressTestSessionManager::new());
        let (event_sender, _event_receiver) = mpsc::unbounded_channel();

        let flow = create_test_flow();
        // 无效配置：并发用户数为 0
        let config = StressTestConfig {
            concurrent_users: 0,
            ramp_up_seconds: 5,
            duration_seconds: 30,
            flow_uuid: flow.uuid.clone(),
            ..Default::default()
        };

        let input = StartStressTestInput { flow, config };

        let result = handle_start_stress_test(input, event_sender, manager.clone()).await;
        assert!(result.is_err(), "无效配置应该Failed");
        let error = result.unwrap_err();
        assert!(error.contains("并发用户数"));

        assert!(!manager.is_running().await);
    }

    #[tokio::test]
    async fn test_session_manager_stop_success() {
        let manager = Arc::new(StressTestSessionManager::new());
        let (event_sender, _event_receiver) = mpsc::unbounded_channel();

        // 先启动
        let flow = create_test_flow();
        let config = StressTestConfig {
            concurrent_users: 5,
            ramp_up_seconds: 5,
            duration_seconds: 30,
            flow_uuid: flow.uuid.clone(),
            ..Default::default()
        };
        let input = StartStressTestInput { flow, config };
        let _ = handle_start_stress_test(input, event_sender, manager.clone()).await;

        // 然后停止
        let result = handle_stop_stress_test(manager.clone()).await;
        assert!(result.is_ok(), "停止应该Success");
        assert!(!manager.is_running().await);

        // 再次停止应该Failed
        let result2 = handle_stop_stress_test(manager).await;
        assert!(result2.is_err());
    }

    #[tokio::test]
    async fn test_session_manager_get_status_running() {
        let manager = Arc::new(StressTestSessionManager::new());
        let (event_sender, _event_receiver) = mpsc::unbounded_channel();

        // 启动
        let flow = create_test_flow();
        let config = StressTestConfig {
            concurrent_users: 5,
            ramp_up_seconds: 5,
            duration_seconds: 30,
            flow_uuid: flow.uuid.clone(),
            ..Default::default()
        };
        let input = StartStressTestInput { flow, config };
        let _ = handle_start_stress_test(input, event_sender, manager.clone()).await;

        // 获取状态
        let result = handle_get_stress_test_status(manager.clone()).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        // 状态应该是 RampUp 或 Running（取决于执行速度）
        assert_ne!(output.status, StressTestStatus::Idle);

        // 清理
        let _ = handle_stop_stress_test(manager).await;
    }

    #[tokio::test]
    async fn test_handle_get_stress_test_status_idle() {
        let manager = Arc::new(StressTestSessionManager::new());

        let result = handle_get_stress_test_status(manager).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.status, StressTestStatus::Idle);
        assert!(output.progress.is_none());
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let manager = Arc::new(StressTestSessionManager::new());

        // 初始状态：空闲
        assert!(!manager.is_running().await);
        let status = manager.get_status().await.expect("获取状态Failed");
        assert_eq!(status.status, StressTestStatus::Idle);

        // 启动
        let (event_sender, _event_receiver) = mpsc::unbounded_channel();
        let flow = create_test_flow();
        let config = StressTestConfig {
            concurrent_users: 10,
            ramp_up_seconds: 10,
            duration_seconds: 60,
            flow_uuid: flow.uuid.clone(),
            ..Default::default()
        };

        manager
            .start(flow, config, event_sender)
            .await
            .expect("启动Failed");

        assert!(manager.is_running().await);

        // 停止
        manager.stop().await.expect("停止Failed");
        assert!(!manager.is_running().await);

        // 验证可以重新启动
        let (event_sender2, _event_receiver2) = mpsc::unbounded_channel::<StressTestEvent>();
        let flow2 = Flow {
            uuid: "flow-002".to_string(),
            name: "Test Flow 2".to_string(),
            description: None,
            nodes: vec![Node {
                uuid: "node-002".to_string(),
                node_type: "entry".to_string(),
                name: "Entry".to_string(),
                position: NodePosition { x: 0.0, y: 0.0 },
                data: NodeData::entry(),
                ports: None,
                has_breakpoint: None,
                on_error: None,
            }],
            connections: vec![],
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            hooks: None,
        };
        let config2 = StressTestConfig {
            concurrent_users: 5,
            ramp_up_seconds: 5,
            duration_seconds: 30,
            flow_uuid: flow2.uuid.clone(),
            ..Default::default()
        };

        let result = manager.start(flow2, config2, event_sender2).await;
        assert!(result.is_ok(), "重新启动应该Success");

        // 清理
        let _ = manager.stop().await;
    }

    /// 测试并发启动安全性
    ///
    /// 验证多个并发启动请求只有一个能Success，确保 TOCTOU 竞态条件已修复。
    #[tokio::test]
    async fn test_concurrent_start_safety() {
        use futures_util::future::join_all;

        let manager = Arc::new(StressTestSessionManager::new());
        let (event_sender, _event_receiver) = mpsc::unbounded_channel();

        // 创建多个相同的启动请求
        let mut handles = vec![];

        for _ in 0..5 {
            let manager_clone = Arc::clone(&manager);
            let sender_clone = event_sender.clone();
            let flow = create_test_flow();
            let config = StressTestConfig {
                concurrent_users: 1,
                ramp_up_seconds: 0,
                duration_seconds: 10,
                flow_uuid: flow.uuid.clone(),
                ..Default::default()
            };

            let handle =
                tokio::spawn(async move { manager_clone.start(flow, config, sender_clone).await });
            handles.push(handle);
        }

        // 等待所有请求完成
        let results: Vec<_> = join_all(handles).await;

        // 统计Success和Failed的数量
        let success_count = results.iter().filter(|r| matches!(r, Ok(Ok(())))).count();
        let failure_count = results.iter().filter(|r| matches!(r, Ok(Err(_)))).count();

        // 只有一个应该Success
        assert_eq!(
            success_count, 1,
            "只有一个启动请求应该Success，但有 {} 个Success了",
            success_count
        );
        assert_eq!(
            failure_count, 4,
            "四个启动请求应该Failed，但有 {} 个Failed了",
            failure_count
        );

        // 验证管理器状态
        assert!(manager.is_running().await);

        // 清理
        let _ = manager.stop().await;
    }
}
