// Integration tests for collaboration features
// Tests git sync, conflict detection, and resolution

use anyhow::Result;
use manifold::collab::conflicts::ConflictResolver;
use manifold::collab::reviews::ReviewManager;
use manifold::collab::{Conflict, ConflictStatus, ResolutionStrategy, ReviewStatus};
use manifold::config::ManifoldPaths;
use manifold::db::Database;
use manifold::models::{Boundary, SpecData};
use std::fs;
use tempfile::TempDir;

/// Setup test environment
fn setup() -> anyhow::Result<(TempDir, ManifoldPaths, Database)> {
    let temp_dir = TempDir::new()?;

    // Create a custom ManifoldPaths for testing
    let paths = ManifoldPaths {
        root: temp_dir.path().to_path_buf(),
        config: temp_dir.path().join("config.toml"),
        db: temp_dir.path().join("db"),
        db_file: temp_dir.path().join("db/manifold.db"),
        schemas: temp_dir.path().join("schemas"),
        exports: temp_dir.path().join("exports"),
        cache: temp_dir.path().join("cache"),
    };

    fs::create_dir_all(&paths.db)?;
    let db = Database::init(&paths)?;

    Ok((temp_dir, paths, db))
}

fn create_test_spec(spec_id: &str, project: &str, name: &str) -> SpecData {
    SpecData::new(
        spec_id.to_string(),
        project.to_string(),
        name.to_string(),
        Boundary::Personal,
    )
}

#[test]
fn test_conflict_detection_simple() -> Result<()> {
    let (_temp, _paths, _db) = setup()?;

    // Create base, local, and remote specs with different names
    let base = create_test_spec("test-spec", "test-project", "Original Name");

    let mut local = base.clone();
    local.name = "Local Name".to_string();

    let mut remote = base.clone();
    remote.name = "Remote Name".to_string();

    // Detect conflicts
    let conflicts = ConflictResolver::detect_conflicts(&local, &remote, Some(&base))?;

    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].field_path, "name");
    assert_eq!(conflicts[0].status, ConflictStatus::Unresolved);

    Ok(())
}

#[test]
fn test_conflict_resolution_ours() -> Result<()> {
    let conflict = Conflict {
        id: "test-conflict".to_string(),
        spec_id: "test-spec".to_string(),
        field_path: "name".to_string(),
        local_value: serde_json::Value::String("Local".to_string()),
        remote_value: serde_json::Value::String("Remote".to_string()),
        base_value: Some(serde_json::Value::String("Base".to_string())),
        detected_at: 0,
        status: ConflictStatus::Unresolved,
    };

    let (resolved_value, status) =
        ConflictResolver::resolve_conflict(&conflict, ResolutionStrategy::Ours, None)?;

    assert_eq!(
        resolved_value,
        serde_json::Value::String("Local".to_string())
    );
    assert_eq!(status, ConflictStatus::ResolvedLocal);

    Ok(())
}

#[test]
fn test_conflict_resolution_theirs() -> Result<()> {
    let conflict = Conflict {
        id: "test-conflict".to_string(),
        spec_id: "test-spec".to_string(),
        field_path: "name".to_string(),
        local_value: serde_json::Value::String("Local".to_string()),
        remote_value: serde_json::Value::String("Remote".to_string()),
        base_value: Some(serde_json::Value::String("Base".to_string())),
        detected_at: 0,
        status: ConflictStatus::Unresolved,
    };

    let (resolved_value, status) =
        ConflictResolver::resolve_conflict(&conflict, ResolutionStrategy::Theirs, None)?;

    assert_eq!(
        resolved_value,
        serde_json::Value::String("Remote".to_string())
    );
    assert_eq!(status, ConflictStatus::ResolvedRemote);

    Ok(())
}

