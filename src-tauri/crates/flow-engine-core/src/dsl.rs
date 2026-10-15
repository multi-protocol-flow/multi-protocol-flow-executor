//! 声明式 Flow YAML DSL 模块
//!
//! 提供面向人类可读、无坐标绑定的声明式流程模型（`DeclarativeFlow` / `DeclarativeNode`），
//! 以及与运行时 `Flow` 模型之间的双向拓扑无损转换与自动排版（Auto-Layout）算法。

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::connection::Connection;
use crate::flow::{Flow, FlowSubGraph};
use crate::node::{ExecutionErrorStrategy, Node, NodePosition};
use crate::node_config::NodeData;

/// 流程命名空间 UUID，用于 `UUIDv5` 稳定散列生成
pub const FLOW_NAMESPACE: Uuid = Uuid::from_bytes([
    0x3f, 0x82, 0xa1, 0x5b, 0x4a, 0x9c, 0x4d, 0x2e,
    0x8f, 0x1a, 0x7c, 0x3d, 0x5e, 0x2b, 0x9a, 0x0f,
]);

fn default_dsl_version() -> u32 {
    1
}

/// 声明式流程模型（面向 YAML/DSL 存储与 CI 编排）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeclarativeFlow {
    /// 声明式 DSL 版本号，默认为 1
    #[serde(default = "default_dsl_version")]
    pub version: u32,
    /// 流程名称
    pub name: String,
    /// 流程 slug（用于路由前缀与稳定标识），可选
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// 流程描述（Markdown）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 运行环境名称（如 dev, staging, prod）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    /// 流程级初始变量
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, serde_json::Value>>,
    /// Setup 准备阶段节点列表
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub setup: Vec<DeclarativeNode>,
    /// Main 核心阶段节点列表
    pub main: Vec<DeclarativeNode>,
    /// Teardown 收尾阶段节点列表
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub teardown: Vec<DeclarativeNode>,
}

/// 声明式节点模型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeclarativeNode {
    /// 节点语义化 ID
    pub id: String,
    /// 节点类型（如 `entry`, `http:request`, `conditional` 等）
    #[serde(rename = "type")]
    pub node_type: String,
    /// 节点显示名称（可选，若未指定则回退为 id）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 扁平化的节点专有配置（自动展开为平级键值对）
    #[serde(flatten)]
    pub config: serde_json::Value,
    /// 单线后继节点 ID（普通输出端口）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    /// 条件满足/断言成功路由分支（true 端口）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_true: Option<String>,
    /// 条件不满足/失败路由分支（false 端口）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_false: Option<String>,
    /// 多前驱合流依赖节点列表
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// 节点执行失败策略
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_error: Option<ExecutionErrorStrategy>,
}

/// 字符串转语义化 Slug
pub fn slugify(s: &str) -> String {
    let mut result = String::new();
    let mut prev_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !result.is_empty() {
            result.push('-');
            prev_dash = true;
        }
    }
    if result.ends_with('-') {
        result.pop();
    }
    result
}

fn is_clean_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn generate_node_id(node: &Node, used_ids: &mut HashSet<String>) -> String {
    let candidate = if !node.name.trim().is_empty() {
        slugify(&node.name)
    } else {
        String::new()
    };

    let base_id = if !candidate.is_empty() && !used_ids.contains(&candidate) {
        candidate
    } else if !node.uuid.is_empty() && is_clean_id(&node.uuid) && !used_ids.contains(&node.uuid) {
        node.uuid.clone()
    } else {
        let short: String = if !node.uuid.is_empty() {
            node.uuid.chars().take(8).collect()
        } else {
            "node".to_string()
        };
        let sanitized = slugify(&short);
        if sanitized.is_empty() {
            "node".to_string()
        } else {
            sanitized
        }
    };

    let mut final_id = base_id.clone();
    let mut counter = 2;
    while used_ids.contains(&final_id) {
        final_id = format!("{}-{}", base_id, counter);
        counter += 1;
    }
    used_ids.insert(final_id.clone());
    final_id
}

