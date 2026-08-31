//! The pipelined `PluginConnection`: pending map, per-request timeout,
//! notification subscriptions, and lifecycle.
//!
//! Ownership model:
//! - a **writer task** owns the child's stdin (see `super::writer`),
//! - a **reader task** owns the child's stdout (see `super::reader`),
//! - the connection itself owns the pending map, the notification registry,
//!   the inflight semaphore, and the process handle.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mpe_plugin_sdk::protocol::notification::Notification;
use mpe_plugin_sdk::protocol::request::Request;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, oneshot, watch};

use crate::error::PluginError;
use crate::process::PluginProcess;
use crate::transport::backpressure::Backpressure;
use crate::transport::reader::reader_task;
use crate::transport::writer::{writer_task, OutboundFrame};

/// Configuration for a plugin connection.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Maximum in-flight requests (bounded-inflight semaphore).
    pub max_concurrent_requests: usize,
    /// Bounded writer queue capacity (mpsc channel).
    pub queue_capacity: usize,
    /// Per-request timeout; the only mechanism covering stuck-but-alive
    /// plugins.
    pub request_timeout: Duration,
    /// Consecutive malformed lines tolerated before the plugin is killed.
    pub max_malformed_lines: u32,
}

impl ConnectionConfig {
    /// Sets the maximum number of concurrent requests.
    pub fn with_max_concurrent_requests(mut self, max: usize) -> Self {
        self.max_concurrent_requests = max;
        self
    }

    /// Sets the bounded writer queue capacity.
    pub fn with_queue_capacity(mut self, cap: usize) -> Self {
        self.queue_capacity = cap;
        self
    }

    /// Sets the per-request default timeout.
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        let max_concurrent_requests = std::env::var("MPE_PLUGIN_MAX_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(256);
        let queue_capacity = std::env::var("MPE_PLUGIN_QUEUE_CAPACITY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2048);

        Self {
            max_concurrent_requests,
            queue_capacity,
            request_timeout: Duration::from_secs(30),
            max_malformed_lines: 10,
        }
    }
}

/// Why the connection reached its terminal state.
#[derive(Debug, Clone, PartialEq)]
pub enum ExitReason {
    /// The connection is still running.
    Running,
    /// Clean EOF: the plugin closed stdout after shutdown with no pending
    /// requests outstanding.
    CleanEof,
    /// The plugin crashed / its stdio was lost while requests were pending,
    /// or it was killed after malformed-line spam.
    Crashed(PluginError),
}

/// Correlation filter for notification subscriptions.
///
/// A `None` field matches anything; an `Some` field matches only
/// notifications whose `params` carry the same `execution_id` /
/// `node_instance_id` (see `mpe_plugin_sdk::protocol::Notification::correlation`).
#[derive(Debug, Clone, Default)]
pub struct CorrelationFilter {
    /// Restrict to notifications from this execution.
    pub execution_id: Option<String>,
    /// Restrict to notifications from this node instance.
    pub node_instance_id: Option<String>,
}

/// A live notification subscription; dropped to unsubscribe.
pub struct SubscriptionHandle {
    shared: Arc<ConnectionShared>,
    id: u64,
}

impl Drop for SubscriptionHandle {
    fn drop(&mut self) {
        self.shared
            .subscriptions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|sub| sub.id != self.id);
    }
}

/// Transport metrics snapshot.
#[derive(Debug, Clone, Copy, Default)]
pub struct TransportMetrics {
    /// Malformed lines observed (never fatal individually).
    pub malformed_lines: u64,
    /// Slow-consumer waits (writer queue was full).
    pub slow_consumer: u64,
    /// Responses for unknown/duplicate ids.
    pub unknown_ids: u64,
    /// Responses received after the request timed out or was cancelled.
    pub late_responses: u64,
    /// Requests that timed out.
    pub timeouts: u64,
    /// Total requests dispatched.
    pub requests_total: u64,
    /// Free in-flight slots right now.
    pub available_inflight: usize,
    /// Configured in-flight budget.
    pub max_concurrent: usize,
}

struct NotificationSub {
    id: u64,
    method: String,
    filter: CorrelationFilter,
    handler: Box<dyn Fn(&Notification) + Send + Sync>,
}

