//! 删除节点命令

use crate::command::{DeleteNodeCommandInput, DeleteNodeCommandOutput};
use flow_engine_core::flow::Flow;
use flow_engine_core::node_types;

/// 删除节点命令（桩实现）
///
/// 删除指定流程中的节点和相关连接
///
/// # 参数
/// - `input`: 包含流程 ID 和节点 ID 的输入参数
/// - `flow`: 流程对象
///
/// # 返回
/// - `DeleteNodeCommandOutput`: 包含删除结果
///
/// # 桩实现说明
/// - 如果节点是开始节点，返回Error "cannot delete start node"
/// - 否则返回Error信息 "persistence not yet implemented"
/// - `说明原因：delete_node` 需要先加载流程、删除节点和相关连接、再保存流程
/// - 这将在任务 2.5（保存流程）中完整实现
pub fn delete_node(input: DeleteNodeCommandInput, flow: Flow) -> DeleteNodeCommandOutput {
    // 查找节点
    let node = flow.nodes.iter().find(|n| n.uuid == input.node_id);

    // 验证节点存在
    let node = match node {
        Some(node) => node,
        None => {
            return DeleteNodeCommandOutput {
                success: false,
                error: Some(format!("node {} not found", input.node_id)),
            };
        }
    };

    // 检查是否是开始节点
    if node.node_type == node_types::ENTRY {
        return DeleteNodeCommandOutput {
            success: false,
            error: Some("cannot delete start node".to_string()),
        };
    }

    // 桩实现：返回持久化未实现的Error
    DeleteNodeCommandOutput {
        success: false,
        error: Some(
            "persistence not yet implemented: delete_node needs to load flow, delete node and connections, then save flow"
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::DeleteNodeCommandInput;
    use flow_engine_core::flow::Flow;
    use flow_engine_core::node::{Node, NodePosition};
    use flow_engine_core::node_config::NodeData;

    /// 创建测试用流程
    fn create_test_flow() -> Flow {
        let entry_node = Node {
            uuid: "entry-001".to_string(),
            node_type: "entry".to_string(),
            name: "入口节点".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        let http_node = Node {
            uuid: "http-001".to_string(),
            node_type: "http".to_string(),
            name: "HTTP 请求节点".to_string(),
            position: NodePosition { x: 200.0, y: 0.0 },
            data: NodeData::http("https://api.example.com"),
            ports: None,
            has_breakpoint: None,
            on_error: None,
        };

        Flow {
            uuid: "flow-001".to_string(),
            name: "Test Flow".to_string(),
            description: None,
            initial_variables: None,
            environments: vec![],
            active_environment: None,
            nodes: vec![entry_node, http_node],
            connections: vec![],
            hooks: None,
        }
    }

    #[test]
    fn test_delete_non_existing_node() {
        // 测试删除不存在的节点
        let flow = create_test_flow();
        let input = DeleteNodeCommandInput {
            flow_id: "flow-001".to_string(),
            node_id: "non-existing-node".to_string(),
        };

        let result = delete_node(input, flow);

        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result
            .error
            .unwrap_or_else(|| panic!("expected Some"))
            .contains("not found"));
    }

    #[test]
    fn test_delete_entry_node_should_fail() {
        // 测试删除开始节点应该Failed
        let flow = create_test_flow();
        let input = DeleteNodeCommandInput {
            flow_id: "flow-001".to_string(),
            node_id: "entry-001".to_string(),
        };

        let result = delete_node(input, flow);

        assert!(!result.success);
        assert!(result.error.is_some());
        let error_msg = result.error.unwrap_or_else(|| panic!("expected Some"));
        assert!(error_msg.contains("start node"));
    }

    #[test]
    fn test_delete_existing_node_returns_stub_error() {
        // 测试删除存在的非 Entry node（桩实现）
        let flow = create_test_flow();
        let input = DeleteNodeCommandInput {
            flow_id: "flow-001".to_string(),
            node_id: "http-001".to_string(),
        };

        let result = delete_node(input, flow);

        // 桩实现：返回持久化未实现的Error
        assert!(!result.success);
        assert!(result.error.is_some());
        let error_msg = result.error.unwrap_or_else(|| panic!("expected Some"));
        assert!(error_msg.contains("persistence not yet implemented"));
    }
}
