//! 删除节点命令

use crate::command::{DeleteNodeCommandInput, DeleteNodeCommandOutput};
use crate::flow::Flow;
use crate::node::NodeType;

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
/// - 如果节点是开始节点，返回错误 "不能删除开始节点"
/// - 否则返回错误信息 "暂未实现持久化"
/// - 说明原因：delete_node 需要先加载流程、删除节点和相关连接、再保存流程
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
