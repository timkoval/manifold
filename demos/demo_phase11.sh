#!/bin/bash
# Demo script for Phase 11: Enhanced TUI Conflict Resolution
# Demonstrates manual editing, visual diffs, bulk operations, and auto-merge

set -e

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  Manifold Phase 11: Enhanced TUI Conflict Resolution Demo    ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

MANIFOLD_DIR="$HOME/.manifold-demo-phase11"
SYNC_DIR="$HOME/.manifold-sync-phase11"

# Cleanup previous demo
if [ -d "$MANIFOLD_DIR" ]; then
    echo "🧹 Cleaning up previous demo..."
    rm -rf "$MANIFOLD_DIR"
fi
if [ -d "$SYNC_DIR" ]; then
    rm -rf "$SYNC_DIR"
fi

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  Step 1: Initialize Manifold Database"
echo "══════════════════════════════════════════════════════════════"
export MANIFOLD_DATA_DIR="$MANIFOLD_DIR"
./target/release/manifold init

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  Step 2: Create Test Specification"
echo "══════════════════════════════════════════════════════════════"
SPEC_ID=$(./target/release/manifold new \
    --project "auth-service" \
    --name "User Authentication" \
    --boundary personal \
    --description "OAuth 2.0 authentication system" \
    | grep -o 'spec-[a-z0-9]*')

echo "✓ Created spec: $SPEC_ID"

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  Step 3: Initialize Git Sync"
echo "══════════════════════════════════════════════════════════════"
./target/release/manifold sync init --repo "$SYNC_DIR"
echo "✓ Git sync initialized at: $SYNC_DIR"

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  Step 4: Push Initial Spec (Base Version)"
echo "══════════════════════════════════════════════════════════════"
./target/release/manifold sync push "$SPEC_ID" --message "Initial version"
echo "✓ Pushed to sync repository (base version)"

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  Step 5: Create Local Modifications"
echo "══════════════════════════════════════════════════════════════"

# Simulate local changes using workflow commands
./target/release/manifold workflow advance "$SPEC_ID" --stage design
echo "✓ Advanced to design stage locally"

# Modify the spec directly via database (simulating local edits)
sqlite3 "$MANIFOLD_DIR/manifold.db" <<EOF
UPDATE specs 
SET data = json_set(
    data,
    '$.name', 'User Authentication v2.0',
    '$.requirements[0].shall', 'The system SHALL support OAuth 2.0 and SAML'
)
WHERE id = '$SPEC_ID';
EOF
echo "✓ Modified spec locally (changed name and requirements)"

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  Step 6: Create Remote Modifications (Simulated)"
echo "══════════════════════════════════════════════════════════════"

# Modify the synced JSON file to simulate remote changes
SPEC_FILE="$SYNC_DIR/${SPEC_ID}.json"
if [ -f "$SPEC_FILE" ]; then
    # Use jq to modify the JSON
    jq '.name = "Enterprise Authentication System" | 
        .stage = "approval" | 
        .requirements[0].shall = "The system SHALL authenticate via biometrics"' \
        "$SPEC_FILE" > "$SPEC_FILE.tmp"
    mv "$SPEC_FILE.tmp" "$SPEC_FILE"
    
    # Commit the remote change
    cd "$SYNC_DIR"
    git add "${SPEC_ID}.json"
    git commit -m "Remote: Updated to Enterprise Auth with biometrics"
    cd - > /dev/null
    
    echo "✓ Created remote modifications (different name, stage, and requirements)"
else
    echo "⚠ Warning: Spec file not found at $SPEC_FILE"
fi

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  Step 7: Pull Remote Changes (Creates Conflicts)"
echo "══════════════════════════════════════════════════════════════"
./target/release/manifold sync pull "$SPEC_ID" || echo "✓ Pull completed with conflicts detected"

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  Step 8: List Detected Conflicts"
echo "══════════════════════════════════════════════════════════════"
./target/release/manifold conflicts list

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  Step 9: Create Additional Conflicts for Bulk Demo"
echo "══════════════════════════════════════════════════════════════"

