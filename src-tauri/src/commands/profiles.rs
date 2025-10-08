//! Server Profile Commands
//!
//! Tauri commands for managing server profiles.
//! - Create and update profiles
//! - Add/remove servers from profiles
//! - Activate/deactivate profiles
//! - List and query profiles
//!
//! These commands are now fully registered and available to the frontend.

use crate::types::{
    ActiveProfileState, AddServerToProfileRequest, CreateProfileRequest, ProfileActivation,
    ProfileServer, ServerProfile, ServerProfileWithCount,
};
use crate::AppState;
use serde_json;
use sqlx::Row;
use tauri::State;
use uuid::Uuid;

/// Create a new server profile
#[tauri::command]
pub async fn create_server_profile(
    request: CreateProfileRequest,
    app_state: State<'_, AppState>,
) -> Result<ServerProfile, String> {
    tracing::info!("Creating server profile: {}", request.name);

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let profile = ServerProfile {
        id: id.clone(),
        name: request.name.clone(),
        description: request.description,
        icon: request.icon,
        color: request.color,
        auto_activate: request.auto_activate,
        created_at: now.clone(),
        updated_at: now,
    };

    // Save to database
    let auto_activate_int = if profile.auto_activate { 1 } else { 0 };

    sqlx::query(
        r#"
        INSERT INTO server_profiles (id, name, description, icon, color, auto_activate, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&profile.id)
    .bind(&profile.name)
    .bind(&profile.description)
    .bind(&profile.icon)
    .bind(&profile.color)
    .bind(auto_activate_int)
    .bind(&profile.created_at)
    .bind(&profile.updated_at)
    .execute(database.pool())
    .await
    .map_err(|e| format!("Failed to create profile: {}", e))?;

    tracing::info!("✓ Profile created: {} ({})", profile.name, profile.id);
    Ok(profile)
}

/// Update an existing server profile
#[tauri::command]
pub async fn update_server_profile(
    id: String,
    request: CreateProfileRequest,
    app_state: State<'_, AppState>,
) -> Result<ServerProfile, String> {
    tracing::info!("Updating server profile: {}", id);

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    let now = chrono::Utc::now().to_rfc3339();
    let auto_activate_int = if request.auto_activate { 1 } else { 0 };

    sqlx::query(
        r#"
        UPDATE server_profiles
        SET name = ?, description = ?, icon = ?, color = ?, auto_activate = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&request.name)
    .bind(&request.description)
    .bind(&request.icon)
    .bind(&request.color)
    .bind(auto_activate_int)
    .bind(&now)
    .bind(&id)
    .execute(database.pool())
    .await
    .map_err(|e| format!("Failed to update profile: {}", e))?;

    // Release lock before recursive call
    drop(db_lock);

    // Fetch and return the updated profile
    get_server_profile(id, app_state).await
}

/// Delete a server profile
#[tauri::command]
pub async fn delete_server_profile(
    id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Deleting server profile: {}", id);

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    // Check if this is the active profile
    let active_profile = sqlx::query_scalar::<_, Option<String>>(
        "SELECT profile_id FROM active_profile_state WHERE id = 1",
    )
    .fetch_optional(database.pool())
    .await
    .map_err(|e| format!("Failed to check active profile: {}", e))?;

    if let Some(active_id) = active_profile.flatten() {
        if active_id == id {
            return Err("Cannot delete the active profile. Deactivate it first.".to_string());
        }
    }

    sqlx::query("DELETE FROM server_profiles WHERE id = ?")
        .bind(&id)
        .execute(database.pool())
        .await
        .map_err(|e| format!("Failed to delete profile: {}", e))?;

    tracing::info!("✓ Profile deleted: {}", id);
    Ok(())
}

/// List all server profiles
#[tauri::command]
pub async fn list_server_profiles(
    app_state: State<'_, AppState>,
) -> Result<Vec<ServerProfileWithCount>, String> {
    tracing::info!("Listing server profiles");

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    let rows = sqlx::query(
        r#"
        SELECT p.id, p.name, p.description, p.icon, p.color, p.auto_activate, p.created_at, p.updated_at,
               COUNT(ps.server_id) as server_count
        FROM server_profiles p
        LEFT JOIN profile_servers ps ON p.id = ps.profile_id
        GROUP BY p.id
        ORDER BY p.updated_at DESC
        "#,
    )
    .fetch_all(database.pool())
    .await
    .map_err(|e| format!("Failed to list profiles: {}", e))?;

    let profiles: Vec<ServerProfileWithCount> = rows
        .into_iter()
        .map(|row| {
            let auto_activate: i32 = row.try_get("auto_activate").unwrap_or(0);
            ServerProfileWithCount {
                profile: ServerProfile {
                    id: row.try_get("id").unwrap_or_default(),
                    name: row.try_get("name").unwrap_or_default(),
                    description: row.try_get("description").ok(),
                    icon: row.try_get("icon").ok(),
                    color: row.try_get("color").ok(),
                    auto_activate: auto_activate != 0,
                    created_at: row.try_get("created_at").unwrap_or_default(),
                    updated_at: row.try_get("updated_at").unwrap_or_default(),
                },
                server_count: row.try_get("server_count").unwrap_or(0),
            }
        })
        .collect();

    tracing::info!("✓ Found {} profiles", profiles.len());
    Ok(profiles)
}

/// Get a specific server profile
#[tauri::command]
pub async fn get_server_profile(
    id: String,
    app_state: State<'_, AppState>,
) -> Result<ServerProfile, String> {
    tracing::info!("Getting server profile: {}", id);

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    let row = sqlx::query(
        r#"
        SELECT id, name, description, icon, color, auto_activate, created_at, updated_at
        FROM server_profiles
        WHERE id = ?
        "#,
    )
    .bind(&id)
    .fetch_one(database.pool())
    .await
    .map_err(|e| format!("Profile not found: {}", e))?;

    let auto_activate: i32 = row.try_get("auto_activate").unwrap_or(0);

    Ok(ServerProfile {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        icon: row.try_get("icon").ok(),
        color: row.try_get("color").ok(),
        auto_activate: auto_activate != 0,
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    })
}

/// Add a server to a profile
#[tauri::command]
pub async fn add_server_to_profile(
    request: AddServerToProfileRequest,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!(
        "Adding server {} to profile {}",
        request.server_id,
        request.profile_id
    );

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    let now = chrono::Utc::now().to_rfc3339();
    let env_json = request
        .config
        .environment_overrides
        .as_ref()
        .and_then(|env| serde_json::to_string(env).ok());

    sqlx::query(
        r#"
        INSERT INTO profile_servers (
            profile_id, server_id, startup_order, startup_delay_ms,
            auto_connect, auto_restart, required, environment_overrides, created_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(profile_id, server_id) DO UPDATE SET
            startup_order = excluded.startup_order,
            startup_delay_ms = excluded.startup_delay_ms,
            auto_connect = excluded.auto_connect,
            auto_restart = excluded.auto_restart,
            required = excluded.required,
            environment_overrides = excluded.environment_overrides
        "#,
    )
    .bind(&request.profile_id)
    .bind(&request.server_id)
    .bind(request.config.startup_order)
    .bind(request.config.startup_delay_ms)
    .bind(if request.config.auto_connect { 1 } else { 0 })
    .bind(if request.config.auto_restart { 1 } else { 0 })
    .bind(if request.config.required { 1 } else { 0 })
    .bind(env_json)
    .bind(now)
    .execute(database.pool())
    .await
    .map_err(|e| format!("Failed to add server to profile: {}", e))?;

    tracing::info!("✓ Server added to profile");
    Ok(())
}

/// Remove a server from a profile
#[tauri::command]
pub async fn remove_server_from_profile(
    profile_id: String,
    server_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Removing server {} from profile {}", server_id, profile_id);

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    sqlx::query("DELETE FROM profile_servers WHERE profile_id = ? AND server_id = ?")
        .bind(&profile_id)
        .bind(&server_id)
        .execute(database.pool())
        .await
        .map_err(|e| format!("Failed to remove server from profile: {}", e))?;

    tracing::info!("✓ Server removed from profile");
    Ok(())
}

/// Get all servers in a profile
#[tauri::command]
pub async fn get_profile_servers(
    profile_id: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<ProfileServer>, String> {
    tracing::info!("Getting servers for profile: {}", profile_id);

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    let rows = sqlx::query(
        r#"
        SELECT
            ps.profile_id, ps.server_id, ps.startup_order, ps.startup_delay_ms,
            ps.auto_connect, ps.auto_restart, ps.required, ps.environment_overrides, ps.created_at,
            sc.name as server_name, sc.description as server_description,
            sc.transport_type
        FROM profile_servers ps
        JOIN server_configs sc ON ps.server_id = sc.id
        WHERE ps.profile_id = ?
        ORDER BY ps.startup_order, sc.name
        "#,
    )
    .bind(&profile_id)
    .fetch_all(database.pool())
    .await
    .map_err(|e| format!("Failed to get profile servers: {}", e))?;

    let servers: Vec<ProfileServer> = rows
        .into_iter()
        .map(|row| {
            let auto_connect: i32 = row.try_get("auto_connect").unwrap_or(1);
            let auto_restart: i32 = row.try_get("auto_restart").unwrap_or(0);
            let required: i32 = row.try_get("required").unwrap_or(0);

            let env_overrides: Option<std::collections::HashMap<String, String>> = row
                .try_get::<Option<String>, _>("environment_overrides")
                .ok()
                .flatten()
                .and_then(|json| serde_json::from_str(json.as_str()).ok());

            ProfileServer {
                profile_id: row.try_get("profile_id").unwrap_or_default(),
                server_id: row.try_get("server_id").unwrap_or_default(),
                server_name: row.try_get("server_name").unwrap_or_default(),
                server_description: row.try_get("server_description").ok(),
                transport_type: row.try_get("transport_type").unwrap_or_default(),
                startup_order: row.try_get("startup_order").unwrap_or(0),
                startup_delay_ms: row.try_get("startup_delay_ms").unwrap_or(0),
                auto_connect: auto_connect != 0,
                auto_restart: auto_restart != 0,
                required: required != 0,
                environment_overrides: env_overrides,
                created_at: row.try_get("created_at").unwrap_or_default(),
            }
        })
        .collect();

    tracing::info!("✓ Found {} servers in profile", servers.len());
    Ok(servers)
}

/// Activate a server profile (connect all servers in the profile)
#[tauri::command]
pub async fn activate_profile(
    profile_id: String,
    app_state: State<'_, AppState>,
) -> Result<ProfileActivation, String> {
    tracing::info!("Activating profile: {}", profile_id);

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    // 1. Get the profile
    let profile = sqlx::query(
        "SELECT id, name, description, icon, color, auto_activate, created_at, updated_at FROM server_profiles WHERE id = ?"
    )
    .bind(&profile_id)
    .fetch_one(database.pool())
    .await
    .map_err(|e| format!("Profile not found: {}", e))?;

    let profile_name: String = profile.try_get("name").unwrap_or_default();

    // 2. Get servers in the profile, ordered by startup_order
    let rows = sqlx::query(
        r#"
        SELECT
            ps.profile_id, ps.server_id, ps.startup_order, ps.startup_delay_ms,
            ps.auto_connect, ps.auto_restart, ps.required, ps.environment_overrides, ps.created_at,
            sc.name as server_name, sc.description as server_description,
            sc.transport_type
        FROM profile_servers ps
        JOIN server_configs sc ON ps.server_id = sc.id
        WHERE ps.profile_id = ? AND ps.auto_connect = 1
        ORDER BY ps.startup_order, sc.name
        "#,
    )
    .bind(&profile_id)
    .fetch_all(database.pool())
    .await
    .map_err(|e| format!("Failed to get profile servers: {}", e))?;

    // 3. Group servers by startup_order for orchestrated startup
    let mut servers_by_order: std::collections::BTreeMap<i32, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut server_delays: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();
    let mut required_servers: std::collections::HashSet<String> = std::collections::HashSet::new();

    for row in rows {
        let server_id: String = row.try_get("server_id").unwrap_or_default();
        let startup_order: i32 = row.try_get("startup_order").unwrap_or(0);
        let startup_delay_ms: i32 = row.try_get("startup_delay_ms").unwrap_or(0);
        let required: i32 = row.try_get("required").unwrap_or(0);

        servers_by_order
            .entry(startup_order)
            .or_default()
            .push(server_id.clone());
        server_delays.insert(server_id.clone(), startup_delay_ms as u64);

        if required != 0 {
            required_servers.insert(server_id);
        }
    }

    drop(db_lock);

    // 4. Start servers group by group
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut errors = Vec::new();

    for (_order, server_group) in servers_by_order {
        // Connect all servers in this startup order group concurrently
        let mut tasks = Vec::new();

        for server_id in &server_group {
            let server_id_clone = server_id.clone();
            let app_state_inner = app_state.inner().clone();

            tasks.push(tokio::spawn(async move {
                let mcp_manager = &app_state_inner.mcp_manager;

                // Parse server ID as UUID and get server config
                let uuid = match Uuid::parse_str(&server_id_clone) {
                    Ok(u) => u,
                    Err(e) => return Err((server_id_clone, format!("Invalid UUID: {}", e))),
                };

                // Load server config from database
                let db_lock = app_state_inner.database.read().await;
                let database = match db_lock.as_ref() {
                    Some(db) => db,
                    None => return Err((server_id_clone, "Database not initialized".to_string())),
                };

                let server_config = match database.load_server_config(uuid).await {
                    Ok(Some(config)) => config,
                    Ok(None) => {
                        return Err((server_id_clone, "Server config not found".to_string()))
                    }
                    Err(e) => {
                        return Err((server_id_clone, format!("Failed to load config: {}", e)))
                    }
                };

                drop(db_lock);

                // Try to connect
                match mcp_manager.connect_server(server_config).await {
                    Ok(_) => Ok(server_id_clone),
                    Err(e) => Err((server_id_clone, e.to_string())),
                }
            }));
        }

        // Wait for all servers in this group to connect
        let results = futures::future::join_all(tasks).await;

        for result in results {
            match result {
                Ok(Ok(server_id)) => {
                    success_count += 1;
                    tracing::info!("✓ Server {} connected", server_id);

                    // Apply startup delay if configured
                    if let Some(delay_ms) = server_delays.get(&server_id) {
                        if *delay_ms > 0 {
                            tokio::time::sleep(tokio::time::Duration::from_millis(*delay_ms)).await;
                        }
                    }
                }
                Ok(Err((server_id, error))) => {
                    failure_count += 1;
                    let error_msg = format!("{}: {}", server_id, error);
                    errors.push(error_msg.clone());
                    tracing::error!("✗ Server {} failed: {}", server_id, error);

                    // Check if this was a required server
                    if required_servers.contains(&server_id) {
                        return Err(format!(
                            "Required server '{}' failed to connect: {}",
                            server_id, error
                        ));
                    }
                }
                Err(e) => {
                    failure_count += 1;
                    let error_msg = format!("Task error: {}", e);
                    errors.push(error_msg);
                    tracing::error!("✗ Task failed: {}", e);
                }
            }
        }
    }

    // 5. Record activation
    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    let activation_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let errors_json = if errors.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&errors).unwrap_or_default())
    };

    sqlx::query(
        r#"
        INSERT INTO profile_activations (id, profile_id, activated_at, success_count, failure_count, errors)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&activation_id)
    .bind(&profile_id)
    .bind(&now)
    .bind(success_count)
    .bind(failure_count)
    .bind(&errors_json)
    .execute(database.pool())
    .await
    .map_err(|e| format!("Failed to record activation: {}", e))?;

    // 6. Update active profile state
    sqlx::query(
        r#"
        INSERT INTO active_profile_state (id, profile_id, activated_at)
        VALUES (1, ?, ?)
        ON CONFLICT(id) DO UPDATE SET profile_id = excluded.profile_id, activated_at = excluded.activated_at
        "#,
    )
    .bind(&profile_id)
    .bind(&now)
    .execute(database.pool())
    .await
    .map_err(|e| format!("Failed to set active profile: {}", e))?;

    let activation = ProfileActivation {
        id: activation_id,
        profile_id,
        profile_name,
        activated_at: now,
        deactivated_at: None,
        success_count,
        failure_count,
        errors: if errors.is_empty() {
            None
        } else {
            Some(errors)
        },
    };

    tracing::info!(
        "✓ Profile activated: {}/{} servers connected",
        success_count,
        success_count + failure_count
    );
    Ok(activation)
}

