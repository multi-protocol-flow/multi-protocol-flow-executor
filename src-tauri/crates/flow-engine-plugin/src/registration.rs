//! Registration of describe-produced nodes into the host registry (M2c).
//!
//! Collision policy (OD2, locked): built-in ids are NEVER shadowed by plugins
//! (rejected with a warning); plugin-vs-plugin collisions are first-wins +
//! warn. The check lives in `PluginManager` — NOT in
//! [`flow_engine_core::registry::NodeRegistry::register`], which silently
//! overwrites.
//!
//! [`PluginManager::register_nodes_with`] applies the collision policy and
//! registers nodes using a caller-provided factory (e.g. `SidecarPlugin::new`).

use std::collections::HashSet;
use std::sync::Arc;

use flow_engine_core::plugin::NodePlugin;
use flow_engine_core::registry::NodeRegistry;
use mpe_plugin_sdk::protocol::describe::NodeDescription;
use crate::error::PluginError;
use crate::manager::{PluginManager, PluginRegistration};
use crate::reserved::is_reserved_type_id;
use crate::transport::PluginConnection;

/// Why a plugin node was rejected by the collision policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionReason {
    /// The `type_id` is a reserved built-in (never shadowed).
    Reserved,
    /// Another plugin already registered the `type_id` (first wins).
    PluginCollision,
}

/// One rejected plugin node — the test-observable collision log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionReport {
    /// Plugin whose node was rejected.
    pub plugin: String,
    /// The node type id that collided.
    pub type_id: String,
    /// Which policy rule rejected it.
    pub reason: CollisionReason,
}

/// Decides whether a plugin node `type_id` may be registered, applying the
/// locked OD2 collision policy. Shared by
/// [`PluginManager::register_nodes_with`] and
/// `flow-engine-executor::default_registry_with_plugins` so the two paths can
/// never drift.
///
/// `Ok(())` registers; `Err(Reserved)` rejects (a reserved built-in OR an
/// unclaimed-but-present entry — a built-in the reserved list drifted from);
/// `Err(PluginCollision)` rejects (first wins).
///
/// Order of checks: (1) reserved built-in → `Err(Reserved)`; (2) present in
/// the registry but not claimed by a plugin → `Err(Reserved)` — a built-in
/// must never be overwritten; (3) already claimed by a plugin →
/// `Err(PluginCollision)` (first wins); (4) otherwise `Ok(())`.
pub fn decide_registration(
    type_id: &str,
    registry_has_entry: bool,
    claimed: &HashSet<String>,
) -> Result<(), CollisionReason> {
    if is_reserved_type_id(type_id) || (registry_has_entry && !claimed.contains(type_id)) {
        Err(CollisionReason::Reserved)
    } else if claimed.contains(type_id) {
        Err(CollisionReason::PluginCollision)
    } else {
        Ok(())
    }
}


impl PluginManager {
    /// Appends a describe-produced registration (from any source) to the
    /// manager; its nodes are inserted by [`PluginManager::register_nodes`] /
    /// [`PluginManager::register_nodes_with`].
    ///
    /// `scan_and_describe` calls this internally; tests and M3 use it to inject
    /// registrations without spawning a plugin process. Indexes every node's
    /// `type_id` → plugin name so [`PluginManager::connection_for_node`] can
    /// resolve a node to its lifecycle entry.
    pub fn add_registration(&mut self, registration: PluginRegistration) {
        for node in &registration.nodes {
            self.type_id_to_plugin
                .insert(node.type_id.clone(), registration.name.clone());
        }
        self.registrations.push(registration);
    }

    /// Acquires a live connection for the plugin that owns a node `type_id`.
    ///
    /// Maps `type_id` → plugin name via the index populated by
    /// [`PluginManager::add_registration`], then delegates to
    /// [`PluginManager::connection_for`] (whose restart-once policy covers
    /// dead connections). Unknown type ids are a protocol error.
    pub async fn connection_for_node(
        &self,
        type_id: &str,
    ) -> Result<Arc<PluginConnection>, PluginError> {
        let name = self.type_id_to_plugin.get(type_id).ok_or_else(|| {
            PluginError::ProtocolError(format!("unknown plugin node type {type_id}"))
        })?;
        self.connection_for(name).await
    }

    /// Registrations retained from successful describes, in scan order.
    pub fn registrations(&self) -> &[PluginRegistration] {
        &self.registrations
    }

    /// Every node the collision policy rejected, in rejection order
    /// (test-observable; cleared only by constructing a new manager).
    pub fn collision_reports(&self) -> &[CollisionReport] {
        &self.collisions
    }

    /// Applies the collision policy to every registered plugin node and
    /// inserts accepted nodes into `registry` via `factory`.
    ///
    /// Reserved built-ins are never shadowed; plugin-vs-plugin collisions are
    /// first-wins + warn. Rejections are logged and recorded in
    /// [`PluginManager::collision_reports`]. Ids claimed by a successful
    /// registration stay claimed for this manager's lifetime, so a later call
    /// cannot shadow an earlier plugin either.
    pub fn register_nodes_with(
        &mut self,
        registry: &mut NodeRegistry,
        factory: impl Fn(&NodeDescription) -> Arc<dyn NodePlugin>,
    ) {
        let registrations = std::mem::take(&mut self.registrations);
        for registration in &registrations {
            for node in &registration.nodes {
                match decide_registration(
                    &node.type_id,
                    registry.get(&node.type_id).is_some(),
                    &self.claimed_node_ids,
                ) {
                    Ok(()) => {
                        registry.register(factory(node));
                        self.claimed_node_ids.insert(node.type_id.clone());
                    }
                    Err(reason) => {
                        log::warn!(
                            "plugin {} node {} rejected: {:?} (collision policy)",
                            registration.name,
                            node.type_id,
                            reason
                        );
                        self.collisions.push(CollisionReport {
                            plugin: registration.name.clone(),
                            type_id: node.type_id.clone(),
                            reason,
                        });
                    }
                }
            }
        }
        self.registrations = registrations;
    }

}
