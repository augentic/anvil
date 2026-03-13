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

Both schemas share the same artifact templates:
- `proposal.md`: Change proposal
- `spec.md`: Behavioral specification (delta format)
- `design.md`: Technical design document
- `tasks.md`: Implementation checklist

## Configuration

The active schema is defined in `.specify/config.yaml`:

```yaml
schema: omnia # or realtime
```
