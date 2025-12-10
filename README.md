# manifold — The Global Spec Manifold

A **local-first, MCP-native, JSON-canonical** specification engine that unifies all your projects into a single living manifold of requirements, scenarios, tasks, and change history.

**Built from the ground up for LLM-first and agent-ready.**

## Philosophy

- **Specs are not files** → they are living nodes in a global manifold
- **Canonical format = JSON** (Markdown is only a rendered view in the TUI)
- **Designed for LLM structured output** + JSON Patch from day one
- **MCP server** so Claude, Cursor, your future brain agents can act on it
- **Rust CLI + Ratatui TUI**
- **SQLite backend**, embeddable, self-hostable via Docker

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

## Data Model (LLM-Native)

```json
{
  "$schema": "manifold://core/v1",
  "spec_id": "eager-anchor-auric",
  "project": "auric-raptor",
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
      "description": "Implement Field-Oriented Control algorithm in Rust",
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

## Getting Started

### Install & Initialize

```bash
# Build the CLI
cargo build --release

# Initialize manifold
./target/release/manifold init

# Create your first spec
./target/release/manifold new auric-raptor --name "Closed-Loop Torque Control"

# List specs
./target/release/manifold list

# Show a spec
./target/release/manifold show eager-anchor-auric --json
```

## CLI Commands

```bash
manifold init                         # First-time setup
manifold new <project> [--name "..."] [--boundary personal|work|company]
manifold list [--boundary all] [--stage tasks]
manifold show <id> [--json]          # JSON output or human summary
```

## Directory Structure

```
~/.manifold/
├── config.toml                  # default boundary, LLM endpoints, MCP settings
├── db/
│   └── manifold.db              # SQLite with JSON1 + FTS5
├── schemas/
│   ├── core.json                # JSON Schema for validation
│   └── plugins/                 # drop-in extensions
├── exports/                     # optional .md / .pdf renders
└── cache/                       # embeddings, patch history
```

## Key Design Choices

### Borrowed from OpenSpec

- **GIVEN/WHEN/THEN** scenarios (but structured as JSON objects)
- **Requirements vs Tasks** separation
- **Change tracking** via JSON Patch (vs. file-based deltas)

### Different from OpenSpec

| Feature | OpenSpec | Manifold |
|---------|----------|----------|
| **Storage** | Markdown files in repo | SQLite database |
| **Canonical Format** | Markdown (human-first) | JSON (LLM-first) |
| **Scope** | Per-repository | Global across all projects |
| **Architecture** | File-based, git-native | Database-backed, MCP native |
| **Integration** | Slash commands in editors | MCP server for any agent |
| **Queryable** | File search | FTS5 + SQL queries |

### LLM-Native Design Decisions

| Feature | Why it's LLM-native |
|---------|---------------------|
| `shall` as explicit field | LLMs can directly read requirement |
| `given`/`when`/`then` as arrays | Structured, no parsing needed |
| `requirement_ids` on tasks | Explicit traceability |
| `decisions` array | Design rationale inline |
| `acceptance` on tasks | Clear completion criteria |
| `edge_cases` on scenarios | Prompts LLM to consider failures |
| `priority`: must/should/could | MoSCoW, machine-parseable |
| Flat `requirements` array | Easier traversal than nested objects |

## Implementation Phases

- **Phase 1** ✅ — Skeleton + DB + CLI (`init`, `new`, `list`, `show`)
- **Phase 2** — Schema validation & `join` command
- **Phase 3** — MCP server (Rust)
- **Phase 4** — Workflow engine + events table
- **Phase 5** — LLM editing loop
- **Phase 6** — Ratatui TUI
- **Phase 7** — Markdown renderer & export
- **Phase 8** — Plugin system & Docker

## License

MIT
