//! Ingress 模块：流程级 Mock 监听服务与 HTTP 响应处理

pub mod plugin_ingress;
pub mod plugin_respond;
pub mod server;

pub use plugin_ingress::HttpIngressPlugin;
pub use plugin_respond::HttpRespondPlugin;
pub use server::{
    start_ingress_server, IngressDebugLauncher, IngressListenerConfig, IngressManager,
    IngressRequest, IngressResponseChannel, IngressResponsePayload, IngressServerHandle,
    INGRESS_RESPONSE_PROVIDER,
};