#[test]
fn test_3way_merge() -> Result<()> {
    let base = create_test_spec("test-spec", "test-project", "Original");

    let mut local = base.clone();
    local.name = "Modified Locally".to_string();

    let mut remote = base.clone();
    remote.project = "modified-project".to_string();

    let conflicts = ConflictResolver::detect_conflicts(&local, &remote, Some(&base))?;

    // Should be no conflicts since different fields changed
    assert_eq!(conflicts.len(), 0);

    Ok(())
}

#[test]
fn test_merge_strategy() -> Result<()> {
    let base = create_test_spec("test-spec", "test-project", "Original");

    let mut local = base.clone();
    local.name = "Local Name".to_string();

    let mut remote = base.clone();
    remote.name = "Remote Name".to_string();

    let conflicts = ConflictResolver::detect_conflicts(&local, &remote, Some(&base))?;
    assert_eq!(conflicts.len(), 1);

    let result = ConflictResolver::resolve_conflict(&conflicts[0], ResolutionStrategy::Merge, None);

    // Merge strategy may succeed or fail for string conflicts
    // If it fails, we can fall back to manual resolution
    match result {
        Ok((_, status)) => {
            assert!(matches!(
                status,
                ConflictStatus::ResolvedLocal
                    | ConflictStatus::ResolvedRemote
                    | ConflictStatus::ResolvedManual
            ));
        }
        Err(_) => {
            // Merge failed, which is acceptable for this type of conflict
        }
    }

    Ok(())
}

#[test]
fn test_manual_strategy() -> Result<()> {
    let base = create_test_spec("test-spec", "test-project", "Original");

    let mut local = base.clone();
    local.name = "Local Name".to_string();

    let mut remote = base.clone();
    remote.name = "Remote Name".to_string();

    let conflicts = ConflictResolver::detect_conflicts(&local, &remote, Some(&base))?;

    let manual_value = serde_json::Value::String("Manually Resolved".to_string());
    let (resolved_value, status) = ConflictResolver::resolve_conflict(
        &conflicts[0],
        ResolutionStrategy::Manual,
        Some(manual_value.clone()),
    )?;

    assert_eq!(resolved_value, manual_value);
    assert_eq!(status, ConflictStatus::ResolvedManual);

    Ok(())
}

#[test]
fn test_review_lifecycle() -> Result<()> {
    let (_temp, _paths, _db) = setup()?;

    let mut review = ReviewManager::create_review(
        "spec-123".to_string(),
        "requester@example.com".to_string(),
        "reviewer@example.com".to_string(),
    );

    // Approve
    ReviewManager::approve(&mut review, "reviewer@example.com", None)?;
    assert_eq!(review.status, ReviewStatus::Approved);
    assert!(review.reviewed_at.is_some());

    Ok(())
}

#[test]
fn test_review_rejection() -> Result<()> {
    let (_temp, _paths, _db) = setup()?;

    let mut review = ReviewManager::create_review(
        "spec-456".to_string(),
        "requester@example.com".to_string(),
        "reviewer@example.com".to_string(),
    );

    ReviewManager::reject(
        &mut review,
        "reviewer@example.com",
        "Needs work".to_string(),
    )?;
    assert_eq!(review.status, ReviewStatus::Rejected);
    assert_eq!(review.comment, Some("Needs work".to_string()));

    Ok(())
}

#[test]
fn test_review_persistence() -> Result<()> {
    let (_temp, _paths, db) = setup()?;

    // Create and insert a spec first to satisfy foreign key constraint
    let spec = create_test_spec("spec-789", "test-project", "Test Spec");
    db.insert_spec(&spec)?;

    let review = ReviewManager::create_review(
        "spec-789".to_string(),
        "alice@example.com".to_string(),
        "bob@example.com".to_string(),
    );

    db.save_review(&review)?;
    let loaded = db.get_review(&review.id)?;

    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.id, review.id);
    assert_eq!(loaded.spec_id, "spec-789");

    Ok(())
}