/// Deactivate the current profile (disconnect all servers)
#[tauri::command]
pub async fn deactivate_profile(app_state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("Deactivating current profile");

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    // Get active profile
    let active_profile = sqlx::query_scalar::<_, Option<String>>(
        "SELECT profile_id FROM active_profile_state WHERE id = 1",
    )
    .fetch_optional(database.pool())
    .await
    .map_err(|e| format!("Failed to get active profile: {}", e))?;

    if let Some(Some(profile_id)) = active_profile {
        // Get all servers in the profile
        let server_ids: Vec<String> =
            sqlx::query_scalar("SELECT server_id FROM profile_servers WHERE profile_id = ?")
                .bind(&profile_id)
                .fetch_all(database.pool())
                .await
                .map_err(|e| format!("Failed to get profile servers: {}", e))?;

        drop(db_lock);

        // Disconnect all servers
        let mcp_manager = &app_state.mcp_manager;
        for server_id in server_ids {
            let uuid = match Uuid::parse_str(&server_id) {
                Ok(u) => u,
                Err(e) => {
                    tracing::warn!("Invalid server ID {}: {}", server_id, e);
                    continue;
                }
            };

            match mcp_manager.disconnect_server(uuid).await {
                Ok(_) => tracing::info!("✓ Server {} disconnected", server_id),
                Err(e) => tracing::warn!("Failed to disconnect server {}: {}", server_id, e),
            }
        }

        // Update activation record
        let db_lock = app_state.database.read().await;
        let database = db_lock
            .as_ref()
            .ok_or_else(|| "Database not yet initialized".to_string())?;

        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE profile_activations
            SET deactivated_at = ?
            WHERE profile_id = ? AND deactivated_at IS NULL
            "#,
        )
        .bind(&now)
        .bind(&profile_id)
        .execute(database.pool())
        .await
        .map_err(|e| format!("Failed to update activation: {}", e))?;

        // Clear active profile state
        sqlx::query("DELETE FROM active_profile_state WHERE id = 1")
            .execute(database.pool())
            .await
            .map_err(|e| format!("Failed to clear active profile: {}", e))?;

        tracing::info!("✓ Profile deactivated");
    } else {
        tracing::info!("No active profile to deactivate");
    }

    Ok(())
}

