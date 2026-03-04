// 报告结构定义模块

/// 执行状态枚举
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    /// 执行成功
    Success,
    /// 执行失败
    Failed,
    /// 跳过执行
    Skipped,
}
impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Success => write!(f, "success"),
            ExecutionStatus::Failed => write!(f, "failed"),
            ExecutionStatus::Skipped => write!(f, "skipped"),
        }
    }
}



/// 节点执行报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeExecutionReport {
    /// 节点 UUID
    pub node_uuid: String,
    /// 节点类型
    pub node_type: String,
    /// 节点名称
    pub node_name: String,
    /// 开始时间（毫秒时间戳）
    pub start_time: u64,
    /// 结束时间（毫秒时间戳）
    pub end_time: u64,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 输入数据（完整存储，不截断）
    pub input_data: Option<serde_json::Value>,
    /// 输出数据（完整存储，不截断）
    pub output_data: Option<serde_json::Value>,
    /// 实际使用的输出端口 ID（仅已执行节点有值）
    /// - Entry 节点：固定为 "out"
    /// - Http 节点：固定为 "out"
    /// - Conditional 节点：根据条件评估结果为 "true" 或 "false"
    pub used_port_id: Option<String>,
    /// 错误详情
    pub error: Option<crate::error::ErrorDetail>,
}

/// 报告元数据（用于历史列表）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportMetadata {
    /// 报告唯一 ID
    pub uuid: String,
    /// 关联的流程 ID
    pub flow_uuid: String,
    /// 流程名称
    pub flow_name: String,
    /// 执行状态 ("success" | "failed")
    pub status: String,
    /// 执行开始时间（毫秒）
    pub timestamp: u64,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
}


/// 流程执行报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowExecutionReport {
    /// 报告唯一 ID（新增）
    pub uuid: String,
    /// 流程 UUID
    pub flow_uuid: String,
    /// 流程名称
    pub flow_name: String,
    /// 开始时间（毫秒时间戳）
    pub start_time: u64,
    /// 结束时间（毫秒时间戳）
    pub end_time: u64,
    /// 总执行时长（毫秒）
    pub duration_ms: u64,
    /// 总节点数
    pub total_nodes: u32,
    /// 已执行节点数
    pub executed_count: u32,
    /// 整体执行状态
    pub status: ExecutionStatus,
    /// 节点执行报告列表（扁平结构）
    pub node_reports: Vec<NodeExecutionReport>,
    /// 流程统计信息
    pub statistics: crate::statistics::FlowStatistics,
    /// 流程级错误详情（可选）
    pub flow_error: Option<crate::error::ErrorDetail>,
}