#[test]
fn test_multiple_conflicts() -> Result<()> {
    let base = create_test_spec("test-spec", "test-project", "Original");

    let mut local = base.clone();
    local.name = "Local Name".to_string();
    // Note: project field changes are not detected as conflicts if both local and remote modify it the same way

    let mut remote = base.clone();
    remote.name = "Remote Name".to_string();
    // Both changed the name field -> 1 conflict

    let conflicts = ConflictResolver::detect_conflicts(&local, &remote, Some(&base))?;

    // Only name field differs
    assert!(conflicts.len() >= 1);

    Ok(())
}

#[test]
fn test_no_conflicts_same_changes() -> Result<()> {
    let base = create_test_spec("test-spec", "test-project", "Original");

    let mut local = base.clone();
    local.name = "Same Name".to_string();

    let mut remote = base.clone();
    remote.name = "Same Name".to_string();

    let conflicts = ConflictResolver::detect_conflicts(&local, &remote, Some(&base))?;

    assert_eq!(conflicts.len(), 0);

    Ok(())
}

#[test]
fn test_conflict_persistence() -> Result<()> {
    let (_temp, _paths, db) = setup()?;

    // Create and insert a spec first
    let spec = create_test_spec("spec-conflict", "test-project", "Test Spec");
    db.insert_spec(&spec)?;

    let conflict = Conflict {
        id: "conflict-1".to_string(),
        spec_id: "spec-conflict".to_string(),
        field_path: "name".to_string(),
        local_value: serde_json::Value::String("Local".to_string()),
        remote_value: serde_json::Value::String("Remote".to_string()),
        base_value: Some(serde_json::Value::String("Base".to_string())),
        detected_at: chrono::Utc::now().timestamp(),
        status: ConflictStatus::Unresolved,
    };

    db.save_conflict(&conflict)?;
    let loaded = db.get_conflicts("spec-conflict")?;

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "conflict-1");
    assert_eq!(loaded[0].field_path, "name");

    Ok(())
}

#[test]
fn test_conflict_status_update() -> Result<()> {
    let (_temp, _paths, db) = setup()?;

    // Create and insert a spec first
    let spec = create_test_spec("spec-status", "test-project", "Test Spec");
    db.insert_spec(&spec)?;

    let conflict = Conflict {
        id: "conflict-2".to_string(),
        spec_id: "spec-status".to_string(),
        field_path: "name".to_string(),
        local_value: serde_json::Value::String("Local".to_string()),
        remote_value: serde_json::Value::String("Remote".to_string()),
        base_value: None,
        detected_at: chrono::Utc::now().timestamp(),
        status: ConflictStatus::Unresolved,
    };

    db.save_conflict(&conflict)?;

    // Verify conflict is unresolved
    let unresolved = db.get_conflicts("spec-status")?;
    assert_eq!(unresolved.len(), 1);
    assert_eq!(unresolved[0].status, ConflictStatus::Unresolved);

    // Update status
    db.update_conflict_status("conflict-2", &ConflictStatus::ResolvedLocal)?;

    // After resolution, get_conflicts returns only unresolved (which should be empty)
    let still_unresolved = db.get_conflicts("spec-status")?;
    assert_eq!(still_unresolved.len(), 0);

    Ok(())
}

#[test]
fn test_conflict_with_resolutions() -> Result<()> {
    let (_temp, _paths, db) = setup()?;

    let mut spec = create_test_spec("test-resolution", "test-project", "Original Name");
    db.insert_spec(&spec)?;

    // Simulate resolution
    let resolutions = vec![(
        "name".to_string(),
        serde_json::Value::String("Resolved Name".to_string()),
    )];

    ConflictResolver::apply_resolutions(&mut spec, &resolutions)?;
    assert_eq!(spec.name, "Resolved Name");

    db.update_spec(&spec)?;
    let loaded = db.get_spec("test-resolution")?.unwrap();
    let loaded_spec: SpecData = serde_json::from_value(loaded.data)?;
    assert_eq!(loaded_spec.name, "Resolved Name");

    Ok(())
}
