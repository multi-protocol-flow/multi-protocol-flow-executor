//! Tests for the registration collision policy (M2c).
//!
//! Lives in a separate file (declared from `registration.rs`) to keep both
//! files under the workspace 250-pure-LOC ceiling. All four plan-named tests
//! (`registration_builtin_never_shadowed`, `registration_plugin_collision_
//! first_wins_warn`, `registration_shadowing_regression_test`,
//! `registration_success_registers_nodes`) plus the M3 seam test.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use flow_engine_core::plugin::{NodePlugin, PortDef};
use flow_engine_core::registry::NodeRegistry;
use mpe_protocol::describe::NodeDescription;
use mpe_protocol::manifest::{PluginCapabilities, PluginEntry, PluginManifest};

use crate::manager::{PluginManager, PluginRegistration};
use crate::registration::{decide_registration, CollisionReason, CollisionReport};

/// Test-double `NodePlugin`; the factory under test wraps every accepted
/// `NodeDescription` in one of these so tests can identify WHO won a slot.
struct TestPlugin {
    type_id: String,
    display_name: String,
}

#[async_trait]
impl NodePlugin for TestPlugin {
    fn type_id(&self) -> &str {
        &self.type_id
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn category(&self) -> &str {
        "test"
    }

    fn icon(&self) -> &str {
        "test"
    }

    fn color(&self) -> &str {
        "#000000"
    }

    fn ports(&self) -> Vec<PortDef> {
        Vec::new()
    }

    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({})
    }

    async fn execute(
        &self,
        _node: &flow_engine_core::node::Node,
        _context: &mut flow_engine_core::context::ExecutionContext,
        _connections: &[flow_engine_core::connection::Connection],
        _node_map: &HashMap<String, &flow_engine_core::node::Node>,
    ) -> Result<(Vec<String>, Option<serde_json::Value>), flow_engine_core::error::ErrorDetail>
    {
        Ok((Vec::new(), None))
    }
}

/// Factory M3 will replace with `SidecarPlugin::new(desc, manager)`.
fn test_factory(node: &NodeDescription) -> Arc<dyn NodePlugin> {
    Arc::new(TestPlugin {
        type_id: node.type_id.clone(),
        display_name: "TEST-DOUBLE".to_string(),
    })
}

/// Display name of the plugin registered under `type_id`, if any.
fn display_name_of(registry: &NodeRegistry, type_id: &str) -> Option<String> {
    registry.get(type_id).map(|p| p.display_name().to_string())
}

/// A minimal `NodeDescription` for one node type.
fn node_desc(type_id: &str) -> NodeDescription {
    NodeDescription {
        type_id: type_id.to_string(),
        display_name: format!("Node {type_id}"),
        category: None,
        icon: None,
        color: None,
        ports: Vec::new(),
        default_config: serde_json::json!({}),
        config_schema: None,
        frontend: None,
        capabilities: PluginCapabilities::default(),
    }
}

/// A minimal manifest (`PluginManifest` has no `Default`).
fn manifest(name: &str) -> PluginManifest {
    PluginManifest {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        author: None,
        min_host_version: None,
        description: None,
        entry: PluginEntry {
            command: "test".to_string(),
            args: Vec::new(),
        },
        env: HashMap::new(),
        permissions: Vec::new(),
        capabilities: PluginCapabilities::default(),
    }
}

/// A fake describe-produced registration declaring `type_ids`.
fn registration(name: &str, type_ids: &[&str]) -> PluginRegistration {
    PluginRegistration {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        dir: PathBuf::from("/nonexistent"),
        manifest: manifest(name),
        nodes: type_ids.iter().map(|t| node_desc(t)).collect(),
    }
}

/// OD2-1: a plugin node colliding with a reserved built-in `type_id` is
/// rejected with `Reserved`, and the built-in registry entry is untouched.
#[test]
fn registration_builtin_never_shadowed() {
    let mut manager = PluginManager::new(PathBuf::from("/nonexistent"));
    manager.add_registration(registration("evil", &["http"]));

    let mut registry = NodeRegistry::new();
    registry.register(Arc::new(TestPlugin {
        type_id: "http".to_string(),
        display_name: "BUILTIN".to_string(),
    }));

    manager.register_nodes_with(&mut registry, test_factory);

    assert_eq!(
        manager.collision_reports(),
        &[CollisionReport {
            plugin: "evil".to_string(),
            type_id: "http".to_string(),
            reason: CollisionReason::Reserved,
        }]
    );
    // The plugin's entry never made it in — the built-in still wins.
    assert_eq!(
        display_name_of(&registry, "http"),
        Some("BUILTIN".to_string())
    );
    assert_eq!(registry.len(), 1);
}