/// Shared state between the connection handle, writer task, and reader task.
pub(crate) struct ConnectionShared {
    pub config: ConnectionConfig,
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<serde_json::Value, PluginError>>>>,
    /// Ring buffer of recently cancelled or timed out request IDs.
    recent_cancelled_or_timed_out: Mutex<std::collections::VecDeque<u64>>,
    next_id: AtomicU64,
    pub backpressure: Arc<Backpressure>,
    metrics: Mutex<TransportMetrics>,
    exit_set: AtomicBool,
    exit_tx: watch::Sender<ExitReason>,
    /// Kept alive so `exit_tx.send` always has at least one receiver — a
    /// zero-receiver watch channel silently drops updates.
    #[allow(dead_code)]
    _exit_rx: watch::Receiver<ExitReason>,
    child: Mutex<Option<tokio::process::Child>>,
    subscriptions: Mutex<Vec<NotificationSub>>,
    next_sub_id: AtomicU64,
    shutdown_initiated: AtomicBool,
}

impl ConnectionShared {
    fn new(config: ConnectionConfig) -> Self {
        let (exit_tx, _exit_rx) = watch::channel(ExitReason::Running);
        let backpressure = Arc::new(Backpressure::new(config.max_concurrent_requests));
        Self {
            config,
            pending: Mutex::new(HashMap::new()),
            recent_cancelled_or_timed_out: Mutex::new(std::collections::VecDeque::with_capacity(4096)),
            next_id: AtomicU64::new(1), // 0 is reserved for notifications
            backpressure,
            metrics: Mutex::new(TransportMetrics::default()),
            exit_set: AtomicBool::new(false),
            exit_tx,
            _exit_rx,
            child: Mutex::new(None),
            subscriptions: Mutex::new(Vec::new()),
            next_sub_id: AtomicU64::new(1),
            shutdown_initiated: AtomicBool::new(false),
        }
    }


    /// Marks the connection as exited exactly once (first caller wins),
    /// publishing the reason and draining the pending map so every waiter
    /// gets `Err(PluginCrashed)`. The exit reason is published **before** the
    /// drain so waiters that wake on their oneshot already observe the final
    /// state.
    pub(crate) fn mark_exit(&self, reason: ExitReason) -> bool {
        if self.exit_set.swap(true, Ordering::AcqRel) {
            return false;
        }
        let _ = self.exit_tx.send(reason.clone());
        let detail = match &reason {
            ExitReason::Crashed(e) => format!("{e}"),
            _ => "connection closed".to_string(),
        };
        let pending = std::mem::take(&mut *self.pending.lock().unwrap_or_else(|e| e.into_inner()));
        for (_, tx) in pending {
            let _ = tx.send(Err(PluginError::PluginCrashed(detail.clone())));
        }
        true
    }

    /// Routes a response to its pending oneshot, if any.
    pub(crate) fn resolve_response(&self, id: u64, result: Result<serde_json::Value, PluginError>) {
        let removed = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
        match removed {
            Some(tx) => {
                let _ = tx.send(result);
            }
            None => {
                let is_recent_timed_out = {
                    let history = self
                        .recent_cancelled_or_timed_out
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    history.contains(&id)
                };
                if is_recent_timed_out {
                    self.metrics
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .late_responses += 1;
                    log::debug!("late response for timed-out/cancelled request id {id}");
                } else {
                    self.metrics
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .unknown_ids += 1;
                    log::warn!("response for unknown/unmatched request id {id}");
                }
            }
        }
    }

    /// Records a request id that timed out or was cancelled.
    pub(crate) fn record_timed_out_or_cancelled(&self, id: u64) {
        let mut history = self
            .recent_cancelled_or_timed_out
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if history.len() >= 4096 {
            history.pop_front();
        }
        history.push_back(id);
    }

    /// Demuxes a notification to matching subscriptions (before any pending
    /// lookup — notifications never carry an id, so they cannot be
    /// responses).
    pub(crate) fn dispatch_notification(&self, notification: &Notification) {
        let subscriptions = self.subscriptions.lock().unwrap_or_else(|e| e.into_inner());
        let correlation = notification.correlation();
        for sub in subscriptions
            .iter()
            .filter(|s| s.method == notification.method)
        {
            if let Some(expected) = &sub.filter.execution_id {
                match &correlation {
                    Some(c) if &c.execution_id == expected => {}
                    _ => continue,
                }
            }
            if let Some(expected) = &sub.filter.node_instance_id {
                match &correlation {
                    Some(c) if &c.node_instance_id == expected => {}
                    _ => continue,
                }
            }
            (sub.handler)(notification);
        }
    }

    /// Records a malformed line in the metrics.
    pub(crate) fn note_malformed_line(&self) {
        self.metrics
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .malformed_lines += 1;
    }

