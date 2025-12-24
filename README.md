# manifold — The Global Spec Manifold

A **local-first, MCP-native, JSON-canonical** specification engine that unifies all your projects into a single living manifold of requirements, scenarios, tasks, and change history.

**Built from the ground up for LLM-first workflows and agent-ready automation.**

[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Philosophy

- **Specs are not files** → they are living nodes in a global manifold
- **Canonical format = JSON** (Markdown is only a rendered view)
- **Designed for LLM structured output** + JSON Patch from day one
- **MCP server** so Claude, Cursor, and AI agents can interact directly
- **Rust CLI + Ratatui TUI** for performance and rich UI
- **SQLite backend** with FTS5 search, embeddable, Docker-ready

## Why LLM-Native?

Traditional spec formats (Markdown, YAML) are optimized for humans. Manifold optimizes for **LLMs consuming and producing specs**.

### What LLMs Need

| When building | JSON gives LLMs | Markdown makes LLMs guess |
|---------------|-----------------|--------------------------|
| "What are all the requirements?" | `spec.requirements` | Regex for `### Requirement:` |
| "What scenarios does req-001 have?" | `req.scenarios` | Find content until next `###` |
| "Is this task done?" | `task.status === "completed"` | Parse checkbox `- [x]` |
| "What should I test?" | `scenario.then` array | Extract lines starting with `- THEN` |
| "Why was this decision made?" | `decision.rationale` | Hope there's a section for it |

## ✨ Features

### Core Capabilities
- ✅ **JSON-Canonical Storage** - Specs stored as structured JSON in SQLite
- ✅ **Workflow Engine** - State machine with validation (requirements → design → tasks → approval → implemented)
- ✅ **Full-Text Search** - FTS5-powered search across all specs
- ✅ **Boundary Isolation** - Separate personal/work/company specs
- ✅ **Change Tracking** - Complete history with JSON Patch operations
- ✅ **Git-Based Sync** - Collaborate with team using git workflows
- ✅ **Conflict Resolution** - Automatic and manual conflict detection/resolution
- ✅ **Review & Approval** - Formal review workflow for spec changes

### User Interfaces
- ✅ **CLI Commands** - Full-featured command-line interface
- ✅ **TUI Dashboard** - Interactive Ratatui terminal UI with tabs
- ✅ **LLM Chat Session** - Conversational editing with AI assistance
- ✅ **MCP Server** - JSON-RPC 2.0 server for AI agent integration

### Export & Integration
- ✅ **Markdown Export** - Beautiful documentation generation
- ✅ **Table Formatting** - Compact overview with GitHub-flavored tables
- ✅ **Multi-Spec Collections** - Export all specs to single document
- ✅ **JSON Schema Validation** - Ensure spec integrity

### DevOps Ready
- ✅ **Docker Support** - Multi-stage builds for minimal images
- ✅ **Docker Compose** - Multi-service orchestration
- ✅ **Cross-Platform** - Linux, macOS, Windows support

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/manifold.git
cd manifold

# Build the CLI
cargo build --release

# Initialize manifold
./target/release/manifold init
```

### Basic Usage

```bash
# Create your first spec
manifold new robot-control --name "Robot Control System"

# List all specs
manifold list

# Show a spec (human-readable)
manifold show <spec-id>

# Show as JSON
manifold show <spec-id> --json

# Launch TUI dashboard
manifold tui

# Export to Markdown
manifold export <spec-id> -o spec.md
```

## 📖 Data Model (LLM-Native)

```json
{
  "$schema": "manifold://core/v1",
  "spec_id": "eager-anchor-auric",
  "project": "robot-control",
  "boundary": "personal",
  "name": "Closed-Loop Torque Control",
  
  "stage": "design",
  "stages_completed": ["requirements"],
  
  "requirements": [
    {
      "id": "req-001",
      "capability": "torque_control",
      "title": "High-precision torque accuracy",
      "shall": "The system SHALL maintain ±0.1 Nm accuracy at 100 Hz",
      "rationale": "Precision required for safe human-robot interaction",
      "priority": "must",
      "tags": ["realtime", "safety"],
      
      "scenarios": [
        {
          "id": "sc-001",
          "name": "Nominal load accuracy",
          "given": ["motor is operational", "5 kg payload attached"],
          "when": "torque setpoint of 2.0 Nm is commanded",
          "then": ["measured torque is within 1.9-2.1 Nm", "control loop maintains 100 Hz"],
          "edge_cases": ["payload exceeds 10 kg"]
        }
      ]
    }
  ],
  
  "tasks": [
    {
      "id": "task-001",
      "requirement_ids": ["req-001"],
      "title": "Implement FOC controller",
      "description": "Implement Field-Oriented Control algorithm",
      "status": "in_progress",
      "assignee": "agent",
      "acceptance": ["unit tests pass", "benchmark shows <10ms loop time"]
    }
  ],
  
  "decisions": [
    {
      "id": "dec-001",
      "title": "Control algorithm selection",
      "context": "Need 100Hz control with ±0.1Nm accuracy",
      "decision": "Use Field-Oriented Control (FOC) over PID",
      "rationale": "FOC provides better torque linearity",
      "alternatives_rejected": ["PID - insufficient accuracy"],
      "date": "2024-01-15"
    }
  ],
  
  "history": {
    "created_at": 1704067200,
    "updated_at": 1704153600,
    "patches": [...]
  }
}
```

## 🔧 CLI Commands

### Initialization & Setup
```bash
manifold init                         # First-time setup
```

### Spec Management
```bash
manifold new <project> [--name "..."] [--boundary personal|work|company]
manifold list [--boundary all] [--stage requirements]
manifold show <id> [--json]
manifold validate <id> [--strict]
manifold join <source-id> <target-boundary>
```

### Workflow Operations
```bash
manifold workflow <id> --operation status
manifold workflow <id> --operation advance
manifold workflow <id> --operation history
```

### Collaboration
```bash
# Git-based sync
manifold sync init --repo ~/sync-dir
manifold sync push <id> --message "Update requirements"
manifold sync pull <id>
manifold sync status

# Review & approval
manifold review request <spec-id> --reviewer alice@example.com
manifold review approve <review-id> --comment "LGTM"
manifold review list --spec-id <id>

# Conflict resolution
manifold conflicts list
manifold conflicts resolve <conflict-id> --strategy ours|theirs|merge
```

See [docs/COLLABORATION.md](docs/COLLABORATION.md) for detailed examples.

### Export
```bash
manifold export <id> -o output.md
manifold export <id> -o output.md --tables
manifold export all -o collection.md
```

### Interactive Interfaces
```bash
manifold tui              # Launch TUI dashboard
manifold edit <id>        # LLM chat session (requires OPENAI_API_KEY)
manifold serve            # Start MCP server (stdio)
```

## 🎨 TUI Dashboard

The Terminal UI provides a rich, interactive experience:

```
┌─────────────────────────────────────────────────────────┐
│ ╔═══════════════════════════════════════════════════╗   │
│ ║  Manifold Dashboard - Specification Management   ║   │
│ ╚═══════════════════════════════════════════════════╝   │
├──────────────┬────────────────────────────────────────────┤
│              │                                            │
│  Specs List  │  Detail View with Tabs                    │
│  (30%)       │  (70%)                                     │
│              │                                            │
│  📋 Robot... │  [Overview][Requirements][Tasks][...]     │
│  📐 Web...   │                                            │
│              │  Workflow: ✓ requirements → [DESIGN] → ...│
│              │                                            │
├──────────────┴────────────────────────────────────────────┤
│ ↑/↓: Navigate  Tab: Switch  1-4: Filter  q: Quit         │
└─────────────────────────────────────────────────────────┘
```

**Features:**
- Two-pane layout with spec list and detail view
- 6 tabs: Overview, Requirements, Tasks, Decisions, History, **Conflicts**
- Boundary filtering (1-4 keys)
- Real-time refresh (r key)
- Workflow visualization with progress indicators
- **Conflict resolution** with visual diffs and multiple strategies
- **Bulk operations** for resolving multiple conflicts at once
- **Auto-merge** for compatible changes
- **Manual editing** with inline text input
- **Real-time statistics** showing resolved/unresolved conflicts
- Visual workflow progress indicators
- Keyboard navigation (vim-style)
- Real-time filtering by boundary
- Color-coded UI with emojis

## 🤖 MCP Server Integration

Manifold includes a Model Context Protocol (MCP) server for AI agent integration:

### Available Tools

1. **create_spec** - Create new specifications
2. **query_manifold** - Search and filter specs
3. **advance_workflow** - Move specs through workflow stages
4. **apply_patch** - Apply JSON Patch operations (RFC 6902)

### Usage

```bash
# Start MCP server (stdio)
manifold serve

# Or use with Docker
docker-compose up manifold-server
```

### Example MCP Request

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "create_spec",
    "arguments": {
      "project": "my-project",
      "boundary": "personal",
      "name": "My Specification"
    }
  }
}
```

## 💬 LLM Chat Session

Interactive editing with AI assistance:

```bash
export OPENAI_API_KEY=sk-...
manifold edit <spec-id>

You> Help me add a requirement for user authentication
AI> I suggest adding: "The system SHALL authenticate users 
    via OAuth 2.0 with multi-factor authentication support..."

You> /advance
✓ Advanced to design stage

You> /status
Current stage: design
Requirements: 3
...
```

**Features:**
- Conversational interface with context-aware prompts
- Slash commands (/status, /advance, /show, /exit)
- Full spec context in system prompt
- Suggestions for SHALL statements and scenarios
- Automatic workflow validation

## 📝 Markdown Export

Convert JSON specs to beautiful documentation:

```markdown
# Robot Control System

> **Project:** robot-control  
> **Stage:** design  

## Workflow Status

```
✓ requirements → [DESIGN] → · tasks → · approval → · implemented
```

## Requirements

### req-001 - Real-time motion control

**Priority:** 🔴 (must)

#### Requirement

> The system SHALL provide real-time motion control with <10ms latency

#### Scenarios

**Normal operation** (sc-001)

- **GIVEN**
  - Robot is powered on
  - Motion controller is initialized
- **WHEN** A motion command is issued
- **THEN**
  - Motion begins within 10ms
  - Target position is reached accurately
```

**Export formats:**
- Standard: Detailed sections with full content
- Tables: Compact overview with GFM tables
- Multi-spec: Collection documents with TOC

## 🐳 Docker Deployment

### Build & Run

```bash
# Build image
docker build -t manifold .

# Run MCP server
docker run -it manifold

# Run TUI
docker run -it manifold manifold tui
```

### Docker Compose

```bash
# Start MCP server
docker-compose up manifold-server

# Run TUI (interactive)
docker-compose --profile interactive up manifold-tui

# Export all specs
docker-compose --profile export up manifold-export
```

## 📂 Directory Structure

### User Data (~/.manifold/)
```
~/.manifold/
├── config.toml                  # Configuration
├── db/
│   └── manifold.db              # SQLite with JSON1 + FTS5
├── schemas/
│   └── core.json                # JSON Schema for validation
├── exports/                     # Markdown/PDF exports
└── cache/                       # Temporary files
```

### Project Structure
```
manifold/
├── demos/                       # Demo scripts
│   ├── demo_collab.sh          # Collaboration features demo
│   ├── demo_phase11.sh         # Enhanced TUI demo
│   ├── demo_phase5.sh          # LLM editing demo
│   ├── demo_phase6.sh          # TUI dashboard demo
│   ├── demo_phase7.sh          # Export demo
│   └── test_mcp.sh             # MCP server test
├── docs/                        # Documentation
│   ├── COLLABORATION.md        # Collaboration guide
│   ├── COLLABORATION_IMPLEMENTATION.md
│   ├── ENHANCEMENTS.md         # Roadmap
│   ├── PHASE11_SUMMARY.md      # Phase 11 summary
│   ├── TUI_CONFLICTS.md        # TUI implementation
│   ├── TUI_ENHANCEMENTS.md     # TUI features
│   └── TUI_QUICK_REFERENCE.md  # Quick reference
├── schemas/
│   └── core.json               # JSON Schema
├── src/                         # Rust source code
│   ├── collab/                 # Collaboration features
│   ├── commands/               # CLI commands
│   ├── db/                     # Database layer
│   ├── mcp/                    # MCP server
│   ├── tui/                    # Terminal UI
│   └── ...
├── Cargo.toml                   # Rust dependencies
├── Dockerfile                   # Docker image
├── docker-compose.yml          # Docker Compose config
└── README.md                    # This file
```

## 🎯 Workflow Engine

Manifold enforces a rigorous workflow with validation:

```
requirements → design → tasks → approval → implemented
```

**Validation rules:**
- `requirements → design`: Must have ≥1 requirement with SHALL statement
- `design → tasks`: Must have ≥1 design decision
- `tasks → approval`: Must have ≥1 task with requirement traceability
- `approval → implemented`: Manual approval

**Event logging:**
- All transitions logged to workflow_events table
- Actor tracking (user, mcp, llm-session)
- Timestamp and details for audit trail

## 🔌 Architecture

### Technology Stack
- **Language:** Rust 1.82+
- **Database:** SQLite with FTS5
- **TUI:** Ratatui + Crossterm
- **HTTP Client:** Reqwest (for LLM API)
- **CLI:** Clap
- **Async:** Tokio

### Design Principles

1. **JSON-Canonical** - JSON is truth, Markdown is view
2. **Local-First** - SQLite, no cloud dependencies
3. **MCP-Native** - Built for AI agent integration
4. **Type-Safe** - Rust's type system ensures correctness
5. **Fast** - Compiled binary, efficient queries

## 🛠️ Development Phases

- ✅ **Phase 1** - Core infrastructure, CLI, database
- ✅ **Phase 2** - Schema validation, cross-boundary sharing
- ✅ **Phase 3** - MCP server implementation
- ✅ **Phase 4** - Workflow engine with state machine
- ✅ **Phase 5** - LLM editing loop
- ✅ **Phase 6** - Ratatui TUI dashboard
- ✅ **Phase 7** - Markdown renderer & export
- ✅ **Phase 8** - Docker deployment
- ✅ **Phase 9** - Collaboration features (git sync, conflicts, reviews)
- ✅ **Phase 10** - TUI conflict resolution
- ✅ **Phase 11** - Enhanced TUI (manual editing, bulk ops, auto-merge)

## 🧪 Testing

```bash
# Run unit tests
cargo test

# Run with demo data
./demos/demo_phase5.sh   # LLM editing session demo
./demos/demo_phase6.sh   # TUI dashboard demo
./demos/demo_phase7.sh   # Markdown export demo
./demos/demo_collab.sh   # Collaboration features demo
./demos/demo_phase11.sh  # Enhanced TUI conflict resolution demo

# Test MCP server
./demos/test_mcp.sh
```

## 🚧 Roadmap

See [docs/ENHANCEMENTS.md](docs/ENHANCEMENTS.md) for detailed roadmap and potential improvements.

## 🤝 Contributing

Contributions welcome! Please see [docs/ENHANCEMENTS.md](docs/ENHANCEMENTS.md) for ideas.

### Development Setup

```bash
# Clone and build
git clone https://github.com/yourusername/manifold.git
cd manifold
cargo build

# Initialize test database
./target/debug/manifold init

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- tui
```

## ⚙️ Configuration

The config file is located at `~/.manifold/config.toml`:

```toml
[database]
path = "~/.manifold/db/manifold.db"

[export]
default_format = "standard"
output_dir = "~/.manifold/exports"

[llm]
provider = "openai"
api_base = "https://api.openai.com/v1"
model = "gpt-4"
temperature = 0.7

[ui]
theme = "default"
```

## 🔍 Search & Query

```bash
# Full-text search across all specs
manifold list --search "authentication"

# Filter by boundary
manifold list --boundary work

# Filter by stage
manifold list --stage requirements

# Combine filters
manifold list --boundary personal --stage design
```

## 📊 Database Schema

```sql
-- Specs table with JSON1 support
CREATE TABLE specs (
    spec_id TEXT PRIMARY KEY,
    project TEXT NOT NULL,
    boundary TEXT NOT NULL,
    data TEXT NOT NULL,  -- JSON blob
    created_at INTEGER,
    updated_at INTEGER
);

-- Full-text search index
CREATE VIRTUAL TABLE specs_fts USING fts5(
    spec_id, project, name, content
);

-- Workflow events for audit trail
CREATE TABLE workflow_events (
    id INTEGER PRIMARY KEY,
    spec_id TEXT NOT NULL,
    from_stage TEXT,
    to_stage TEXT NOT NULL,
    actor TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    details TEXT,
    FOREIGN KEY (spec_id) REFERENCES specs(spec_id)
);
```

## 📄 License

MIT License - see [LICENSE](LICENSE) for details

## 🙏 Acknowledgments

Inspired by:
- **OpenSpec** - For GIVEN/WHEN/THEN scenarios and requirements engineering best practices
- **Model Context Protocol (MCP)** - For AI agent integration patterns
- **Ratatui** - For terminal UI framework
- The Rust community for excellent tooling

---

**Built with 🦀 Rust**
