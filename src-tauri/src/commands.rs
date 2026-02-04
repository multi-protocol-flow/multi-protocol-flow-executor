// Tauri 命令模块

use crate::command::CreateFlowCommandInput;
use crate::command::CreateFlowCommandOutput;
use crate::flow::Flow;
use crate::node::{Node, NodePosition, NodeType};
use serde_json::json;
use uuid::Uuid;

/// 创建流程命令
///
/// 创建一个新的流程对象，自动生成唯一流程 ID，并创建开始节点
///
/// # 参数
/// - `input`: 包含流程名称的输入参数
///
/// # 返回
/// - `CreateFlowCommandOutput`: 包含创建的流程数据
#[tauri::command]
pub fn create_flow(input: CreateFlowCommandInput) -> CreateFlowCommandOutput {
    // 生成唯一流程 ID
    let flow_id = Uuid::new_v4().to_string();

    // 创建开始节点
    let start_node = Node {
        id: format!("start-{}", flow_id),
        node_type: NodeType::Start,
        name: "开始".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: json!({}),
        ports: None,
    };

    // 创建流程对象
    let flow = Flow {
        id: flow_id.clone(),
        name: input.name,
        nodes: vec![start_node],
        connections: vec![],
    };

    CreateFlowCommandOutput { flow }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_flow() {
        let input = CreateFlowCommandInput {
            name: "测试流程".to_string(),
        };

        let output = create_flow(input);

        // 验证流程创建成功
        assert_eq!(output.flow.name, "测试流程");

        // 验证流程有唯一 ID
        assert!(!output.flow.id.is_empty());

        // 验证流程包含一个开始节点
        assert_eq!(output.flow.nodes.len(), 1);

        // 验证开始节点类型
        assert!(matches!(output.flow.nodes[0].node_type, NodeType::Start));

        // 验证开始节点名称
        assert_eq!(output.flow.nodes[0].name, "开始");

        // 验证开始节点 ID 格式
        assert!(output.flow.nodes[0].id.starts_with("start-"));
        assert!(output.flow.nodes[0].id.contains(&output.flow.id));

        // 验证连接列表为空
        assert_eq!(output.flow.connections.len(), 0);
    }

    #[test]
    fn test_unique_flow_ids() {
        let input1 = CreateFlowCommandInput {
            name: "流程1".to_string(),
        };
        let input2 = CreateFlowCommandInput {
            name: "流程2".to_string(),
        };

        let output1 = create_flow(input1);
        let output2 = create_flow(input2);

        // 验证两个流程有不同的 ID
        assert_ne!(output1.flow.id, output2.flow.id);

        // 验证两个开始节点有不同的 ID
        assert_ne!(output1.flow.nodes[0].id, output2.flow.nodes[0].id);
    }
}