    /// Pending-request count (diagnostics).
    #[allow(dead_code)]
    pub(crate) fn pending_count(&self) -> usize {
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Best-effort kill of the plugin process (used after malformed spam).
    pub(crate) async fn kill_child(&self) {
        let child = self.child.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(mut child) = child {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }

    /// Reader-EOF handling: distinguishes clean EOF (pending empty AND
    /// shutdown initiated) from a crash (pending non-empty / no shutdown),
    /// consulting `child.try_wait()` for the exit status.
    pub(crate) async fn on_eof(&self) {
        let pending_non_empty = !self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty();
        let shutdown = self.shutdown_initiated.load(Ordering::SeqCst);
        let status_desc = {
            let mut guard = self.child.lock().unwrap_or_else(|e| e.into_inner());
            guard
                .as_mut()
                .and_then(|c| c.try_wait().ok().flatten())
                .map(|status| format!("{status:?}"))
        };
        if pending_non_empty || !shutdown {
            let detail = status_desc
                .unwrap_or_else(|| "no exit status available (still running?)".to_string());
            self.mark_exit(ExitReason::Crashed(PluginError::PluginExited(detail)));
        } else {
            self.mark_exit(ExitReason::CleanEof);
        }
    }

    /// Rejects requests once shutdown begins or the connection has exited.
    pub(crate) fn check_alive(&self) -> Result<(), PluginError> {
        if self.shutdown_initiated.load(Ordering::SeqCst) {
            return Err(PluginError::Shutdown);
        }
        match self.exit_tx.borrow().clone() {
            ExitReason::Running => Ok(()),
            ExitReason::CleanEof => Err(PluginError::Shutdown),
            ExitReason::Crashed(e) => Err(e),
        }
    }

    fn snapshot(&self) -> TransportMetrics {
        let metrics = self.metrics.lock().unwrap_or_else(|e| e.into_inner());
        TransportMetrics {
            malformed_lines: metrics.malformed_lines,
            slow_consumer: self.backpressure.slow_consumer_count(),
            unknown_ids: metrics.unknown_ids,
            late_responses: metrics.late_responses,
            timeouts: metrics.timeouts,
            requests_total: metrics.requests_total,
            available_inflight: self.backpressure.available(),
            max_concurrent: self.config.max_concurrent_requests,
        }
    }
}

/// Removes its pending entry on drop (covers callers cancelling the request
/// future, e.g. via an outer `select!`).
struct PendingGuard<'a> {
    shared: &'a ConnectionShared,
    id: u64,
    armed: bool,
}

impl Drop for PendingGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.shared
                .pending
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&self.id);
            self.shared.record_timed_out_or_cancelled(self.id);
        }
    }
}

/// A pipelined JSON-RPC connection to a plugin process.
///
/// Cloning is cheap (all state is shared behind `Arc`); each clone can issue
/// requests and subscribe to notifications concurrently. The plugin process
/// is killed only when the **last** handle is dropped (each clone drops its
/// own writer-channel sender; the child is reaped once).
pub struct PluginConnection {
    writer_tx: Option<mpsc::Sender<OutboundFrame>>,
    shared: Arc<ConnectionShared>,
    config: ConnectionConfig,
    #[cfg_attr(not(windows), allow(dead_code))]
    #[allow(dead_code)] // RAII: kept alive so the Job Object outlives the child
    job_guard: Option<Arc<crate::process::PluginProcessGuard>>,
    /// Live handle count; the child is reaped when this reaches zero.
    alive: Arc<AtomicUsize>,
}

impl Clone for PluginConnection {
    fn clone(&self) -> Self {
        self.alive.fetch_add(1, Ordering::Relaxed);
        Self {
            writer_tx: self.writer_tx.clone(),
            shared: Arc::clone(&self.shared),
            config: self.config.clone(),
            job_guard: self.job_guard.clone(),
            alive: Arc::clone(&self.alive),
        }
    }
}

