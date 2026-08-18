//! Plugin process lifecycle: on-demand start, idle recycle, restart policy.
//!
//! Each registered plugin is wrapped in a [`LifecycleEntry`] that owns the
//! process's connection and applies the residency policy:
//!
//! - **On-demand** entries spawn lazily on the first
//!   [`LifecycleEntry::connection`] call and are idle-recycled after
//!   [`LifecycleConfig::idle_recycle`] with no in-flight work.
//! - **Resident** entries (`capabilities.streaming`) are never recycled; a
//!   reaper is not even started for them.
//! - A dead connection is restarted at most once per
//!   [`LifecycleConfig::restart_min_interval`]; further attempts inside the
//!   window fail fast with a rate-limit [`PluginError::ProtocolError`].
//! - Epoch tokens identify each connection generation: callers can pin a
//!   generation via [`LifecycleEntry::connection_stamped`] and reject stale
//!   connections after a restart.
//!
//! The connector ([`LifecycleConfig::connector`]) is the only place that
//! knows how to build a connection; production uses the real spawn path and
//! tests inject in-memory duplex fakes through it.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mpe_plugin_sdk::protocol::manifest::PluginManifest;

use crate::error::PluginError;
use crate::process;
use crate::transport::{ConnectionConfig, ExitReason, PluginConnection};

/// Residency policy for a plugin process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidencyMode {
    /// Spawn lazily and idle-recycle the process after `idle_recycle`.
    OnDemand,
    /// Keep the process alive until shutdown (streaming plugins).
    Resident,
}

/// Connection factory shared by lifecycle entries.
///
/// Produces a [`PluginConnection`] for a plugin manifest. The default
/// configuration spawns the plugin process ([`process::spawn_plugin`] +
/// [`PluginConnection::spawn`]); tests replace it with in-memory duplex
/// fakes.
pub type Connector =
    Arc<dyn Fn(&PluginManifest) -> Result<PluginConnection, PluginError> + Send + Sync>;

/// Lifecycle policy parameters shared by all entries of a manager.
#[derive(Clone)]
pub struct LifecycleConfig {
    /// How long an idle on-demand plugin stays alive before recycle.
    pub idle_recycle: Duration,
    /// Minimum gap between two restarts of the same plugin.
    pub restart_min_interval: Duration,
    /// Grace period handed to a connection during shutdown/recycle.
    pub shutdown_grace: Duration,
    /// The host's current UI language (BCP-47, e.g. `en-US`), injected as
    /// `MPE_LOCALE` into plugin processes at spawn when it matches the
    /// plugin's declared `locales` (falling back to the plugin's
    /// `default_locale`). `None` = no injection (plugins keep their own
    /// hard-coded default). Fixed for a plugin's process lifetime.
    pub host_locale: Option<String>,
    /// How connections are created (real spawn path or test fake).
    pub connector: Connector,
}

impl LifecycleConfig {
    /// Sets the host UI language used for `MPE_LOCALE` injection at spawn.
    pub fn with_host_locale(mut self, locale: Option<String>) -> Self {
        self.host_locale = locale;
        self.connector = build_connector(self.host_locale.clone());
        self
    }
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            idle_recycle: Duration::from_secs(60),
            restart_min_interval: Duration::from_secs(1),
            shutdown_grace: Duration::from_secs(5),
            host_locale: None,
            connector: build_connector(None),
        }
    }
}

/// Builds the production connector, injecting the host UI language as
/// `MPE_LOCALE` into the plugin process environment at spawn.
fn build_connector(host_locale: Option<String>) -> Connector {
    Arc::new(move |manifest: &PluginManifest| {
        let mut env = manifest.env.clone();
        inject_locale(&mut env, manifest, host_locale.as_deref());
        let process = process::spawn_plugin(&manifest.entry, &env)?;
        PluginConnection::spawn(process, ConnectionConfig::default())
    })
}

/// Merges the plugin's effective UI language into `env` as `MPE_LOCALE`.
///
/// Language resolution (fixed at spawn, for the process lifetime):
/// - when the plugin declares `locales`, the host's current language is used
///   if listed, otherwise the plugin's `default_locale`;
/// - a plugin declaring neither is spawned without `MPE_LOCALE` (it keeps
///   its own hard-coded default).
fn inject_locale(
    env: &mut HashMap<String, String>,
    manifest: &PluginManifest,
    host_locale: Option<&str>,
) {
    let effective = if manifest.locales.is_empty() {
        // 未声明支持语言 → 不参与匹配，只可能用 default_locale（老插件无此字段则不注入）。
        manifest.default_locale.clone()
    } else {
        host_locale
            .filter(|l| manifest.locales.iter().any(|declared| declared == l))
            .map(str::to_string)
            .or_else(|| manifest.default_locale.clone())
    };
    if let Some(locale) = effective {
        env.insert(mpe_plugin_sdk::protocol::LOCALE_ENV.to_string(), locale);
    }
}

