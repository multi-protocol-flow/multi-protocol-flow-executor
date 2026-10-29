//! Tool extraction and JSON Schema generation from Flow files.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use flow_engine_core::flow::Flow;
use regex::Regex;
use serde_json::{json, Map, Value};

use super::protocol::ToolDefinition;

/// Registered tool with its source flow and path
#[derive(Debug, Clone)]
pub struct RegisteredFlowTool {
    pub tool_def: ToolDefinition,
    pub flow: Flow,
    pub source_path: PathBuf,
}

/// Registry of tools loaded from flow files or directories
#[derive(Debug, Clone, Default)]
pub struct ToolRegistry {
    pub tools: HashMap<String, RegisteredFlowTool>,
    pub flow_target: Option<PathBuf>,
    pub dir_target: Option<PathBuf>,
}

#[derive(serde::Deserialize)]
struct FlowEnvelope {
    flow: Flow,
}

/// Parse flow content supporting YAML (default for .mpf) and legacy JSON
pub fn parse_flow_content(content: &str) -> Result<Flow, String> {
    // 1. Try standard MPE FlowFile format via deserialize_flow_file (supports YAML, DSL, JSON)
    if let Ok(flow_file) = flow_engine_executor::storage::deserialize_flow_file(content) {
        return Ok(flow_file.flow);
    }

    // 2. Default to YAML first as current .mpf format defaults to YAML
    if let Ok(env) = serde_yml::from_str::<FlowEnvelope>(content) {
        return Ok(env.flow);
    }
    if let Ok(flow) = serde_yml::from_str::<Flow>(content) {
        return Ok(flow);
    }

    // 3. Fallback to JSON (legacy flows or direct flow exports)
    if let Ok(env) = serde_json::from_str::<FlowEnvelope>(content) {
        return Ok(env.flow);
    }
    if let Ok(flow) = serde_json::from_str::<Flow>(content) {
        return Ok(flow);
    }

    Err(format!(
        "Failed to deserialize flow file (tried YAML and JSON formats). Content preview: {}",
        content.chars().take(80).collect::<String>()
    ))
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load tools from either a single flow file or a directory
    pub fn load(flow_path: Option<&Path>, dir_path: Option<&Path>) -> Result<Self, String> {
        let mut registry = Self {
            tools: HashMap::new(),
            flow_target: flow_path.map(|p| p.to_path_buf()),
            dir_target: dir_path.map(|p| p.to_path_buf()),
        };

        if let Some(path) = flow_path {
            registry.load_file(path)?;
        }

        if let Some(dir) = dir_path {
            registry.load_directory(dir)?;
        }

        Ok(registry)
    }

    /// Re-read flows from disk and return true if any tool definition or tool set changed.
    ///
    /// Beyond the configured `flow_target` / `dir_target`, every flow registered
    /// at runtime (shared gateway `POST /register`) is re-read from its source
    /// path so a reload never drops dynamically attached flows.
    pub fn reload(&mut self) -> Result<bool, String> {
        let mut new_registry = Self {
            tools: HashMap::new(),
            flow_target: self.flow_target.clone(),
            dir_target: self.dir_target.clone(),
        };

        if let Some(ref path) = self.flow_target {
            new_registry.load_file(path)?;
        }
        if let Some(ref dir) = self.dir_target {
            new_registry.load_directory(dir)?;
        }

        let dynamic_sources: Vec<PathBuf> = self
            .tools
            .values()
            .filter(|tool| Some(&tool.source_path) != self.flow_target.as_ref())
            .map(|tool| tool.source_path.clone())
            .collect();

        for path in dynamic_sources {
            if let Err(e) = new_registry.load_file(&path) {
                eprintln!(
                    "[mpe-mcp-server] Reload skipped {}: {e}",
                    path.display()
                );
            }
        }

        let changed = self.has_changed(&new_registry);
        self.tools = new_registry.tools;
        Ok(changed)
    }

    fn has_changed(&self, other: &Self) -> bool {
        if self.tools.len() != other.tools.len() {
            return true;
        }
        for (k, v) in &self.tools {
            if let Some(other_tool) = other.tools.get(k) {
                if v.tool_def.name != other_tool.tool_def.name
                    || v.tool_def.description != other_tool.tool_def.description
                    || v.tool_def.input_schema != other_tool.tool_def.input_schema
                {
                    return true;
                }
            } else {
                return true;
            }
        }
        false
    }

    /// Load a single flow file, returning the derived tool name.
    ///
    /// Tool names are unique per registry: a second flow deriving the same name
    /// gets a deterministic `-<flow-uuid>` suffix instead of silently replacing
    /// the first one, so one shared gateway can host many flows whose tools are
    /// all discoverable through a single `tools/list`.
    pub fn load_file(&mut self, path: &Path) -> Result<String, String> {
        if !path.exists() {
            return Err(format!("Flow file does not exist: {}", path.display()));
        }
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read flow file {}: {e}", path.display()))?;

        let flow = parse_flow_content(&content)
            .map_err(|e| format!("Failed to parse flow from {}: {e}", path.display()))?;

        let base_name = derive_tool_name(&flow, path);
        let tool_name = self.unique_tool_name(&base_name, path, &flow.uuid);
        let tool_def = build_tool_definition(&tool_name, &flow);

        self.tools.insert(
            tool_name.clone(),
            RegisteredFlowTool {
                tool_def,
                flow,
                source_path: path.to_path_buf(),
            },
        );
        Ok(tool_name)
    }

    /// Resolve a tool name that does not collide with another flow.
    fn unique_tool_name(&self, base: &str, source: &Path, flow_uuid: &str) -> String {
        // Re-registering the same file must keep its existing name.
        if let Some(existing) = self.tools.get(base) {
            if existing.source_path == source {
                return base.to_string();
            }
        } else {
            return base.to_string();
        }

        let suffix: String = flow_uuid
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .take(8)
            .collect::<String>()
            .to_lowercase();
        let suffix = if suffix.is_empty() {
            "dup".to_string()
        } else {
            suffix
        };

        let mut candidate = truncate_tool_name(&format!("{base}-{suffix}"));
        let mut counter = 2;
        while let Some(existing) = self.tools.get(&candidate) {
            if existing.source_path == source {
                return candidate;
            }
            candidate = truncate_tool_name(&format!("{base}-{suffix}-{counter}"));
            counter += 1;
        }
        candidate
    }

    /// Remove the tool backed by `source`, returning whether it existed.
    pub fn remove_by_source(&mut self, source: &Path) -> bool {
        let target = self
            .tools
            .iter()
            .find(|(_, tool)| tool.source_path == source)
            .map(|(name, _)| name.clone());
        match target {
            Some(name) => self.tools.remove(&name).is_some(),
            None => false,
        }
    }

    /// Recursively load flows from a directory
    pub fn load_directory(&mut self, dir: &Path) -> Result<(), String> {
        if !dir.is_dir() {
            return Err(format!("Directory does not exist: {}", dir.display()));
        }

        let entries = fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory {}: {e}", dir.display()))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if ext.eq_ignore_ascii_case("json") || ext.eq_ignore_ascii_case("mpf") {
                        if let Err(e) = self.load_file(&path) {
                            eprintln!("[mpe-mcp-server] Skip file {}: {e}", path.display());
                        }
                    }
                }
            } else if path.is_dir() {
                let _ = self.load_directory(&path);
            }
        }

        Ok(())
    }

    pub fn list_tools(&self) -> Vec<ToolDefinition> {
        let mut list: Vec<ToolDefinition> = self.tools.values().map(|t| t.tool_def.clone()).collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// List tools restricted to a route slug (`/{slug}/mcp` shared-gateway scope).
    pub fn list_tools_in_scope(&self, scope: Option<&str>) -> Vec<ToolDefinition> {
        let tools = self.list_tools();
        match scope {
            None => tools,
            Some(slug) => tools
                .into_iter()
                .filter(|t| slug_for_tool(&t.name) == slug)
                .collect(),
        }
    }

    pub fn get_tool(&self, name: &str) -> Option<&RegisteredFlowTool> {
        self.tools.get(name)
    }

    /// Remove a tool by name, returning whether it existed.
    pub fn remove_tool(&mut self, name: &str) -> bool {
        self.tools.remove(name).is_some()
    }
}

