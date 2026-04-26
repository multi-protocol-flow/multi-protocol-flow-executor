//! 更新节点命令

use crate::command::{UpdateNodeCommandInput, UpdateNodeCommandOutput};
use flow_engine_core::flow::Flow;

/// 更新节点命令（桩实现）
///
/// 更新指定流程中的节点数据
///
/// # 参数
/// - `_input`: 包含流程 ID、节点 ID 和新数据的输入参数
/// - `_flow`: 流程对象
///
/// # 返回
/// - `UpdateNodeCommandOutput`: 包含更新结果
///
/// # 桩实现说明
/// - 返回错误信息 "暂未实现持久化"
/// - `说明原因：update_node` 需要先加载流程、更新节点、再保存流程
/// - 这将在任务 2.5（保存流程）中完整实现
pub fn update_node(_input: UpdateNodeCommandInput, _flow: Flow) -> UpdateNodeCommandOutput {
    // 桩实现：返回持久化未实现的错误
    UpdateNodeCommandOutput {
        success: false,
        error: Some("暂未实现持久化：update_node 需要先加载流程、更新节点、再保存流程".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::node::{Node, NodePosition};
    use flow_engine_core::node_config::NodeData;
    use serde_json::json;

    /// 创建测试用的流程
    fn create_test_flow() -> Flow {
        Flow { uuid: "test-flow-uuid".to_string(), name: "测试流程".to_string(), initial_variables: None, nodes: vec![], connections: vec![], hooks: None }
    }

    /// 创建带有节点的测试流程
    fn create_test_flow_with_node() -> Flow {
        let node = Node {
            uuid: "test-node-uuid".to_string(),
            name: "测试节点".to_string(),
            node_type: "entry".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
            has_breakpoint: None,
        };
        Flow { uuid: "test-flow-uuid".to_string(), name: "测试流程".to_string(), initial_variables: None, nodes: vec![node], connections: vec![], hooks: None }
    }

    /// 测试：更新节点配置 - 桩实现返回错误
    #[test]
    fn test_update_node_config_stub_returns_error() {
        let flow = create_test_flow_with_node();
        let input = UpdateNodeCommandInput {
            flow_id: "test-flow-uuid".to_string(),
            node_id: "test-node-uuid".to_string(),
            data: json!({
                "type": "http",
                "url": "https://example.com/api",
                "method": "get"
            }),
        };

        let output = update_node(input, flow);

        // 桩实现应该返回失败
        assert!(!output.success);
        assert!(output.error.is_some());
        let error_msg = output.error.unwrap();
        assert!(error_msg.contains("暂未实现持久化"));
    }

    /// 测试：节点不存在 - 桩实现仍返回相同错误
    #[test]
    fn test_update_node_not_found_stub_returns_error() {
        let flow = create_test_flow();
        let input = UpdateNodeCommandInput {
            flow_id: "test-flow-uuid".to_string(),
            node_id: "non-existent-node".to_string(),
            data: json!({
                "type": "http",
                "url": "https://example.com/api"
            }),
        };

        let output = update_node(input, flow);

        // 桩实现总是返回相同的错误
        assert!(!output.success);
        assert!(output.error.is_some());
        let error_msg = output.error.unwrap();
        assert!(error_msg.contains("暂未实现持久化"));
    }

    /// 测试：部分更新 - 桩实现返回错误
    #[test]
    fn test_update_node_partial_update_stub_returns_error() {
        let flow = create_test_flow_with_node();
        let input = UpdateNodeCommandInput {
            flow_id: "test-flow-uuid".to_string(),
            node_id: "test-node-uuid".to_string(),
            // 仅更新部分字段
            data: json!({
                "name": "更新后的节点名"
            }),
        };

        let output = update_node(input, flow);

        // 桩实现应该返回失败
        assert!(!output.success);
        assert!(output.error.is_some());
        let error_msg = output.error.unwrap();
        assert!(error_msg.contains("暂未实现持久化"));
    }

    /// 测试：空数据更新 - 桩实现返回错误
    #[test]
    fn test_update_node_empty_data_stub_returns_error() {
        let flow = create_test_flow_with_node();
        let input = UpdateNodeCommandInput {
            flow_id: "test-flow-uuid".to_string(),
            node_id: "test-node-uuid".to_string(),
            data: json!({}),
        };

        let output = update_node(input, flow);

        assert!(!output.success);
        assert!(output.error.is_some());
    }

    /// 测试：验证输入结构正确解析
    #[test]
    fn test_update_node_input_structure() {
        let input = UpdateNodeCommandInput {
            flow_id: "flow-123".to_string(),
            node_id: "node-456".to_string(),
            data: json!({
                "key": "value",
                "nested": {
                    "inner": 42
                }
            }),
        };

        assert_eq!(input.flow_id, "flow-123");
        assert_eq!(input.node_id, "node-456");
        assert_eq!(input.data["key"], "value");
        assert_eq!(input.data["nested"]["inner"], 42);
    }

    /// 测试：输出结构序列化
    #[test]
    fn test_update_node_output_serialization() {
        let output = UpdateNodeCommandOutput {
            success: false,
            error: Some("测试错误信息".to_string()),
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("测试错误信息"));
    }
}
