// MCP Studio Tauri Commands - Modular Architecture
//
// This module organizes 80+ Tauri commands into focused sub-modules
// for better maintainability, testability, and developer experience.
//
// Architecture:
// - Each module contains related commands (5-15 commands)
// - Clear separation of concerns
// - Easy to navigate and modify
// - Parallel development friendly

// Server Lifecycle & Management
pub mod server_config;
pub mod server_lifecycle;

// MCP Protocol Operations
pub mod mcp_prompts;
pub mod mcp_resources;
pub mod mcp_tools;

// Advanced Features
pub mod elicitation;
pub mod sampling;

// Data Management
pub mod collections;
pub mod profiles;
pub mod workflows;

// Client Installation
pub mod client_install;

// Registry Integration
pub mod registry;

// Development & Debugging
pub mod database;
pub mod protocol_inspector;
pub mod step_editor;

// Utilities
pub mod utility;

// Re-export all commands for easy registration in lib.rs
pub use client_install::*;
pub use collections::*;
pub use elicitation::*;
pub use mcp_prompts::*;
pub use mcp_resources::*;
pub use mcp_tools::*;
pub use profiles::*;
pub use protocol_inspector::*;
pub use registry::*;
pub use sampling::*;
pub use server_config::*;
pub use server_lifecycle::*;
pub use utility::*;
pub use workflows::*;
