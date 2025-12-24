//! CLI commands for manifold

use anyhow::{bail, Context, Result};

use crate::collab::conflicts::ConflictResolver;
use crate::collab::reviews::ReviewManager;
use crate::collab::sync::SyncManager;
use crate::collab::{ResolutionStrategy, SyncConfig};
use crate::config::{save_config, Config, ManifoldPaths};
use crate::db::Database;
use crate::models::{Boundary, SpecData, WorkflowStage};
use crate::workflow::{WorkflowEngine, WorkflowError};

/// Initialize manifold for first-time setup
pub fn init() -> Result<()> {
    let paths = ManifoldPaths::new()?;

    if paths.is_initialized() {
        println!("Manifold is already initialized at {}", paths.root.display());
        return Ok(());
    }

    println!("Initializing manifold at {}...", paths.root.display());

    // Create directory structure
    paths.ensure_dirs()?;
    println!("  Created directory structure");

    // Create default config
    let config = Config::default();
    save_config(&config)?;
    println!("  Created config.toml");

    // Initialize database
    Database::init(&paths)?;
    println!("  Created database with FTS5 indexing");

    // Create core schema
    create_core_schema(&paths)?;
    println!("  Created core.json schema");

    println!();
    println!("Manifold initialized successfully!");
    println!();
    println!("Next steps:");
    println!("  manifold new <project-id>     Create a new spec");
    println!("  manifold list                 List all specs");
    println!("  manifold tui                  Open the TUI dashboard");

    Ok(())
}

/// Create a new spec
pub fn new_spec(project_id: &str, name: Option<&str>, boundary: Option<&str>) -> Result<String> {
    let paths = ManifoldPaths::new()?;
    ensure_initialized(&paths)?;

    let boundary = match boundary {
        Some(b) => b.parse::<Boundary>().map_err(|e| anyhow::anyhow!(e))?,
        std::option::Option::None => Boundary::Personal,
    };

    let spec_name = name.unwrap_or(project_id).to_string();
    
    // Generate spec_id
    let spec_id = crate::db::generate_spec_id(project_id);
    let spec = SpecData::new(spec_id.clone(), project_id.to_string(), spec_name, boundary);

    let db = Database::open(&paths)?;
    let id = db.insert_spec(&spec)?;

    println!("Created spec: {}", id);
    println!("  Project:  {}", project_id);
    println!("  Boundary: {}", spec.boundary);
    println!("  Stage:    {}", spec.stage);

    Ok(id)
}

/// List specs with optional filters
pub fn list(boundary: Option<&str>, stage: Option<&str>) -> Result<()> {
    let paths = ManifoldPaths::new()?;
    ensure_initialized(&paths)?;

    let boundary = match boundary {
        Some("all") | None => None,
        Some(b) => Some(b.parse::<Boundary>().map_err(|e| anyhow::anyhow!(e))?),
    };

    let stage = match stage {
        Some(s) => Some(s.parse::<WorkflowStage>().map_err(|e| anyhow::anyhow!(e))?),
        None => None,
    };

    let db = Database::open(&paths)?;
    let specs = db.list_specs(boundary.as_ref(), stage.as_ref())?;

    if specs.is_empty() {
        println!("No specs found.");
        println!("Create one with: manifold new <project-id>");
        return Ok(());
    }

    // Print header
    println!(
        "{:<30} {:<20} {:<12} {:<15}",
        "ID", "PROJECT", "BOUNDARY", "STAGE"
    );
    println!("{}", "-".repeat(77));

    for spec in specs {
        let name = spec
            .data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&spec.project);
        println!(
            "{:<30} {:<20} {:<12} {:<15}",
            truncate(&spec.id, 28),
            truncate(name, 18),
            spec.boundary,
            spec.stage
        );
    }

    Ok(())
}

/// Show a spec by ID
pub fn show(id: &str, format: OutputFormat) -> Result<()> {
    let paths = ManifoldPaths::new()?;
    ensure_initialized(&paths)?;

    let db = Database::open(&paths)?;
    let spec = db
        .get_spec(id)?
        .with_context(|| format!("Spec not found: {}", id))?;

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&spec.data)?;
            println!("{}", json);
        }
        OutputFormat::Summary => {
            print_spec_summary(&spec);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Json,
    Summary,
}

