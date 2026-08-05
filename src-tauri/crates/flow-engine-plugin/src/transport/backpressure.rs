//! Backpressure primitives for the pipelined transport.
//!
//! Two bounds protect the transport from a slow or stalled plugin:
//! - a **bounded inflight semaphore** at `max_concurrent_requests`, and
//! - a **bounded writer queue** (the mpsc channel capacity).
//!
//! When either is exhausted the caller **awaits** — requests are never
//! rejected. A slow-consumer counter is exposed so stress metrics can detect
//! head-of-line blocking: one slow consumer stalls every flow sharing the
//! plugin process. That pipe-full head-of-line behavior is documented and
//! accepted, not redesigned (see `docs/plugin-protocol.md` §7).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::error::PluginError;

/// Bounded in-flight concurrency with a slow-consumer counter.
#[derive(Debug)]
pub struct Backpressure {
    /// Permits bounded to `max_concurrent_requests`; each in-flight request
    /// holds one permit for its whole lifetime (until response or timeout).
    inflight: Arc<Semaphore>,
    /// Number of times a caller had to await because the writer queue was
    /// full (the plugin's stdin pipe is backed up).
    slow_consumer: AtomicU64,
}

impl Backpressure {
    /// Creates backpressure allowing at most `max_concurrent` in-flight
    /// requests (the writer-queue bound lives in the mpsc channel).
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            inflight: Arc::new(Semaphore::new(max_concurrent)),
            slow_consumer: AtomicU64::new(0),
        }
    }

    /// Acquires an in-flight permit, awaiting if the budget is exhausted.
    ///
    /// The returned permit is held for the whole request lifetime and
    /// released when the request completes or times out.
    pub async fn acquire(&self) -> Result<OwnedSemaphorePermit, PluginError> {
        self.inflight
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| PluginError::Shutdown)
    }

    /// Records that a caller had to wait on the full writer queue.
    pub fn note_slow_consumer(&self) {
        self.slow_consumer.fetch_add(1, Ordering::Relaxed);
    }

    /// Total slow-consumer waits observed since the connection was created.
    pub fn slow_consumer_count(&self) -> u64 {
        self.slow_consumer.load(Ordering::Relaxed)
    }

    /// Currently free in-flight slots (diagnostics).
    pub fn available(&self) -> usize {
        self.inflight.available_permits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn backpressure_never_rejects_and_awaits() -> Result<(), Box<dyn std::error::Error>> {
        let bp = Backpressure::new(2);
        let p1 = bp.acquire().await?;
        let p2 = bp.acquire().await?;
        assert_eq!(bp.available(), 0);
        // A third acquire must await (never error), then succeed once a
        // permit is released.
        let bp2 = Arc::new(bp);
        let waiter_bp = Arc::clone(&bp2);
        let handle = tokio::spawn(async move { waiter_bp.acquire().await });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(
            !handle.is_finished(),
            "third acquire must await, not resolve"
        );
        drop(p1);
        drop(p2);
        let permit = handle.await??;
        drop(permit);
        Ok(())
    }

    #[tokio::test]
    async fn backpressure_slow_consumer_counter() -> Result<(), Box<dyn std::error::Error>> {
        let bp = Backpressure::new(1);
        assert_eq!(bp.slow_consumer_count(), 0);
        bp.note_slow_consumer();
        bp.note_slow_consumer();
        assert_eq!(bp.slow_consumer_count(), 2);
        Ok(())
    }
}
