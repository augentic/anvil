# Augentic Plugins - Agent Instructions

## Cursor Cloud specific instructions

This is a **documentation/prompt-engineering repository**. The codebase consists of markdown skill definitions, reference docs, templates, and shell scripts. Generated Rust crates appear in downstream projects, not in this repository itself.

### Workflow overview

Humans are expected to work through stock Specify:

- `/spec:init` (once per project)
- `/spec:propose`
- `/spec:apply`
- `/spec:archive`
- `/spec:abandon`
- `/spec:verify` (detect drift between code and baseline specs)

This repository provides specialist skills and references that support that workflow.

### Commands

All commands are run from the repository root:

- **`make checks`** -- runs `./scripts/checks.sh` for documentation and workflow consistency checks
- **`make dev-plugins`** -- symlink local plugins into Cursor for development/testing
- **`make prod-plugins`** -- restore Augentic marketplace plugins (reload Cursor after either)

### Gotchas

- In a fresh clone, run `/spec:init` before using other `/spec:*` commands. The workflow skills expect the `.specify/` project structure to exist.
- `checks.sh` enforces documentation consistency; if you remove or rename workflow terms, update the checks in the same change.
- Some skills use symlinks to share reference documents from `plugins/references/`. If a symlink target is removed, the skill's documentation may reference content that no longer resolves.