/// Route slug for a tool: MCP tool names are already URL-safe (`[a-z0-9_-]`),
/// so the slug is the tool name normalised through `slugify`.
pub fn slug_for_tool(tool_name: &str) -> String {
    let slug = flow_engine_core::dsl::slugify(tool_name);
    if slug.is_empty() {
        "mcp".to_string()
    } else {
        slug
    }
}

/// Clamp a tool name to the MCP limit of 64 characters.
fn truncate_tool_name(name: &str) -> String {
    if name.len() <= 64 {
        name.to_string()
    } else {
        name.chars().take(64).collect()
    }
}

/// Derive a sanitary tool name conforming to MCP specification `^[a-zA-Z0-9_-]{1,64}$`
pub fn sanitize_tool_name(raw: &str) -> String {
    let mut s: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Trim trailing/leading underscores
    s = s.trim_matches('_').to_lowercase();
    if s.is_empty() {
        s = "mpe_flow".to_string();
    }
    if s.len() > 64 {
        s.truncate(64);
    }
    s
}

fn derive_tool_name(flow: &Flow, path: &Path) -> String {
    if !flow.name.trim().is_empty() {
        sanitize_tool_name(&flow.name)
    } else if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        sanitize_tool_name(stem)
    } else {
        "flow_tool".to_string()
    }
}

