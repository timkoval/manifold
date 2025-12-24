# Manifold Documentation

Comprehensive documentation for the Manifold specification management system.

## Quick Links

- **[Main README](../README.md)** - Project overview and quick start
- **[Quick Reference](TUI_QUICK_REFERENCE.md)** - TUI keyboard shortcuts and commands
- **[Collaboration Guide](COLLABORATION.md)** - Git-based collaboration workflow

## Documentation Index

### User Guides

#### [COLLABORATION.md](COLLABORATION.md)
Complete guide to collaboration features including git sync, conflict resolution, and reviews.

**Topics:**
- Git-based sync workflow
- Push/pull specifications
- Conflict detection and resolution
- Review and approval process
- CLI examples and workflows

**Audience:** Users wanting to collaborate on specifications across teams.

---

#### [TUI_QUICK_REFERENCE.md](TUI_QUICK_REFERENCE.md)
Quick reference card for the Terminal UI.

**Topics:**
- Keyboard shortcuts (all tabs)
- Conflict resolution workflows
- Resolution strategies
- Visual indicators
- Common scenarios
- Tips and tricks

**Audience:** All TUI users, especially for quick lookup.

---

### Technical Documentation

#### [COLLABORATION_IMPLEMENTATION.md](COLLABORATION_IMPLEMENTATION.md)
Technical details of collaboration features implementation.

**Topics:**
- Architecture and design
- Database schema (sync_metadata, conflicts, reviews)
- Git integration details
- Three-way merge algorithm
- API reference

**Audience:** Developers and contributors.

---

#### [TUI_CONFLICTS.md](TUI_CONFLICTS.md)
TUI conflict resolution implementation summary.

**Topics:**
- Conflicts tab design
- Resolution popup implementation
- State management
- Event handling
- Code structure

**Audience:** Developers working on TUI features.

---

#### [TUI_ENHANCEMENTS.md](TUI_ENHANCEMENTS.md)
Phase 11 TUI enhancements - comprehensive feature guide.

**Topics:**
- Manual conflict editing
- Visual diff highlighting
- Bulk conflict resolution
- Auto-merge capability
- Conflict statistics
- Implementation details
- Performance considerations
- Future enhancements

**Audience:** Developers and power users.

---

#### [PHASE11_SUMMARY.md](PHASE11_SUMMARY.md)
Complete summary of Phase 11 implementation.

**Topics:**
- Features implemented
- Code statistics
- Build status
- Technical decisions
- Testing workflow
- Known limitations
- Deliverables

**Audience:** Project reviewers and contributors.

---

### Planning & Roadmap

#### [ENHANCEMENTS.md](ENHANCEMENTS.md)
Future enhancements, roadmap, and potential improvements.

**Topics:**
- Planned features (Phase 12+)
- Improvement ideas
- Community suggestions
- Long-term vision
- Integration opportunities

**Audience:** Contributors, users interested in future features.

---

## Documentation by Phase

Manifold development phases and their documentation:

### Phase 1-8: Foundation
See [Main README](../README.md) for:
- Core infrastructure (Phase 1)
- Schema validation (Phase 2)
- MCP server (Phase 3)
- Workflow engine (Phase 4)
- LLM editing (Phase 5)
- TUI dashboard (Phase 6)
- Markdown export (Phase 7)
- Docker deployment (Phase 8)

### Phase 9: Collaboration Features
- **[COLLABORATION.md](COLLABORATION.md)** - User guide
- **[COLLABORATION_IMPLEMENTATION.md](COLLABORATION_IMPLEMENTATION.md)** - Implementation

**Features:**
- Git sync (push/pull)
- Conflict detection
- Review workflow

### Phase 10: TUI Conflict Resolution
- **[TUI_CONFLICTS.md](TUI_CONFLICTS.md)** - Implementation summary

**Features:**
- Conflicts tab
- Resolution popup
- Visual indicators

### Phase 11: Enhanced TUI
- **[TUI_ENHANCEMENTS.md](TUI_ENHANCEMENTS.md)** - Feature guide
- **[TUI_QUICK_REFERENCE.md](TUI_QUICK_REFERENCE.md)** - Quick reference
- **[PHASE11_SUMMARY.md](PHASE11_SUMMARY.md)** - Complete summary

**Features:**
- Manual editing
- Visual diffs
- Bulk operations
- Auto-merge
- Statistics

---

## Documentation by Audience

### For End Users

**Getting Started:**
1. [Main README](../README.md) - Overview and installation
2. [TUI Quick Reference](TUI_QUICK_REFERENCE.md) - Learn keyboard shortcuts
3. [Demos](../demos/README.md) - Try interactive demos