/// Snapshot of a lifecycle entry's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleMetrics {
    /// Current connection generation (0 = never started).
    pub epoch: u64,
    /// Connection handles currently leased to callers.
    pub in_flight: usize,
    /// Number of idle recycles performed.
    pub recycle_count: u64,
    /// Number of crash restarts performed.
    pub restart_count: u64,
    /// Whether a connection is currently stored (may still be dead).
    pub connected: bool,
    /// The entry's residency mode.
    pub mode: ResidencyMode,
}

/// Per-plugin lifecycle state, shared between the manager, callers, and the
/// idle reaper.
pub struct LifecycleEntry {
    /// Plugin name (unique per manager).
    pub name: String,
    manifest: Mutex<PluginManifest>,
    mode: ResidencyMode,
    /// 0 = never started; incremented on every spawn (start or restart).
    epoch: AtomicU64,
    connection: tokio::sync::RwLock<Option<Arc<PluginConnection>>>,
    /// Leased connection handles (Arc clones beyond the entry's own hold).
    inflight: AtomicUsize,
    recycle_count: AtomicU64,
    restart_count: AtomicU64,
    last_restart: Mutex<Option<Instant>>,
    #[allow(dead_code)] // informational; refreshed by `note_activity`
    last_activity: Mutex<Option<Instant>>,
    /// Ping channel for the idle reaper; every `note_activity` resets the
    /// idle window.
    activity_tx: tokio::sync::watch::Sender<()>,
    shutting_down: AtomicBool,
    idle_recycle: Duration,
    restart_min_interval: Duration,
    shutdown_grace: Duration,
    connector: Connector,
}

impl LifecycleEntry {
    /// Creates a lifecycle entry with the given residency mode and timing
    /// policy. The connection is spawned lazily on first use, not here.
    pub fn new(
        name: String,
        manifest: PluginManifest,
        mode: ResidencyMode,
        idle_recycle: Duration,
        restart_min_interval: Duration,
        shutdown_grace: Duration,
        connector: Connector,
    ) -> Arc<Self> {
        let (activity_tx, _) = tokio::sync::watch::channel(());
        Arc::new(Self {
            name,
            manifest: Mutex::new(manifest),
            mode,
            epoch: AtomicU64::new(0),
            connection: tokio::sync::RwLock::new(None),
            inflight: AtomicUsize::new(0),
            recycle_count: AtomicU64::new(0),
            restart_count: AtomicU64::new(0),
            last_restart: Mutex::new(None),
            last_activity: Mutex::new(None),
            activity_tx,
            shutting_down: AtomicBool::new(false),
            idle_recycle,
            restart_min_interval,
            shutdown_grace,
            connector,
        })
    }