fn print_spec_summary(spec: &crate::models::SpecRow) {
    let data = &spec.data;
    
    println!("Spec: {}", spec.id);
    println!("{}", "=".repeat(50));
    
    if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
        println!("Name:     {}", name);
    }
    println!("Project:  {}", spec.project);
    println!("Boundary: {}", spec.boundary);
    println!("Stage:    {}", spec.stage);
    
    // Show requirements summary (LLM-native format)
    if let Some(reqs) = data.get("requirements").and_then(|v| v.as_array()) {
        println!();
        println!("Requirements: {}", reqs.len());
        for (idx, req) in reqs.iter().take(5).enumerate() {
            if let Some(title) = req.get("title").and_then(|v| v.as_str()) {
                let id = req.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                println!("  {}: {}", id, title);
            }
            if idx == 4 && reqs.len() > 5 {
                println!("  ... and {} more", reqs.len() - 5);
            }
        }
    }
    
    // Show tasks summary
    if let Some(tasks) = data.get("tasks").and_then(|v| v.as_array()) {
        println!();
        println!("Tasks: {}", tasks.len());
        let completed = tasks.iter().filter(|t| {
            t.get("status").and_then(|v| v.as_str()) == Some("completed")
        }).count();
        println!("  Completed: {}/{}", completed, tasks.len());
    }
    
    // Timestamps
    println!();
    println!(
        "Created:  {}",
        format_timestamp(spec.created_at)
    );
    println!(
        "Updated:  {}",
        format_timestamp(spec.updated_at)
    );
}

