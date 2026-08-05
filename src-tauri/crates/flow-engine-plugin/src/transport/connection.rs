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

use mpe_protocol::notification::Notification;
use mpe_protocol::request::Request;
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

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 32,
            queue_capacity: 256,
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
/// `node_instance_id` (see `mpe_protocol::Notification::correlation`).
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

    /// Test-only shared state with default config and no tasks/child.
    #[cfg(test)]
    pub(crate) fn dummy() -> Self {
        Self::new(ConnectionConfig::default())
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
                self.metrics
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .unknown_ids += 1;
                log::warn!("response for unknown/unmatched request id {id}");
            }
        }
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

    /// Sends a request and awaits its response (or timeout).
    ///
    /// The request holds an inflight permit for its whole lifetime. Every
    /// blocking step — permit acquisition, queue enqueue, response wait — is
    /// bounded by the request timeout, so a stuck-but-alive plugin can never
    /// wedge a caller forever. On timeout the pending entry is removed and a
    /// best-effort `cancel` notification is fired; `Err(Timeout)` is
    /// returned.
    pub async fn request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError> {
        self.shared.check_alive()?;
        let timeout = self.config.request_timeout;

        // Bounded inflight acquisition (never blocks forever).
        let _permit = tokio::time::timeout(timeout, self.shared.backpressure.acquire())
            .await
            .map_err(|_| PluginError::Timeout(timeout))??;

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
        // its stdin), the caller waits up to the request timeout, then gives
        // up — never rejected, but never blocked forever either.
        let enqueued =
            tokio::time::timeout(timeout, self.enqueue(OutboundFrame { id, bytes })).await;
        match enqueued {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e), // guard + permit released on return
            Err(_) => {
                self.shared
                    .metrics
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .timeouts += 1;
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
            _ = tokio::time::sleep(timeout) => {
                guard.armed = false;
                self.shared.pending.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
                self.shared.metrics.lock().unwrap_or_else(|e| e.into_inner()).timeouts += 1;
                drop(_permit);
                self.fire_cancel(id);
                Err(PluginError::Timeout(timeout))
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framing::FrameReader;
    use crate::framing::FrameWriter;
    use mpe_protocol::notification::Notification;
    use mpe_protocol::request::Request;

    /// The plugin side of an in-memory stdio pair: reads host requests from
    /// `reader`, writes responses to `tx`.
    struct PluginSide {
        reader: FrameReader<tokio::io::DuplexStream>,
        tx: tokio::io::DuplexStream,
    }

    impl PluginSide {
        async fn read_frame(
            &mut self,
        ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(self.reader.next_frame().await?)
        }

        async fn read_request(
            &mut self,
        ) -> Result<Request, Box<dyn std::error::Error + Send + Sync>> {
            let frame = self
                .read_frame()
                .await?
                .ok_or_else(|| "unexpected eof while reading request".to_string())?;
            Ok(serde_json::from_slice(&frame)?)
        }

        async fn write_frame(
            &mut self,
            bytes: &[u8],
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let mut writer = FrameWriter::new(&mut self.tx);
            writer.write_frame(bytes).await?;
            Ok(())
        }

        async fn respond(
            &mut self,
            id: u64,
            result: serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let response = serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result });
            self.write_frame(&serde_json::to_vec(&response)?).await
        }

        async fn respond_error(
            &mut self,
            id: u64,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let response = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32000, "message": "mock failure" }
            });
            self.write_frame(&serde_json::to_vec(&response)?).await
        }
    }

    /// Wires an in-memory duplex pair with the given capacity into a host
    /// connection + plugin side.
    fn host_and_plugin(cap: usize, config: ConnectionConfig) -> (PluginConnection, PluginSide) {
        let (host_tx, plugin_rx) = tokio::io::duplex(cap);
        let (plugin_tx, host_rx) = tokio::io::duplex(cap);
        let conn = PluginConnection::from_io(host_tx, host_rx, config);
        (
            conn,
            PluginSide {
                reader: FrameReader::new(plugin_rx),
                tx: plugin_tx,
            },
        )
    }

    #[tokio::test]
    async fn reader_demuxes_notification_before_pending(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (conn, mut plugin) = host_and_plugin(64 * 1024, ConnectionConfig::default());
        let (delivered_tx, delivered_rx) = std::sync::mpsc::channel();
        let _subscription = conn.register_notification_handler(
            "log",
            CorrelationFilter::default(),
            move |n: &Notification| {
                let _ = delivered_tx.send(n.clone());
            },
        );
        let plugin_task = tokio::spawn(async move {
            let request = plugin.read_request().await?;
            // Emit the notification first: the request is still pending when
            // the handler must fire.
            let notif = Notification::new(
                "log",
                serde_json::json!({"execution_id": "e1", "node_instance_id": "n1"}),
            );
            plugin.write_frame(&serde_json::to_vec(&notif)?).await?;
            plugin.respond(request.id, request.params).await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
        let result = conn.request("echo", serde_json::json!({"a": 1})).await?;
        assert_eq!(result, serde_json::json!({"a": 1}));
        plugin_task.await??;
        let delivered = delivered_rx
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "handler did not receive notification".to_string())?;
        assert_eq!(delivered.method, "log");
        let correlation = delivered
            .correlation()
            .ok_or_else(|| "missing correlation fields".to_string())?;
        assert_eq!(correlation.execution_id, "e1");
        assert_eq!(correlation.node_instance_id, "n1");
        Ok(())
    }

    #[tokio::test]
    async fn reader_skips_malformed_never_exits() -> Result<(), Box<dyn std::error::Error>> {
        let (conn, mut plugin) = host_and_plugin(64 * 1024, ConnectionConfig::default());
        let plugin_task = tokio::spawn(async move {
            // Garbage line first — the reader must skip it, not die.
            plugin.write_frame(b"this is not json at all").await?;
            let request = plugin.read_request().await?;
            plugin.respond(request.id, request.params).await?;
            let second = plugin.read_request().await?;
            plugin.respond(second.id, second.params).await?;
            // Stay alive so the host can observe the reader never died.
            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
        let first = conn.request("echo", serde_json::json!({"n": 1})).await?;
        assert_eq!(first, serde_json::json!({"n": 1}));
        // Reader is still alive after the malformed line.
        let second = conn.request("echo", serde_json::json!({"n": 2})).await?;
        assert_eq!(second, serde_json::json!({"n": 2}));
        assert_eq!(conn.exit_reason(), ExitReason::Running);
        assert_eq!(conn.metrics().malformed_lines, 1);
        plugin_task.abort();
        let _ = plugin_task.await;
        Ok(())
    }

    #[tokio::test]
    async fn reader_kills_after_consecutive_malformed(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = ConnectionConfig {
            max_malformed_lines: 3,
            ..ConnectionConfig::default()
        };
        let (conn, mut plugin) = host_and_plugin(64 * 1024, config);
        let plugin_task = tokio::spawn(async move {
            for _ in 0..3 {
                plugin.write_frame(b"garbage").await?;
            }
            // Stay alive; the host must kill us after 3 consecutive junk.
            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
        let result = conn.request("echo", serde_json::json!({"n": 1})).await;
        // The pending request is drained with a crash-style error.
        assert!(
            matches!(result, Err(PluginError::PluginCrashed(_))),
            "expected PluginCrashed, got {result:?}"
        );
        assert!(matches!(
            conn.exit_reason(),
            ExitReason::Crashed(PluginError::MalformedLine(_))
        ));
        plugin_task.abort();
        let _ = plugin_task.await;
        Ok(())
    }

    #[tokio::test]
    async fn reader_maps_error_response_to_typed_error(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (conn, mut plugin) = host_and_plugin(64 * 1024, ConnectionConfig::default());
        let plugin_task = tokio::spawn(async move {
            let request = plugin.read_request().await?;
            plugin.respond_error(request.id).await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
        let result = conn.request("boom", serde_json::json!({})).await;
        match result {
            Err(PluginError::JsonRpc { code, message, .. }) => {
                assert_eq!(code, -32000);
                assert_eq!(message, "mock failure");
            }
            other => panic!("expected JsonRpc error, got {other:?}"),
        }
        plugin_task.await??;
        Ok(())
    }

    #[tokio::test]
    async fn writer_writes_atomic_frames() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = ConnectionConfig {
            max_concurrent_requests: 64,
            queue_capacity: 128,
            ..ConnectionConfig::default()
        };
        let (conn, mut plugin) = host_and_plugin(64 * 1024, config);
        let plugin_task = tokio::spawn(async move {
            let mut received = Vec::new();
            for _ in 0..50 {
                let request = plugin.read_request().await?;
                received.push((request.id, request.params));
            }
            for (id, params) in received {
                plugin.respond(id, params).await?;
            }
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
        // True pipelining: fire all 50 requests as independent concurrent
        // tasks WITHOUT awaiting any of them, then collect the results.
        let mut handles = Vec::new();
        for task in 0..5u64 {
            for i in 0..10u64 {
                let seq = task * 10 + i;
                let conn = conn.clone();
                handles.push(tokio::spawn(async move {
                    conn.request("echo", serde_json::json!({"seq": seq})).await
                }));
            }
        }
        let mut completed = 0;
        for (seq, handle) in handles.into_iter().enumerate() {
            let result = handle.await?;
            // Every frame parsed as a complete request proves no
            // interleaving: torn frames would be malformed and dropped.
            assert_eq!(result?, serde_json::json!({"seq": seq as u64}));
            completed += 1;
        }
        assert_eq!(completed, 50);
        plugin_task.await??;
        Ok(())
    }

    #[tokio::test]
    async fn pending_timeout_removes_and_cancels(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = ConnectionConfig {
            request_timeout: Duration::from_millis(100),
            ..ConnectionConfig::default()
        };
        let (conn, mut plugin) = host_and_plugin(64 * 1024, config);
        let plugin_task = tokio::spawn(async move {
            let request = plugin.read_request().await?;
            // Never respond to the first request.
            let frame = plugin
                .read_frame()
                .await?
                .ok_or_else(|| "expected cancel notification".to_string())?;
            let cancel: Notification = serde_json::from_slice(&frame)?;
            assert_eq!(cancel.method, "cancel");
            assert_eq!(cancel.params["id"], serde_json::json!(request.id));
            let second = plugin.read_request().await?;
            plugin.respond(second.id, second.params).await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
        let timed_out = conn.request("hang", serde_json::json!({})).await;
        assert!(
            matches!(timed_out, Err(PluginError::Timeout(_))),
            "expected Timeout, got {timed_out:?}"
        );
        // The connection is still usable after the timeout: pending entry was
        // removed and the cancel notification fired.
        let second = conn
            .request("echo", serde_json::json!({"ok": true}))
            .await?;
        assert_eq!(second, serde_json::json!({"ok": true}));
        plugin_task.await??;
        assert_eq!(conn.metrics().timeouts, 1);
        Ok(())
    }

    #[tokio::test]
    async fn pending_drained_on_reader_exit() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    {
        let (conn, mut plugin) = host_and_plugin(64 * 1024, ConnectionConfig::default());
        let plugin_task = tokio::spawn(async move {
            let _request = plugin.read_request().await?;
            // Drop the stdout half without responding: reader sees EOF while
            // a request is pending → crash.
            drop(plugin.tx);
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
        let result = conn.request("echo", serde_json::json!({"n": 1})).await;
        assert!(
            matches!(result, Err(PluginError::PluginCrashed(_))),
            "expected PluginCrashed, got {result:?}"
        );
        plugin_task.await??;
        Ok(())
    }

    #[tokio::test]
    async fn clean_eof_vs_crash_detected() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Clean path: request completes, then graceful shutdown → CleanEof.
        let (mut conn, mut plugin) = host_and_plugin(64 * 1024, ConnectionConfig::default());
        let plugin_task = tokio::spawn(async move {
            let request = plugin.read_request().await?;
            plugin.respond(request.id, request.params).await?;
            // Wait for the host to half-close stdin, then close stdout.
            let eof = plugin.read_frame().await?;
            assert!(eof.is_none(), "expected stdin EOF after shutdown");
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
        let result = conn.request("echo", serde_json::json!({"x": 1})).await?;
        assert_eq!(result, serde_json::json!({"x": 1}));
        let reason = conn.shutdown(Duration::from_secs(2)).await?;
        assert_eq!(reason, ExitReason::CleanEof);
        plugin_task.await??;

        // Crash path: plugin dies with a request pending → Crashed.
        let (conn, mut plugin) = host_and_plugin(64 * 1024, ConnectionConfig::default());
        let plugin_task = tokio::spawn(async move {
            let _request = plugin.read_request().await?;
            drop(plugin.tx);
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
        let result = conn.request("echo", serde_json::json!({"y": 2})).await;
        assert!(matches!(result, Err(PluginError::PluginCrashed(_))));
        assert!(matches!(conn.exit_reason(), ExitReason::Crashed(_)));
        plugin_task.await??;
        Ok(())
    }

    #[tokio::test]
    async fn backpressure_awaits_when_queue_full(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = ConnectionConfig {
            max_concurrent_requests: 4,
            queue_capacity: 2,
            request_timeout: Duration::from_millis(300),
            ..ConnectionConfig::default()
        };
        let (conn, plugin) = host_and_plugin(64, config);
        // Keep the plugin side alive but never read: the host pipe fills, the
        // writer blocks, the queue fills, and further callers must await.
        let keepalive = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            drop(plugin);
        });
        let mut handles = Vec::new();
        for i in 0..6u64 {
            let conn = conn.clone();
            handles.push(tokio::spawn(async move {
                conn.request("echo", serde_json::json!({"i": i})).await
            }));
        }
        let mut timed_out = 0;
        for handle in handles {
            match handle.await? {
                // Never rejected: the caller awaited and only then timed out.
                Err(PluginError::Timeout(_)) => timed_out += 1,
                other => panic!("expected Timeout (never reject), got {other:?}"),
            }
        }
        assert_eq!(timed_out, 6);
        // At least one caller had to wait on the full writer queue.
        assert!(
            conn.metrics().slow_consumer >= 1,
            "expected slow-consumer counter >= 1, got {}",
            conn.metrics().slow_consumer
        );
        keepalive.abort();
        let _ = keepalive.await;
        Ok(())
    }
}
