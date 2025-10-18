//! Docker MCP Registry Integration
//!
//! Support for discovering, configuring, and testing MCP servers from the official
//! Docker MCP Registry (https://github.com/docker/mcp-registry)

pub mod types;
pub mod fetch;
pub mod config_generator;
pub mod platform;

pub use types::*;
pub use fetch::*;
pub use config_generator::*;
pub use platform::*;
