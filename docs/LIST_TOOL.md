# The List Tool — Docs Summary

This note summarizes the existing documentation files in `docs/` and provides a short, actionable overview of what each file contains and suggested next steps for someone onboarding or continuing work on the repository.

Summary of docs in this folder

- `COLLABORATION.md`
  - Describes the collaboration model, review and sync workflows, and high-level guidance for contributors. Useful for understanding how specs are created, shared, and synced across boundaries.

- `COLLABORATION_IMPLEMENTATION.md`
  - Implementation notes that map collaboration processes to repository components (DB schema, sync manager, conflict resolver, review manager). Good reference for contributors implementing collaboration features.

- `ENHANCEMENTS.md`
  - A backlog/roadmap of enhancement ideas and planned improvements. Includes higher-level design notes and feature suggestions.

- `PHASE11_SUMMARY.md`
  - Phase-based project notes; likely contains a summary of Phase 1 work, outcomes, and open items. Useful for historical context and planning further phases.

- `README.md`
  - Project overview, quick start, key commands, and environment variables. The central entrypoint for new users.

- `TUI_CONFLICTS.md`
  - Design and usage notes specifically for TUI conflict-viewing and resolution flows. Explains how conflicts are surfaced in the dashboard and how users should resolve them.

- `TUI_ENHANCEMENTS.md`
  - Ideas and TODOs for improving the TUI (UX refinements, new panels, performance notes).

- `TUI_QUICK_REFERENCE.md`
  - Handy quick reference for TUI keyboard shortcuts, panels, and common workflows.

Overall context (brief)

- The docs together describe collaboration, conflict resolution, the TUI dashboard, and project roadmap/phase notes. They complement the code by describing intended user flows and design decisions.
- Key developer-facing docs: `COLLABORATION_IMPLEMENTATION.md`, `TUI_CONFLICTS.md`, and `README.md` — these are the best starting points when changing collaboration, sync, or TUI code.

Suggested next actions

- Keep the `README.md` up to date with the latest CLI commands (especially the new `agent` CLI) and environment variables (`GITHUB_TOKEN`, `OPENAI_API_KEY`).
- Add a short note in `COLLABORATION_IMPLEMENTATION.md` documenting the new MCP-based agent control flow and the `agent/start`, `agent/stop`, `agent/list` MCP tools.
- Move any operational instructions for agents (how to start MCP server, how CLI forwards to MCP) to `README.md` or a new `OPERATIONS.md` so operators know how to run MCP and control agents.
- Consider adding a small architecture diagram or sequence in `PHASE11_SUMMARY.md` showing how CLI -> commands -> MCP -> AgentManager interact.

If you want, I can:
- Add the MCP/agent notes into `COLLABORATION_IMPLEMENTATION.md` in-place.
- Update `README.md` with the new agent CLI usage and required env vars.
- Create an `OPERATIONS.md` with step-by-step run commands for serving the MCP and managing agents.

Document created: `/Users/tkoval/git-local/tk/manifold/docs/LIST_TOOL.md`