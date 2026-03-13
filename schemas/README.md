# Specify Schemas

This directory contains the schema definitions for the Specify workflow.

## Schemas

### `omnia`

- **Purpose**: Greenfield development (JIRA -> Rust WASM)
- **Source**: JIRA Epic (`/plan:epic-analyzer`) or Manual
- **Target**: Rust WASM (Omnia SDK)
- **Workflow**: `propose` -> `specs` (from Epic) -> `design` (from Epic) -> `tasks` -> `apply` (crate-writer)

### `realtime`

- **Purpose**: Migration (TypeScript -> Rust WASM)
- **Source**: Git Repository (`/rt:code-analyzer`) or Manual
- **Target**: Rust WASM (Omnia SDK)
- **Workflow**: `propose` -> `specs` (from Code) -> `design` (from Code) -> `tasks` -> `apply` (crate-writer)

## Templates

Each schema has its own `templates/` directory. The `spec.md`, `design.md`,
and `tasks.md` templates share the same structure across schemas. The
`proposal.md` templates differ:

- **Omnia**: uses "Crates" (New Crates / Modified Crates); Source supports
  Repository, Epic, and Manual.
- **Realtime**: uses "Capabilities" (New Capabilities / Modified
  Capabilities); Source supports Repository and Manual.

Schema instructions reference `references/specify.md` for artifact guidance.
This path resolves to `plugins/references/specify.md` in the skill execution
context (where symlinks map `references/` to the correct location).

## Configuration

The active schema is defined in `.specify/config.yaml`:

```yaml
schema: omnia # or realtime
```
