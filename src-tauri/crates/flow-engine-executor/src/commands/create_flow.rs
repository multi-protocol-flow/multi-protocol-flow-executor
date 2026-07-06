//! 创建流程命令

use crate::command::{CreateFlowCommandInput, CreateFlowCommandOutput};
use flow_engine_core::flow::Flow;
use flow_engine_core::node::{Node, NodePosition};
use flow_engine_core::node_config::NodeData;
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
        node_type: "entry".to_string(),
        name: "开始".to_string(),
        position: NodePosition { x: 0.0, y: 0.0 },
        data: NodeData::entry(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    // 创建结束节点
    let end_node = Node {
        uuid: format!("end-{}", flow_id),
        node_type: "end".to_string(),
        name: "End".to_string(),
        position: NodePosition { x: 400.0, y: 0.0 },
        data: NodeData::end(),
        ports: None,
        has_breakpoint: None,
        on_error: None,
    };

    // 创建流程对象
    let flow = Flow {
        uuid: flow_id.clone(),
        name: input.name,
        description: None,
        nodes: vec![start_node, end_node],
        connections: vec![],
        initial_variables: None,
        environments: vec![],
        active_environment: None,
        hooks: None,
    };

    CreateFlowCommandOutput { flow }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试基本流程创建
    #[test]
    fn test_create_flow_basic() {
        let input = CreateFlowCommandInput {
            name: "Test Flow".to_string(),
        };
        let output = create_flow(input);

        assert_eq!(output.flow.name, "Test Flow");
        assert!(!output.flow.uuid.is_empty());
        assert!(output.flow.connections.is_empty());
        assert!(output.flow.initial_variables.is_none());
    }

    /// 测试创建流程包含入口节点
    #[test]
    fn test_create_flow_has_start_node() {
        let input = CreateFlowCommandInput {
            name: "带节点的流程".to_string(),
        };
        let output = create_flow(input);

        // 应该有两个节点：入口和结束
        assert_eq!(output.flow.nodes.len(), 2);

        let start_node = &output.flow.nodes[0];
        assert_eq!(start_node.node_type, "entry");
        assert_eq!(start_node.name, "开始");
        assert_eq!(start_node.position.x, 0.0);
        assert_eq!(start_node.position.y, 0.0);
        assert!(start_node.ports.is_none());
        assert!(start_node.has_breakpoint.is_none());

        let end_node = &output.flow.nodes[1];
        assert_eq!(end_node.node_type, "end");
        assert_eq!(end_node.name, "End");
        assert_eq!(end_node.position.x, 400.0);
        assert_eq!(end_node.position.y, 0.0);
        assert!(end_node.ports.is_none());
        assert!(end_node.has_breakpoint.is_none());
    }

    /// Test Flow UUID 格式有效性
    #[test]
    fn test_create_flow_uuid_valid() {
        let input = CreateFlowCommandInput {
            name: "UUID 测试".to_string(),
        };
        let output = create_flow(input);

        // 验证 UUID 可以被解析
        let parsed = uuid::Uuid::parse_str(&output.flow.uuid);
        assert!(parsed.is_ok(), "Flow UUID should be a valid UUID");

        // 验证是 UUID v4
        let uuid = parsed.unwrap();
        assert_eq!(
            uuid.get_version(),
            Some(uuid::Version::Random),
            "Flow UUID should be version 4 (random)"
        );
    }

    /// 测试开始节点 ID 遵循 start-{flow_id} 模式
    #[test]
    fn test_create_flow_start_node_id_pattern() {
        let input = CreateFlowCommandInput {
            name: "节点 ID 模式测试".to_string(),
        };
        let output = create_flow(input);

        let expected_node_id = format!("start-{}", output.flow.uuid);
        assert_eq!(
            output.flow.nodes[0].uuid, expected_node_id,
            "Start node ID should follow pattern: start-{{flow_uuid}}"
        );
    }

    /// 测试空名称流程创建
    #[test]
    fn test_create_flow_empty_name() {
        let input = CreateFlowCommandInput {
            name: String::new(),
        };
        let output = create_flow(input);

        assert_eq!(output.flow.name, "");
        assert!(!output.flow.uuid.is_empty());
        assert_eq!(output.flow.nodes.len(), 2);
    }

    /// 测试 Unicode 名称流程创建
    #[test]
    fn test_create_flow_unicode_name() {
        let input = CreateFlowCommandInput {
            name: "Test Flow🚀Тестαβγ".to_string(),
        };
        let output = create_flow(input);

        assert_eq!(output.flow.name, "Test Flow🚀Тестαβγ");
        assert!(!output.flow.uuid.is_empty());
        assert_eq!(output.flow.nodes.len(), 2);
    }

    /// 测试多次调用生成不同 UUID
    #[test]
    fn test_create_flow_unique_uuids() {
        let input1 = CreateFlowCommandInput {
            name: "流程1".to_string(),
        };
        let input2 = CreateFlowCommandInput {
            name: "流程2".to_string(),
        };

        let output1 = create_flow(input1);
        let output2 = create_flow(input2);

        assert_ne!(
            output1.flow.uuid, output2.flow.uuid,
            "Each flow should have a unique UUID"
        );
        assert_ne!(
            output1.flow.nodes[0].uuid, output2.flow.nodes[0].uuid,
            "Each start node should have a unique UUID"
        );
    }

    /// Test Flow序列化
    #[test]
    fn test_create_flow_serialization() {
        let input = CreateFlowCommandInput {
            name: "序列化测试".to_string(),
        };
        let output = create_flow(input);

        let json = serde_json::to_string(&output.flow).unwrap();
        assert!(json.contains(&output.flow.uuid));
        assert!(json.contains("序列化测试"));
        assert!(json.contains("start-"));
        assert!(json.contains("end-"));

        // 反序列化验证
        let deserialized: flow_engine_core::flow::Flow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uuid, output.flow.uuid);
        assert_eq!(deserialized.name, "序列化测试");
        assert_eq!(deserialized.nodes.len(), 2);
    }
}