/// Scan a flow and generate its `ToolDefinition` with JSON Schema
pub fn build_tool_definition(tool_name: &str, flow: &Flow) -> ToolDefinition {
    let description = flow
        .description
        .as_deref()
        .unwrap_or("")
        .trim();
    let description = if description.is_empty() {
        format!("Execute flow '{}'", flow.name)
    } else {
        description.to_string()
    };

    let input_schema = derive_input_schema(flow);

    ToolDefinition {
        name: tool_name.to_string(),
        description,
        input_schema,
    }
}

/// Derive JSON Schema from {{var}} usages and `flow.initial_variables`
pub fn derive_input_schema(flow: &Flow) -> Value {
    let scanned_vars = scan_variables_in_flow(flow);
    let initial_vars = flow.initial_variables.clone().unwrap_or_default();

    let mut properties = Map::new();
    let mut required = Vec::new();

    // BTreeSet for deterministic ordering
    let mut all_vars: BTreeMap<String, Option<Value>> = BTreeMap::new();

    for var in scanned_vars {
        all_vars.insert(var, None);
    }

    for (k, v) in initial_vars {
        if !is_system_variable(&k) {
            all_vars.insert(k, Some(v));
        }
    }

    for (var_name, default_val) in all_vars {
        let mut prop = Map::new();
        let (val_type, has_default) = match &default_val {
            Some(Value::Number(_)) => ("number", true),
            Some(Value::Bool(_)) => ("boolean", true),
            Some(Value::Array(_)) => ("array", true),
            Some(Value::Object(_)) => ("object", true),
            Some(Value::String(_)) => ("string", true),
            Some(Value::Null) => ("string", false),
            None => ("string", false),
        };

        prop.insert("type".to_string(), json!(val_type));
        prop.insert(
            "description".to_string(),
            json!(format!("Flow parameter '{var_name}'")),
        );

        if let Some(val) = default_val {
            if !val.is_null() {
                prop.insert("default".to_string(), val);
            }
        }

        properties.insert(var_name.clone(), Value::Object(prop));

        // If no default value was provided, consider it required
        if !has_default {
            required.push(json!(var_name));
        }
    }

    json!({
        "type": "object",
        "properties": properties,
        "required": required
    })
}

/// Check if a variable identifier is a system-internal variable
pub fn is_system_variable(name: &str) -> bool {
    let n = name.trim();
    n.starts_with('$')
        || n.starts_with('_')
        || n.starts_with("env.")
        || n.starts_with("report.")
        || n.starts_with("prev.")
        || n.starts_with("context.")
        || n.starts_with("response.")
        || n.starts_with("res.")
        || n.starts_with("input.")
        || n.starts_with("flow.")
        || n.starts_with("node.")
}

