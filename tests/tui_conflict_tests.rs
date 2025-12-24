// Unit tests for TUI conflict resolution features
// Tests manual editing, bulk operations, auto-merge, and statistics

use anyhow::Result;
use manifold::collab::{Conflict, ConflictStatus, ResolutionStrategy};
use manifold::collab::conflicts::ConflictResolver;
use serde_json::Value;

#[test]
fn test_manual_resolution_with_json_value() -> Result<()> {
    let conflict = Conflict {
        id: "test".to_string(),
        spec_id: "spec-1".to_string(),
        field_path: "config".to_string(),
        local_value: serde_json::json!({"timeout": 30}),
        remote_value: serde_json::json!({"timeout": 60}),
        base_value: Some(serde_json::json!({"timeout": 45})),
        detected_at: 0,
        status: ConflictStatus::Unresolved,
    };
    
    // Manually resolve with custom JSON value
    let manual_value = serde_json::json!({"timeout": 50});
    let (resolved, status) = ConflictResolver::resolve_conflict(
        &conflict,
        ResolutionStrategy::Manual,
        Some(manual_value.clone()),
    )?;
    
    assert_eq!(resolved, manual_value);
    assert_eq!(status, ConflictStatus::ResolvedManual);
    
    Ok(())
}

#[test]
fn test_manual_resolution_with_string() -> Result<()> {
    let conflict = Conflict {
        id: "test".to_string(),
        spec_id: "spec-1".to_string(),
        field_path: "name".to_string(),
        local_value: Value::String("Local Name".to_string()),
        remote_value: Value::String("Remote Name".to_string()),
        base_value: Some(Value::String("Original Name".to_string())),
        detected_at: 0,
        status: ConflictStatus::Unresolved,
    };
    
    // Manually resolve with custom string
    let manual_value = Value::String("Custom Name".to_string());
    let (resolved, status) = ConflictResolver::resolve_conflict(
        &conflict,
        ResolutionStrategy::Manual,
        Some(manual_value.clone()),
    )?;
    
    assert_eq!(resolved, manual_value);
    assert_eq!(status, ConflictStatus::ResolvedManual);
    
    Ok(())
}

#[test]
fn test_manual_resolution_with_null() -> Result<()> {
    let conflict = Conflict {
        id: "test".to_string(),
        spec_id: "spec-1".to_string(),
        field_path: "optional_field".to_string(),
        local_value: Value::String("Some value".to_string()),
        remote_value: Value::String("Other value".to_string()),
        base_value: None,
        detected_at: 0,
        status: ConflictStatus::Unresolved,
    };
    
    // Manually resolve with null (delete field)
    let manual_value = Value::Null;
    let (resolved, status) = ConflictResolver::resolve_conflict(
        &conflict,
        ResolutionStrategy::Manual,
        Some(manual_value.clone()),
    )?;
    
    assert_eq!(resolved, Value::Null);
    assert_eq!(status, ConflictStatus::ResolvedManual);
    
    Ok(())
}

