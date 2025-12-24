#!/bin/bash
# Phase 7 Demo: Markdown Renderer & Export
# Demonstrates exporting JSON specs to human-readable Markdown

set -e

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║  Manifold Phase 7 Demo: Markdown Export                      ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

echo "📊 Available specs:"
cargo run -q -- list 2>/dev/null

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║  Export Feature Overview                                      ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "Export formats:"
echo "  1. Standard Markdown   - Detailed sections with full content"
echo "  2. Table format        - Requirements/tasks in table format"
echo "  3. Multi-spec export   - All specs in single document"
echo ""

# Export single spec (standard format)
echo "═══════════════════════════════════════════════════════════════"
echo "1. Exporting Robot Control spec (standard format)..."
echo "═══════════════════════════════════════════════════════════════"
cargo run -q -- export calm-flux-robot --output /tmp/robot-spec.md 2>/dev/null

echo ""
echo "Preview:"
head -60 /tmp/robot-spec.md
echo "..."
echo ""
echo "Full export saved to: /tmp/robot-spec.md"
wc -l /tmp/robot-spec.md

# Export with tables
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "2. Exporting Robot Control spec (table format)..."
echo "═══════════════════════════════════════════════════════════════"
cargo run -q -- export calm-flux-robot --output /tmp/robot-spec-tables.md --tables 2>/dev/null

echo ""
echo "Requirements table:"
grep -A5 "| ID | Title | Priority" /tmp/robot-spec-tables.md || true

echo ""
echo "Full export saved to: /tmp/robot-spec-tables.md"

# Export all specs
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "3. Exporting ALL specs to single document..."
echo "═══════════════════════════════════════════════════════════════"
cargo run -q -- export all --output /tmp/all-specs.md 2>/dev/null

echo ""
echo "Preview of multi-spec document:"
head -30 /tmp/all-specs.md
echo "..."
echo ""
echo "Full export saved to: /tmp/all-specs.md"
wc -l /tmp/all-specs.md

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║  Markdown Features                                            ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "Document Structure:"
echo "  ✓ Title and metadata block"
echo "  ✓ Table of contents with anchor links"
echo "  ✓ Workflow visualization (ASCII diagram)"
echo "  ✓ Requirements with SHALL statements"
echo "  ✓ GIVEN/WHEN/THEN scenarios"
echo "  ✓ Design decisions with rationale"
echo "  ✓ Tasks with traceability"
echo "  ✓ Change history timeline"
echo "  ✓ Generated timestamp footer"
echo ""
echo "Visual Enhancements:"
echo "  ✓ Priority emojis: 🔴 must, 🟡 should, 🟢 could, ⚫ wont"
echo "  ✓ Status emojis: ⏳ pending, 🔄 in_progress, ✅ completed, 🚫 blocked"
echo "  ✓ Workflow progress: ✓ completed → [CURRENT] → · upcoming"
echo "  ✓ Blockquotes for key statements"
echo "  ✓ Code blocks for technical diagrams"
echo "  ✓ Horizontal rules for section separation"
echo ""
echo "Table Format:"
echo "  ✓ Compact overview in tables"
echo "  ✓ Detailed sections follow tables"
echo "  ✓ GitHub-flavored Markdown compatible"
echo ""

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║  Example Output Sections                                      ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

echo "Workflow Status Section:"
grep -A10 "## Workflow Status" /tmp/robot-spec.md || true

echo ""
echo "Requirements Section:"
grep -A20 "### req-001" /tmp/robot-spec.md | head -20 || true

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║  Phase 7 Complete!                                            ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "Key features implemented:"
echo "  ✓ Comprehensive Markdown renderer"
echo "  ✓ Single spec export"
echo "  ✓ Multi-spec collection export"
echo "  ✓ Table formatting option"
echo "  ✓ Workflow visualization"
echo "  ✓ Priority and status emojis"
echo "  ✓ GIVEN/WHEN/THEN scenario formatting"
echo "  ✓ Change history timeline"
echo "  ✓ Automatic table of contents"
echo "  ✓ Markdown anchor links"
echo ""
echo "Usage examples:"
echo "  manifold export <spec-id> -o output.md"
echo "  manifold export <spec-id> -o output.md --tables"
echo "  manifold export all -o collection.md"
echo ""
echo "Exported files:"
echo "  - /tmp/robot-spec.md (standard format)"
echo "  - /tmp/robot-spec-tables.md (table format)"
echo "  - /tmp/all-specs.md (multi-spec collection)"
echo ""