/// Extract root variable name from an expression like `user.profile.name` -> `user`
fn extract_root_var(var: &str) -> Option<String> {
    let trimmed = var.trim();
    if is_system_variable(trimmed) {
        return None;
    }
    let root = trimmed.split('.').next()?.trim();
    let root = root.split('[').next()?.trim();
    if root.is_empty() || is_system_variable(root) {
        None
    } else {
        Some(root.to_string())
    }
}

/// Regex scan of {{var}} across all nodes and subgraphs
pub fn scan_variables_in_flow(flow: &Flow) -> HashSet<String> {
    let mut vars = HashSet::new();
    let re = match Regex::new(r"\{\{\s*([a-zA-Z0-9_$.\[\]]+)\s*\}\}") {
        Ok(r) => r,
        Err(_) => return vars,
    };

    let mut collect_from_nodes = |nodes: &[flow_engine_core::node::Node]| {
        for node in nodes {
            if let Ok(json_str) = serde_json::to_string(&node.data) {
                for cap in re.captures_iter(&json_str) {
                    if let Some(matched) = cap.get(1) {
                        if let Some(root) = extract_root_var(matched.as_str()) {
                            vars.insert(root);
                        }
                    }
                }
            }
        }
    };

    collect_from_nodes(&flow.nodes);

    if let Some(ref setup) = flow.setup {
        collect_from_nodes(&setup.nodes);
    }
    if let Some(ref teardown) = flow.teardown {
        collect_from_nodes(&teardown.nodes);
    }

    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_flow(dir: &Path, file: &str, uuid: &str, name: &str) -> PathBuf {
        let path = dir.join(file);
        let yaml = format!(
            r#"_version: 1
flow:
  uuid: {uuid}
  name: {name}
  nodes:
    - uuid: n1
      type: "http:request"
      name: HTTP Call
      data:
        type: "http:request"
        url: "https://api.example.com?q={{{{query}}}}"
  connections: []
"#
        );
        fs::write(&path, yaml).unwrap();
        path
    }

    #[test]
    fn same_name_flows_get_distinct_tools_in_one_registry() {
        let dir = tempfile::tempdir().unwrap();
        let first = write_flow(dir.path(), "a.mpf", "uuid-aaaaaaaa-1", "Shared Name");
        let second = write_flow(dir.path(), "b.mpf", "uuid-bbbbbbbb-2", "Shared Name");

        let mut registry = ToolRegistry::new();
        let first_tool = registry.load_file(&first).unwrap();
        let second_tool = registry.load_file(&second).unwrap();

        assert_eq!(first_tool, "shared_name");
        assert_ne!(second_tool, first_tool);
        assert!(second_tool.starts_with("shared_name-"));

        // One `tools/list` on the shared gateway discovers both flows.
        let names: Vec<String> = registry
            .list_tools_in_scope(None)
            .into_iter()
            .map(|t| t.name)
            .collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&first_tool));
        assert!(names.contains(&second_tool));
    }

    #[test]
    fn reloading_same_file_keeps_its_tool_name() {
        let dir = tempfile::tempdir().unwrap();
        let first = write_flow(dir.path(), "a.mpf", "uuid-aaaaaaaa-1", "Shared Name");
        let second = write_flow(dir.path(), "b.mpf", "uuid-bbbbbbbb-2", "Shared Name");

        let mut registry = ToolRegistry::new();
        let _ = registry.load_file(&first).unwrap();
        let second_tool = registry.load_file(&second).unwrap();
        let again = registry.load_file(&second).unwrap();

        assert_eq!(again, second_tool, "re-registering a file must not rename it");
        assert_eq!(registry.tools.len(), 2);
    }

    #[test]
    fn remove_by_source_detaches_only_that_flow() {
        let dir = tempfile::tempdir().unwrap();
        let first = write_flow(dir.path(), "a.mpf", "uuid-aaaaaaaa-1", "Flow A");
        let second = write_flow(dir.path(), "b.mpf", "uuid-bbbbbbbb-2", "Flow B");

        let mut registry = ToolRegistry::new();
        let _ = registry.load_file(&first).unwrap();
        let _ = registry.load_file(&second).unwrap();

        assert!(registry.remove_by_source(&first));
        assert_eq!(registry.tools.len(), 1);
        assert!(registry.get_tool("flow_b").is_some());
        assert!(!registry.remove_by_source(&first));
    }
}
