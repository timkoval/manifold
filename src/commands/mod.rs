//! CLI commands for manifold

use anyhow::{bail, Context, Result};

use crate::config::{save_config, Config, ManifoldPaths};
use crate::db::Database;
use crate::models::{Boundary, SpecData, WorkflowStage};

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
