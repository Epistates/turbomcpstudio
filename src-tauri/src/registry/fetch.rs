//! Registry fetching and caching

use super::types::*;
use anyhow::{Context, Result};
use serde_yaml;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const REGISTRY_URL: &str = "https://raw.githubusercontent.com/docker/mcp-registry/main/servers";
const CACHE_DURATION: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CacheMetadata {
    version: u32, // Cache format version
    last_updated: SystemTime,
    etag: Option<String>,
    servers: HashMap<String, RegistryServer>,
}

const CACHE_VERSION: u32 = 2; // Increment this to invalidate old caches

/// Fetches the Docker MCP registry
pub async fn fetch_registry() -> Result<HashMap<String, RegistryServer>> {
    // Try to load from cache first
    match load_from_cache()? {
        Some(metadata) => {
            // Check if cache is still fresh
            let age = SystemTime::now().duration_since(metadata.last_updated)?;

            if age < CACHE_DURATION {
                // Cache is fresh, spawn background update task
                let etag = metadata.etag.clone();
                tokio::spawn(async move {
                    let _ = check_for_updates(etag).await;
                });

                return Ok(metadata.servers);
            }

            // Cache expired, fetch fresh data
            let servers = fetch_from_github().await?;
            save_to_cache(&servers, None)?;
            Ok(servers)
        }
        None => {
            // No cache, fetch fresh
            let servers = fetch_from_github().await?;
            save_to_cache(&servers, None)?;
            Ok(servers)
        }
    }
}

/// Checks for registry updates in the background
async fn check_for_updates(cached_etag: Option<String>) -> Result<()> {
    let client = reqwest::Client::new();
    let api_url = "https://api.github.com/repos/docker/mcp-registry/commits/main";

    let mut request = client
        .get(api_url)
        .header("User-Agent", "TurboMCP-Studio");

    if let Some(etag) = &cached_etag {
        request = request.header("If-None-Match", etag);
    }

    let response = request.send().await?;

    // 304 Not Modified = no updates needed
    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(());
    }

    // Get new ETag
    let new_etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Fetch fresh data if there are updates
    if new_etag != cached_etag {
        let servers = fetch_from_github().await?;
        save_to_cache(&servers, new_etag)?;
    }

    Ok(())
}

/// Fetches servers from GitHub by discovering all server.yaml files
async fn fetch_from_github() -> Result<HashMap<String, RegistryServer>> {
    // First, fetch the list of servers from the directory
    let client = reqwest::Client::new();

    // GitHub API to list directory contents
    let api_url = "https://api.github.com/repos/docker/mcp-registry/contents/servers";

    let response = client
        .get(api_url)
        .header("User-Agent", "TurboMCP-Studio")
        .send()
        .await
        .context("Failed to fetch server list from registry")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch registry: HTTP {}", response.status());
    }

    let dir_listing: Vec<GitHubDirEntry> = response
        .json()
        .await
        .context("Failed to parse GitHub directory listing")?;

    // Fetch each server.yaml file
    let mut servers = HashMap::new();

    for entry in dir_listing {
        if entry.entry_type == "dir" {
            // Fetch server.yaml for this server
            if let Ok(server) = fetch_server_config(&client, &entry.name).await {
                servers.insert(entry.name.clone(), server);
            }
        }
    }

    Ok(servers)
}

#[derive(serde::Deserialize)]
struct GitHubDirEntry {
    name: String,
    #[serde(rename = "type")]
    entry_type: String,
}

/// Fetches a single server's configuration
async fn fetch_server_config(client: &reqwest::Client, server_name: &str) -> Result<RegistryServer> {
    let url = format!("{}/{}/server.yaml", REGISTRY_URL, server_name);

    let response = client
        .get(&url)
        .header("User-Agent", "TurboMCP-Studio")
        .send()
        .await
        .context("Failed to fetch server config")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch server config: HTTP {}", response.status());
    }

    let yaml_text = response
        .text()
        .await
        .context("Failed to read server config")?;

    let server: RegistryServer = serde_yaml::from_str(&yaml_text)
        .context("Failed to parse server YAML")?;

    Ok(server)
}

