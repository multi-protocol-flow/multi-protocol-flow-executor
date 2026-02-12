// Tauri 命令模块

use crate::command::CreateFlowCommandInput;
use crate::command::CreateFlowCommandOutput;
use crate::command::DeleteNodeCommandInput;
use crate::command::DeleteNodeCommandOutput;
use crate::command::ExecuteFlowCommandInput;
use crate::command::ExecuteFlowCommandOutput;
use crate::command::UpdateNodeCommandInput;
use crate::command::UpdateNodeCommandOutput;
use crate::flow::Flow;
use crate::node::{Node, NodePosition, NodeType};
use crate::node_config::NodeData;
use serde_json::json;
use std::time::Instant;
use uuid::Uuid;

#[allow(unused_imports)]
use crate::connection::Connection;

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

/// 执行流程命令（桩实现）
///
/// 执行指定的流程，验证开始节点存在并检查是否有输出连接
///
/// # 参数
/// - `flow`: 要执行的流程对象
///
/// # 返回
/// - `ExecuteFlowCommandOutput`: 包含执行结果
///
/// # 桩实现说明
/// - 验证流程包含开始节点
/// - 检查开始节点是否有输出连接
/// - 如果无连接，返回错误 "无连接"
/// - 否则返回执行成功，执行时间 < 1ms
#[tauri::command]
pub fn execute_flow(_input: ExecuteFlowCommandInput, flow: Flow) -> ExecuteFlowCommandOutput {
    let start_time = Instant::now();

    // 查找开始节点
    let start_node = flow
        .nodes
        .iter()
        .find(|node| matches!(node.node_type, NodeType::Entry));

    // 验证开始节点存在
    let start_node = match start_node {
        Some(node) => node,
        None => {
            return ExecuteFlowCommandOutput {
                success: false,
                data: None,
                error: Some("流程缺少开始节点".to_string()),
                execution_time: start_time.elapsed().as_millis() as u64,
            };
        }
    };

    // 检查开始节点是否有输出连接
    let has_connection = flow
        .connections
        .iter()
        .any(|conn| conn.source_node_uuid == start_node.uuid);

    if !has_connection {
        return ExecuteFlowCommandOutput {
            success: false,
            data: None,
            error: Some("无连接".to_string()),
            execution_time: start_time.elapsed().as_millis() as u64,
        };
    }

    // 执行成功（桩实现：开始节点立即传递）
    ExecuteFlowCommandOutput {
        success: true,
        data: Some(json!({ "flow_id": flow.uuid })),
        error: None,
        execution_time: start_time.elapsed().as_millis() as u64,
    }
}

/// 更新节点命令（桩实现）
///
/// 更新指定流程中的节点数据
///
/// # 参数
/// - `flow`: 流程对象
/// - `input`: 包含流程 ID、节点 ID 和新数据的输入参数
///
/// # 返回
/// - `UpdateNodeCommandOutput`: 包含更新结果
///
/// # 桩实现说明
/// - 返回错误信息 "暂未实现持久化"
/// - 说明原因：update_node 需要先加载流程、更新节点、再保存流程
/// - 这将在任务 2.5（保存流程）中完整实现
#[tauri::command]
pub fn update_node(_input: UpdateNodeCommandInput, _flow: Flow) -> UpdateNodeCommandOutput {
    // 桩实现：返回持久化未实现的错误
    UpdateNodeCommandOutput {
        success: false,
        error: Some("暂未实现持久化：update_node 需要先加载流程、更新节点、再保存流程".to_string()),
    }
}

