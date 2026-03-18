// 命令模块 - Tauri 命令输入/输出结构体
use crate::db::types::DbConnection;
use crate::report::FlowExecutionReport;

use crate::flow::Flow;
use serde_json::Value;

/// 加载流程命令的输入参数
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LoadFlowCommandInput {
    /// 流程文件路径
    pub path: String,
}

/// 加载流程命令的输出结果
#[derive(Debug, serde::Serialize)]
pub struct LoadFlowCommandOutput {
    /// 加载的流程数据
    pub flow: Flow,
}

/// 保存流程命令的输入参数
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SaveFlowCommandInput {
    /// 要保存的流程数据
    pub flow: Flow,
    /// 保存路径
    pub path: String,
}

/// 保存流程命令的输出结果
#[derive(Debug, serde::Serialize)]
pub struct SaveFlowCommandOutput {
    /// 是否保存成功
    pub success: bool,
}

/// 创建流程命令的输入参数
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CreateFlowCommandInput {
    /// 流程名称
    pub name: String,
}

/// 创建流程命令的输出结果
#[derive(Debug, serde::Serialize)]
pub struct CreateFlowCommandOutput {
    /// 创建的流程数据
    pub flow: Flow,
}

/// 执行流程命令的输入参数
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExecuteFlowCommandInput {
    /// 流程 ID
    pub flow_id: String,
    /// 执行上下文变量（可选）
    pub variables: Option<Value>,
    /// 数据库连接配置列表（可选）
    /// 前端从连接配置中心获取，传递给后端执行器
    #[serde(default)]
    pub db_connections: Option<Vec<DbConnection>>,
}

/// 执行流程命令的输出结果
#[derive(Debug, serde::Serialize)]
pub struct ExecuteFlowCommandOutput {
    /// 执行结果
    pub success: bool,
    /// 返回数据（可选）
    pub data: Option<Value>,
    /// 执行报告（可选）
    pub flow_report: Option<FlowExecutionReport>,
    /// 错误信息（可选）
    pub error: Option<String>,
    /// 执行时间（毫秒）
    pub execution_time: u64,
}

/// 更新节点命令的输入参数
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdateNodeCommandInput {
    /// 流程 ID
    pub flow_id: String,
    /// 节点 ID
    pub node_id: String,
    /// 节点数据
    pub data: Value,
}

/// 更新节点命令的输出结果
#[derive(Debug, serde::Serialize)]
pub struct UpdateNodeCommandOutput {
    /// 是否更新成功
    pub success: bool,
    /// 错误信息（可选）
    pub error: Option<String>,
}

/// 删除节点命令的输入参数
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DeleteNodeCommandInput {
    /// 流程 ID
    pub flow_id: String,
    /// 节点 ID
    pub node_id: String,
}

/// 删除节点命令的输出结果
#[derive(Debug, serde::Serialize)]
pub struct DeleteNodeCommandOutput {
    /// 是否删除成功
    pub success: bool,
    /// 错误信息（可选）
    pub error: Option<String>,
}
