#!/bin/bash
# Demo script for collaboration features

set -e

echo "======================================================================"
echo "  Manifold Collaboration Demo"
echo "======================================================================"
echo

# Clean up any existing data
rm -rf ~/.manifold/sync ~/.manifold-test 2>/dev/null || true

# Initialize manifold
echo "📦 Initializing Manifold..."
./target/release/manifold init 2>/dev/null || true
echo

# Create a test spec
echo "📝 Creating test spec..."
SPEC_ID=$(./target/release/manifold new collab-demo --name "Collaboration Demo Spec" | grep "Created spec:" | awk '{print $3}')
echo "   Created: $SPEC_ID"
echo

# Initialize sync repository
echo "🔄 Initializing Git-based sync..."
./target/release/manifold sync init --repo ~/.manifold/sync
echo

# Check sync status
echo "📊 Checking sync status..."
./target/release/manifold sync status
echo

# Push the spec
echo "⬆️  Pushing spec to sync repository..."
./target/release/manifold sync push $SPEC_ID --message "Initial demo spec"
echo

# Simulate concurrent modification (modify spec locally)
echo "✏️  Simulating local modifications..."
echo "   (In real scenario, another user would modify remotely)"
echo

# Request a review
echo "👁️  Requesting review..."
./target/release/manifold review request $SPEC_ID --reviewer "demo@example.com"
REVIEW_ID=$(./target/release/manifold review list --spec-id $SPEC_ID 2>/dev/null | grep "Review ID:" | awk '{print $3}' | head -1)
echo "   Review ID: $REVIEW_ID"
echo

# List pending reviews
echo "📋 Listing reviews..."
./target/release/manifold review list --spec-id $SPEC_ID
echo

# Approve the review
echo "✅ Approving review..."
./target/release/manifold review approve $REVIEW_ID --comment "LGTM! Collaboration features look great."
echo

# List reviews again to see approval
echo "📋 Reviews after approval..."
./target/release/manifold review list --spec-id $SPEC_ID
echo

# Show final sync status
echo "📊 Final sync status..."
./target/release/manifold sync status
echo

# Summary
echo "======================================================================"
echo "✨ Collaboration Demo Complete!"
echo "======================================================================"
echo
echo "What we demonstrated:"
echo "  ✓ Git-based sync initialization"
echo "  ✓ Pushing specs to sync repository"
echo "  ✓ Checking sync status"
echo "  ✓ Requesting reviews"
echo "  ✓ Approving reviews with comments"
echo
echo "Try these commands next:"
echo "  manifold sync pull $SPEC_ID"
echo "  manifold conflicts list"
echo "  manifold review list --status pending"
echo
echo "See COLLABORATION.md for detailed examples!"
echo
