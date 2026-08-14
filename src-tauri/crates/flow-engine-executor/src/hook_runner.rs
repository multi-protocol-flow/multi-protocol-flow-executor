// 默认钩子执行器实现
//
// 提供基于日志的 Webhook/Script/Notify 三种动作的默认执行器。
// 当前实现以日志输出为主，后续可扩展为真实 HTTP 请求/脚本执行。

use flow_engine_core::hook::{FlowHook, HookAction, HookContext};
use flow_engine_core::traits::HookExecutor;

/// 默认钩子执行器
///
/// 对三种动作类型的处理:
/// - Webhook: 记录 HTTP 请求日志（暂不发送真实请求）
/// - Script: 记录脚本代码日志（暂不执行脚本）
/// - Notify: 记录通知内容日志
pub struct DefaultHookExecutor;

impl DefaultHookExecutor {
    /// 创建新的默认钩子执行器
    pub fn new() -> Self {
        Self
    }

    /// 渲染模板字符串，替换 {{variable}} 占位符
    pub fn render_template(template: &str, context: &HookContext) -> String {
        let mut result = template.to_string();
        result = result.replace("{{flow_name}}", &context.flow_name);
        result = result.replace("{{flow_uuid}}", &context.flow_uuid);
        result = result.replace("{{status}}", &context.status);
        result = result.replace("{{duration_ms}}", &context.duration_ms.to_string());
        if let Some(ref error) = context.error {
            result = result.replace("{{error}}", error);
        }
        result
    }
}

impl Default for DefaultHookExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl HookExecutor for DefaultHookExecutor {
    fn execute(&self, hook: &FlowHook, context: &HookContext) {
        match &hook.action {
            HookAction::Webhook {
                url,
                method,
                headers,
                body_template,
                timeout_ms,
            } => {
                let rendered_url = Self::render_template(url, context);
                let rendered_body = body_template
                    .as_ref()
                    .map(|t| Self::render_template(t, context));
                log::info!(
                    "[Hook] Webhook call: {} {} (headers: {}, body: {}, timeout: {}ms)",
                    method,
                    rendered_url,
                    headers.len(),
                    rendered_body.as_deref().unwrap_or("<none>"),
                    timeout_ms.unwrap_or(30000),
                );
            }
            HookAction::Script { code, timeout_ms } => {
                log::info!(
                    "[Hook] script execution: {} chars, timeout: {}ms",
                    code.len(),
                    timeout_ms.unwrap_or(30000),
                );
            }
            HookAction::Notify {
                title_template,
                message_template,
            } => {
                let title = Self::render_template(title_template, context);
                let message = Self::render_template(message_template, context);
                log::info!("[Hook] Notification: {} - {}", title, message);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_engine_core::hook::HookTrigger;
    use std::collections::HashMap;

    fn test_context() -> HookContext {
        HookContext {
            flow_uuid: "flow-001".to_string(),
            flow_name: "Test Flow".to_string(),
            status: "success".to_string(),
            duration_ms: 1234,
            error: None,
        }
    }

    fn test_context_with_error() -> HookContext {
        HookContext {
            flow_uuid: "flow-002".to_string(),
            flow_name: "Failed流程".to_string(),
            status: "failure".to_string(),
            duration_ms: 5000,
            error: Some("连接超时".to_string()),
        }
    }

    #[test]
    fn test_render_template_basic() {
        let template = "流程 {{flow_name}} Status: {{status}}, Duration: {{duration_ms}}ms";
        let ctx = test_context();
        let result = DefaultHookExecutor::render_template(template, &ctx);
        assert_eq!(result, "流程 Test Flow Status: success, Duration: 1234ms");
    }

    #[test]
    fn test_render_template_with_error() {
        let template = "Error: {{error}}";
        let ctx = test_context_with_error();
        let result = DefaultHookExecutor::render_template(template, &ctx);
        assert_eq!(result, "Error: 连接超时");
    }

    #[test]
    fn test_render_template_no_error() {
        let template = "Error: {{error}}";
        let ctx = test_context();
        let result = DefaultHookExecutor::render_template(template, &ctx);
        // 无Error时 {{error}} 不被替换
        assert_eq!(result, "Error: {{error}}");
    }

    #[test]
    fn test_execute_webhook() {
        let executor = DefaultHookExecutor::new();
        let hook = FlowHook {
            uuid: "hook-1".to_string(),
            name: "Webhook 测试".to_string(),
            enabled: true,
            trigger: HookTrigger::OnSuccess,
            action: HookAction::Webhook {
                url: "https://example.com/hook".to_string(),
                method: "POST".to_string(),
                headers: HashMap::new(),
                body_template: Some(r#"{"status":"{{status}}"}"#.to_string()),
                timeout_ms: Some(5000),
            },
        };
        // 不应 panic
        executor.execute(&hook, &test_context());
    }

    #[test]
    fn test_execute_script() {
        let executor = DefaultHookExecutor::new();
        let hook = FlowHook {
            uuid: "hook-2".to_string(),
            name: "Script 测试".to_string(),
            enabled: true,
            trigger: HookTrigger::OnFailure,
            action: HookAction::Script {
                code: "console.log('hello')".to_string(),
                timeout_ms: None,
            },
        };
        executor.execute(&hook, &test_context());
    }

    #[test]
    fn test_execute_notify() {
        let executor = DefaultHookExecutor::new();
        let hook = FlowHook {
            uuid: "hook-3".to_string(),
            name: "Notify 测试".to_string(),
            enabled: true,
            trigger: HookTrigger::OnComplete,
            action: HookAction::Notify {
                title_template: "流程 {{flow_name}} 完成".to_string(),
                message_template: "Status: {{status}}".to_string(),
            },
        };
        executor.execute(&hook, &test_context());
    }
}
