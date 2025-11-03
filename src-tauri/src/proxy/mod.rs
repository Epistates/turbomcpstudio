//! Proxy module - Universal MCP server proxying and adaptation

pub mod manager;
pub mod types;

pub use manager::ProxyManager;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_module_loads() {
        // Ensure module compiles
    }
}