/// 将单个阶段的 nodes 与 connections 折叠为 `Vec<DeclarativeNode>`
fn stage_to_declarative(nodes: &[Node], connections: &[Connection]) -> Vec<DeclarativeNode> {
    if nodes.is_empty() {
        return Vec::new();
    }

    let mut used_ids = HashSet::new();
    let mut uuid_to_id: HashMap<String, String> = HashMap::new();
    let mut id_to_uuid: HashMap<String, String> = HashMap::new();

    for node in nodes {
        let id = generate_node_id(node, &mut used_ids);
        uuid_to_id.insert(node.uuid.clone(), id.clone());
        id_to_uuid.insert(id, node.uuid.clone());
    }

    let node_uuids: HashSet<&str> = nodes.iter().map(|n| n.uuid.as_str()).collect();

    // 识别条件/断言分支连接，避免其作为普通连接混入 depends_on
    let is_branch_conn = |conn: &Connection| -> bool {
        nodes
            .iter()
            .find(|n| n.uuid == conn.source_node_uuid)
            .map(|n| {
                (n.node_type == "conditional" || n.node_type == "assertion")
                    && (conn.source_port_id == "true" || conn.source_port_id == "false")
            })
            .unwrap_or(false)
    };

    // 统计各节点普通入边情况（仅考虑本阶段内的非分支连接）
    let mut normal_incoming_sources: HashMap<String, Vec<String>> = HashMap::new();
    // 统计各节点普通出边数量
    let mut normal_outgoing_counts: HashMap<String, usize> = HashMap::new();

    for conn in connections {
        if node_uuids.contains(conn.source_node_uuid.as_str())
            && node_uuids.contains(conn.target_node_uuid.as_str())
            && conn.source_port_id != "false"
            && !is_branch_conn(conn)
        {
            if let Some(src_id) = uuid_to_id.get(&conn.source_node_uuid) {
                normal_incoming_sources
                    .entry(conn.target_node_uuid.clone())
                    .or_default()
                    .push(src_id.clone());
                *normal_outgoing_counts
                    .entry(conn.source_node_uuid.clone())
                    .or_insert(0) += 1;
            }
        }
    }

    // 统计各节点出边情况
    let mut outgoing_conns: HashMap<String, Vec<&Connection>> = HashMap::new();
    for conn in connections {
        if node_uuids.contains(conn.source_node_uuid.as_str())
            && node_uuids.contains(conn.target_node_uuid.as_str())
        {
            outgoing_conns
                .entry(conn.source_node_uuid.clone())
                .or_default()
                .push(conn);
        }
    }

    let mut dnodes: Vec<DeclarativeNode> = Vec::with_capacity(nodes.len());

    for node in nodes {
        let node_id = match uuid_to_id.get(&node.uuid) {
            Some(id) => id.clone(),
            None => continue,
        };

        let mut next: Option<String> = None;
        let mut on_true: Option<String> = None;
        let mut on_false: Option<String> = None;
        let mut depends_on: Vec<String> = Vec::new();

        let outs = outgoing_conns.get(&node.uuid).cloned().unwrap_or_default();

        for conn in &outs {
            let target_id = match uuid_to_id.get(&conn.target_node_uuid) {
                Some(id) => id.clone(),
                None => continue,
            };

            if conn.source_port_id == "false" {
                on_false = Some(target_id);
            } else if (node.node_type == "conditional" || node.node_type == "assertion")
                && conn.source_port_id == "true"
            {
                on_true = Some(target_id);
            } else {
                let target_incoming_count = normal_incoming_sources
                    .get(&conn.target_node_uuid)
                    .map(|v| v.len())
                    .unwrap_or(0);
                let source_outgoing_count = normal_outgoing_counts
                    .get(&node.uuid)
                    .copied()
                    .unwrap_or(0);

                // 仅当源节点单出且目标节点单入时使用 next；
                // 一对多（扇出）或多对一（扇入）由目标节点通过 depends_on 记录，保证连接不丢失
                if source_outgoing_count == 1 && target_incoming_count == 1 && next.is_none() {
                    next = Some(target_id);
                }
            }
        }

        // 若当前节点由多前驱合流或由多分支后继扇出到达，记录其 depends_on
        if let Some(srcs) = normal_incoming_sources.get(&node.uuid) {
            let mut deps = Vec::new();
            for src_id in srcs {
                let src_uuid = &id_to_uuid[src_id];
                let src_out_count = normal_outgoing_counts.get(src_uuid).copied().unwrap_or(0);
                if srcs.len() > 1 || src_out_count > 1 {
                    deps.push(src_id.clone());
                }
            }
            if !deps.is_empty() {
                deps.sort();
                deps.dedup();
                depends_on = deps;
            }
        }

        // 提取 config 配置（序列化 node.data 并移除 type 字段）
        let mut config_value = serde_json::to_value(&node.data).unwrap_or(serde_json::Value::Null);
        if let serde_json::Value::Object(map) = &mut config_value {
            map.remove("type");
        } else {
            config_value = serde_json::Value::Object(serde_json::Map::new());
        }

        let name_field = if !node.name.trim().is_empty() && node.name != node_id {
            Some(node.name.clone())
        } else {
            None
        };

        dnodes.push(DeclarativeNode {
            id: node_id,
            node_type: node.node_type.clone(),
            name: name_field,
            config: config_value,
            next,
            on_true,
            on_false,
            depends_on,
            on_error: node.on_error.clone(),
        });
    }

    // 按拓扑执行序确定性排序
    sort_nodes_topologically(&mut dnodes);
    dnodes
}

