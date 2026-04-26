//! GraphQL 协议支持模块
//!
//! 提供 GraphQL 连接管理、查询/变更执行、订阅（WebSocket）、Schema 探索等功能。

pub mod graphql_client;
pub mod graphql_connect_executor;
pub mod graphql_connection_manager;
#[allow(missing_docs)]
pub mod graphql_disconnect_executor;
#[allow(missing_docs)]
pub mod graphql_introspect_executor;
pub mod graphql_query_executor;
pub mod graphql_subscribe_executor;
pub mod graphql_unsubscribe_executor;
#[allow(missing_docs)]
pub mod plugin;

pub use graphql_client::{GraphqlClient, GraphqlError, GraphqlResponse, GraphqlSubscriptionEvent, GraphqlSubscriptionResponse, GraphqlTiming};
pub use graphql_connection_manager::{GraphqlConnection, GraphqlConnectionManager};
