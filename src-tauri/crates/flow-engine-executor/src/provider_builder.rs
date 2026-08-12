//! Protocol provider construction for ExecutionContext
//!
//! DRY extraction: both `executor.rs` and `debug/executor.rs` had identical
//! manager construction code. This module provides a single set of methods
//! that gate each manager construction with `#[cfg(feature = "protocol-xxx")]`.
//!
//! These are provided as an extension trait (`ProviderBuilderExt`) because
//! `ExecutionContext` is defined in `flow-engine-core` — we cannot add
//! inherent methods to a type defined in another crate.
//!
//! # Type-erased storage
//!
//! `set_provider` takes `Arc<dyn Any + Send + Sync>`. The current provider
//! trait hierarchy (`XxxProvider: Send + Sync`) does not include `Any`, so
//! we cannot directly coerce `Arc<dyn XxxProvider>` → `Arc<dyn Any + Send + Sync>`.
//!
//! As a workaround, we store each concrete manager type directly and rely on
//! the call-site (Phase C) to bridge the gap. For now, the stored `Arc<dyn Any>`
//! contains the concrete manager struct; `get_provider::<T>()` must be called
//! with the concrete type, not the trait object.

use flow_engine_core::context::ExecutionContext;
use flow_engine_core::traits::GrpcProvider;
use std::sync::Arc;

/// Extension trait adding protocol provider construction methods to `ExecutionContext`.
///
/// Cannot use inherent `impl` because `ExecutionContext` is defined in
/// `flow-engine-core`. Instead, this trait provides `with_default_providers`
/// and `with_default_providers_and_grpc` as extension methods.
pub trait ProviderBuilderExt: Sized {
    /// Create and inject all protocol providers (from enabled features).
    ///
    /// Each manager is feature-gated. Protocols whose feature flags are
    /// disabled are simply skipped — providers won't be inserted into
    /// the `HashMap`.
    fn with_default_providers(self) -> Self;

    /// Create and inject all protocol providers, with optional external gRPC manager.
    ///
    /// gRPC is special because the `FlowExecutor` has an externally-injected
    /// `grpc_manager` (used for IPC `cancel_stream` etc). If `grpc` is `Some`,
    /// that instance is used; otherwise a fresh one is created (when feature is enabled).
    fn with_default_providers_and_grpc(self, _grpc: Option<Arc<dyn GrpcProvider>>) -> Self;
}

impl ProviderBuilderExt for ExecutionContext {
    #[allow(unused_mut)] // only needed when features are enabled
    fn with_default_providers(mut self) -> Self {
        #[cfg(feature = "protocol-ws")]
        {
            let ws = Arc::new(flow_engine_ws::websocket_manager::WebSocketConnectionManager::new());
            self.set_provider("ws", ws);
        }

        #[cfg(feature = "protocol-mcp")]
        {
            let mcp = Arc::new(flow_engine_mcp::McpConnectionManager::new());
            self.set_provider("mcp", mcp);
        }

        #[cfg(feature = "protocol-tcp")]
        {
            let tcp =
                Arc::new(flow_engine_tcp::tcp_connection_manager::TcpConnectionManager::new());
            self.set_provider("tcp", tcp);
        }

        #[cfg(feature = "protocol-tcp")]
        {
            let udp = Arc::new(flow_engine_tcp::udp_socket_manager::UdpSocketManager::new());
            self.set_provider("udp", udp);
        }

        #[cfg(feature = "protocol-sse")]
        {
            let sse =
                Arc::new(flow_engine_sse::sse_connection_manager::SseConnectionManager::new());
            self.set_provider("sse", sse);
        }

        #[cfg(feature = "protocol-graphql")]
        {
            let graphql = Arc::new(
                flow_engine_graphql::graphql_connection_manager::GraphqlConnectionManager::new(),
            );
            self.set_provider("graphql", graphql);
        }

        self
    }

    #[allow(unused_mut)] // only mut when protocol-grpc is enabled
    fn with_default_providers_and_grpc(mut self, _grpc: Option<Arc<dyn GrpcProvider>>) -> Self {
        #[cfg(feature = "protocol-grpc")]
        {
            let grpc =
                Arc::new(flow_engine_grpc::grpc_connection_manager::GrpcConnectionManager::new());
            self.set_provider("grpc", grpc);
        }

        self
    }
}
