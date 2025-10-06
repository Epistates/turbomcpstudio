//! MCP Client Module - Modular Architecture
//!
//! This module provides a comprehensive, enterprise-grade MCP (Model Context Protocol) client
//! implementation with support for multiple transports, health monitoring, and advanced features.
//!
//! ## Architecture
//!
//! The module is organized into focused sub-modules for better maintainability:
//!
//! - **connection** - Managed connection state with health monitoring and metrics
//! - **elicitation** - Server-initiated user input request handling
//! - **events** - Connection events for UI updates and monitoring
//! - **manager** - Main orchestrator for MCP server connections
//! - **process** - Child process management for STDIO servers
//! - **transport_client** - Transport-agnostic client wrapper for all MCP operations
//!
//! ## Features
//!
//! - **Multi-Transport Support**: STDIO, HTTP/SSE, WebSocket, TCP, Unix sockets
//! - **Enterprise Reliability**: Connection pooling, retry logic, circuit breakers
//! - **TurboMCP Integration**: Full 2.0 API with SIMD-accelerated JSON processing
//! - **Health Monitoring**: Process tracking, resource usage, connection metrics
//! - **Event-Driven**: Real-time updates for UI and protocol inspection
//! - **Handler Support**: Elicitation, sampling, progress, logging, resource updates
//!
//! ## Usage
//!
//! ```rust
//! use crate::mcp_client::McpClientManager;
//!
//! let (manager, event_receiver) = McpClientManager::new(app_handle);
//! let server_id = manager.connect_server(config).await?;
//! let tools = manager.list_tools(server_id).await?;
//! ```

// Module declarations
mod connection;
mod elicitation;
mod events;
pub mod manager;
mod process;
mod transport_client;

// Re-export main types for convenient access
pub use manager::McpClientManager;
