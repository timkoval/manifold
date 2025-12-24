# Manifold Demo Scripts

This directory contains interactive demo scripts that showcase different features of Manifold.

## Available Demos

### Core Features

#### `demo_phase5.sh` - LLM Editing Session
Demonstrates the interactive LLM chat interface for editing specifications.

**Requirements:**
- `OPENAI_API_KEY` environment variable set

**Features shown:**
- Creating a spec via CLI
- Launching LLM editing session
- Chat-based spec modification
- Requirements and tasks generation

**Run:**
```bash
./demos/demo_phase5.sh
```

---

#### `demo_phase6.sh` - TUI Dashboard
Shows the Terminal User Interface (TUI) with interactive navigation.

**Features shown:**
- Two-pane layout
- Tab navigation (Overview, Requirements, Tasks, Decisions, History, Conflicts)
- Boundary filtering
- Workflow visualization
- Real-time refresh

**Run:**
```bash
./demos/demo_phase6.sh
```

---

#### `demo_phase7.sh` - Markdown Export
Demonstrates the markdown export functionality with different formats.

**Features shown:**
- Standard export format
- Table format
- Multi-spec collection export
- Requirements and tasks rendering

**Run:**
```bash
./demos/demo_phase7.sh
```

---

### Collaboration Features

#### `demo_collab.sh` - Collaboration Workflow
Full demonstration of git-based collaboration features.

**Features shown:**
- Git sync initialization
- Push/pull specs to remote repository
- Conflict detection and resolution
- Review workflow (request, approve, reject)
- Three-way merge

**Run:**
```bash
./demos/demo_collab.sh
```

**What it creates:**
- Test spec with requirements and tasks
- Local git repository for sync
- Simulated remote changes
- Merge conflicts for resolution

---

#### `demo_phase11.sh` - Enhanced TUI Conflict Resolution
Interactive demo of advanced conflict resolution in the TUI.

**Features shown:**
- Visual diff highlighting
- Manual conflict editing with text input
- Bulk conflict resolution
- Auto-merge capability
- Real-time conflict statistics
- Multiple resolution strategies (Ours, Theirs, Merge, Manual)

**Run:**
```bash
./demos/demo_phase11.sh
```

**Interactive checklist:**
1. Visual diff highlighting
2. Conflict statistics in footer
3. Manual editing workflow
4. Auto-merge operation
5. Bulk resolution

---

### Testing

#### `test_mcp.sh` - MCP Server Test
Tests the Model Context Protocol (MCP) server implementation.

**Features tested:**
- Server initialization
- Tool discovery
- Spec creation via MCP
- Spec retrieval via MCP
- Error handling

**Run:**
```bash
./demos/test_mcp.sh
```

---

## General Usage

All demo scripts:
1. Create temporary test environments
2. Are idempotent (can be run multiple times)
3. Clean up after themselves (or provide cleanup instructions)
4. Use relative paths from project root

### Running from Project Root

```bash
# From manifold/ directory
./demos/demo_phase11.sh
./demos/demo_collab.sh
# etc.
```

### Running from demos/ Directory

```bash
cd demos
./demo_phase11.sh
./demo_collab.sh
# etc.
```

## Demo Environments

Most demos create temporary data directories to avoid polluting your real Manifold data:

- `~/.manifold-demo-phase5/` - LLM demo data
- `~/.manifold-demo-phase11/` - TUI conflict demo data
- `~/.manifold-sync-phase11/` - Git sync demo repository
- etc.

**Cleanup:**
```bash
rm -rf ~/.manifold-demo-*
rm -rf ~/.manifold-sync-*
```

## Prerequisites

### All Demos
- Manifold built: `cargo build --release`
- Binary available: `./target/release/manifold`

### LLM Demo (demo_phase5.sh)
- OpenAI API key: `export OPENAI_API_KEY=sk-...`

### Collaboration Demos
- Git installed
- `jq` installed (for JSON manipulation)

### TUI Demos
- Terminal with color support
- Minimum 80x24 terminal size

## Demo Sequence

For a complete walkthrough of Manifold features, run demos in this order:

```bash
# 1. Basic spec creation and viewing
./demos/demo_phase6.sh

# 2. Export functionality
./demos/demo_phase7.sh

# 3. LLM editing (requires API key)
./demos/demo_phase5.sh

# 4. Collaboration workflow
./demos/demo_collab.sh

# 5. Advanced conflict resolution
./demos/demo_phase11.sh

# 6. MCP server testing
./demos/test_mcp.sh
```

## Customization

All demo scripts use environment variables that can be overridden:

```bash
# Custom data directory
export MANIFOLD_DATA_DIR=/tmp/my-manifold-demo
./demos/demo_phase11.sh

# Custom sync directory
export SYNC_DIR=/tmp/my-sync-demo
./demos/demo_collab.sh
```

## Troubleshooting

### Demo Script Fails
1. Check that you're in the project root directory
2. Ensure `./target/release/manifold` exists
3. Check script has execute permissions: `chmod +x demos/*.sh`

### Permission Denied
```bash
chmod +x demos/*.sh
```

### MCP Test Fails
```bash
# Rebuild first
cargo build --release
./demos/test_mcp.sh
```

### Cleanup Previous Demo Data
```bash
rm -rf ~/.manifold-demo-*
rm -rf ~/.manifold-sync-*
```

## Contributing

To add a new demo:

1. Create `demo_<feature>.sh` in this directory
2. Follow the existing pattern:
   - Clear header with feature name
   - Cleanup previous runs
   - Step-by-step output with section headers
   - Cleanup instructions at the end
3. Add entry to this README
4. Make executable: `chmod +x demos/demo_<feature>.sh`

## Documentation

For more details on the features demonstrated:

- **Collaboration:** `../docs/COLLABORATION.md`
- **TUI Enhancements:** `../docs/TUI_ENHANCEMENTS.md`
- **TUI Quick Reference:** `../docs/TUI_QUICK_REFERENCE.md`
- **Roadmap:** `../docs/ENHANCEMENTS.md`

---

**Manifold** - Local-first, MCP-native specification engine
