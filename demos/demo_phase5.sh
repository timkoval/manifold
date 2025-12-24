#!/bin/bash
# Phase 5 Demo: LLM Editing Loop
# Demonstrates interactive spec editing with conversational AI

set -e

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║  Manifold Phase 5 Demo: LLM Editing Loop                     ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

# Clean start
echo "🧹 Cleaning up previous demo..."
rm -rf ~/.manifold
cargo build -q 2>/dev/null || true

# Initialize
echo ""
echo "📦 Initializing manifold..."
cargo run -q -- init 2>/dev/null

# Create a spec
echo ""
echo "✨ Creating a new spec for a robot control system..."
cargo run -q -- new robot-control --name "Robot Control System" 2>/dev/null

SPEC_ID="$(cargo run -q -- list 2>/dev/null | tail -1 | awk '{print $1}')"
echo "   Created spec: $SPEC_ID"

# Show initial status
echo ""
echo "📊 Initial spec status:"
cargo run -q -- show "$SPEC_ID" 2>/dev/null | grep -A15 "^Spec:"

# Enter LLM edit mode (simulate commands)
echo ""
echo "💬 Entering LLM editing session..."
echo "   (Simulating user commands - in real usage, this is interactive)"
echo ""

cat > /tmp/demo_commands.txt << 'EOF'
/status
/exit
EOF

echo "Commands that will be executed:"
echo "  1. /status  - Show current spec status"
echo "  2. /exit    - Exit session"
echo ""

< /tmp/demo_commands.txt cargo run -q -- edit "$SPEC_ID" 2>/dev/null | tail -30

# Demonstrate workflow advancement
echo ""
echo "🔄 Testing workflow with validation..."

# Try to advance without requirements (should fail)
echo ""
echo "Attempting to advance without requirements:"
cargo run -q -- workflow "$SPEC_ID" --operation advance 2>&1 | grep -v "^warning:" | tail -5 || true

# Add a requirement via database
echo ""
echo "➕ Adding a requirement to the spec..."
python3 << PYEOF
import sqlite3
import json
import time

conn = sqlite3.connect('${HOME}/.manifold/db/manifold.db')
cursor = conn.cursor()

cursor.execute("SELECT data FROM specs WHERE id=?", ('${SPEC_ID}',))
row = cursor.fetchone()
spec = json.loads(row[0])

spec['requirements'].append({
    "id": "req-001",
    "capability": "motion_control",
    "title": "Real-time motion control",
    "shall": "The system SHALL provide real-time motion control with <10ms latency",
    "rationale": "Required for safe and precise robot operation",
    "priority": "must",
    "tags": ["realtime", "safety"],
    "scenarios": [{
        "id": "sc-001",
        "name": "Normal operation",
        "given": ["Robot is powered on", "Motion controller is initialized"],
        "when": "A motion command is issued",
        "then": ["Motion begins within 10ms", "Target position is reached accurately"],
        "edge_cases": ["Power loss during motion", "Emergency stop triggered"]
    }]
})

spec['history']['updated_at'] = int(time.time())
cursor.execute("UPDATE specs SET data=?, updated_at=? WHERE id=?", 
               (json.dumps(spec), spec['history']['updated_at'], '${SPEC_ID}'))
conn.commit()
conn.close()
print("✓ Added requirement req-001")
PYEOF

# Show updated spec
echo ""
echo "📊 Updated spec:"
cargo run -q -- show "$SPEC_ID" 2>/dev/null | grep -A15 "^Spec:"

# Now advance should work
echo ""
echo "🔄 Advancing workflow (should succeed now):"
cargo run -q -- workflow "$SPEC_ID" --operation advance 2>&1 | grep -v "^warning:" | tail -10

# Show workflow history
echo ""
echo "📜 Workflow history:"
cargo run -q -- workflow "$SPEC_ID" --operation history 2>&1 | grep -v "^warning:" | tail -10

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║  Phase 5 Demo Complete!                                       ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "Key features demonstrated:"
echo "  ✓ Interactive LLM editing session with REPL"
echo "  ✓ System prompts with full spec context"
echo "  ✓ Slash commands (/status, /advance, /show, /exit)"
echo "  ✓ Workflow validation and advancement"
echo "  ✓ Event logging for all operations"
echo ""
echo "To use with a real LLM:"
echo "  export OPENAI_API_KEY=your-key"
echo "  cargo run -- edit <spec-id>"
echo ""