fn format_timestamp(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

fn ensure_initialized(paths: &ManifoldPaths) -> Result<()> {
    if !paths.is_initialized() {
        bail!(
            "Manifold not initialized. Run `manifold init` first."
        );
    }
    Ok(())
}

fn create_core_schema(paths: &ManifoldPaths) -> Result<()> {
    let schema = include_str!("../../schemas/core.json");
    std::fs::write(paths.schemas.join("core.json"), schema)
        .context("Failed to write core.json schema")?;
    Ok(())
}

/// Validate a spec against the schema
pub fn validate(id: &str, strict: bool) -> Result<()> {
    let paths = ManifoldPaths::new()?;
    ensure_initialized(&paths)?;

    let db = Database::open(&paths)?;
    let spec_row = db
        .get_spec(id)?
        .with_context(|| format!("Spec not found: {}", id))?;

    // Parse spec data
    let spec: SpecData = serde_json::from_value(spec_row.data)
        .context("Failed to parse spec data")?;

    println!("Validating spec: {}", id);
    println!();

    // Schema validation
    print!("Schema validation... ");
    match crate::validation::validate_spec(&spec) {
        Ok(_) => println!("✓ passed"),
        Err(e) => {
            println!("✗ failed");
            println!("{}", e);
            bail!("Schema validation failed");
        }
    }

    // Linting
    print!("Linting... ");
    let warnings = crate::validation::lint_spec(&spec);
    if warnings.is_empty() {
        println!("✓ no warnings");
    } else {
        println!("⚠ {} warning(s)", warnings.len());
        for warning in &warnings {
            println!("  ⚠ {}", warning);
        }
        if strict {
            bail!("Validation failed in strict mode due to warnings");
        }
    }

    println!();
    println!("Validation complete!");
    Ok(())
}

/// Join (merge) a spec into another boundary
pub fn join(source_id: &str, target_boundary: &str, dedup: bool) -> Result<()> {
    let paths = ManifoldPaths::new()?;
    ensure_initialized(&paths)?;

    let target_boundary = target_boundary
        .parse::<Boundary>()
        .map_err(|e| anyhow::anyhow!(e))?;

    let db = Database::open(&paths)?;
    
    // Get source spec
    let source_row = db
        .get_spec(source_id)?
        .with_context(|| format!("Source spec not found: {}", source_id))?;

    let mut source_spec: SpecData = serde_json::from_value(source_row.data)
        .context("Failed to parse source spec")?;

    println!("Joining spec: {} → {}", source_id, target_boundary);
    println!("  Source boundary: {}", source_spec.boundary);
    println!();

    // Check if already in target boundary
    if source_spec.boundary == target_boundary {
        bail!("Spec is already in the target boundary");
    }

    // Deduplication: check for existing specs in target boundary with same project
    if dedup {
        print!("Checking for duplicates... ");
        let existing = db.list_specs(Some(&target_boundary), std::option::Option::None)?;
        let duplicates: Vec<_> = existing
            .iter()
            .filter(|s| s.project == source_spec.project)
            .collect();

        if !duplicates.is_empty() {
            println!("found {} duplicate(s)", duplicates.len());
            for dup in &duplicates {
                println!("  - {}: {}", dup.id, dup.data.get("name").and_then(|v| v.as_str()).unwrap_or("?"));
            }
            
            // For now, just warn - in future we could implement merging logic
            println!();
            println!("⚠ Warning: Duplicates exist in target boundary");
            println!("  Creating a separate spec anyway...");
        } else {
            println!("✓ none");
        }
    }

    // Create new spec in target boundary
    let new_spec_id = crate::db::generate_spec_id(&source_spec.project);
    source_spec.spec_id = new_spec_id.clone();
    let old_boundary = source_spec.boundary.clone();
    source_spec.boundary = target_boundary;
    
    // Update history
    source_spec.history.updated_at = chrono::Utc::now().timestamp();
    source_spec.history.patches.push(crate::models::PatchEntry {
        timestamp: chrono::Utc::now().timestamp(),
        actor: "user".to_string(),
        op: "join".to_string(),
        path: "/boundary".to_string(),
        summary: format!("Joined from {} to {}", old_boundary, source_spec.boundary),
    });

    db.insert_spec(&source_spec)?;

    println!();
    println!("✓ Created new spec in target boundary: {}", new_spec_id);
    println!();
    println!("Note: Original spec in {} boundary is unchanged", source_row.boundary);

    Ok(())
}

/// Workflow operations: advance stage or show history
pub fn workflow(id: &str, operation: WorkflowOperation) -> Result<()> {
    let paths = ManifoldPaths::new()?;
    ensure_initialized(&paths)?;

    let db = Database::open(&paths)?;
    let spec_row = db
        .get_spec(id)?
        .with_context(|| format!("Spec not found: {}", id))?;

    let mut spec: SpecData = serde_json::from_value(spec_row.data)
        .context("Failed to parse spec data")?;

    match operation {
        WorkflowOperation::Advance { target_stage } => {
            println!("Current stage: {}", spec.stage);
            
            let target_stage = match target_stage {
                Some(stage_str) => {
                    stage_str.parse::<WorkflowStage>().map_err(|e| anyhow::anyhow!(e))?
                }
                None => {
                    // Auto-advance to next stage
                    match WorkflowEngine::can_advance(&spec) {
                        Ok(next) => next,
                        Err(e) => {
                            println!("✗ Cannot advance: {}", e);
                            return Err(e.into());
                        }
                    }
                }
            };
            
            println!("Target stage:  {}", target_stage);
            println!();
            
            // Validate and execute transition
            match WorkflowEngine::advance_stage(&spec, target_stage) {
                Ok(transition) => {
                    println!("✓ Validation passed");
                    
                    // Update spec
                    let old_stage = spec.stage.clone();
                    if !spec.stages_completed.contains(&old_stage) {
                        spec.stages_completed.push(old_stage.clone());
                    }
                    spec.stage = transition.to.clone();
                    spec.history.updated_at = chrono::Utc::now().timestamp();
                    
                    // Log event
                    let timestamp = spec.history.updated_at;
                    db.log_workflow_event(
                        &spec.spec_id,
                        &transition.to.to_string(),
                        &transition.event.as_string(),
                        "user",
                        timestamp,
                        Some(&format!("Advanced from {} to {}", transition.from, transition.to)),
                    )?;
                    
                    // Update database
                    db.update_spec(&spec)?;
                    
                    println!("✓ Advanced to stage: {}", spec.stage);
                    println!();
                    println!("Stages completed: {:?}", spec.stages_completed);
                }
                Err(e) => {
                    println!("✗ Transition failed: {}", e);
                    
                    // Log failed validation
                    if let WorkflowError::ValidationFailed(msg) = &e {
                        db.log_workflow_event(
                            &spec.spec_id,
                            &spec.stage.to_string(),
                            &format!("validation_failed:{}", msg),
                            "user",
                            chrono::Utc::now().timestamp(),
                            Some(&e.to_string()),
                        )?;
                    }
                    
                    return Err(e.into());
                }
            }
        }
        
        WorkflowOperation::History => {
            println!("Workflow history for: {}", id);
            println!("{}", "=".repeat(80));
            
            let events = db.get_workflow_events(id)?;
            
            if events.is_empty() {
                println!("No workflow events recorded");
            } else {
                for event in events {
                    println!(
                        "{} | {} | {} | {}",
                        format_timestamp(event.timestamp),
                        event.stage,
                        event.event,
                        event.actor
                    );
                    if let Some(details) = event.details {
                        println!("  {}", details);
                    }
                }
            }
        }
        
        WorkflowOperation::Status => {
            println!("Workflow status for: {}", id);
            println!("{}", "=".repeat(50));
            println!("Current stage: {}", spec.stage);
            println!("Stages completed: {:?}", spec.stages_completed);
            println!();
            
            match WorkflowEngine::can_advance(&spec) {
                Ok(next_stage) => {
                    println!("✓ Can advance to: {}", next_stage);
                }
                Err(e) => {
                    println!("✗ Cannot advance: {}", e);
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub enum WorkflowOperation {
    Advance { target_stage: Option<String> },
    History,
    Status,
}

// Collaboration commands

/// Sync command handler
pub async fn sync_command(operation: crate::SyncOperation) -> Result<()> {
    use crate::SyncOperation;
    
    let paths = ManifoldPaths::new()?;
    ensure_initialized(&paths)?;

    match operation {
        SyncOperation::Init { repo, remote } => {
            let repo_path = std::path::PathBuf::from(&repo);
            let mut config = SyncConfig::new(repo_path);
            
            if let Some(url) = remote {
                config.remote_url = Some(url.clone());
            }

            let manager = SyncManager::new(config.clone());
            manager.init()?;

            if let Some(url) = &config.remote_url {
                manager.add_remote("origin", url)?;
            }

            println!("✓ Sync repository initialized");
            println!("  Path: {}", repo);
            if let Some(url) = config.remote_url {
                println!("  Remote: {}", url);
            }
        }

        SyncOperation::Push { id, message, remote, branch } => {
            // Load sync config (in real impl, this would be stored)
            let sync_dir = paths.root.join("sync");
            let config = SyncConfig::new(sync_dir);
            let manager = SyncManager::new(config);

            let db = Database::open(&paths)?;

            if id == "all" {
                // Push all specs
                let specs = db.list_specs(None, None)?;
                let mut pushed_count = 0;

                for spec_row in specs {
                    let spec: SpecData = serde_json::from_value(spec_row.data)?;
                    manager.export_spec(&spec)?;
                    pushed_count += 1;
                }

                let commit_msg = message.unwrap_or_else(|| format!("Update {} specs", pushed_count));
                let files: Vec<_> = (0..pushed_count).map(|_| std::path::PathBuf::new()).collect();
                
                if let Ok(hash) = manager.commit(&commit_msg, &[]) {
                    if hash != "no-changes" {
                        manager.push(&remote, &branch)?;
                        println!("✓ Pushed {} specs (commit: {})", pushed_count, &hash[..8]);
                    } else {
                        println!("✓ No changes to push");
                    }
                }
            } else {
                // Push single spec
                let spec_row = db.get_spec(&id)?.context("Spec not found")?;
                let spec: SpecData = serde_json::from_value(spec_row.data)?;
                
                let file_path = manager.export_spec(&spec)?;
                let commit_msg = message.unwrap_or_else(|| format!("Update spec: {}", id));
                
                let hash = manager.commit(&commit_msg, &[file_path])?;
                if hash != "no-changes" {
                    manager.push(&remote, &branch)?;
                    println!("✓ Pushed spec {} (commit: {})", id, &hash[..8]);
                } else {
                    println!("✓ No changes to push");
                }
            }
        }

        SyncOperation::Pull { id, remote, branch } => {
            let sync_dir = paths.root.join("sync");
            let config = SyncConfig::new(sync_dir);
            let manager = SyncManager::new(config);

            manager.pull(&remote, &branch)?;

            let db = Database::open(&paths)?;

            if id == "all" {
                // Pull all specs
                let spec_ids = manager.list_specs()?;
                let mut pulled_count = 0;

                for spec_id in &spec_ids {
                    match manager.import_spec(&spec_id) {
                        Ok(remote_spec) => {
                            // Check for conflicts
                            if let Ok(Some(local_row)) = db.get_spec(&spec_id) {
                                let local_spec: SpecData = serde_json::from_value(local_row.data)?;
                                
                                let conflicts = ConflictResolver::detect_conflicts(
                                    &local_spec,
                                    &remote_spec,
                                    None,
                                )?;

                                if !conflicts.is_empty() {
                                    println!("⚠ Conflict detected in spec: {}", spec_id);
                                    for conflict in &conflicts {
                                        db.save_conflict(conflict)?;
                                    }
                                    println!("  Run 'manifold conflicts list' to review");
                                } else {
                                    db.update_spec(&remote_spec)?;
                                    pulled_count += 1;
                                }
                            } else {
                                db.insert_spec(&remote_spec)?;
                                pulled_count += 1;
                            }
                        }
                        Err(e) => {
                            eprintln!("⚠ Failed to import {}: {}", spec_id, e);
                        }
                    }
                }

                println!("✓ Pulled {} specs", pulled_count);
            } else {
                // Pull single spec
                let remote_spec = manager.import_spec(&id)?;
                
                if let Ok(Some(local_row)) = db.get_spec(&id) {
                    let local_spec: SpecData = serde_json::from_value(local_row.data)?;
                    
                    let conflicts = ConflictResolver::detect_conflicts(
                        &local_spec,
                        &remote_spec,
                        None,
                    )?;

                    if !conflicts.is_empty() {
                        println!("⚠ Conflict detected in spec: {}", id);
                        for conflict in &conflicts {
                            db.save_conflict(conflict)?;
                            println!("  {}", ConflictResolver::format_conflict(conflict));
                        }
                        println!();
                        println!("Run 'manifold conflicts resolve <conflict-id>' to resolve");
                    } else {
                        db.update_spec(&remote_spec)?;
                        println!("✓ Pulled spec: {}", id);
                    }
                } else {
                    db.insert_spec(&remote_spec)?;
                    println!("✓ Pulled new spec: {}", id);
                }
            }
        }

        SyncOperation::Status => {
            let sync_dir = paths.root.join("sync");
            let config = SyncConfig::new(sync_dir);
            let manager = SyncManager::new(config);

            println!("Sync status:");
            println!("{}", "=".repeat(60));

            let modified_files = manager.status()?;
            
            if modified_files.is_empty() {
                println!("✓ No local modifications");
            } else {
                println!("Modified specs:");
                for file in modified_files {
                    println!("  {}", file);
                }
            }
        }
    }

    Ok(())
}

/// Review command handler
pub fn review_command(operation: crate::ReviewOperation) -> Result<()> {
    use crate::ReviewOperation;
    
    let paths = ManifoldPaths::new()?;
    ensure_initialized(&paths)?;
    let db = Database::open(&paths)?;

    // Get current user (in real impl, this would come from config)
    let current_user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());

    match operation {
        ReviewOperation::Request { spec_id, reviewer } => {
            // Check spec exists
            db.get_spec(&spec_id)?.context("Spec not found")?;

            let review = ReviewManager::create_review(
                spec_id.clone(),
                current_user.clone(),
                reviewer.clone(),
            );

            db.save_review(&review)?;

            println!("✓ Review requested");
            println!("  Review ID: {}", review.id);
            println!("  Spec: {}", spec_id);
            println!("  Reviewer: {}", reviewer);
        }

        ReviewOperation::Approve { review_id, comment } => {
            let mut review = db.get_review(&review_id)?.context("Review not found")?;

            ReviewManager::approve(&mut review, &current_user, comment)?;
            db.save_review(&review)?;

            println!("✓ Review approved");
            println!("{}", ReviewManager::format_review(&review));
        }

        ReviewOperation::Reject { review_id, comment } => {
            let mut review = db.get_review(&review_id)?.context("Review not found")?;

            ReviewManager::reject(&mut review, &current_user, comment)?;
            db.save_review(&review)?;

            println!("✓ Review rejected");
            println!("{}", ReviewManager::format_review(&review));
        }

        ReviewOperation::List { spec_id, status } => {
            let reviews = if let Some(spec_id) = spec_id {
                db.get_reviews(&spec_id)?
            } else {
                // Would need to add a method to get all reviews
                Vec::new()
            };

            let filtered: Vec<_> = if let Some(status_filter) = status {
                reviews.into_iter().filter(|r| r.status.to_string() == status_filter).collect()
            } else {
                reviews
            };

            if filtered.is_empty() {
                println!("No reviews found");
            } else {
                println!("Reviews:");
                println!("{}", "=".repeat(60));
                for review in &filtered {
                    println!("{}", ReviewManager::format_review(review));
                    println!();
                }

                let stats = ReviewManager::get_stats(&filtered);
                println!("{}", stats.format());
            }
        }
    }

    Ok(())
}

/// Conflict command handler
pub fn conflict_command(operation: crate::ConflictOperation) -> Result<()> {
    use crate::ConflictOperation;
    
    let paths = ManifoldPaths::new()?;
    ensure_initialized(&paths)?;
    let db = Database::open(&paths)?;

    match operation {
        ConflictOperation::List { spec_id } => {
            let conflicts = if let Some(spec_id) = spec_id {
                db.get_conflicts(&spec_id)?
            } else {
                // Would need method to get all conflicts
                Vec::new()
            };

            if conflicts.is_empty() {
                println!("✓ No conflicts");
            } else {
                println!("Conflicts:");
                println!("{}", "=".repeat(60));
                for conflict in conflicts {
                    println!("ID: {}", conflict.id);
                    println!("{}", ConflictResolver::format_conflict(&conflict));
                    println!();
                }
            }
        }

        ConflictOperation::Resolve { conflict_id, strategy } => {
            let conflicts = db.get_conflicts("")?; // Get all conflicts
            let conflict = conflicts.iter()
                .find(|c| c.id == conflict_id)
                .context("Conflict not found")?;

            let resolution_strategy = match strategy.as_str() {
                "ours" => ResolutionStrategy::Ours,
                "theirs" => ResolutionStrategy::Theirs,
                "manual" => ResolutionStrategy::Manual,
                "merge" => ResolutionStrategy::Merge,
                _ => bail!("Invalid strategy. Use: ours, theirs, manual, or merge"),
            };

            println!("Resolving conflict:");
            println!("{}", ConflictResolver::format_conflict(conflict));
            println!();

            let (resolved_value, status) = ConflictResolver::resolve_conflict(
                conflict,
                resolution_strategy,
                None,
            )?;

            // Update conflict status
            db.update_conflict_status(&conflict_id, &status)?;

            // Apply resolution to spec
            let spec_row = db.get_spec(&conflict.spec_id)?.context("Spec not found")?;
            let mut spec: SpecData = serde_json::from_value(spec_row.data)?;

            ConflictResolver::apply_resolutions(&mut spec, &[(conflict.field_path.clone(), resolved_value)])?;
            db.update_spec(&spec)?;

            println!("✓ Conflict resolved with strategy: {}", strategy);
            println!("  Status: {}", status);
        }
    }

    Ok(())
}
