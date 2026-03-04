//! 更新节点命令

use crate::command::{UpdateNodeCommandInput, UpdateNodeCommandOutput};
use crate::flow::Flow;

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
/// - 说明原因：update_node 需要先加载流程、更新节点、再保存流程
/// - 这将在任务 2.5（保存流程）中完整实现
pub fn update_node(_input: UpdateNodeCommandInput, _flow: Flow) -> UpdateNodeCommandOutput {
    // 桩实现：返回持久化未实现的错误
    UpdateNodeCommandOutput {
        success: false,
        error: Some("暂未实现持久化：update_node 需要先加载流程、更新节点、再保存流程".to_string()),
    }
}
