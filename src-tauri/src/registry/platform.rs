//! Cross-platform utilities for Docker integration
//!
//! Handles platform-specific differences for Docker commands, especially:
//! - Windows path conversion (C:\Users -> /c/Users)
//! - Path separator normalization
//! - Docker availability detection

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Normalizes a host path for Docker volume mounting
///
/// # Platform-specific behavior
/// - **Windows**: Converts `C:\Users\foo` to `/c/Users/foo`
/// - **Unix**: Returns path as-is with forward slashes
///
/// # Examples
/// ```no_run
/// use turbomcpstudio_lib::registry::platform::normalize_docker_path;
///
/// // Windows
/// #[cfg(target_os = "windows")]
/// {
///     assert_eq!(normalize_docker_path(r"C:\Users\Alice\data"), "/c/Users/Alice/data");
/// }
///
/// // Unix (macOS, Linux)
/// #[cfg(not(target_os = "windows"))]
/// {
///     assert_eq!(normalize_docker_path("/Users/alice/data"), "/Users/alice/data");
/// }
/// ```
pub fn normalize_docker_path<P: AsRef<Path>>(path: P) -> String {
    let path = path.as_ref();

    #[cfg(target_os = "windows")]
    {
        normalize_windows_path(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Unix systems, just convert to string with forward slashes
        path.to_string_lossy().replace('\\', "/")
    }
}

/// Converts Windows path to Docker-compatible format
///
/// Handles:
/// - Drive letters: C:\ -> /c/
/// - Backslashes: \ -> /
/// - UNC paths: \\server\share -> //server/share
#[cfg(target_os = "windows")]
fn normalize_windows_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();

    // Handle UNC paths (\\server\share)
    if path_str.starts_with(r"\\") {
        return path_str
            .trim_start_matches(r"\\")
            .replace('\\', "/")
            .replacen("", "//", 1); // Add leading //
    }

    // Handle drive letters (C:\Users\...)
    if let Some(drive_letter) = extract_drive_letter(&path_str) {
        let rest = &path_str[2..]; // Skip "C:"
        let normalized_rest = rest.trim_start_matches('\\').replace('\\', "/");
        return format!("/{}/{}", drive_letter.to_lowercase(), normalized_rest);
    }

    // Fallback: just replace backslashes
    path_str.replace('\\', "/")
}

/// Extracts drive letter from Windows path (e.g., "C" from "C:\Users")
#[cfg(target_os = "windows")]
fn extract_drive_letter(path: &str) -> Option<char> {
    if path.len() >= 2 && path.chars().nth(1) == Some(':') {
        path.chars().next()
    } else {
        None
    }
}

/// Normalizes a volume mount specification for Docker
///
/// Converts host:container volume syntax to be cross-platform
///
/// # Examples
/// ```no_run
/// use turbomcpstudio_lib::registry::platform::normalize_volume_mount;
///
/// // Windows
/// #[cfg(target_os = "windows")]
/// {
///     assert_eq!(normalize_volume_mount(r"C:\data:/app/data"), "/c/data:/app/data");
/// }
///
/// // Unix
/// #[cfg(not(target_os = "windows"))]
/// {
///     assert_eq!(normalize_volume_mount("/Users/data:/app/data"), "/Users/data:/app/data");
/// }
/// ```
pub fn normalize_volume_mount(volume_spec: &str) -> String {
    // Find the colon that separates host:container
    // On Windows, skip the drive letter colon (e.g., C:)
    let start_search = if cfg!(target_os = "windows")
        && volume_spec.len() >= 2
        && volume_spec.chars().nth(1) == Some(':')
        && volume_spec
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic())
            .unwrap_or(false)
    {
        2 // Skip "C:"
    } else {
        0
    };

    // Find the first colon after start_search (the host:container separator)
    let colon_pos = volume_spec[start_search..]
        .find(':')
        .map(|p| p + start_search);

    let (host_path, container_and_options) = if let Some(pos) = colon_pos {
        (&volume_spec[..pos], &volume_spec[pos + 1..])
    } else {
        // No colon found, entire string is the path
        (volume_spec, "")
    };

    // Normalize the host path
    let normalized_host = normalize_docker_path(host_path);

    // If there's no container path, just return the normalized host path
    if container_and_options.is_empty() {
        return normalized_host;
    }

    // Check if there's an options part (container:options)
    if let Some(opt_pos) = container_and_options.find(':') {
        let (container, options) = container_and_options.split_at(opt_pos);
        format!("{}:{}{}", normalized_host, container, options)
    } else {
        format!("{}:{}", normalized_host, container_and_options)
    }
}

/// Checks if Docker is available on the system
///
/// Returns Ok(true) if Docker is installed and in PATH
pub async fn check_docker_available() -> Result<bool> {
    let output = tokio::process::Command::new(get_docker_command())
        .arg("--version")
        .output()
        .await
        .context("Failed to execute docker command")?;

    Ok(output.status.success())
}

/// Gets the Docker command name for the current platform
///
/// Returns "docker" on all platforms (Docker Desktop handles this)
pub fn get_docker_command() -> &'static str {
    "docker"
}

/// Expands user home directory in paths
///
/// Converts:
/// - `~` -> user home directory
/// - `~/foo` -> `<home>/foo`
#[allow(dead_code)]
pub fn expand_home_dir<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();

    if let Ok(stripped) = path.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }

    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_normalize_windows_drive_path() {
        assert_eq!(
            normalize_docker_path(r"C:\Users\Alice\data"),
            "/c/Users/Alice/data"
        );
        assert_eq!(normalize_docker_path(r"D:\Projects"), "/d/Projects");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_normalize_windows_unc_path() {
        assert_eq!(
            normalize_docker_path(r"\\server\share\data"),
            "//server/share/data"
        );
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_normalize_unix_path() {
        assert_eq!(
            normalize_docker_path("/Users/alice/data"),
            "/Users/alice/data"
        );
        assert_eq!(
            normalize_docker_path("/home/bob/project"),
            "/home/bob/project"
        );
    }

    #[test]
    fn test_normalize_volume_mount() {
        #[cfg(target_os = "windows")]
        {
            assert_eq!(
                normalize_volume_mount(r"C:\data:/app/data"),
                "/c/data:/app/data"
            );
            assert_eq!(
                normalize_volume_mount(r"C:\data:/app/data:ro"),
                "/c/data:/app/data:ro"
            );
        }

        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(normalize_volume_mount("/data:/app/data"), "/data:/app/data");
            assert_eq!(
                normalize_volume_mount("/data:/app/data:ro"),
                "/data:/app/data:ro"
            );
        }
    }

    #[test]
    fn test_expand_home_dir() {
        if let Some(home) = dirs::home_dir() {
            let expanded = expand_home_dir("~/Documents");
            assert_eq!(expanded, home.join("Documents"));
        }
    }
}