/// OD2-2: two plugins declaring the same non-built-in `type_id` → first wins,
/// the second is rejected with `PluginCollision` (its other nodes still
/// register).
#[test]
fn registration_plugin_collision_first_wins_warn() {
    let mut manager = PluginManager::new(PathBuf::from("/nonexistent"));
    manager.add_registration(registration("alpha", &["acme:ping"]));
    manager.add_registration(registration("beta", &["acme:ping", "acme:pong"]));

    let mut registry = NodeRegistry::new();
    manager.register_nodes_with(&mut registry, test_factory);

    assert_eq!(
        display_name_of(&registry, "acme:ping"),
        Some("TEST-DOUBLE".to_string())
    );
    assert_eq!(
        manager.collision_reports(),
        &[CollisionReport {
            plugin: "beta".to_string(),
            type_id: "acme:ping".to_string(),
            reason: CollisionReason::PluginCollision,
        }]
    );
    // beta's non-colliding node still registered.
    assert_eq!(
        display_name_of(&registry, "acme:pong"),
        Some("TEST-DOUBLE".to_string())
    );
    assert_eq!(registry.len(), 2);
}

/// Shadowing regression: a fake built-in-colliding plugin registered FIRST
/// (into an empty registry) must still be rejected — the reserved list, not
/// registration order, protects built-ins. The built-in registered afterwards
/// wins.
#[test]
fn registration_shadowing_regression_test() {
    let mut manager = PluginManager::new(PathBuf::from("/nonexistent"));
    manager.add_registration(registration("evil", &["http"]));

    let mut registry = NodeRegistry::new();
    manager.register_nodes_with(&mut registry, test_factory);

    // No plugin entry exists for "http" even though the plugin was first.
    assert!(registry.get("http").is_none());
    assert_eq!(
        manager.collision_reports(),
        &[CollisionReport {
            plugin: "evil".to_string(),
            type_id: "http".to_string(),
            reason: CollisionReason::Reserved,
        }]
    );

    // The built-in registered afterwards still wins.
    registry.register(Arc::new(TestPlugin {
        type_id: "http".to_string(),
        display_name: "BUILTIN".to_string(),
    }));
    assert_eq!(
        display_name_of(&registry, "http"),
        Some("BUILTIN".to_string())
    );
}

/// Happy path: valid plugin nodes register through the factory, and the
/// collision log stays empty.
#[test]
fn registration_success_registers_nodes() {
    let mut manager = PluginManager::new(PathBuf::from("/nonexistent"));
    manager.add_registration(registration("acme", &["acme:ping", "acme:pong"]));

    let mut registry = NodeRegistry::new();
    manager.register_nodes_with(&mut registry, test_factory);

    assert_eq!(
        display_name_of(&registry, "acme:ping"),
        Some("TEST-DOUBLE".to_string())
    );
    assert_eq!(
        display_name_of(&registry, "acme:pong"),
        Some("TEST-DOUBLE".to_string())
    );
    assert_eq!(registry.len(), 2);
    assert!(manager.collision_reports().is_empty());
}

/// M3 seam: `register_nodes` (no factory) registers via the `PendingPlugin`
/// placeholder so the plan's call signature works today; M3 swaps in the
/// `SidecarPlugin` factory. Describe metadata survives the placeholder.
#[test]
fn register_nodes_placeholder_marks_pending_until_m3() {
    let mut manager = PluginManager::new(PathBuf::from("/nonexistent"));
    manager.add_registration(registration("acme", &["acme:ping"]));

    let mut registry = NodeRegistry::new();
    manager.register_nodes(&mut registry);

    assert_eq!(
        display_name_of(&registry, "acme:ping"),
        Some("Node acme:ping".to_string())
    );
    assert!(manager.collision_reports().is_empty());
}

/// Direct unit coverage of the shared `decide_registration` decision,
/// independent of `register_nodes_with` plumbing — pins the 4 policy cases.
#[test]
fn decide_registration_rejects_reserved_builtin() {
    let claimed = HashSet::new();
    assert_eq!(
        decide_registration("http", false, &claimed),
        Err(CollisionReason::Reserved)
    );
}

/// Drift-guard: an unclaimed-but-present entry (a built-in the reserved list
/// missed) is treated as reserved and never overwritten.
#[test]
fn decide_registration_rejects_unclaimed_present_entry() {
    let claimed = HashSet::new();
    assert_eq!(
        decide_registration("legacy_node", true, &claimed),
        Err(CollisionReason::Reserved)
    );
}

/// Plugin-vs-plugin: an already-claimed `type_id` is rejected (first wins).
#[test]
fn decide_registration_rejects_plugin_collision() {
    let mut claimed = HashSet::new();
    claimed.insert("acme:ping".to_string());
    assert_eq!(
        decide_registration("acme:ping", false, &claimed),
        Err(CollisionReason::PluginCollision)
    );
}

/// Fresh, unreserved, unclaimed `type_id` registers.
#[test]
fn decide_registration_registers_fresh_type_id() {
    let claimed = HashSet::new();
    assert_eq!(decide_registration("acme:pong", false, &claimed), Ok(()));
}
