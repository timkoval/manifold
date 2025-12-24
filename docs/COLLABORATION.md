# Collaboration Features Demo

This guide demonstrates the new collaboration features in Manifold:
- Git-based synchronization
- Conflict detection and resolution
- Review and approval workflows

## Setup

```bash
# Initialize manifold
manifold init

# Create a spec
manifold new robot-control --name "Robot Control System"
SPEC_ID=$(manifold list | tail -1 | awk '{print $1}')

echo "Created spec: $SPEC_ID"
```

## Git-Based Sync

### Initialize Sync Repository

```bash
# Initialize sync with local git repo
manifold sync init --repo ~/manifold-sync

# Initialize with remote
manifold sync init --repo ~/manifold-sync --remote git@github.com:user/manifold-specs.git
```

### Push Specs

```bash
# Push single spec
manifold sync push $SPEC_ID --message "Initial requirements"

# Push all specs
manifold sync push all --message "Sync all project specs"

# Push to specific remote/branch
manifold sync push $SPEC_ID --remote origin --branch develop
```

### Pull Specs

```bash
# Pull single spec
manifold sync pull $SPEC_ID

# Pull all specs
manifold sync pull all

# Pull from specific remote/branch
manifold sync pull all --remote origin --branch develop
```

### Check Sync Status

```bash
# See modified specs
manifold sync status
```

## Conflict Resolution

When pulling specs that have conflicting changes:

### CLI Conflict Resolution

```bash
# Pull will detect conflicts
manifold sync pull $SPEC_ID

# List conflicts
manifold conflicts list
manifold conflicts list --spec-id $SPEC_ID

# Resolve conflicts
CONFLICT_ID=$(manifold conflicts list | grep "ID:" | awk '{print $2}')

# Resolve with different strategies:
manifold conflicts resolve $CONFLICT_ID --strategy ours      # Keep local changes
manifold conflicts resolve $CONFLICT_ID --strategy theirs    # Accept remote changes
manifold conflicts resolve $CONFLICT_ID --strategy merge     # Auto-merge if possible
manifold conflicts resolve $CONFLICT_ID --strategy manual    # Manual resolution
```

### TUI Conflict Resolution

For a visual, interactive experience, use the TUI:

```bash
# Launch TUI
manifold tui

# Navigate to Conflicts tab (press Tab until you reach "Conflicts")
# Press 'c' to load conflicts for the selected spec
# Use ↑/↓ arrows to navigate through conflicts
# Press 'o' to open resolution dialog
# Use ←/→ arrows to select strategy:
#   - Ours (Keep Local)
#   - Theirs (Accept Remote)
#   - Merge (Auto)
#   - Manual
# Press Enter to apply the selected strategy
# Press Esc to cancel
```

**TUI Features:**
- **Visual Diff Display** - See local vs remote values side-by-side
- **Color-Coded Status** - Red for unresolved, green for resolved
- **Interactive Selection** - Navigate with keyboard shortcuts
- **Real-Time Updates** - See changes immediately after resolution
- **Status Messages** - Confirmation of successful resolutions

## Review & Approval Workflow

### Request Review

```bash
# Request review from team member
manifold review request $SPEC_ID --reviewer alice@example.com

# Get review ID
REVIEW_ID=$(manifold review list --spec-id $SPEC_ID | grep "Review ID:" | awk '{print $3}')
```

### Approve Review

```bash
# Approve with comment
manifold review approve $REVIEW_ID --comment "LGTM! Great requirements."

# Approve without comment
manifold review approve $REVIEW_ID
```

### Reject Review

```bash
# Reject with required comment
manifold review reject $REVIEW_ID --comment "Needs more detail on authentication requirements"
```

### List Reviews

```bash
# List all reviews for a spec
manifold review list --spec-id $SPEC_ID

# List all pending reviews
manifold review list --status pending

# List all reviews
manifold review list
```

## Complete Workflow Example

