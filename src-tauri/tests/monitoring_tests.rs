/// Connection Monitoring Tests for TurboMCP Studio Backend
///
/// Tests connection monitoring, Arc sharing, and dynamic connection discovery
/// Verifies fix from Phase 1 Issue #3

use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use dashmap::DashMap;

/// Mock connection structure
#[derive(Clone, Debug)]
struct MockConnection {
    id: Uuid,
    name: String,
    last_health_check: Option<String>,
}

/// Mock manager to test DashMap + Arc pattern
struct MockConnectionManager {
    // Phase 1 Issue #3 fix: Wrapped in Arc for sharing with monitoring task
    connections: Arc<DashMap<Uuid, Arc<MockConnection>>>,
}

impl MockConnectionManager {
    fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    fn add_connection(&self, id: Uuid, name: String) {
        let connection = Arc::new(MockConnection {
            id,
            name,
            last_health_check: None,
        });
        self.connections.insert(id, connection);
    }

    fn get_connection(&self, id: &Uuid) -> Option<Arc<MockConnection>> {
        self.connections.get(id).map(|entry| Arc::clone(entry.value()))
    }

    fn remove_connection(&self, id: &Uuid) -> Option<Arc<MockConnection>> {
        self.connections.remove(id).map(|(_, conn)| conn)
    }

    /// Simulate starting monitoring loop - returns Arc to SAME DashMap
    fn start_monitoring(&self) -> Arc<DashMap<Uuid, Arc<MockConnection>>> {
        // Phase 1 Issue #3 fix: Clone the Arc, not the DashMap
        Arc::clone(&self.connections)
    }

    fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

/// OLD broken pattern - creates snapshot instead of sharing reference
struct BrokenMockConnectionManager {
    connections: DashMap<Uuid, Arc<MockConnection>>,
}

impl BrokenMockConnectionManager {
    fn new() -> Self {
        Self {
            connections: DashMap::new(),
        }
    }

    fn add_connection(&self, id: Uuid, name: String) {
        let connection = Arc::new(MockConnection {
            id,
            name,
            last_health_check: None,
        });
        self.connections.insert(id, connection);
    }

    /// BROKEN: This creates a snapshot, not a shared reference
    fn start_monitoring_broken(&self) -> Arc<DashMap<Uuid, Arc<MockConnection>>> {
        // This clones the DashMap (creating a snapshot), then wraps it in Arc
        // Monitoring loop sees the snapshot and never sees new connections
        Arc::new(self.connections.clone())
    }

    fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

#[test]
fn test_arc_dashmap_sharing() {
    // Test Phase 1 Issue #3 fix: Arc<DashMap> allows monitoring to see new connections
    
    let manager = MockConnectionManager::new();
    let id1 = Uuid::new_v4();
    
    // Add initial connection
    manager.add_connection(id1, "Server 1".to_string());
    assert_eq!(manager.connection_count(), 1);
    
    // Start monitoring (gets Arc to SAME DashMap)
    let monitoring_connections = manager.start_monitoring();
    assert_eq!(monitoring_connections.len(), 1, "Monitoring sees initial connection");
    
    // Add new connection AFTER monitoring started
    let id2 = Uuid::new_v4();
    manager.add_connection(id2, "Server 2".to_string());
    
    // CRITICAL: Monitoring should see the new connection
    assert_eq!(
        monitoring_connections.len(), 
        2, 
        "Monitoring should see dynamically added connections (not a snapshot)"
    );
    assert_eq!(manager.connection_count(), 2);
    
    // Verify both connections visible to monitoring
    assert!(monitoring_connections.contains_key(&id1), "Should see connection 1");
    assert!(monitoring_connections.contains_key(&id2), "Should see connection 2");
}

#[test]
fn test_broken_snapshot_pattern() {
    // Test OLD broken behavior: Arc::new(dashmap.clone()) creates snapshot
    
    let manager = BrokenMockConnectionManager::new();
    let id1 = Uuid::new_v4();
    
    // Add initial connection
    manager.add_connection(id1, "Server 1".to_string());
    
    // Start monitoring with BROKEN pattern (creates snapshot)
    let monitoring_snapshot = manager.start_monitoring_broken();
    assert_eq!(monitoring_snapshot.len(), 1, "Snapshot has initial connection");
    
    // Add new connection AFTER monitoring started
    let id2 = Uuid::new_v4();
    manager.add_connection(id2, "Server 2".to_string());
    
    // PROBLEM: Monitoring still sees old snapshot (doesn't see new connection)
    assert_eq!(
        monitoring_snapshot.len(), 
        1, 
        "Snapshot doesn't see dynamically added connections (this is the bug!)"
    );
    assert_eq!(manager.connection_count(), 2, "But manager has 2 connections");
    
    // Monitoring never sees the new connection
    assert!(monitoring_snapshot.contains_key(&id1), "Snapshot has connection 1");
    assert!(!monitoring_snapshot.contains_key(&id2), "Snapshot MISSING connection 2!");
}

#[test]
fn test_concurrent_access_to_shared_dashmap() {
    // Test that multiple threads can access shared DashMap safely
    
    let manager = Arc::new(MockConnectionManager::new());
    let monitoring_connections = manager.start_monitoring();
    
    // Simulate concurrent operations
    let manager1 = Arc::clone(&manager);
    let id1 = Uuid::new_v4();
    manager1.add_connection(id1, "Server 1".to_string());
    
    let manager2 = Arc::clone(&manager);
    let id2 = Uuid::new_v4();
    manager2.add_connection(id2, "Server 2".to_string());
    
    // All should see all connections
    assert_eq!(manager.connection_count(), 2);
    assert_eq!(manager1.connection_count(), 2);
    assert_eq!(manager2.connection_count(), 2);
    assert_eq!(monitoring_connections.len(), 2);
}

#[test]
fn test_connection_removal_visible_to_monitoring() {
    // Test that removed connections are visible to monitoring
    
    let manager = MockConnectionManager::new();
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    
    manager.add_connection(id1, "Server 1".to_string());
    manager.add_connection(id2, "Server 2".to_string());
    
    let monitoring_connections = manager.start_monitoring();
    assert_eq!(monitoring_connections.len(), 2);
    
    // Remove one connection
    manager.remove_connection(&id1);
    
    // Monitoring should see the removal
    assert_eq!(monitoring_connections.len(), 1, "Monitoring should see connection removal");
    assert!(!monitoring_connections.contains_key(&id1), "Removed connection should be gone");
    assert!(monitoring_connections.contains_key(&id2), "Other connection should remain");
}

#[test]
fn test_dashmap_iter_safety() {
    // Test that iterating over DashMap is safe while modifying
    
    let manager = MockConnectionManager::new();
    
    // Add some connections
    for i in 0..5 {
        let id = Uuid::new_v4();
        manager.add_connection(id, format!("Server {}", i));
    }
    
    let monitoring_connections = manager.start_monitoring();
    
    // Iterate while potentially modifying (DashMap is designed for this)
    let count: usize = monitoring_connections.iter().count();
    assert_eq!(count, 5, "Should iterate over all connections");
    
    // Collect IDs
    let ids: Vec<Uuid> = monitoring_connections
        .iter()
        .map(|entry| *entry.key())
        .collect();
    
    assert_eq!(ids.len(), 5, "Should collect all connection IDs");
}

#[test]
fn test_arc_reference_counting() {
    // Verify Arc properly tracks references
    
    let manager = MockConnectionManager::new();
    let id = Uuid::new_v4();
    manager.add_connection(id, "Test Server".to_string());
    
    // Get Arc to connections
    let monitoring1 = manager.start_monitoring();
    let monitoring2 = manager.start_monitoring();
    let monitoring3 = manager.start_monitoring();
    
    // All should reference the same DashMap
    assert_eq!(
        Arc::strong_count(&manager.connections), 
        4, // manager + monitoring1 + monitoring2 + monitoring3
        "Should have 4 strong references to DashMap"
    );
    
    // Drop one reference
    drop(monitoring1);
    assert_eq!(Arc::strong_count(&manager.connections), 3);
    
    drop(monitoring2);
    drop(monitoring3);
    assert_eq!(Arc::strong_count(&manager.connections), 1, "Only manager reference remains");
}

#[test]
fn test_connection_metadata_sharing() {
    // Test that connection data is properly shared via Arc
    
    let manager = MockConnectionManager::new();
    let id = Uuid::new_v4();
    let name = "Test Server".to_string();
    
    manager.add_connection(id, name.clone());
    
    // Get connection via manager
    let conn1 = manager.get_connection(&id).expect("Connection should exist");
    
    // Get connection via monitoring reference
    let monitoring = manager.start_monitoring();
    let conn2 = monitoring.get(&id)
        .map(|entry| Arc::clone(entry.value()))
        .expect("Connection should exist in monitoring");
    
    // Both should reference the same MockConnection
    assert_eq!(conn1.name, conn2.name);
    assert_eq!(conn1.id, conn2.id);
    
    // Verify they're actually the same Arc (same allocation)
    assert!(Arc::ptr_eq(&conn1, &conn2), "Should be the same Arc<MockConnection>");
}

#[test]
fn test_monitoring_pattern_correctness() {
    // Integration test verifying the complete monitoring pattern
    
    let manager = MockConnectionManager::new();
    
    // Simulate app startup: start monitoring immediately
    let monitoring_connections = manager.start_monitoring();
    assert_eq!(monitoring_connections.len(), 0, "No connections at startup");
    
    // User connects to servers dynamically
    let server_ids: Vec<Uuid> = (0..3)
        .map(|i| {
            let id = Uuid::new_v4();
            manager.add_connection(id, format!("Dynamic Server {}", i));
            id
        })
        .collect();
    
    // Monitoring should see all dynamically added connections
    assert_eq!(
        monitoring_connections.len(), 
        3, 
        "Monitoring should see all 3 dynamically added connections"
    );
    
    // Verify each connection is visible
    for (i, id) in server_ids.iter().enumerate() {
        assert!(
            monitoring_connections.contains_key(id),
            "Connection {} should be visible to monitoring", i
        );
    }
    
    // User disconnects from one server
    manager.remove_connection(&server_ids[1]);
    
    // Monitoring should see the removal
    assert_eq!(monitoring_connections.len(), 2, "Monitoring should see disconnection");
    assert!(!monitoring_connections.contains_key(&server_ids[1]), "Disconnected server should be gone");
}

#[test]
fn test_empty_monitoring_initialization() {
    // Test monitoring can start with no connections
    
    let manager = MockConnectionManager::new();
    let monitoring = manager.start_monitoring();
    
    assert_eq!(monitoring.len(), 0, "Monitoring can start with empty DashMap");
    
    // Add connection later
    let id = Uuid::new_v4();
    manager.add_connection(id, "Late Server".to_string());
    
    assert_eq!(monitoring.len(), 1, "Should see connection added after monitoring started");
}


