//! manifold - The Global Spec Manifold
//!
//! A local-first, MCP-native, JSON-canonical specification engine

mod commands;
mod config;
mod db;
mod models;

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
}

fn main() -> anyhow::Result<()> {
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
    }

    Ok(())
}