/// 删除节点命令（桩实现）
///
/// 删除指定流程中的节点和相关连接
///
/// # 参数
/// - `flow`: 流程对象
/// - `input`: 包含流程 ID 和节点 ID 的输入参数
///
/// # 返回
/// - `DeleteNodeCommandOutput`: 包含删除结果
///
/// # 桩实现说明
/// - 如果节点是开始节点，返回错误 "不能删除开始节点"
/// - 否则返回错误信息 "暂未实现持久化"
/// - 说明原因：delete_node 需要先加载流程、删除节点和相关连接、再保存流程
/// - 这将在任务 2.5（保存流程）中完整实现
#[tauri::command]
pub fn delete_node(input: DeleteNodeCommandInput, flow: Flow) -> DeleteNodeCommandOutput {
    // 查找节点
    let node = flow.nodes.iter().find(|n| n.uuid == input.node_id);

    // 验证节点存在
    let node = match node {
        Some(node) => node,
        None => {
            return DeleteNodeCommandOutput {
                success: false,
                error: Some(format!("节点 {} 不存在", input.node_id)),
            };
        }
    };

    // 检查是否是开始节点
    if matches!(node.node_type, NodeType::Entry) {
        return DeleteNodeCommandOutput {
            success: false,
            error: Some("不能删除开始节点".to_string()),
        };
    }

    // 桩实现：返回持久化未实现的错误
    DeleteNodeCommandOutput {
        success: false,
        error: Some(
            "暂未实现持久化：delete_node 需要先加载流程、删除节点和相关连接、再保存流程"
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{
        CreateFlowCommandInput, DeleteNodeCommandInput, ExecuteFlowCommandInput,
        UpdateNodeCommandInput,
    };
    use crate::connection::Connection;
    use crate::flow::Flow;
    use crate::node::{Node, NodePosition, NodeType};
    use crate::node_config::NodeData;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn test_create_flow() {
        let input = CreateFlowCommandInput {
            name: "测试流程".to_string(),
        };

        let output = create_flow(input);

        // 验证流程创建成功
        assert_eq!(output.flow.name, "测试流程");

        // 验证流程有唯一 ID
        assert!(!output.flow.uuid.is_empty());

        // 验证流程包含一个开始节点
        assert_eq!(output.flow.nodes.len(), 1);

        // 验证开始节点类型
        assert!(matches!(output.flow.nodes[0].node_type, NodeType::Entry));

        // 验证开始节点名称
        assert_eq!(output.flow.nodes[0].name, "开始");

        // 验证开始节点 ID 格式
        assert!(output.flow.nodes[0].uuid.starts_with("start-"));
        assert!(output.flow.nodes[0].uuid.contains(&output.flow.uuid));

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
        assert_ne!(output1.flow.uuid, output2.flow.uuid);

        // 验证两个开始节点有不同的 ID
        assert_ne!(output1.flow.nodes[0].uuid, output2.flow.nodes[0].uuid);
    }

    #[test]
    fn test_execute_flow_success() {
        // 创建一个测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
            initial_variables: None,
        };

        let input = ExecuteFlowCommandInput {
            flow_id: flow_id.clone(),
            variables: None,
        };

        // 执行流程
        let output = execute_flow(input, flow);

        // 验证执行成功
        assert!(output.success, "流程应该执行成功");
        assert!(output.error.is_none(), "成功时不应该有错误信息");
        assert!(output.data.is_some(), "成功时应该有返回数据");
        assert_eq!(
            output.data.unwrap()["flow_id"],
            flow_id,
            "返回数据应该包含流程 ID"
        );
    }

    #[test]
    fn test_execute_flow_no_connection() {
        // 创建一个没有连接的测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
            initial_variables: None,
        };

        let input = ExecuteFlowCommandInput {
            flow_id: flow_id.clone(),
            variables: None,
        };

        // 执行流程
        let output = execute_flow(input, flow);

        // 验证执行失败
        assert!(!output.success, "流程应该执行失败（无连接）");
        assert!(output.error.is_some(), "失败时应该有错误信息");
        assert!(
            output.error.unwrap().contains("无连接"),
            "错误信息应该包含 '无连接'"
        );
        assert!(output.data.is_none(), "失败时不应该有返回数据");
    }

    #[test]
    fn test_execute_flow_no_start_node() {
        // 创建一个没有开始节点的测试流程
        let flow_id = Uuid::new_v4().to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![http_node],
            connections: vec![],
            initial_variables: None,
        };

        let input = ExecuteFlowCommandInput {
            flow_id: flow_id.clone(),
            variables: None,
        };

        // 执行流程
        let output = execute_flow(input, flow);

        // 验证执行失败
        assert!(!output.success, "流程应该执行失败（无开始节点）");
        assert!(output.error.is_some(), "失败时应该有错误信息");
        assert!(
            output.error.unwrap().contains("缺少开始节点"),
            "错误信息应该包含 '缺少开始节点'"
        );
    }

    #[test]
    fn test_update_node_not_implemented() {
        // 创建一个测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
            initial_variables: None,
        };

        let input = UpdateNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: format!("start-{}", flow_id),
            data: json!({ "updated": true }),
        };

        // 更新节点
        let output = update_node(input, flow);

        // 验证更新失败（持久化未实现）
        assert!(!output.success, "update_node 应该返回失败");
        assert!(output.error.is_some(), "应该有错误信息");
        let error = output.error.unwrap();
        assert!(
            error.contains("暂未实现持久化"),
            "错误信息应该包含 '暂未实现持久化'，实际: {}",
            error
        );
        assert!(
            error.contains("update_node 需要先加载流程、更新节点、再保存流程"),
            "错误信息应该说明原因"
        );
    }

    #[test]
    fn test_delete_start_node() {
        // 创建一个测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
            initial_variables: None,
        };

        let input = DeleteNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: format!("start-{}", flow_id),
        };

        // 尝试删除开始节点
        let output = delete_node(input, flow);

        // 验证删除失败
        assert!(!output.success, "删除开始节点应该失败");
        assert!(output.error.is_some(), "应该有错误信息");
        let error = output.error.unwrap();
        assert!(
            error.contains("不能删除开始节点"),
            "错误信息应该包含 '不能删除开始节点'，实际: {}",
            error
        );
    }

    #[test]
    fn test_delete_node_not_implemented() {
        // 创建一个测试流程
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node, http_node],
            connections: vec![],
            initial_variables: None,
        };

        let input = DeleteNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: format!("http-{}", flow_id),
        };

        // 尝试删除非开始节点
        let output = delete_node(input, flow);

        // 验证删除失败（持久化未实现）
        assert!(!output.success, "delete_node 应该返回失败");
        assert!(output.error.is_some(), "应该有错误信息");
        let error = output.error.unwrap();
        assert!(
            error.contains("暂未实现持久化"),
            "错误信息应该包含 '暂未实现持久化'，实际: {}",
            error
        );
        assert!(
            error.contains("delete_node 需要先加载流程、删除节点和相关连接、再保存流程"),
            "错误信息应该说明原因"
        );
    }

    #[test]
    fn test_create_flow_generates_unique_uuid() {
        let input = CreateFlowCommandInput {
            name: "测试流程".to_string(),
        };

        let output = create_flow(input);

        // 验证 flow.uuid 是有效的 UUID 格式
        let parsed_uuid = Uuid::parse_str(&output.flow.uuid);
        assert!(parsed_uuid.is_ok(), "flow.uuid 应该是有效的 UUID 格式");

        // 验证 start_node.uuid 基于 flow.uuid
        assert!(
            output.flow.nodes[0].uuid.starts_with("start-"),
            "start_node.uuid 应该以 'start-' 开头"
        );
        assert!(
            output.flow.nodes[0].uuid.contains(&output.flow.uuid),
            "start_node.uuid 应该包含 flow.uuid"
        );
    }

    #[test]
    fn test_execute_flow_uses_connection_source_node_uuid() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let end_node = Node {
            uuid: format!("end-{}", flow_id),
            node_type: NodeType::Entry,
            name: "结束".to_string(),
            position: NodePosition { x: 100.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        // 验证 conn.source_node_uuid 使用正确的 uuid 字段
        let connection = Connection {
            id: format!("conn-{}", flow_id),
            source_node_uuid: start_node.uuid.clone(),
            target_node_uuid: end_node.uuid.clone(),
            source_port_id: "output".to_string(),
            target_port_id: "input".to_string(),
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node, end_node],
            connections: vec![connection],
            initial_variables: None,
        };

        let input = ExecuteFlowCommandInput {
            flow_id: flow_id.clone(),
            variables: None,
        };

        let output = execute_flow(input, flow);

        // 验证使用 conn.source_node_uuid 检查连接
        assert!(output.success, "流程应该执行成功");
    }

    #[test]
    fn test_delete_node_uses_node_uuid() {
        let flow_id = Uuid::new_v4().to_string();
        let http_node = Node {
            uuid: format!("http-{}", flow_id),
            node_type: NodeType::Http,
            name: "HTTP 请求".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::http("https://example.com"),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![http_node],
            connections: vec![],
            initial_variables: None,
        };

        let input = DeleteNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: format!("http-{}", flow_id),
        };

        // 验证 delete_node 使用 node.uuid 字段查找节点
        let output = delete_node(input, flow);

        // 验证找到了节点（通过 uuid）
        assert!(
            !output.error.unwrap().contains("不存在"),
            "应该通过 uuid 找到节点"
        );
    }

    #[test]
    fn test_delete_node_nonexistent_uuid() {
        let flow_id = Uuid::new_v4().to_string();
        let start_node = Node {
            uuid: format!("start-{}", flow_id),
            node_type: NodeType::Entry,
            name: "开始".to_string(),
            position: NodePosition { x: 0.0, y: 0.0 },
            data: NodeData::entry(),
            ports: None,
        };

        let flow = Flow {
            uuid: flow_id.clone(),
            name: "测试流程".to_string(),
            nodes: vec![start_node],
            connections: vec![],
            initial_variables: None,
        };

        let input = DeleteNodeCommandInput {
            flow_id: flow_id.clone(),
            node_id: "non-existent-uuid".to_string(),
        };

        let output = delete_node(input, flow);

        // 验证找不到节点（通过 uuid）
        assert!(!output.success, "删除不存在的节点应该失败");
        assert!(
            output.error.unwrap().contains("不存在"),
            "错误信息应该包含 '不存在'"
        );
    }
}
