//! manifold - The Global Spec Manifold
//!
//! A local-first, MCP-native, JSON-canonical specification engine

mod commands;
mod config;
mod db;
mod models;
mod validation;
mod mcp;
mod workflow;
mod llm;
mod tui;
mod export;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "manifold")]
#[command(author, version, about = "The Global Spec Manifold - A local-first, MCP-native specification engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize manifold (first-time setup)
    Init,

    /// Create a new spec
    New {
        /// Project identifier (e.g., "auric-raptor")
        project_id: String,

        /// Human-readable name for the spec
        #[arg(short, long)]
        name: Option<String>,

        /// Boundary: personal, work, or company
        #[arg(short, long, default_value = "personal")]
        boundary: String,
    },

    /// List all specs
    List {
        /// Filter by boundary (personal, work, company, or "all")
        #[arg(short, long, default_value = "all")]
        boundary: String,

        /// Filter by workflow stage
        #[arg(short, long)]
        stage: Option<String>,
    },

    /// Show a spec by ID
    Show {
        /// Spec ID
        id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate a spec against the schema
    Validate {
        /// Spec ID
        id: String,

        /// Strict mode (fail on warnings)
        #[arg(long)]
        strict: bool,
    },

    /// Join (merge) a spec into another boundary
    Join {
        /// Source spec ID
        source_id: String,

        /// Target boundary (personal, work, company)
        target_boundary: String,

        /// Skip deduplication
        #[arg(long)]
        no_dedup: bool,
    },

    /// Start the MCP server (JSON-RPC 2.0 over stdio)
    Serve,

    /// Workflow operations (advance stage, show history)
    Workflow {
        /// Spec ID
        id: String,

        /// Operation: advance, history, or status
        #[arg(short, long, default_value = "status")]
        operation: String,

        /// Target stage for advance operation (optional, auto-advances if not specified)
        #[arg(long)]
        stage: Option<String>,
    },

    /// Interactive LLM editing session
    Edit {
        /// Spec ID to edit
        id: String,
    },

    /// Launch TUI dashboard
    Tui,

    /// Export spec(s) to Markdown
    Export {
        /// Spec ID (or 'all' for all specs)
        id: String,

        /// Output file path
        #[arg(short, long)]
        output: String,

        /// Use table formatting
        #[arg(long)]
        tables: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            commands::init()?;
        }
        Commands::New {
            project_id,
            name,
            boundary,
        } => {
            commands::new_spec(&project_id, name.as_deref(), Some(&boundary))?;
        }
        Commands::List { boundary, stage } => {
            commands::list(Some(&boundary), stage.as_deref())?;
        }
        Commands::Show { id, json } => {
            let format = if json {
                commands::OutputFormat::Json
            } else {
                commands::OutputFormat::Summary
            };
            commands::show(&id, format)?;
        }
        Commands::Validate { id, strict } => {
            commands::validate(&id, strict)?;
        }
        Commands::Join {
            source_id,
            target_boundary,
            no_dedup,
        } => {
            commands::join(&source_id, &target_boundary, !no_dedup)?;
        }
        Commands::Serve => {
            let mut server = mcp::McpServer::new()?;
            server.run().await?;
        }
        Commands::Workflow { id, operation, stage } => {
            let op = match operation.as_str() {
                "advance" => commands::WorkflowOperation::Advance {
                    target_stage: stage,
                },
                "history" => commands::WorkflowOperation::History,
                "status" => commands::WorkflowOperation::Status,
                _ => {
                    eprintln!("Invalid operation: {}. Use: advance, history, or status", operation);
                    std::process::exit(1);
                }
            };
            commands::workflow(&id, op)?;
        }
        Commands::Edit { id } => {
            let paths = config::ManifoldPaths::new()?;
            let mut session = llm::LlmSession::new(id, &paths)?;
            session.run().await?;
        }
        Commands::Tui => {
            let paths = config::ManifoldPaths::new()?;
            let mut app = tui::TuiApp::new(&paths)?;
            app.run()?;
        }
        Commands::Export { id, output, tables } => {
            let paths = config::ManifoldPaths::new()?;
            let db = db::Database::open(&paths)?;
            
            let output_path = std::path::Path::new(&output);
            
            if id == "all" {
                // Export all specs
                let spec_rows = db.list_specs(None, None)?;
                let specs: Vec<models::SpecData> = spec_rows
                    .into_iter()
                    .filter_map(|row| serde_json::from_value(row.data).ok())
                    .collect();
                
                export::MarkdownRenderer::export_multi(&specs, output_path, tables)?;
                println!("✓ Exported {} specs to {}", specs.len(), output);
            } else {
                // Export single spec
                let spec_row = db.get_spec(&id)?
                    .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", id))?;
                let spec: models::SpecData = serde_json::from_value(spec_row.data)?;
                
                export::MarkdownRenderer::export_to_file(&spec, output_path, tables)?;
                println!("✓ Exported spec {} to {}", id, output);
            }
        }
    }

    Ok(())
}