# Add more conflicts by direct database insertion
sqlite3 "$MANIFOLD_DIR/manifold.db" <<EOF
INSERT INTO conflicts (spec_id, field_path, local_value, remote_value, base_value, detected_at, status)
VALUES 
    ('$SPEC_ID', 'decisions/dec-001', 
     json('"Use JWT tokens"'), 
     json('"Use session cookies"'), 
     json('"Use basic auth"'),
     $(date +%s), 
     'unresolved'),
    ('$SPEC_ID', 'tasks/task-001', 
     json('{"id": "task-001", "title": "Implement OAuth", "status": "in-progress"}'), 
     json('{"id": "task-001", "title": "Implement SAML", "status": "completed"}'),
     json('{"id": "task-001", "title": "Implement auth", "status": "pending"}'),
     $(date +%s), 
     'unresolved');
EOF

echo "✓ Created additional test conflicts (decisions, tasks)"

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  Conflicts Created Successfully!"
echo "══════════════════════════════════════════════════════════════"
./target/release/manifold conflicts list

echo ""
echo "╔════════════════════════════════════════════════════════════════╗"
echo "║                    TUI Demo Instructions                      ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""
echo "Now launch the TUI to test the enhanced conflict resolution:"
echo ""
echo "  ./target/release/manifold tui"
echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  TUI Feature Demo Checklist:"
echo "══════════════════════════════════════════════════════════════"
echo ""
echo "✓ 1. Visual Diff Highlighting"
echo "   - Tab to Conflicts tab"
echo "   - Press 'c' to load conflicts"
echo "   - Navigate with ↑/↓ to see different conflicts"
echo "   - Notice: BASE, LOCAL (different), REMOTE (different)"
echo ""
echo "✓ 2. Conflict Statistics"
echo "   - Check footer: shows 'N/M unresolved'"
echo "   - Updates in real-time after resolutions"
echo ""
echo "✓ 3. Single Resolution with Manual Edit"
echo "   - Select a conflict with ↑/↓"
echo "   - Press 'o' to open resolution dialog"
echo "   - Use ←/→ to select 'Manual'"
echo "   - Press Enter to open text input"
echo "   - Type: Hybrid OAuth and SAML Authentication"
echo "   - Press Enter to apply"
echo "   - See success message and updated stats"
echo ""
echo "✓ 4. Auto-Merge Compatible Conflicts"
echo "   - Press 'a' to auto-merge"
echo "   - Watch status: 'X merged, Y skipped, Z failed'"
echo "   - Conflicts reload automatically"
echo "   - Stats update to show remaining unresolved"
echo ""
echo "✓ 5. Bulk Resolution"
echo "   - Press 'b' to open bulk dialog"
echo "   - See: 'will apply to N conflicts'"
echo "   - Use ←/→ to select strategy (e.g., 'Ours')"
echo "   - Press Enter to resolve all at once"
echo "   - Verify: 'X resolved, Y failed'"
echo ""
echo "✓ 6. Verify Resolutions"
echo "   - All conflicts should show ✓ (green)"
echo "   - Stats show: '0/N unresolved'"
echo "   - Press 'q' to quit TUI"
echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  CLI Verification:"
echo "══════════════════════════════════════════════════════════════"
echo ""
echo "After TUI demo, verify via CLI:"
echo ""
echo "  # Check all conflicts resolved"
echo "  ./target/release/manifold conflicts list"
echo ""
echo "  # View updated spec"
echo "  ./target/release/manifold show $SPEC_ID"
echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  Cleanup:"
echo "══════════════════════════════════════════════════════════════"
echo ""
echo "  rm -rf $MANIFOLD_DIR $SYNC_DIR"
echo ""
echo "╔════════════════════════════════════════════════════════════════╗"
echo "║                      Demo Ready!                              ║"
echo "║  Run: ./target/release/manifold tui                           ║"
echo "╚════════════════════════════════════════════════════════════════╝"