/// 对声明式节点按拓扑执行序与 ID 稳定排序
fn sort_nodes_topologically(nodes: &mut [DeclarativeNode]) {
    let mut in_degrees: HashMap<String, usize> = HashMap::new();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();

    for n in nodes.iter() {
        in_degrees.entry(n.id.clone()).or_insert(0);
        adj.entry(n.id.clone()).or_default();
    }

    for n in nodes.iter() {
        if let Some(nxt) = &n.next {
            adj.entry(n.id.clone()).or_default().push(nxt.clone());
            *in_degrees.entry(nxt.clone()).or_insert(0) += 1;
        }
        if let Some(ot) = &n.on_true {
            adj.entry(n.id.clone()).or_default().push(ot.clone());
            *in_degrees.entry(ot.clone()).or_insert(0) += 1;
        }
        if let Some(of) = &n.on_false {
            adj.entry(n.id.clone()).or_default().push(of.clone());
            *in_degrees.entry(of.clone()).or_insert(0) += 1;
        }
        for dep in &n.depends_on {
            adj.entry(dep.clone()).or_default().push(n.id.clone());
            *in_degrees.entry(n.id.clone()).or_insert(0) += 1;
        }
    }

    let mut ready: VecDeque<String> = nodes
        .iter()
        .filter(|n| in_degrees.get(&n.id).copied().unwrap_or(0) == 0)
        .map(|n| n.id.clone())
        .collect();

    let mut sorted_ids: Vec<String> = Vec::with_capacity(nodes.len());
    let mut visited = HashSet::new();

    while let Some(id) = ready.pop_front() {
        if !visited.insert(id.clone()) {
            continue;
        }
        sorted_ids.push(id.clone());

        if let Some(neighbors) = adj.get(&id) {
            for neighbor in neighbors {
                if let Some(deg) = in_degrees.get_mut(neighbor) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 && !visited.contains(neighbor) {
                        ready.push_back(neighbor.clone());
                    }
                }
            }
        }
    }

    // 若有环或未访问节点，追加在其后
    for n in nodes.iter() {
        if !visited.contains(&n.id) {
            sorted_ids.push(n.id.clone());
        }
    }

    let order_map: HashMap<&str, usize> = sorted_ids
        .iter()
        .enumerate()
        .map(|(idx, id)| (id.as_str(), idx))
        .collect();

    nodes.sort_by_key(|n| order_map.get(n.id.as_str()).copied().unwrap_or(usize::MAX));
}