impl Drop for PluginConnection {
    fn drop(&mut self) {
        // Closing this handle's sender moves the channel one step closer to
        // close; when the last handle drops, the writer half-closes stdin.
        self.writer_tx.take();
        // Only the last handle reaps (kills) the plugin process.
        if self.alive.fetch_sub(1, Ordering::AcqRel) == 1
            && tokio::runtime::Handle::try_current().is_ok()
        {
            let shared = Arc::clone(&self.shared);
            tokio::spawn(async move {
                let child = shared
                    .child
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take();
                if let Some(mut child) = child {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
            });
        }
    }
}

impl PluginConnection {
    /// Attaches to a freshly spawned plugin process.
    ///
    /// The process's stderr must already be drained (see
    /// [`crate::process::spawn_plugin`], which does it at spawn time).
    pub fn spawn(process: PluginProcess, config: ConnectionConfig) -> Result<Self, PluginError> {
        let (mut child, job_guard) = process.into_parts();
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| PluginError::ProtocolError("child stdin pipe missing".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| PluginError::ProtocolError("child stdout pipe missing".to_string()))?;
        let shared = Arc::new(ConnectionShared::new(config.clone()));
        *shared.child.lock().unwrap_or_else(|e| e.into_inner()) = Some(child);
        let (writer_tx, writer_rx) = mpsc::channel(config.queue_capacity);
        let writer_shared = Arc::clone(&shared);
        tokio::spawn(writer_task(Box::new(stdin), writer_rx, writer_shared));
        let reader_shared = Arc::clone(&shared);
        tokio::spawn(reader_task(Box::new(stdout), reader_shared));
        Ok(Self {
            writer_tx: Some(writer_tx),
            shared,
            config,
            job_guard: Some(Arc::new(job_guard)),
            alive: Arc::new(AtomicUsize::new(1)),
        })
    }

    /// Attaches to arbitrary stdio halves (tests use in-memory duplex pipes;
    /// there is no process to kill or reap).
    pub fn from_io<W, R>(stdin_writer: W, stdout_reader: R, config: ConnectionConfig) -> Self
    where
        W: AsyncWrite + Send + Unpin + 'static,
        R: AsyncRead + Send + Unpin + 'static,
    {
        let shared = Arc::new(ConnectionShared::new(config.clone()));
        let (writer_tx, writer_rx) = mpsc::channel(config.queue_capacity);
        let writer_shared = Arc::clone(&shared);
        tokio::spawn(writer_task(
            Box::new(stdin_writer),
            writer_rx,
            writer_shared,
        ));
        let reader_shared = Arc::clone(&shared);
        tokio::spawn(reader_task(Box::new(stdout_reader), reader_shared));
        Self {
            writer_tx: Some(writer_tx),
            shared,
            config,
            job_guard: None,
            alive: Arc::new(AtomicUsize::new(1)),
        }
    }

    /// Sends a request and awaits its response (or dynamic timeout).
    ///
    /// The request holds an inflight permit for its whole lifetime. Every
    /// blocking step — permit acquisition, queue enqueue, response wait — is
    /// bounded by the supplied timeout, so a stuck-but-alive plugin can never
    /// wedge a caller forever. On timeout the pending entry is removed and a
    /// best-effort `cancel` notification is fired; `Err(Timeout)` is
    /// returned.
    pub async fn request_with_timeout(
        &self,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, PluginError> {
        self.shared.check_alive()?;

        let deadline = tokio::time::Instant::now() + timeout;

        // Bounded inflight acquisition (never blocks forever).
        let _permit = match tokio::time::timeout_at(deadline, self.shared.backpressure.acquire()).await {
            Ok(Ok(permit)) => permit,
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(PluginError::Timeout(timeout)),
        };

        let id = self.shared.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        // The guard removes the pending entry if this future is dropped at
        // any point (timeout, enqueue failure, or caller cancellation).
        let mut guard = PendingGuard {
            shared: &self.shared,
            id,
            armed: true,
        };
        self.shared
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, tx);
        let request = Request::new(id, method, params);
        let bytes = serde_json::to_vec(&request)
            .map_err(|e| PluginError::ProtocolError(format!("serialize request: {e}")))?;

        // Bounded enqueue: if the writer queue is wedged (plugin not reading
        // its stdin), the caller waits up to the deadline, then gives
        // up — never rejected, but never blocked forever either.
        let enqueued =
            tokio::time::timeout_at(deadline, self.enqueue(OutboundFrame { id, bytes })).await;
        match enqueued {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e), // guard + permit released on return
            Err(_) => {
                self.shared
                    .metrics
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .timeouts += 1;
                self.shared.record_timed_out_or_cancelled(id);
                return Err(PluginError::Timeout(timeout));
            }
        }
        self.shared
            .metrics
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .requests_total += 1;