**Collaboration:**
1. [Collaboration Guide](COLLABORATION.md) - Setup git sync
2. [TUI Quick Reference](TUI_QUICK_REFERENCE.md) - Resolve conflicts in TUI

### For Developers

**Understanding the Codebase:**
1. [Main README](../README.md) - Architecture overview
2. [Collaboration Implementation](COLLABORATION_IMPLEMENTATION.md) - Sync & conflicts
3. [TUI Conflicts](TUI_CONFLICTS.md) - TUI implementation
4. [TUI Enhancements](TUI_ENHANCEMENTS.md) - Advanced features

**Contributing:**
1. [Enhancements](ENHANCEMENTS.md) - See roadmap
2. [Phase 11 Summary](PHASE11_SUMMARY.md) - Study recent implementation
3. [Main README](../README.md) - Development setup

### For Project Managers

**Feature Overview:**
1. [Main README](../README.md) - Complete feature list
2. [Phase 11 Summary](PHASE11_SUMMARY.md) - Recent deliverables
3. [Enhancements](ENHANCEMENTS.md) - Future roadmap

**Demonstrations:**
- [Demos README](../demos/README.md) - All available demos
- Run `./demos/demo_phase11.sh` for latest features

---

## Document Formats

### Markdown (.md)
All documentation uses GitHub-flavored Markdown:
- Code blocks with syntax highlighting
- Tables for structured data
- Task lists for checklists
- Emojis for visual markers

### Examples
Most documents include:
- **Code examples** - Copy-paste ready
- **Command-line examples** - Real usage
- **Workflow diagrams** - ASCII art visualizations

---

## Finding Information

### Quick Lookup

**"How do I...?"**
- Resolve conflicts? → [TUI Quick Reference](TUI_QUICK_REFERENCE.md)
- Set up collaboration? → [Collaboration Guide](COLLABORATION.md)
- See keyboard shortcuts? → [TUI Quick Reference](TUI_QUICK_REFERENCE.md)

**"What's new?"**
- Latest features → [Phase 11 Summary](PHASE11_SUMMARY.md)
- Future plans → [Enhancements](ENHANCEMENTS.md)

**"How does it work?"**
- Git sync → [Collaboration Implementation](COLLABORATION_IMPLEMENTATION.md)
- Conflict resolution → [TUI Conflicts](TUI_CONFLICTS.md)
- TUI features → [TUI Enhancements](TUI_ENHANCEMENTS.md)

### Search Tips

```bash
# Find all references to a feature
grep -r "auto-merge" docs/

# Find keyboard shortcuts
grep -r "KeyCode" docs/

# Find CLI commands
grep -r "manifold " docs/
```

---

## Documentation Standards

When contributing documentation:

### Structure
- Start with **Overview** section
- Include **Quick Links** for navigation
- Use **headings** for hierarchy (##, ###, ####)
- Add **examples** for complex topics

### Style
- **Bold** for emphasis and UI elements
- `Code blocks` for commands and code
- > Blockquotes for notes and warnings
- Lists for sequential steps

### Code Examples
```bash
# Always include comments
manifold sync push <spec-id> --message "Update"

# Show expected output when helpful
# Output: ✓ Pushed to remote
```

### Cross-References
- Use relative links: `[Link](./FILE.md)`
- Link to specific sections: `[Link](./FILE.md#section)`
- Keep links up-to-date when renaming

---

## Contributing to Docs

### Adding New Documentation
1. Create file in `docs/` directory
2. Add entry to this README
3. Link from relevant documents
4. Include in appropriate section

### Updating Existing Docs
1. Read existing content first
2. Maintain consistent style
3. Update cross-references if needed
4. Test all code examples

### Documentation Checklist
- [ ] Clear title and overview
- [ ] Table of contents (for long docs)
- [ ] Code examples tested
- [ ] Links work
- [ ] Added to this index
- [ ] Spell-checked

---

## Maintenance

### Keeping Docs Current

**When adding features:**
1. Update relevant user guide
2. Add technical documentation
3. Update README.md
4. Add demo if applicable
5. Update this index

**When changing features:**
1. Update all affected docs
2. Update examples
3. Check cross-references
4. Update version info if applicable

### Documentation Review

Periodically review docs for:
- Outdated screenshots or examples
- Broken links
- Missing new features
- Unclear explanations

---

## Support

Can't find what you need?

1. **Check the demos:** `../demos/README.md`
2. **Try the quick reference:** `TUI_QUICK_REFERENCE.md`
3. **Review the main README:** `../README.md`
4. **Search all docs:** `grep -r "keyword" docs/`

---

**Manifold** - Local-first, MCP-native specification engine

*Documentation version: Phase 11*