impl Flow {
    /// 将运行时 `Flow` 转换为声明式 `DeclarativeFlow` 模型
    pub fn to_declarative(&self) -> DeclarativeFlow {
        let slug = {
            let s = slugify(&self.name);
            if !s.is_empty() {
                Some(s)
            } else if !self.uuid.trim().is_empty() {
                Some(self.uuid.clone())
            } else {
                None
            }
        };

        let setup = self
            .setup
            .as_ref()
            .map(|s| stage_to_declarative(&s.nodes, &s.connections))
            .unwrap_or_default();

        let main = stage_to_declarative(&self.nodes, &self.connections);

        let teardown = self
            .teardown
            .as_ref()
            .map(|t| stage_to_declarative(&t.nodes, &t.connections))
            .unwrap_or_default();

        DeclarativeFlow {
            version: 1,
            name: self.name.clone(),
            slug,
            description: self.description.clone(),
            env: None,
            variables: self.initial_variables.clone(),
            setup,
            main,
            teardown,
        }
    }

    /// 导出为规范化的 YAML 格式字符串
    pub fn to_declarative_yaml(&self) -> Result<String, String> {
        let decl = self.to_declarative();
        decl.to_yaml()
    }
}

impl DeclarativeFlow {
    /// 从 YAML 字符串反序列化为 `DeclarativeFlow`
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        serde_yml::from_str(yaml).map_err(|e| format!("failed to parse declarative YAML: {}", e))
    }

    /// 序列化为规范化的 YAML 字符串
    pub fn to_yaml(&self) -> Result<String, String> {
        serde_yml::to_string(self).map_err(|e| format!("failed to serialize declarative flow: {}", e))
    }

    /// 将声明式模型反序列化还原为带稳定 UUID 与自动排版（Auto-Layout）坐标的 `Flow`
    pub fn to_flow(&self) -> Result<Flow, String> {
        let flow_base = self
            .slug
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                let s = slugify(&self.name);
                if s.is_empty() {
                    "flow".to_string()
                } else {
                    s
                }
            });

        let flow_uuid = Uuid::new_v5(&FLOW_NAMESPACE, flow_base.as_bytes()).to_string();

        let setup = if !self.setup.is_empty() {
            let (nodes, conns) = stage_to_flow(&self.setup, &flow_uuid, "setup")?;
            Some(FlowSubGraph::new(nodes, conns))
        } else {
            None
        };

        let (main_nodes, main_conns) = stage_to_flow(&self.main, &flow_uuid, "main")?;

        let teardown = if !self.teardown.is_empty() {
            let (nodes, conns) = stage_to_flow(&self.teardown, &flow_uuid, "teardown")?;
            Some(FlowSubGraph::new(nodes, conns))
        } else {
            None
        };

        Ok(Flow {
            uuid: flow_uuid,
            name: self.name.clone(),
            description: self.description.clone(),
            initial_variables: self.variables.clone(),
            setup,
            nodes: main_nodes,
            connections: main_conns,
            teardown,
        })
    }
}

