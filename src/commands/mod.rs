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