/// Get all profile-server relationships (for frontend sync)
#[tauri::command]
pub async fn get_all_profile_server_relationships(
    app_state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, Vec<String>>, String> {
    tracing::info!("Getting all profile-server relationships");

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    let rows = sqlx::query(
        r#"
        SELECT profile_id, server_id
        FROM profile_servers
        ORDER BY profile_id, startup_order
        "#,
    )
    .fetch_all(database.pool())
    .await
    .map_err(|e| format!("Failed to get relationships: {}", e))?;

    let mut relationships: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for row in rows {
        let profile_id: String = row.try_get("profile_id").unwrap_or_default();
        let server_id: String = row.try_get("server_id").unwrap_or_default();

        relationships
            .entry(profile_id)
            .or_default()
            .push(server_id);
    }

    tracing::info!(
        "✓ Found {} profiles with server relationships",
        relationships.len()
    );
    Ok(relationships)
}

/// Get the currently active profile state
#[tauri::command]
pub async fn get_active_profile(
    app_state: State<'_, AppState>,
) -> Result<Option<ActiveProfileState>, String> {
    tracing::info!("Getting active profile state");

    let db_lock = app_state.database.read().await;
    let database = db_lock
        .as_ref()
        .ok_or_else(|| "Database not yet initialized".to_string())?;

    // Get active profile ID
    let active_profile = sqlx::query_scalar::<_, Option<String>>(
        "SELECT profile_id FROM active_profile_state WHERE id = 1",
    )
    .fetch_optional(database.pool())
    .await
    .map_err(|e| format!("Failed to get active profile: {}", e))?;

    let Some(Some(profile_id)) = active_profile else {
        return Ok(None);
    };

    // Get profile details
    let profile_row = sqlx::query(
        "SELECT id, name, description, icon, color, auto_activate, created_at, updated_at FROM server_profiles WHERE id = ?"
    )
    .bind(&profile_id)
    .fetch_one(database.pool())
    .await
    .map_err(|e| format!("Profile not found: {}", e))?;

    let auto_activate: i32 = profile_row.try_get("auto_activate").unwrap_or(0);
    let profile = ServerProfile {
        id: profile_row.try_get("id").unwrap_or_default(),
        name: profile_row.try_get("name").unwrap_or_default(),
        description: profile_row.try_get("description").ok(),
        icon: profile_row.try_get("icon").ok(),
        color: profile_row.try_get("color").ok(),
        auto_activate: auto_activate != 0,
        created_at: profile_row.try_get("created_at").unwrap_or_default(),
        updated_at: profile_row.try_get("updated_at").unwrap_or_default(),
    };

    // Get servers in profile
    let server_rows = sqlx::query(
        r#"
        SELECT
            ps.profile_id, ps.server_id, ps.startup_order, ps.startup_delay_ms,
            ps.auto_connect, ps.auto_restart, ps.required, ps.environment_overrides, ps.created_at,
            sc.name as server_name, sc.description as server_description,
            sc.transport_type
        FROM profile_servers ps
        JOIN server_configs sc ON ps.server_id = sc.id
        WHERE ps.profile_id = ?
        ORDER BY ps.startup_order, sc.name
        "#,
    )
    .bind(&profile_id)
    .fetch_all(database.pool())
    .await
    .map_err(|e| format!("Failed to get profile servers: {}", e))?;

    let servers: Vec<ProfileServer> = server_rows
        .into_iter()
        .map(|row| {
            let auto_connect: i32 = row.try_get("auto_connect").unwrap_or(1);
            let auto_restart: i32 = row.try_get("auto_restart").unwrap_or(0);
            let required: i32 = row.try_get("required").unwrap_or(0);

            let env_overrides: Option<std::collections::HashMap<String, String>> = row
                .try_get::<Option<String>, _>("environment_overrides")
                .ok()
                .flatten()
                .and_then(|json| serde_json::from_str(json.as_str()).ok());

            ProfileServer {
                profile_id: row.try_get("profile_id").unwrap_or_default(),
                server_id: row.try_get("server_id").unwrap_or_default(),
                server_name: row.try_get("server_name").unwrap_or_default(),
                server_description: row.try_get("server_description").ok(),
                transport_type: row.try_get("transport_type").unwrap_or_default(),
                startup_order: row.try_get("startup_order").unwrap_or(0),
                startup_delay_ms: row.try_get("startup_delay_ms").unwrap_or(0),
                auto_connect: auto_connect != 0,
                auto_restart: auto_restart != 0,
                required: required != 0,
                environment_overrides: env_overrides,
                created_at: row.try_get("created_at").unwrap_or_default(),
            }
        })
        .collect();

    // Get latest activation
    let activation_row = sqlx::query(
        "SELECT id, profile_id, activated_at, deactivated_at, success_count, failure_count, errors FROM profile_activations WHERE profile_id = ? ORDER BY activated_at DESC LIMIT 1"
    )
    .bind(&profile_id)
    .fetch_optional(database.pool())
    .await
    .map_err(|e| format!("Failed to get activation: {}", e))?;

    let activation = activation_row.map(|row| {
        let errors_json: Option<String> = row.try_get("errors").ok().flatten();
        let errors = errors_json.and_then(|json| serde_json::from_str(&json).ok());

        ProfileActivation {
            id: row.try_get("id").unwrap_or_default(),
            profile_id: row.try_get("profile_id").unwrap_or_default(),
            profile_name: profile.name.clone(),
            activated_at: row.try_get("activated_at").unwrap_or_default(),
            deactivated_at: row.try_get("deactivated_at").ok(),
            success_count: row.try_get("success_count").unwrap_or(0),
            failure_count: row.try_get("failure_count").unwrap_or(0),
            errors,
        }
    });

    Ok(Some(ActiveProfileState {
        profile: Some(profile),
        servers,
        activation,
        is_activating: false,
    }))
}
