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
mod collab;

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

    /// Search specs using full-text search
    Search {
        /// Search query
        query: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
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

    /// Git-based sync operations
    Sync {
        #[command(subcommand)]
        operation: SyncOperationCli,
    },

    /// Review and approval operations
    Review {
        #[command(subcommand)]
        operation: ReviewOperationCli,
    },

    /// Conflict resolution operations
    Conflicts {
        #[command(subcommand)]
        operation: ConflictOperationCli,
    },
}

// Clap-compatible wrapper enums for CLI parsing
#[derive(Subcommand)]
enum SyncOperationCli {
    /// Initialize sync repository
    Init {
        /// Path to git repository
        #[arg(short, long)]
        repo: String,

        /// Remote URL (optional)
        #[arg(short, long)]
        remote: Option<String>,
    },

    /// Push spec(s) to git repository
    Push {
        /// Spec ID (or 'all' for all specs)
        id: String,

        /// Commit message
        #[arg(short, long)]
        message: Option<String>,

        /// Remote name
        #[arg(long, default_value = "origin")]
        remote: String,

        /// Branch name
        #[arg(long, default_value = "main")]
        branch: String,
    },

    /// Pull spec(s) from git repository
    Pull {
        /// Spec ID (or 'all' for all specs)
        id: String,

        /// Remote name
        #[arg(long, default_value = "origin")]
        remote: String,

        /// Branch name
        #[arg(long, default_value = "main")]
        branch: String,
    },

    /// Show sync status
    Status,

    /// Show differences between local and remote
    Diff {
        /// Spec ID
        id: String,

        /// Remote name
        #[arg(long, default_value = "origin")]
        remote: String,

        /// Branch name
        #[arg(long, default_value = "main")]
        branch: String,
    },
}

#[derive(Subcommand)]
enum ReviewOperationCli {
    /// Request a review
    Request {
        /// Spec ID
        spec_id: String,

        /// Reviewer email or username
        reviewer: String,
    },

    /// Approve a review
    Approve {
        /// Review ID
        review_id: String,

        /// Optional comment
        #[arg(short, long)]
        comment: Option<String>,
    },

    /// Reject a review
    Reject {
        /// Review ID
        review_id: String,

        /// Required rejection comment
        #[arg(short, long)]
        comment: String,
    },

    /// List reviews
    List {
        /// Spec ID (optional)
        spec_id: Option<String>,

        /// Filter by status
        #[arg(long)]
        status: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConflictOperationCli {
    /// List conflicts
    List {
        /// Spec ID (optional)
        spec_id: Option<String>,
    },

    /// Resolve a conflict
    Resolve {
        /// Conflict ID
        conflict_id: String,

        /// Resolution strategy: ours, theirs, manual
        #[arg(short, long, default_value = "manual")]
        strategy: String,
    },
}

// Conversion functions from CLI enums to library enums
impl From<SyncOperationCli> for commands::SyncOperation {
    fn from(op: SyncOperationCli) -> Self {
        match op {
            SyncOperationCli::Init { repo, remote } => commands::SyncOperation::Init { repo, remote },
            SyncOperationCli::Push { id, message, remote, branch } => commands::SyncOperation::Push { id, message, remote, branch },
            SyncOperationCli::Pull { id, remote, branch } => commands::SyncOperation::Pull { id, remote, branch },
            SyncOperationCli::Status => commands::SyncOperation::Status,
            SyncOperationCli::Diff { id, remote, branch } => commands::SyncOperation::Diff { id, remote, branch },
        }
    }
}

impl From<ReviewOperationCli> for commands::ReviewOperation {
    fn from(op: ReviewOperationCli) -> Self {
        match op {
            ReviewOperationCli::Request { spec_id, reviewer } => commands::ReviewOperation::Request { spec_id, reviewer },
            ReviewOperationCli::Approve { review_id, comment } => commands::ReviewOperation::Approve { review_id, comment },
            ReviewOperationCli::Reject { review_id, comment } => commands::ReviewOperation::Reject { review_id, comment },
            ReviewOperationCli::List { spec_id, status } => commands::ReviewOperation::List { spec_id, status },
        }
    }
}

impl From<ConflictOperationCli> for commands::ConflictOperation {
    fn from(op: ConflictOperationCli) -> Self {
        match op {
            ConflictOperationCli::List { spec_id } => commands::ConflictOperation::List { spec_id },
            ConflictOperationCli::Resolve { conflict_id, strategy } => commands::ConflictOperation::Resolve { conflict_id, strategy },
        }
    }
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
        Commands::Search { query, json } => {
            let format = if json {
                commands::OutputFormat::Json
            } else {
                commands::OutputFormat::Summary
            };
            commands::search(&query, format)?;
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
        Commands::Sync { operation } => {
            commands::sync_command(operation.into()).await?;
        }
        Commands::Review { operation } => {
            commands::review_command(operation.into())?;
        }
        Commands::Conflicts { operation } => {
            commands::conflict_command(operation.into())?;
        }
    }

    Ok(())
}
