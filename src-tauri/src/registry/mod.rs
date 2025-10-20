//! Docker MCP Registry Integration
//!
//! Support for discovering, configuring, and testing MCP servers from the official
//! Docker MCP Registry (https://github.com/docker/mcp-registry)

pub mod config_generator;
pub mod fetch;
pub mod platform;
pub mod types;

pub use config_generator::*;
pub use fetch::*;
pub use platform::*;
pub use types::*;