        tokio::select! {
            result = rx => {
                guard.armed = false;
                drop(_permit);
                result.map_err(|_| PluginError::PluginCrashed("pending response channel dropped".to_string()))?
            }
            _ = tokio::time::sleep_until(deadline) => {
                guard.armed = false;
                self.shared.pending.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
                self.shared.record_timed_out_or_cancelled(id);
                self.shared.metrics.lock().unwrap_or_else(|e| e.into_inner()).timeouts += 1;
                drop(_permit);
                self.fire_cancel(id);
                Err(PluginError::Timeout(timeout))
            }
        }
    }

    /// Sends a request and awaits its response with the configured default timeout.
    pub async fn request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError> {
        self.request_with_timeout(method, params, self.config.request_timeout)
            .await
    }

    /// Sends a fire-and-forget notification (host → plugin).
    pub async fn send_notification(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), PluginError> {
        self.shared.check_alive()?;
        let notification = Notification::new(method, params);
        let bytes = serde_json::to_vec(&notification)
            .map_err(|e| PluginError::ProtocolError(format!("serialize notification: {e}")))?;
        self.enqueue(OutboundFrame { id: 0, bytes }).await
    }

    /// Subscribes to notifications of `method` matching `filter`.
    ///
    /// The returned handle unsubscribes when dropped.
    pub fn register_notification_handler<F>(
        &self,
        method: impl Into<String>,
        filter: CorrelationFilter,
        handler: F,
    ) -> SubscriptionHandle
    where
        F: Fn(&Notification) + Send + Sync + 'static,
    {
        let id = self.shared.next_sub_id.fetch_add(1, Ordering::Relaxed);
        self.shared
            .subscriptions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(NotificationSub {
                id,
                method: method.into(),
                filter,
                handler: Box::new(handler),
            });
        SubscriptionHandle {
            shared: Arc::clone(&self.shared),
            id,
        }
    }

    /// Best-effort cancel of an in-flight request (fire-and-forget).
    pub fn cancel_request(&self, id: u64) {
        self.fire_cancel(id);
    }

    /// Graceful shutdown: stop accepting requests, close the writer channel
    /// (the writer task half-closes stdin), then wait for the reader EOF and
    /// reap the child. Returns the terminal exit reason.
    pub async fn shutdown(&mut self, grace: Duration) -> Result<ExitReason, PluginError> {
        self.shared.shutdown_initiated.store(true, Ordering::SeqCst);
        self.writer_tx.take();
        let reason = self.await_exit(grace).await;
        let child = self
            .shared
            .child
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(mut child) = child {
            let _ = tokio::time::timeout(grace, child.wait()).await;
        }
        Ok(reason)
    }

    /// The current exit reason (`Running` while alive).
    pub fn exit_reason(&self) -> ExitReason {
        self.shared.exit_tx.borrow().clone()
    }

    /// Snapshot of transport metrics.
    pub fn metrics(&self) -> TransportMetrics {
        self.shared.snapshot()
    }

    /// Waits for the reader to observe EOF/exit, up to `grace`.
    async fn await_exit(&self, grace: Duration) -> ExitReason {
        let mut rx = self.shared.exit_tx.subscribe();
        if *rx.borrow() != ExitReason::Running {
            return (*rx.borrow()).clone();
        }
        let _ = tokio::time::timeout(grace, rx.changed()).await;
        let reason = (*rx.borrow()).clone();
        reason
    }

    /// Enqueues a frame, awaiting if the bounded writer queue is full
    /// (backpressure — the caller blocks, the request is never rejected).
    async fn enqueue(&self, frame: OutboundFrame) -> Result<(), PluginError> {
        let tx = self.writer_tx.as_ref().ok_or(PluginError::Shutdown)?;
        match tx.try_send(frame) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(frame)) => {
                self.shared.backpressure.note_slow_consumer();
                tx.send(frame)
                    .await
                    .map_err(|_| PluginError::PluginCrashed("writer channel closed".to_string()))
            }
            Err(TrySendError::Closed(_)) => Err(PluginError::PluginCrashed(
                "writer channel closed".to_string(),
            )),
        }
    }

    /// Best-effort `cancel` notification for a timed-out request id.
    ///
    /// Uses `try_send` and drops the notification if the queue is full — the
    /// cancel must never block the caller that just timed out.
    fn fire_cancel(&self, id: u64) {
        let notification = Notification::new("cancel", serde_json::json!({ "id": id }));
        if let Ok(bytes) = serde_json::to_vec(&notification) {
            if let Some(tx) = self.writer_tx.as_ref() {
                let _ = tx.try_send(OutboundFrame { id, bytes });
            }
        }
    }
}