```bash
#!/bin/bash

# 1. Team member Alice creates a spec
manifold new auth-service --name "Authentication Service"
SPEC=$(manifold list | tail -1 | awk '{print $1}')

# 2. Initialize sync
manifold sync init --repo ~/manifold-sync --remote git@github.com:team/specs.git

# 3. Push to remote
manifold sync push $SPEC --message "Initial auth service requirements"

# 4. Team member Bob pulls the spec
manifold sync pull $SPEC

# 5. Both Alice and Bob make changes...
# (Alice modifies locally, Bob modifies his clone)

# 6. Alice pushes first
manifold sync push $SPEC --message "Add OAuth2 requirements"

# 7. Bob tries to pull - conflict detected!
manifold sync pull $SPEC
# Output: ⚠ Conflict detected in spec: bold-flux-auth

# 8. Bob reviews conflicts
manifold conflicts list --spec-id $SPEC

# 9. Bob resolves conflicts
CONFLICT=$(manifold conflicts list | grep "ID:" | awk '{print $2}')
manifold conflicts resolve $CONFLICT --strategy merge

# 10. Bob requests review
manifold review request $SPEC --reviewer alice@example.com

# 11. Alice reviews and approves
REVIEW=$(manifold review list --spec-id $SPEC | grep "Review ID:" | awk '{print $3}')
manifold review approve $REVIEW --comment "Looks good after merge!"

# 12. Advance to next workflow stage
manifold workflow $SPEC --operation advance
```

## Integration with Workflow Engine

Reviews can gate workflow transitions:

```bash
# Request review before advancing
manifold review request $SPEC_ID --reviewer senior-engineer@example.com

# Only advance after approval
REVIEW_ID=$(manifold review list --spec-id $SPEC_ID | grep "Review ID:" | awk '{print $3}')
manifold review approve $REVIEW_ID --comment "Approved for design phase"

# Now can advance
manifold workflow $SPEC_ID --operation advance
```

## Tips & Best Practices

### 1. Regular Syncing
```bash
# Pull before making changes
manifold sync pull all

# Make your changes
# ...

# Push when done
manifold sync push all --message "Updated requirements for sprint 5"
```

### 2. Conflict Prevention
```bash
# Check status before pulling
manifold sync status

# Commit local changes first
manifold sync push $SPEC_ID --message "WIP: Adding security requirements"

# Then pull
manifold sync pull $SPEC_ID
```

### 3. Review Workflow
```bash
# Use reviews for important stages
# - Requirements complete
# - Design approved
# - Ready for implementation

manifold review request $SPEC_ID --reviewer tech-lead@example.com
```

### 4. TUI for Conflict Resolution
```bash
# For complex conflicts with many changes, use TUI
manifold tui

# Benefits:
# - Visual side-by-side comparison
# - Quick strategy selection
# - Immediate feedback
# - No need to remember conflict IDs
```

### 5. Automated Workflows
```bash
#!/bin/bash
# auto-sync.sh - Run periodically

# Pull latest
manifold sync pull all

# Push any local changes
manifold sync push all --message "Auto-sync: $(date)"

# Check for pending reviews
manifold review list --status pending
```

## Architecture

### Sync Storage
- Specs are exported as JSON files in the sync repository
- Each spec: `{spec-id}.json`
- Git handles versioning and merging
- Manifold detects conflicts and provides resolution tools

### Conflict Detection
- Three-way merge analysis (base, local, remote)
- Field-level conflict detection
- Automatic merge for non-conflicting changes
- Manual resolution for complex conflicts

### Review Tracking
- Reviews stored in SQLite database
- Linked to specific specs
- Track requester, reviewer, status, timestamps
- Comments preserved for audit trail

## See Also

- `manifold sync --help`
- `manifold review --help`
- `manifold conflicts --help`
- [ENHANCEMENTS.md](ENHANCEMENTS.md) - Future collaboration features
