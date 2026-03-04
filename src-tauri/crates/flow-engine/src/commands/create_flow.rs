//! 创建流程命令

use crate::command::{CreateFlowCommandInput, CreateFlowCommandOutput};
use crate::flow::Flow;
use crate::node::{Node, NodePosition, NodeType};
use crate::node_config::NodeData;
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
pub fn create_flow(input: CreateFlowCommandInput) -> CreateFlowCommandOutput {
    // 生成唯一流程 ID
    let flow_id = Uuid::new_v4().to_string();

    // 创建开始节点
    let start_node = Node {
        uuid: format!("start-{}", flow_id),
        node_type: NodeType::Entry,
        name: "开始".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
    };

    // 创建流程对象
    let flow = Flow {
        uuid: flow_id.clone(),
        name: input.name,
        nodes: vec![start_node],
        connections: vec![],
        initial_variables: None,
    };

    CreateFlowCommandOutput { flow }
}
