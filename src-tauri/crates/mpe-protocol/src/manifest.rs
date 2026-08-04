//! Plugin manifest (`plugin.json`) schema, shared by host discovery and SDK.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A plugin's static manifest, loaded from `plugin.json` in its directory.
///
/// All optional fields default to their "absent" value so minimal manifests
/// (e.g. name + version + entry) parse without ceremony.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin name (the install-directory name by convention).
    pub name: String,
    /// Plugin version (semver).
    pub version: String,
    /// Optional author name.
    #[serde(default)]
    pub author: Option<String>,
    /// Minimum host version required to run this plugin (semver; `None` =
    /// no gating).
    #[serde(default)]
    pub min_host_version: Option<String>,
    /// One-line plugin description.
    #[serde(default)]
    pub description: Option<String>,
    /// Command line used to spawn the plugin process.
    pub entry: PluginEntry,
    /// Extra environment variables injected into the plugin process.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Permission strings the plugin requests from the host.
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Runtime capabilities declared by the plugin.
    #[serde(default)]
    pub capabilities: PluginCapabilities,
}

/// The process-spawn entry point for a plugin.
///
/// Split into `command` + `args` so interpreter-based plugins are expressible
/// (e.g. `{"command": "python", "args": ["plugin.py"]}`) — not just a single
/// executable path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginEntry {
    /// Executable or interpreter command (e.g. `python`).
    pub command: String,
    /// Arguments passed to the command (e.g. `["plugin.py"]`).
    #[serde(default)]
    pub args: Vec<String>,
}

/// Declared plugin capabilities.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PluginCapabilities {
    /// Whether the plugin supports streaming responses (`true` makes the
    /// process resident instead of idle-recycled).
    #[serde(default)]
    pub streaming: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let manifest = PluginManifest {
            name: "demo".to_string(),
            version: "1.2.3".to_string(),
            author: Some("Acme".to_string()),
            min_host_version: Some("0.1.1".to_string()),
            description: Some("A demo plugin".to_string()),
            entry: PluginEntry {
                command: "python".to_string(),
                args: vec!["plugin.py".to_string()],
            },
            env: HashMap::from([("FOO".to_string(), "bar".to_string())]),
            permissions: vec!["http.request".to_string()],
            capabilities: PluginCapabilities { streaming: true },
        };
        let json = serde_json::to_string(&manifest)?;
        let back: PluginManifest = serde_json::from_str(&json)?;
        assert_eq!(back, manifest);
        Ok(())
    }

    #[test]
    fn manifest_parses_command_args_sample() {
        let json = r#"{
            "name": "demo",
            "version": "0.1.0",
            "entry": {"command": "python", "args": ["plugin.py"]}
        }"#;
        let manifest: PluginManifest = serde_json::from_str(json)
            .unwrap_or_else(|e| panic!("minimal manifest should parse: {e}"));
        assert_eq!(manifest.entry.command, "python");
        assert_eq!(manifest.entry.args, vec!["plugin.py"]);
        // Optional fields default to absent/empty.
        assert_eq!(manifest.author, None);
        assert_eq!(manifest.min_host_version, None);
        assert!(manifest.env.is_empty());
        assert!(manifest.permissions.is_empty());
        assert_eq!(manifest.capabilities, PluginCapabilities::default());
    }

    #[test]
    fn manifest_rejects_missing_entry() {
        let json = r#"{"name": "demo", "version": "0.1.0"}"#;
        assert!(serde_json::from_str::<PluginManifest>(json).is_err());
    }

    #[test]
    fn entry_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let entry = PluginEntry {
            command: "node".to_string(),
            args: vec!["plugin.js".to_string(), "--flag".to_string()],
        };
        let json = serde_json::to_string(&entry)?;
        let back: PluginEntry = serde_json::from_str(&json)?;
        assert_eq!(back, entry);
        Ok(())
    }
}
