// use this file to export your custom types; these types will be imported by './i18n-types.ts'
import type { RequiredParams } from 'typesafe-i18n'

declare module './i18n-types' {
	export interface RootTranslation {
		errors: {
			// General
			E_CONFIG_ERROR: string
			E_CONDITION_SYNTAX: string
			E_EXECUTION_ERROR: RequiredParams<'reason'>
			E_TIMEOUT: RequiredParams<'operation' | 'ms'>
			E_CONDITION_EVALUATION: RequiredParams<'expression'>
			E_UNKNOWN: string
			E_CANCELLED: string
			E_PARSE: RequiredParams<'value'>
			E_ASSERTION: string
			E_SCRIPT_ABORT: string
			// Connection
			E_CONN_TIMEOUT: RequiredParams<'host' | 'ms'>
			E_CONN_REFUSED: RequiredParams<'host' | 'port'>
			E_CONN_CLOSED: RequiredParams<'host'>
			// Validation
			E_VAL_PORT_DUPLICATE: string
			E_VAL_CYCLE_DETECTED: string
			E_VAL_MISSING_ENTRY: string
			// HTTP
			E_HTTP_REQUEST_FAILED: RequiredParams<'url' | 'status'>
			E_HTTP_STATUS_ERROR: RequiredParams<'url' | 'status' | 'body'>
			E_HTTP_INVALID_URL: RequiredParams<'url'>
			// WebSocket
			E_WS_CONNECTION: RequiredParams<'url'>
			E_WS_TIMEOUT: RequiredParams<'url'>
			E_WS_MESSAGE: RequiredParams<'url'>
			E_WS_CLOSE: RequiredParams<'url'>
			E_WS_PROTOCOL: RequiredParams<'url'>
			// DB
			E_DB_CONNECTION: RequiredParams<'host'>
			E_DB_QUERY: RequiredParams<'query'>
			E_DB_TIMEOUT: RequiredParams<'host'>
			// Script
			E_SCRIPT_SYNTAX: RequiredParams<'detail'>
			E_SCRIPT_RUNTIME: RequiredParams<'detail'>
			E_SCRIPT_TIMEOUT: RequiredParams<'ms'>
			E_SCRIPT_ASSERTION: RequiredParams<'detail'>
			// gRPC
			E_GRPC_CONNECTION: RequiredParams<'target'>
			E_GRPC_TIMEOUT: RequiredParams<'target'>
			E_GRPC_MESSAGE: RequiredParams<'target'>
			E_GRPC_STATUS: RequiredParams<'target' | 'code'>
			E_GRPC_CLOSE: RequiredParams<'target'>
			// MCP
			E_MCP_CONNECTION: RequiredParams<'server'>
			E_MCP_TOOL_CALL: RequiredParams<'tool'>
			E_MCP_RESOURCE_READ: RequiredParams<'uri'>
			E_MCP_NOT_CONNECTED: RequiredParams<'server'>
			// TCP
			E_TCP_CONNECTION: RequiredParams<'addr'>
			E_TCP_TIMEOUT: RequiredParams<'addr'>
			E_TCP_SEND: RequiredParams<'addr'>
			E_TCP_RECEIVE: RequiredParams<'addr'>
			E_TCP_CLOSE: RequiredParams<'addr'>
			// UDP
			E_UDP_BIND: RequiredParams<'addr'>
			E_UDP_SEND: RequiredParams<'addr'>
			E_UDP_RECEIVE: RequiredParams<'addr'>
			E_UDP_CLOSE: RequiredParams<'addr'>
			// SSE
			E_SSE_CONNECTION: RequiredParams<'url'>
			E_SSE_TIMEOUT: RequiredParams<'url'>
			E_SSE_EVENT: RequiredParams<'url'>
			// GraphQL
			E_GRAPHQL_CONNECTION: RequiredParams<'url'>
			E_GRAPHQL_QUERY: RequiredParams<'url'>
			E_GRAPHQL_SUBSCRIPTION: RequiredParams<'url'>
			E_GRAPHQL_INTROSPECTION: RequiredParams<'url'>
			E_GRAPHQL_CLOSE: RequiredParams<'url'>
			// AMQP
			E_AMQP_CONNECTION: RequiredParams<'addr'>
			E_AMQP_CHANNEL: RequiredParams<'addr'>
			E_AMQP_PUBLISH: RequiredParams<'addr'>
			E_AMQP_CONSUME: RequiredParams<'addr'>
			E_AMQP_QUEUE: RequiredParams<'addr'>
			E_AMQP_EXCHANGE: RequiredParams<'addr'>
			E_AMQP_ACK: RequiredParams<'addr'>
			E_AMQP_MANAGEMENT: RequiredParams<'addr'>
			E_AMQP_PROTOCOL: RequiredParams<'addr'>
			// Redis
			E_REDIS_CONNECTION: RequiredParams<'host'>
			E_REDIS_COMMAND: RequiredParams<'cmd'>
			E_REDIS_TIMEOUT: RequiredParams<'host'>
			E_REDIS_AUTH: RequiredParams<'host'>
			// SMTP
			E_SMTP_CONNECTION: RequiredParams<'host'>
			E_SMTP_SEND: RequiredParams<'host'>
			E_SMTP_AUTH: RequiredParams<'host'>
			E_SMTP_TIMEOUT: RequiredParams<'host'>
			// IMAP
			E_IMAP_CONNECTION: RequiredParams<'host'>
			E_IMAP_FETCH: RequiredParams<'host'>
			E_IMAP_SEARCH: RequiredParams<'host'>
			E_IMAP_AUTH: RequiredParams<'host'>
			E_IMAP_STORE: RequiredParams<'host'>
			E_IMAP_TIMEOUT: RequiredParams<'host'>
			// Non-protocol
			E_VARIABLE_EXTRACTOR_FAILED: RequiredParams<'path'>
			E_PROXY_FAILED: RequiredParams<'host'>
		}
	}
}
