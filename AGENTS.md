# Augentic Plugins - Agent Instructions

## Cursor Cloud specific instructions

This is a **documentation/prompt-engineering repository**. Most of the codebase consists of markdown skill definitions, reference docs, templates, and shell scripts. Generated Rust crates may appear in downstream projects, not in this repository itself.

### Workflow overview

Humans are expected to work through stock Specify:

- `/spec:propose`
- `/spec:apply`
- `/spec:archive`

This repository provides specialist skills and references that support that workflow.

### Validation commands

All validation is run from the repository root:

- **`make checks`** — runs `./scripts/checks.sh` for documentation and workflow consistency checks
- **`./scripts/checks.sh`** — standalone documentation linting (requires `python3` and `bash`)

### Gotchas

- In a fresh clone, `specify init` must be run before `/spec:*` commands can operate in this repository.
- `checks.sh` enforces documentation consistency; if you remove or rename workflow terms, update the checks in the same change.
- Some skills use symlinks to share reference documents from `references/`. If a symlink target is removed, the skill's documentation may reference content that no longer resolves.