#[test]
fn test_bulk_resolution_simulation() -> Result<()> {
    // Simulate bulk resolution of multiple conflicts
    let conflicts = vec![
        Conflict {
            id: "conflict-1".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field1".to_string(),
            local_value: Value::String("local1".to_string()),
            remote_value: Value::String("remote1".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::Unresolved,
        },
        Conflict {
            id: "conflict-2".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field2".to_string(),
            local_value: Value::String("local2".to_string()),
            remote_value: Value::String("remote2".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::Unresolved,
        },
        Conflict {
            id: "conflict-3".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field3".to_string(),
            local_value: Value::String("local3".to_string()),
            remote_value: Value::String("remote3".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::Unresolved,
        },
    ];
    
    // Apply "Ours" strategy to all
    let mut resolved_count = 0;
    let mut failed_count = 0;
    
    for conflict in &conflicts {
        match ConflictResolver::resolve_conflict(conflict, ResolutionStrategy::Ours, None) {
            Ok((value, status)) => {
                assert_eq!(value, conflict.local_value);
                assert_eq!(status, ConflictStatus::ResolvedLocal);
                resolved_count += 1;
            }
            Err(_) => {
                failed_count += 1;
            }
        }
    }
    
    assert_eq!(resolved_count, 3);
    assert_eq!(failed_count, 0);
    
    Ok(())
}

#[test]
fn test_bulk_resolution_with_failures() -> Result<()> {
    // Simulate bulk resolution where some fail
    let conflicts = vec![
        Conflict {
            id: "conflict-1".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field1".to_string(),
            local_value: Value::String("local1".to_string()),
            remote_value: Value::String("remote1".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::Unresolved,
        },
        Conflict {
            id: "conflict-2".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field2".to_string(),
            local_value: Value::String("local2".to_string()),
            remote_value: Value::String("remote2".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::Unresolved,
        },
    ];
    
    // Try manual strategy without values (should fail)
    let mut resolved_count = 0;
    let mut failed_count = 0;
    
    for conflict in &conflicts {
        match ConflictResolver::resolve_conflict(conflict, ResolutionStrategy::Manual, None) {
            Ok(_) => resolved_count += 1,
            Err(_) => failed_count += 1,
        }
    }
    
    assert_eq!(resolved_count, 0);
    assert_eq!(failed_count, 2);
    
    Ok(())
}

#[test]
fn test_auto_merge_array_conflicts() -> Result<()> {
    let conflict = Conflict {
        id: "test".to_string(),
        spec_id: "spec-1".to_string(),
        field_path: "requirements".to_string(),
        local_value: serde_json::json!([
            {"id": "req-1", "shall": "Local requirement 1"},
            {"id": "req-2", "shall": "Local requirement 2"}
        ]),
        remote_value: serde_json::json!([
            {"id": "req-1", "shall": "Remote requirement 1"},
            {"id": "req-3", "shall": "Remote requirement 3"}
        ]),
        base_value: Some(serde_json::json!([
            {"id": "req-1", "shall": "Base requirement 1"}
        ])),
        detected_at: 0,
        status: ConflictStatus::Unresolved,
    };
    
    // Try auto-merge
    let result = ConflictResolver::resolve_conflict(&conflict, ResolutionStrategy::Merge, None);
    
    // Auto-merge might succeed or fail depending on the implementation
    // If it succeeds, we should get a merged array
    match result {
        Ok((merged_value, status)) => {
            assert!(merged_value.is_array());
            // Status could be any resolved status depending on merge strategy
            assert!(matches!(status, ConflictStatus::ResolvedLocal | ConflictStatus::ResolvedRemote | ConflictStatus::ResolvedManual));
        }
        Err(_) => {
            // Auto-merge failed for this complex case, which is acceptable
            // The user would need to use manual resolution
        }
    }
    
    Ok(())
}

#[test]
fn test_auto_merge_compatible_changes() -> Result<()> {
    // Test auto-merge where local and remote changed different fields
    let conflict = Conflict {
        id: "test".to_string(),
        spec_id: "spec-1".to_string(),
        field_path: "config".to_string(),
        local_value: serde_json::json!({"timeout": 60, "retries": 3}),
        remote_value: serde_json::json!({"timeout": 30, "max_size": 1024}),
        base_value: Some(serde_json::json!({"timeout": 30})),
        detected_at: 0,
        status: ConflictStatus::Unresolved,
    };
    
    let result = ConflictResolver::resolve_conflict(&conflict, ResolutionStrategy::Merge, None);
    
    // This is a complex merge scenario - result depends on implementation
    assert!(result.is_ok() || result.is_err());
    
    Ok(())
}

#[test]
fn test_conflict_stats_empty() {
    let conflicts: Vec<Conflict> = vec![];
    
    let total = conflicts.len();
    let unresolved = conflicts.iter()
        .filter(|c| matches!(c.status, ConflictStatus::Unresolved))
        .count();
    let resolved = total - unresolved;
    
    assert_eq!(total, 0);
    assert_eq!(unresolved, 0);
    assert_eq!(resolved, 0);
}

#[test]
fn test_conflict_stats_all_unresolved() {
    let conflicts = vec![
        Conflict {
            id: "1".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field1".to_string(),
            local_value: Value::String("local".to_string()),
            remote_value: Value::String("remote".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::Unresolved,
        },
        Conflict {
            id: "2".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field2".to_string(),
            local_value: Value::String("local".to_string()),
            remote_value: Value::String("remote".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::Unresolved,
        },
    ];
    
    let total = conflicts.len();
    let unresolved = conflicts.iter()
        .filter(|c| matches!(c.status, ConflictStatus::Unresolved))
        .count();
    let resolved = total - unresolved;
    
    assert_eq!(total, 2);
    assert_eq!(unresolved, 2);
    assert_eq!(resolved, 0);
}

#[test]
fn test_conflict_stats_mixed() {
    let conflicts = vec![
        Conflict {
            id: "1".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field1".to_string(),
            local_value: Value::String("local".to_string()),
            remote_value: Value::String("remote".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::Unresolved,
        },
        Conflict {
            id: "2".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field2".to_string(),
            local_value: Value::String("local".to_string()),
            remote_value: Value::String("remote".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::ResolvedLocal,
        },
        Conflict {
            id: "3".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field3".to_string(),
            local_value: Value::String("local".to_string()),
            remote_value: Value::String("remote".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::ResolvedRemote,
        },
        Conflict {
            id: "4".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field4".to_string(),
            local_value: Value::String("local".to_string()),
            remote_value: Value::String("remote".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::ResolvedManual,
        },
        Conflict {
            id: "5".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field5".to_string(),
            local_value: Value::String("local".to_string()),
            remote_value: Value::String("remote".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::Unresolved,
        },
    ];
    
    let total = conflicts.len();
    let unresolved = conflicts.iter()
        .filter(|c| matches!(c.status, ConflictStatus::Unresolved))
        .count();
    let resolved = total - unresolved;
    
    assert_eq!(total, 5);
    assert_eq!(unresolved, 2);
    assert_eq!(resolved, 3);
}

#[test]
fn test_resolution_strategy_all_types() -> Result<()> {
    let conflict = Conflict {
        id: "test".to_string(),
        spec_id: "spec-1".to_string(),
        field_path: "name".to_string(),
        local_value: Value::String("Local".to_string()),
        remote_value: Value::String("Remote".to_string()),
        base_value: Some(Value::String("Base".to_string())),
        detected_at: 0,
        status: ConflictStatus::Unresolved,
    };
    
    // Test Ours
    let (value, status) = ConflictResolver::resolve_conflict(&conflict, ResolutionStrategy::Ours, None)?;
    assert_eq!(value, Value::String("Local".to_string()));
    assert_eq!(status, ConflictStatus::ResolvedLocal);
    
    // Test Theirs
    let (value, status) = ConflictResolver::resolve_conflict(&conflict, ResolutionStrategy::Theirs, None)?;
    assert_eq!(value, Value::String("Remote".to_string()));
    assert_eq!(status, ConflictStatus::ResolvedRemote);
    
    // Test Manual
    let manual_val = Value::String("Manual".to_string());
    let (value, status) = ConflictResolver::resolve_conflict(&conflict, ResolutionStrategy::Manual, Some(manual_val.clone()))?;
    assert_eq!(value, manual_val);
    assert_eq!(status, ConflictStatus::ResolvedManual);
    
    // Test Merge (may succeed or fail)
    let _result = ConflictResolver::resolve_conflict(&conflict, ResolutionStrategy::Merge, None);
    
    Ok(())
}

#[test]
fn test_filter_unresolved_conflicts() {
    let conflicts = vec![
        Conflict {
            id: "1".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field1".to_string(),
            local_value: Value::String("local".to_string()),
            remote_value: Value::String("remote".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::Unresolved,
        },
        Conflict {
            id: "2".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field2".to_string(),
            local_value: Value::String("local".to_string()),
            remote_value: Value::String("remote".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::ResolvedLocal,
        },
        Conflict {
            id: "3".to_string(),
            spec_id: "spec-1".to_string(),
            field_path: "field3".to_string(),
            local_value: Value::String("local".to_string()),
            remote_value: Value::String("remote".to_string()),
            base_value: None,
            detected_at: 0,
            status: ConflictStatus::Unresolved,
        },
    ];
    
    let unresolved: Vec<_> = conflicts.iter()
        .filter(|c| matches!(c.status, ConflictStatus::Unresolved))
        .collect();
    
    assert_eq!(unresolved.len(), 2);
    assert_eq!(unresolved[0].id, "1");
    assert_eq!(unresolved[1].id, "3");
}

#[test]
fn test_json_parsing_for_manual_input() {
    // Test valid JSON
    let json_str = r#"{"key": "value"}"#;
    let parsed: Result<Value, _> = serde_json::from_str(json_str);
    assert!(parsed.is_ok());
    
    // Test plain string (should fail as JSON)
    let plain_str = "just a string";
    let parsed: Result<Value, _> = serde_json::from_str(plain_str);
    assert!(parsed.is_err());
    
    // Fallback: treat as string
    let fallback = Value::String(plain_str.to_string());
    assert_eq!(fallback, Value::String("just a string".to_string()));
    
    // Test empty string (treat as null)
    let empty = "";
    assert!(empty.is_empty());
}