/// Gets the cache directory path
fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .context("Failed to get cache directory")?
        .join("turbomcpstudio")
        .join("registry");

    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

/// Gets the cache file path
fn get_cache_file() -> Result<PathBuf> {
    Ok(get_cache_dir()?.join("catalog.json"))
}

/// Loads servers from cache if valid
fn load_from_cache() -> Result<Option<CacheMetadata>> {
    let cache_file = get_cache_file()?;

    if !cache_file.exists() {
        return Ok(None);
    }

    // Load from cache
    let json = std::fs::read_to_string(&cache_file)?;

    // Try to parse as new format, if it fails or version mismatch, delete old cache and return None
    match serde_json::from_str::<CacheMetadata>(&json) {
        Ok(metadata) => {
            // Check cache version
            if metadata.version != CACHE_VERSION {
                // Version mismatch, clear cache
                let _ = std::fs::remove_file(&cache_file);
                Ok(None)
            } else {
                Ok(Some(metadata))
            }
        }
        Err(_) => {
            // Old format or corrupted, delete it
            let _ = std::fs::remove_file(&cache_file);
            Ok(None)
        }
    }
}

/// Saves servers to cache with metadata
fn save_to_cache(servers: &HashMap<String, RegistryServer>, etag: Option<String>) -> Result<()> {
    let cache_file = get_cache_file()?;

    let metadata = CacheMetadata {
        version: CACHE_VERSION,
        last_updated: SystemTime::now(),
        etag,
        servers: servers.clone(),
    };

    let json = serde_json::to_string_pretty(&metadata)?;
    std::fs::write(cache_file, json)?;
    Ok(())
}

/// Forces a cache refresh
pub async fn refresh_registry() -> Result<HashMap<String, RegistryServer>> {
    // Clear cache
    if let Ok(cache_file) = get_cache_file() {
        let _ = std::fs::remove_file(cache_file);
    }

    // Fetch fresh with new ETag
    let servers = fetch_from_github().await?;
    save_to_cache(&servers, None)?;

    Ok(servers)
}

/// Searches servers by name, title, description, or tags
pub fn search_servers(
    servers: &HashMap<String, RegistryServer>,
    query: &str,
) -> Vec<ServerDisplayInfo> {
    let query_lower = query.to_lowercase();

    servers
        .values()
        .filter(|server| {
            // Match against name
            if server.name.to_lowercase().contains(&query_lower) {
                return true;
            }

            // Match against title
            if let Some(about) = &server.about {
                if let Some(title) = &about.title {
                    if title.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }

                // Match against description
                if let Some(desc) = &about.description {
                    if desc.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }
            }

            // Match against tags
            if let Some(meta) = &server.meta {
                if let Some(tags) = &meta.tags {
                    if tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower)) {
                        return true;
                    }
                }
            }

            false
        })
        .map(ServerDisplayInfo::from)
        .collect()
}

/// Filters servers by category
pub fn filter_by_category(
    servers: &HashMap<String, RegistryServer>,
    category: &str,
) -> Vec<ServerDisplayInfo> {
    servers
        .values()
        .filter(|server| {
            server
                .meta
                .as_ref()
                .and_then(|m| m.category.as_ref())
                .is_some_and(|c| c == category)
        })
        .map(ServerDisplayInfo::from)
        .collect()
}

/// Gets all unique categories
pub fn get_categories(servers: &HashMap<String, RegistryServer>) -> Vec<String> {
    let mut categories: Vec<String> = servers
        .values()
        .filter_map(|server| {
            server
                .meta
                .as_ref()
                .and_then(|m| m.category.clone())
        })
        .collect();

    categories.sort();
    categories.dedup();
    categories
}
