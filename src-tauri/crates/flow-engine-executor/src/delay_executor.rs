//! 延时节点执行器
//!
//! `flow:delay` 的唯一语义就是「等待」：时长按毫秒解析（支持变量模板），
//! 可选随机抖动，等待可被取消信号中断。等待原语复用
//! [`flow_engine_core::context::ExecutionContext::wait_or_cancelled`]，
//! 与 `flow:loop` 的迭代间隔共享同一实现。

use flow_engine_core::connection::Connection;
use flow_engine_core::context::ExecutionContext;
use flow_engine_core::error::{ErrorCode, ErrorDetail, ErrorKind};
use flow_engine_core::executor::NodeExecutor;
use flow_engine_core::node::Node;
use flow_engine_core::node_config::NodeData;
use serde_json::json;
use std::collections::HashMap;

/// 单次延时的硬上限（1 小时）
pub const MAX_DELAY_MS: u64 = 3_600_000;

/// 延时节点执行器
#[derive(Debug, Default, Clone, Copy)]
pub struct DelayNodeExecutor;

impl DelayNodeExecutor {
    /// 创建执行器
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl NodeExecutor for DelayNodeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &mut ExecutionContext,
        connections: &[Connection],
        _node_map: &HashMap<String, &Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), ErrorDetail> {
        let NodeData::Delay(cfg) = &node.data else {
            return Err(ErrorDetail::new(
                ErrorKind::Configuration,
                format!("flow:delay node '{}' missing DelayConfig", node.name),
            )
            .with_error_code_typed(ErrorCode::E_CONFIG_ERROR));
        };

        let raw = flow_engine_core::executor::replace_variables(&cfg.duration_ms, context)
            .map_err(|e| {
                ErrorDetail::new(
                    ErrorKind::Configuration,
                    format!("Delay duration '{}' render failed: {}", cfg.duration_ms, e),
                )
                .with_error_code_typed(ErrorCode::E_CONFIG_ERROR)
            })?;
        let ms: u64 = raw.trim().parse().map_err(|_| {
            ErrorDetail::new(
                ErrorKind::Configuration,
                format!(
                    "Invalid delay duration '{}' (expected integer milliseconds)",
                    raw
                ),
            )
            .with_error_code_typed(ErrorCode::E_CONFIG_ERROR)
        })?;
        if ms > MAX_DELAY_MS {
            return Err(ErrorDetail::new(
                ErrorKind::Configuration,
                format!("Delay duration {} ms exceeds 1 hour limit", ms),
            )
            .with_error_code_typed(ErrorCode::E_CONFIG_ERROR));
        }

        let jitter = if cfg.jitter_ms > 0 {
            use rand::Rng;
            rand::thread_rng().gen_range(0..=cfg.jitter_ms)
        } else {
            0
        };

        let started = std::time::Instant::now();
        // 抖动可能来自未经过校验的流程文件：饱和加法避免 u64 溢出 panic
        let total_ms = ms.saturating_add(jitter);
        if !context.wait_or_cancelled(total_ms).await {
            return Err(ErrorDetail::new(
                ErrorKind::Execution,
                "Delay cancelled by user".to_string(),
            )
            .with_error_code_typed(ErrorCode::E_CANCELLED));
        }

        let output = json!({
            "requested_ms": ms,
            "jitter_ms": jitter,
            "waited_ms": started.elapsed().as_millis() as u64,
        });

        let next: Vec<String> = connections
            .iter()
            .filter(|c| {
                c.source_node_uuid == node.uuid
                    && (c.source_port_id == "out" || c.source_port_id == "true")
            })
            .map(|c| c.target_node_uuid.clone())
            .collect();

        Ok((next, Some(output)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node::NodePosition;
    use flow_engine_core::node_config::DelayConfig;
    use serde_json::json;

    fn make_node(uuid: &str, cfg: DelayConfig) -> Node {
        Node {
            uuid: uuid.to_string(),
            name: "delay".to_string(),
            node_type: flow_engine_core::node_types::DELAY.to_string(),
            position: NodePosition::default(),
            data: NodeData::Delay(cfg),
            on_error: None,
            ..Default::default()
        }
    }

    fn make_context() -> ExecutionContext {
        ExecutionContext::new("flow-1".to_string(), 0)
    }

    #[tokio::test]
    async fn delay_waits_requested_duration() {
        let node = make_node(
            "n1",
            DelayConfig {
                duration_ms: "30".to_string(),
                jitter_ms: 0,
            },
        );
        let mut ctx = make_context();
        let started = std::time::Instant::now();
        let (next, output) = DelayNodeExecutor::new()
            .execute(&node, &mut ctx, &[], &HashMap::new())
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(next.is_empty());
        assert!(started.elapsed().as_millis() >= 30);
        let output = output.unwrap_or_else(|| panic!("expected output"));
        assert_eq!(output["requested_ms"], json!(30));
        assert_eq!(output["jitter_ms"], json!(0));
    }

    #[tokio::test]
    async fn delay_duration_variable_rendered() {
        let node = make_node(
            "n1",
            DelayConfig {
                duration_ms: "{{wait_ms}}".to_string(),
                jitter_ms: 0,
            },
        );
        let mut ctx = make_context();
        ctx.variables
            .set_variable("wait_ms".to_string(), json!("10"), "test".to_string());
        let (_, output) = DelayNodeExecutor::new()
            .execute(&node, &mut ctx, &[], &HashMap::new())
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            output.unwrap_or_else(|| panic!("expected output"))["requested_ms"],
            json!(10)
        );
    }

    #[tokio::test]
    async fn delay_invalid_duration_keeps_source() {
        let node = make_node(
            "n1",
            DelayConfig {
                duration_ms: "abc".to_string(),
                jitter_ms: 0,
            },
        );
        let mut ctx = make_context();
        let err = DelayNodeExecutor::new()
            .execute(&node, &mut ctx, &[], &HashMap::new())
            .await
            .err()
            .unwrap_or_else(|| panic!("expected error"));
        assert!(err.message.contains("abc"), "unexpected: {}", err.message);
        assert_eq!(err.error_kind, ErrorKind::Configuration);
    }

    #[tokio::test]
    async fn delay_exceeds_limit_rejected() {
        let node = make_node(
            "n1",
            DelayConfig {
                duration_ms: format!("{}", MAX_DELAY_MS + 1),
                jitter_ms: 0,
            },
        );
        let mut ctx = make_context();
        let err = DelayNodeExecutor::new()
            .execute(&node, &mut ctx, &[], &HashMap::new())
            .await
            .err()
            .unwrap_or_else(|| panic!("expected error"));
        assert!(
            err.message.contains("exceeds 1 hour"),
            "unexpected: {}",
            err.message
        );
    }

    #[tokio::test]
    async fn delay_cancel_interrupts_wait() {
        let node = make_node(
            "n1",
            DelayConfig {
                duration_ms: "60000".to_string(),
                jitter_ms: 0,
            },
        );
        let (tx, rx) = tokio::sync::watch::channel(false);
        let mut ctx = make_context();
        ctx.cancel_receiver = Some(rx);
        let started = std::time::Instant::now();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let _ = tx.send(true);
        });
        let err = DelayNodeExecutor::new()
            .execute(&node, &mut ctx, &[], &HashMap::new())
            .await
            .err()
            .unwrap_or_else(|| panic!("expected cancellation error"));
        handle.await.unwrap_or_else(|e| panic!("{e}"));
        assert!(started.elapsed().as_millis() < 2000);
        assert_eq!(err.error_kind, ErrorKind::Execution);
    }

    #[tokio::test]
    async fn delay_jitter_within_bounds() {
        let node = make_node(
            "n1",
            DelayConfig {
                duration_ms: "10".to_string(),
                jitter_ms: 40,
            },
        );
        let mut ctx = make_context();
        let started = std::time::Instant::now();
        let (_, output) = DelayNodeExecutor::new()
            .execute(&node, &mut ctx, &[], &HashMap::new())
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        let waited = started.elapsed().as_millis();
        let output = output.unwrap_or_else(|| panic!("expected output"));
        let jitter = output["jitter_ms"].as_u64().unwrap_or(0);
        assert!(jitter <= 40);
        assert!(waited >= (10 + jitter) as u128);
    }
}