    /// The current connection generation (0 = never started).
    pub fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::Relaxed)
    }

    /// The entry's residency mode.
    pub fn mode(&self) -> ResidencyMode {
        self.mode
    }

    /// Snapshot of the entry's counters and connection state.
    pub fn metrics(&self) -> LifecycleMetrics {
        let (connected, in_flight) = match self.connection.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(conn) => {
                    self.refresh_inflight(conn);
                    (true, self.inflight.load(Ordering::Relaxed))
                }
                None => (false, 0),
            },
            Err(_) => (false, self.inflight.load(Ordering::Relaxed)),
        };
        LifecycleMetrics {
            epoch: self.epoch(),
            in_flight,
            recycle_count: self.recycle_count.load(Ordering::Relaxed),
            restart_count: self.restart_count.load(Ordering::Relaxed),
            connected,
            mode: self.mode,
        }
    }

    /// Acquires a live connection, starting or restarting the plugin process
    /// as needed.
    ///
    /// Concurrent callers are serialized on the connection lock, so only one
    /// spawn happens. A dead connection is restarted at most once per
    /// `restart_min_interval`; a second restart attempt inside the window
    /// returns a rate-limit error instead of spawning again.
    pub async fn connection(&self) -> Result<Arc<PluginConnection>, PluginError> {
        let mut guard = self.connection.write().await;
        if let Some(conn) = guard.as_ref() {
            if matches!(conn.exit_reason(), ExitReason::Running) {
                self.note_activity();
                self.refresh_inflight(conn);
                return Ok(Arc::clone(conn));
            }
        }
        if self.shutting_down.load(Ordering::SeqCst) {
            return Err(PluginError::Shutdown);
        }
        // Restart policy: at most one restart per `restart_min_interval`.
        if self.epoch.load(Ordering::Relaxed) > 0 {
            let now = Instant::now();
            let last = *self.last_restart.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(last) = last {
                if now.duration_since(last) < self.restart_min_interval {
                    return Err(PluginError::ProtocolError(
                        "restart rate-limited".to_string(),
                    ));
                }
            }
        }
        let manifest = self
            .manifest
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let conn = Arc::new((self.connector)(&manifest)?);
        // A non-zero previous epoch means this is a crash restart, not the
        // first start.
        let is_restart = self.epoch.fetch_add(1, Ordering::Relaxed) > 0;
        if is_restart {
            self.restart_count.fetch_add(1, Ordering::Relaxed);
            *self.last_restart.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
        }
        *guard = Some(Arc::clone(&conn));
        self.note_activity();
        self.refresh_inflight(&conn);
        Ok(conn)
    }

    /// Acquires a connection only if it still belongs to generation `epoch`.
    ///
    /// A caller that captured `entry.epoch()` before dispatching a long
    /// operation can use this to reject a connection that was restarted while
    /// the operation was in flight — the old generation is gone.
    pub async fn connection_stamped(
        &self,
        epoch: u64,
    ) -> Result<Arc<PluginConnection>, PluginError> {
        if epoch != self.epoch() {
            return Err(PluginError::ProtocolError("stale epoch".to_string()));
        }
        self.connection().await
    }

    /// Peeks at the current live connection **without spawning** the plugin
    /// process.
    ///
    /// Unlike [`LifecycleEntry::connection`], this never starts an on-demand
    /// plugin — it only returns a connection that is already running. Used by
    /// fire-and-forget broadcasts (e.g. `flowEnded`) that must not pull idle
    /// plugins out of hibernation.
    pub async fn try_connection(&self) -> Option<Arc<PluginConnection>> {
        let guard = self.connection.read().await;
        match guard.as_ref() {
            Some(conn) if matches!(conn.exit_reason(), ExitReason::Running) => {
                Some(Arc::clone(conn))
            }
            _ => None,
        }
    }

    /// Shuts down the current connection (if any) within `grace`.
    ///
    /// Marks the entry as shutting down so later [`LifecycleEntry::connection`]
    /// calls fail fast with [`PluginError::Shutdown`]. If callers still hold
    /// connection handles the kill is deferred until the last handle drops
    /// ([`PluginConnection`]'s own Drop reaps the child).
    pub async fn shutdown(&self, grace: Duration) -> ExitReason {
        self.shutting_down.store(true, Ordering::SeqCst);
        let conn = self.connection.write().await.take();
        let Some(mut conn) = conn else {
            return ExitReason::CleanEof;
        };
        let Some(conn) = Arc::get_mut(&mut conn) else {
            log::debug!(
                "connection {} still leased; kill deferred to last drop",
                self.name
            );
            return conn.exit_reason();
        };
        conn.shutdown(grace)
            .await
            .unwrap_or_else(|_| conn.exit_reason())
    }

    /// Starts the idle reaper for this entry.
    ///
    /// A no-op for [`ResidencyMode::Resident`] entries — streaming plugins are
    /// deliberately never recycled. The reaper wakes after `idle_recycle`,
    /// and if the entry is on-demand with no leased connection handles it
    /// takes the connection, shuts it down with `shutdown_grace`, and exits.
    pub fn spawn_reaper(self: &Arc<Self>) {
        if self.mode != ResidencyMode::OnDemand {
            return;
        }
        let entry = Arc::clone(self);
        tokio::spawn(async move {
            entry.reap_loop().await;
        });
    }

    /// Records activity: updates the last-activity timestamp and pings the
    /// reaper so the idle window restarts.
    fn note_activity(&self) {
        *self.last_activity.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
        let _ = self.activity_tx.send(());
    }

    /// Refreshes the lease counter from the connection's strong count: every
    /// [`Arc`] beyond the entry's own hold is a handle leased to a caller.
    fn refresh_inflight(&self, conn: &Arc<PluginConnection>) {
        self.inflight
            .store(Arc::strong_count(conn).saturating_sub(1), Ordering::Relaxed);
    }

    async fn reap_loop(self: Arc<Self>) {
        let mut activity = self.activity_tx.subscribe();
        loop {
            tokio::select! {
                _ = activity.changed() => {
                    // Activity reset the idle window; keep waiting.
                }
                _ = tokio::time::sleep(self.idle_recycle) => {
                    if self.try_recycle().await {
                        return;
                    }
                }
            }
        }
    }

    /// Recycles the idle connection, returning `true` when the reaper should
    /// exit (it recycled or has nothing left to manage).
    async fn try_recycle(&self) -> bool {
        if self.shutting_down.load(Ordering::SeqCst) {
            return true;
        }
        let recyclable = {
            let guard = self.connection.read().await;
            match guard.as_ref() {
                Some(conn) => {
                    self.refresh_inflight(conn);
                    self.inflight.load(Ordering::Relaxed) == 0
                }
                None => false, // never spawned yet; keep waiting for a spawn
            }
        };
        if !recyclable {
            return false;
        }
        let conn = self.connection.write().await.take();
        let Some(mut conn) = conn else {
            return true;
        };
        if let Some(conn) = Arc::get_mut(&mut conn) {
            let _ = conn.shutdown(self.shutdown_grace).await;
        } else {
            // A caller leased the connection between the check and the take;
            // put it back and wait for the next idle window.
            *self.connection.write().await = Some(conn);
            return false;
        }
        self.recycle_count.fetch_add(1, Ordering::Relaxed);
        true
    }
}