/// 将声明式阶段节点数组转换为运行时 `(Vec<Node>, Vec<Connection>)`，并赋予拓扑层级坐标
fn stage_to_flow(
    dnodes: &[DeclarativeNode],
    flow_uuid: &str,
    stage: &str,
) -> Result<(Vec<Node>, Vec<Connection>), String> {
    if dnodes.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    // 1. 生成稳定的 UUIDv5
    let mut id_to_uuid: HashMap<String, String> = HashMap::with_capacity(dnodes.len());
    for dnode in dnodes {
        if id_to_uuid.contains_key(&dnode.id) {
            return Err(format!(
                "Duplicate node id '{}' in stage '{}'",
                dnode.id, stage
            ));
        }
        let key = format!("{}:{}:{}", flow_uuid, stage, dnode.id);
        let node_uuid = Uuid::new_v5(&FLOW_NAMESPACE, key.as_bytes()).to_string();
        id_to_uuid.insert(dnode.id.clone(), node_uuid);
    }

    // 2. 拓扑层级计算（Sugiyama Auto-Layout）
    // 按层级深度分配 x = depth * 280.0，同层分配 y = index * 120.0
    let mut in_degrees: HashMap<String, usize> = HashMap::new();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();

    for n in dnodes {
        in_degrees.entry(n.id.clone()).or_insert(0);
        adj.entry(n.id.clone()).or_default();
    }

    for n in dnodes {
        if let Some(nxt) = &n.next {
            adj.entry(n.id.clone()).or_default().push(nxt.clone());
            *in_degrees.entry(nxt.clone()).or_insert(0) += 1;
        }
        if let Some(ot) = &n.on_true {
            adj.entry(n.id.clone()).or_default().push(ot.clone());
            *in_degrees.entry(ot.clone()).or_insert(0) += 1;
        }
        if let Some(of) = &n.on_false {
            adj.entry(n.id.clone()).or_default().push(of.clone());
            *in_degrees.entry(of.clone()).or_insert(0) += 1;
        }
        for dep in &n.depends_on {
            adj.entry(dep.clone()).or_default().push(n.id.clone());
            *in_degrees.entry(n.id.clone()).or_insert(0) += 1;
        }
    }

    let mut depths: HashMap<String, usize> = HashMap::new();
    let roots: Vec<String> = dnodes
        .iter()
        .filter(|n| in_degrees.get(&n.id).copied().unwrap_or(0) == 0)
        .map(|n| n.id.clone())
        .collect();

    let mut queue: VecDeque<String> = VecDeque::new();
    if roots.is_empty() {
        if let Some(first) = dnodes.first() {
            depths.insert(first.id.clone(), 0);
            queue.push_back(first.id.clone());
        }
    } else {
        for root in roots {
            depths.insert(root.clone(), 0);
            queue.push_back(root);
        }
    }

    // 拓扑最长路径分配 depth
    let mut iterations = 0;
    let max_iterations = dnodes.len() * dnodes.len();
    while let Some(u) = queue.pop_front() {
        iterations += 1;
        if iterations > max_iterations {
            break; // 环图保护
        }
        let cur_d = depths.get(&u).copied().unwrap_or(0);
        if let Some(neighbors) = adj.get(&u) {
            for v in neighbors {
                let next_d = cur_d + 1;
                let old_d = depths.entry(v.clone()).or_insert(0);
                if next_d > *old_d {
                    *old_d = next_d;
                    queue.push_back(v.clone());
                }
            }
        }
    }

    // 对未触及的离散节点补齐 depth
    for n in dnodes {
        depths.entry(n.id.clone()).or_insert(0);
    }

    // 按 depth 分层并计算坐标
    let mut layers: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for dnode in dnodes {
        let d = depths.get(&dnode.id).copied().unwrap_or(0);
        layers.entry(d).or_default().push(dnode.id.clone());
    }

    let mut positions: HashMap<String, NodePosition> = HashMap::new();
    for (depth, mut ids) in layers {
        ids.sort();
        for (index, id) in ids.iter().enumerate() {
            positions.insert(
                id.clone(),
                NodePosition {
                    x: (depth as f64) * 280.0,
                    y: (index as f64) * 120.0,
                },
            );
        }
    }

    // 3. 构建运行时 Node 列表
    let mut nodes: Vec<Node> = Vec::with_capacity(dnodes.len());
    for dnode in dnodes {
        let uuid = id_to_uuid.get(&dnode.id).cloned().unwrap_or_default();
        let name = dnode
            .name
            .clone()
            .unwrap_or_else(|| dnode.id.clone());
        let pos = positions
            .get(&dnode.id)
            .cloned()
            .unwrap_or_else(NodePosition::default);

        // 还原 NodeData
        let mut map = match &dnode.config {
            serde_json::Value::Object(m) => m.clone(),
            _ => serde_json::Map::new(),
        };
        map.insert(
            "type".to_string(),
            serde_json::Value::String(dnode.node_type.clone()),
        );

        let data: NodeData = serde_json::from_value(serde_json::Value::Object(map))
            .unwrap_or_else(|_| {
                NodeData::Plugin(serde_json::json!({
                    "type": &dnode.node_type
                }))
            });

        nodes.push(Node {
            uuid,
            node_type: dnode.node_type.clone(),
            name,
            position: pos,
            data,
            ports: None,
            has_breakpoint: None,
            on_error: dnode.on_error.clone(),
        });
    }

    // 4. 构建连接列表
    let mut connections: Vec<Connection> = Vec::new();
    let mut seen_connections: HashSet<(String, String, String, String)> = HashSet::new();

    let add_connection = |source_id: &str,
                              target_id: &str,
                              source_port: &str,
                              target_port: &str,
                              conns: &mut Vec<Connection>,
                              seen: &mut HashSet<(String, String, String, String)>| {
        let src_uuid = match id_to_uuid.get(source_id) {
            Some(u) => u.clone(),
            None => return,
        };
        let tgt_uuid = match id_to_uuid.get(target_id) {
            Some(u) => u.clone(),
            None => return,
        };
        let key = (
            src_uuid.clone(),
            tgt_uuid.clone(),
            source_port.to_string(),
            target_port.to_string(),
        );
        if !seen.insert(key) {
            return;
        }

        let conn_key = format!(
            "{}:{}:{}:{}:{}",
            flow_uuid, src_uuid, source_port, tgt_uuid, target_port
        );
        let conn_id = Uuid::new_v5(&FLOW_NAMESPACE, conn_key.as_bytes()).to_string();

        conns.push(Connection {
            id: conn_id,
            source_node_uuid: src_uuid,
            target_node_uuid: tgt_uuid,
            source_port_id: source_port.to_string(),
            target_port_id: target_port.to_string(),
        });
    };

    for dnode in dnodes {
        if let Some(nxt) = &dnode.next {
            add_connection(
                &dnode.id,
                nxt,
                "out",
                "in",
                &mut connections,
                &mut seen_connections,
            );
        }
        if let Some(ot) = &dnode.on_true {
            add_connection(
                &dnode.id,
                ot,
                "true",
                "in",
                &mut connections,
                &mut seen_connections,
            );
        }
        if let Some(of) = &dnode.on_false {
            add_connection(
                &dnode.id,
                of,
                "false",
                "in",
                &mut connections,
                &mut seen_connections,
            );
        }
        for dep in &dnode.depends_on {
            add_connection(
                dep,
                &dnode.id,
                "out",
                "in",
                &mut connections,
                &mut seen_connections,
            );
        }
    }

    Ok((nodes, connections))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("HTTP GET Example"), "http-get-example");
        assert_eq!(slugify("Check Status Code"), "check-status-code");
        assert_eq!(slugify("Entry"), "entry");
        assert_eq!(slugify("Order Pay ($100)"), "order-pay-100");
        assert_eq!(slugify("already-slug-123"), "already-slug-123");
    }

    #[test]
    fn test_declarative_yaml_roundtrip() {
        let yaml_input = r#"version: 1
name: HTTP Node Example Flow
slug: demo-http-testing
main:
  - id: entry
    type: entry
    name: Entry
    next: http-get-example
  - id: http-get-example
    type: http:request
    name: HTTP GET Example
    method: get
    url: https://httpbin.org/get
    next: check-status-code
  - id: check-status-code
    type: conditional
    name: Check Status Code
    condition_type: simple
    field_path: status
    operator: equal
    expected_value: 200
    on_true: http-post-example
  - id: http-post-example
    type: http:request
    name: HTTP POST Example
    method: post
    url: https://httpbin.org/post
    body: '{"success":true}'
"#;

        let decl_flow = DeclarativeFlow::from_yaml(yaml_input).expect("parse yaml");
        assert_eq!(decl_flow.name, "HTTP Node Example Flow");
        assert_eq!(decl_flow.main.len(), 4);

        let flow = decl_flow.to_flow().expect("to flow");
        assert_eq!(flow.nodes.len(), 4);
        assert_eq!(flow.connections.len(), 3);

        // 验证自动排版分配了合法正浮点坐标
        for node in &flow.nodes {
            assert!(node.position.x >= 0.0);
            assert!(node.position.y >= 0.0);
        }

        // 验证拓扑层级顺序
        let entry_node = flow.nodes.iter().find(|n| n.name == "Entry").unwrap();
        let get_node = flow.nodes.iter().find(|n| n.name == "HTTP GET Example").unwrap();
        let cond_node = flow.nodes.iter().find(|n| n.name == "Check Status Code").unwrap();
        let post_node = flow.nodes.iter().find(|n| n.name == "HTTP POST Example").unwrap();

        assert!(entry_node.position.x < get_node.position.x);
        assert!(get_node.position.x < cond_node.position.x);
        assert!(cond_node.position.x < post_node.position.x);

        // 往返转回 DeclarativeFlow
        let back_decl = flow.to_declarative();
        assert_eq!(back_decl.main.len(), 4);
        let back_yaml = back_decl.to_yaml().expect("back to yaml");
        assert!(!back_yaml.contains("position"));
        assert!(!back_yaml.contains("ports"));
    }

    #[test]
    fn test_declarative_full_lifecycle_roundtrip() {
        let yaml_input = r#"version: 1
name: Full Lifecycle Flow
slug: full-lifecycle-flow
variables:
  base_url: https://httpbin.org
setup:
  - id: setup-init
    type: script
    name: Setup Init
    script: "fn.variables.set('token', 'abc');"
main:
  - id: entry
    type: entry
    name: Entry
    next: check-cond
  - id: check-cond
    type: conditional
    name: Check Condition
    condition_type: simple
    field_path: status
    operator: equal
    expected_value: 200
    on_true: branch-true
    on_false: branch-false
  - id: branch-true
    type: http:request
    name: Branch True
    url: "{{base_url}}/get"
    method: get
  - id: branch-false
    type: script
    name: Branch False
    script: "throw new Error('failed');"
teardown:
  - id: teardown-clean
    type: script
    name: Teardown Clean
    script: "fn.console.log('done');"
"#;

        let decl = DeclarativeFlow::from_yaml(yaml_input).expect("parse full yaml");
        assert_eq!(decl.setup.len(), 1);
        assert_eq!(decl.main.len(), 4);
        assert_eq!(decl.teardown.len(), 1);

        let flow = decl.to_flow().expect("to flow");
        assert!(flow.setup.is_some());
        assert_eq!(flow.setup.as_ref().unwrap().nodes.len(), 1);
        assert_eq!(flow.nodes.len(), 4);
        assert_eq!(flow.connections.len(), 3);
        assert!(flow.teardown.is_some());
        assert_eq!(flow.teardown.as_ref().unwrap().nodes.len(), 1);

        // 验证各阶段自动排版坐标均为正数
        for n in &flow.setup.as_ref().unwrap().nodes {
            assert!(n.position.x >= 0.0 && n.position.y >= 0.0);
        }
        for n in &flow.nodes {
            assert!(n.position.x >= 0.0 && n.position.y >= 0.0);
        }
        for n in &flow.teardown.as_ref().unwrap().nodes {
            assert!(n.position.x >= 0.0 && n.position.y >= 0.0);
        }

        // 转回 DeclarativeFlow 并输出 YAML
        let back_decl = flow.to_declarative();
        assert_eq!(back_decl.setup.len(), 1);
        assert_eq!(back_decl.main.len(), 4);
        assert_eq!(back_decl.teardown.len(), 1);
        let back_yaml = back_decl.to_yaml().expect("back to yaml");
        assert!(!back_yaml.contains("position"));
        assert!(!back_yaml.contains("ports"));
    }
}
