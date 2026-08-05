// 钩子模块 - 流程生命周期钩子定义
//
// 支持在流程execution complete（Success/Failed/取消）时自动触发预定义动作。
// 三种动作类型: Webhook (HTTP 回调)、Script (QuickJS 脚本)、Notify (系统通知)。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 钩子触发条件
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookTrigger {
    /// 流程执行Success时触发
    OnSuccess,
    /// Flow execution failed时触发
    OnFailure,
    /// 流程被取消时触发
    OnCancel,
    /// 流程execution complete时触发（无论Success/Failed/取消）
    OnComplete,
}

/// Webhook 动作配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookAction {
    /// 请求 URL
    pub url: String,
    /// HTTP 方法
    pub method: String,
    /// 请求头
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// 请求体模板，支持 {{variable}} 占位符
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_template: Option<String>,
    /// 超时时间（毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// 脚本动作配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptAction {
    /// 脚本代码（QuickJS 运行时）
    pub code: String,
    /// 超时时间（毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// 通知动作配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotifyAction {
    /// 通知标题模板
    pub title_template: String,
    /// 通知消息模板
    pub message_template: String,
}

/// 钩子动作（tagged union / internally tagged）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookAction {
    /// HTTP Webhook 回调
    Webhook {
        /// 请求 URL
        url: String,
        /// HTTP 方法
        method: String,
        /// 请求头
        #[serde(default)]
        headers: HashMap<String, String>,
        /// 请求体模板
        #[serde(default, skip_serializing_if = "Option::is_none")]
        body_template: Option<String>,
        /// 超时时间（毫秒）
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    /// `QuickJS` 脚本执行
    Script {
        /// 脚本代码
        code: String,
        /// 超时时间（毫秒）
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    /// 系统通知
    Notify {
        /// 通知标题模板
        title_template: String,
        /// 通知消息模板
        message_template: String,
    },
}

/// 单个钩子定义
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlowHook {
    /// 钩子唯一标识
    pub uuid: String,
    /// 钩子名称
    pub name: String,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 触发条件
    pub trigger: HookTrigger,
    /// 执行动作
    pub action: HookAction,
}

fn default_true() -> bool {
    true
}

/// 钩子执行上下文，提供给动作执行的运行时信息
#[derive(Debug, Clone)]
pub struct HookContext {
    /// 流程 UUID
    pub flow_uuid: String,
    /// 流程名称
    pub flow_name: String,
    /// 执行Status: "success" | "failure" | "cancelled"
    pub status: String,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
    /// Error信息（仅在Failed时有值）
    pub error: Option<String>,
}

impl FlowHook {
    /// 判断钩子是否应该在给定状态下触发
    pub fn should_trigger(&self, status: &str) -> bool {
        if !self.enabled {
            return false;
        }
        match &self.trigger {
            HookTrigger::OnSuccess => status == "success",
            HookTrigger::OnFailure => status == "failure",
            HookTrigger::OnCancel => status == "cancelled",
            HookTrigger::OnComplete => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_trigger_serialization() {
        let trigger = HookTrigger::OnSuccess;
        let json = serde_json::to_string(&trigger).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json, "\"on_success\"");

        let deserialized: HookTrigger =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(deserialized, HookTrigger::OnSuccess);
    }

    #[test]
    fn test_all_trigger_variants() {
        let triggers = vec![
            HookTrigger::OnSuccess,
            HookTrigger::OnFailure,
            HookTrigger::OnCancel,
            HookTrigger::OnComplete,
        ];
        for trigger in &triggers {
            let json = serde_json::to_string(trigger).unwrap_or_else(|e| panic!("{e}"));
            let back: HookTrigger = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(*trigger, back);
        }
    }

    #[test]
    fn test_webhook_action_serialization() {
        let action = HookAction::Webhook {
            url: "https://example.com/hook".to_string(),
            method: "POST".to_string(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            },
            body_template: Some(r#"{"status":"{{status}}"}"#.to_string()),
            timeout_ms: Some(5000),
        };
        let json = serde_json::to_string(&action).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("\"type\":\"webhook\""));
        assert!(json.contains("\"url\":\"https://example.com/hook\""));

        let back: HookAction = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(action, back);
    }

    #[test]
    fn test_script_action_serialization() {
        let action = HookAction::Script {
            code: "console.log('hello')".to_string(),
            timeout_ms: Some(10000),
        };
        let json = serde_json::to_string(&action).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("\"type\":\"script\""));

        let back: HookAction = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(action, back);
    }

    #[test]
    fn test_notify_action_serialization() {
        let action = HookAction::Notify {
            title_template: "流程 {{flow_name}} 完成".to_string(),
            message_template: "Status: {{status}}, Duration: {{duration_ms}}ms".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap_or_else(|e| panic!("{e}"));
        assert!(json.contains("\"type\":\"notify\""));

        let back: HookAction = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(action, back);
    }

    #[test]
    fn test_flow_hook_should_trigger() {
        let hook = FlowHook {
            uuid: "hook-1".to_string(),
            name: "Test Hook".to_string(),
            enabled: true,
            trigger: HookTrigger::OnSuccess,
            action: HookAction::Notify {
                title_template: "Success".to_string(),
                message_template: "{{status}}".to_string(),
            },
        };

        assert!(hook.should_trigger("success"));
        assert!(!hook.should_trigger("failure"));
        assert!(!hook.should_trigger("cancelled"));
    }

    #[test]
    fn test_flow_hook_disabled() {
        let hook = FlowHook {
            uuid: "hook-2".to_string(),
            name: "Disabled Hook".to_string(),
            enabled: false,
            trigger: HookTrigger::OnComplete,
            action: HookAction::Notify {
                title_template: "Done".to_string(),
                message_template: "{{status}}".to_string(),
            },
        };

        assert!(!hook.should_trigger("success"));
        assert!(!hook.should_trigger("failure"));
    }

    #[test]
    fn test_on_complete_triggers_on_all_statuses() {
        let hook = FlowHook {
            uuid: "hook-3".to_string(),
            name: "Complete Hook".to_string(),
            enabled: true,
            trigger: HookTrigger::OnComplete,
            action: HookAction::Notify {
                title_template: "Complete".to_string(),
                message_template: "{{status}}".to_string(),
            },
        };

        assert!(hook.should_trigger("success"));
        assert!(hook.should_trigger("failure"));
        assert!(hook.should_trigger("cancelled"));
    }

    #[test]
    fn test_flow_hook_full_serialization() {
        let hook = FlowHook {
            uuid: "hook-full".to_string(),
            name: "完整测试".to_string(),
            enabled: true,
            trigger: HookTrigger::OnFailure,
            action: HookAction::Webhook {
                url: "https://api.example.com/notify".to_string(),
                method: "POST".to_string(),
                headers: HashMap::new(),
                body_template: None,
                timeout_ms: None,
            },
        };

        let json = serde_json::to_string(&hook).unwrap_or_else(|e| panic!("{e}"));
        let back: FlowHook = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(hook, back);
    }

    #[test]
    fn test_hook_backward_compat_no_hooks() {
        // 无 hooks 字段的 Flow JSON 应能正常反序列化
        let flow_json = r#"{
            "uuid": "flow-001",
            "name": "无钩子流程",
            "nodes": [],
            "connections": []
        }"#;

        let val: serde_json::Value =
            serde_json::from_str(flow_json).unwrap_or_else(|e| panic!("{e}"));
        assert!(val.get("hooks").is_none());
    }

    #[test]
    fn test_webhook_optional_fields_omitted() {
        let action = HookAction::Webhook {
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body_template: None,
            timeout_ms: None,
        };
        let json = serde_json::to_string(&action).unwrap_or_else(|e| panic!("{e}"));
        assert!(!json.contains("body_template"));
        assert!(!json.contains("timeout_ms"));

        let back: HookAction = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(action, back);
    }

    #[test]
    fn test_hook_context_construction() {
        let ctx = HookContext {
            flow_uuid: "flow-001".to_string(),
            flow_name: "Test Flow".to_string(),
            status: "success".to_string(),
            duration_ms: 1234,
            error: None,
        };
        assert_eq!(ctx.flow_uuid, "flow-001");
        assert_eq!(ctx.status, "success");
        assert!(ctx.error.is_none());
    }
}
