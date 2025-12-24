# Manifold TUI - Quick Reference Card

## Conflict Resolution Keyboard Shortcuts

### Conflicts Tab Navigation
```
c        Load conflicts for selected spec
↑/↓      Navigate conflict list
Tab      Switch between tabs
r        Refresh spec list
q/Esc    Quit TUI
```

### Resolution Operations
```
o        Open resolution dialog for selected conflict
b        Bulk resolution (resolve all unresolved)
a        Auto-merge (intelligent merge attempt)
```

### Resolution Popup
```
←/→      Select strategy (Ours/Theirs/Merge/Manual)
Enter    Apply selected strategy
Esc      Cancel resolution
```

### Bulk Resolution Popup
```
←/→      Select strategy (Ours/Theirs/Merge)
Enter    Apply to all unresolved conflicts
Esc      Cancel bulk operation
```

### Manual Input Popup
```
Type     Enter custom value (JSON or text)
Backspace Delete last character
Enter    Apply manual value
Esc      Cancel manual input
```

## Resolution Strategies

| Strategy | Shortcut | Description |
|----------|----------|-------------|
| **Ours** | Keep Local | Uses local value, marks as ResolvedLocal |
| **Theirs** | Accept Remote | Uses remote value, marks as ResolvedRemote |
| **Merge** | Auto | Attempts intelligent merge, best for arrays |
| **Manual** | Custom | Opens text input for custom value |

## Quick Workflow

### Single Conflict Resolution
1. Tab to **Conflicts** tab
2. Press `c` to load conflicts
3. Use `↑/↓` to select conflict
4. Press `o` to open resolution dialog
5. Use `←/→` to select strategy
6. Press `Enter` to apply

### Manual Custom Value
1. Select conflict (steps 1-3 above)
2. Press `o` → select **Manual** → `Enter`
3. Type custom value
4. Press `Enter` to apply

### Auto-Merge All Compatible
1. Tab to **Conflicts** tab
2. Press `c` to load conflicts
3. Press `a` to auto-merge
4. Review results in status message

### Bulk Resolution
1. Tab to **Conflicts** tab
2. Press `c` to load conflicts
3. Press `b` to open bulk dialog
4. Select strategy (e.g., **Ours**)
5. Press `Enter` to resolve all

## Visual Indicators

### Conflict Status
```
⚠   Unresolved conflict (red)
✓   Resolved conflict (green)
```

### Statistics (in footer)
```
N/M unresolved
│
├─ N = number of unresolved conflicts
└─ M = total conflicts for current spec
```

### Diff Markers
```
BASE VALUE:
original-value

────────────────────── ← LOCAL (different)
local-modified-value

────────────────────── → REMOTE (different)
remote-modified-value
```

## CLI Commands

### View Conflicts
```bash
manifold conflicts list
```

### Resolve via CLI
```bash
manifold conflicts resolve <conflict-id> --strategy ours|theirs|merge
```

### View Spec After Resolution
```bash
manifold show <spec-id>
```

## Tips & Tricks

1. **Use auto-merge first** - Press `a` to automatically resolve compatible conflicts
2. **Check statistics** - Footer shows `N/M unresolved` to track progress
3. **Visual diffs** - Review LOCAL vs REMOTE values before resolving
4. **Bulk operations** - Use `b` to resolve all remaining conflicts at once
5. **Manual editing** - Enter JSON objects: `{"key": "value"}` or plain text

## Common Scenarios

### Scenario 1: Accept all remote changes
```
c → b → select "Theirs" → Enter
```

### Scenario 2: Keep all local changes
```
c → b → select "Ours" → Enter
```

### Scenario 3: Smart merge, manual for conflicts
```
c → a (auto-merge compatible)
→ o (for remaining) → select strategy
```

### Scenario 4: Custom resolution
```
c → ↑/↓ to select conflict
→ o → select "Manual" → Enter
→ type custom value → Enter
```

## Status Messages

### Success
```
✓ Conflict resolved with strategy: ours
✓ Bulk resolution complete: 5 resolved, 0 failed
✓ Auto-merge: 3 merged, 2 skipped, 0 failed
✓ Manual value applied successfully
```

### Errors
```
✗ Failed to resolve: <error message>
✗ Manual strategy not supported for bulk resolution
```

## Demo Environment

### Setup Test Conflicts
```bash
../demos/demo_phase11.sh
```

### Launch TUI
```bash
./target/release/manifold tui
```

### Cleanup
```bash
rm -rf ~/.manifold-demo-phase11 ~/.manifold-sync-phase11
```

## Documentation

- **Full documentation:** `TUI_ENHANCEMENTS.md`
- **Implementation details:** `TUI_CONFLICTS.md`
- **Collaboration guide:** `COLLABORATION.md`
- **Phase 11 summary:** `PHASE11_SUMMARY.md`

## Support

For issues or questions:
1. Check `TUI_ENHANCEMENTS.md` for detailed usage
2. Run `../demos/demo_phase11.sh` for working example
3. Review `COLLABORATION.md` for collaboration workflow

---

**Manifold TUI** - Local-first, MCP-native specification engine
