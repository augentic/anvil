# Specify Schemas

This directory contains the schema definitions for the Specify workflow. Each
schema provides artifact definitions (`schema.yaml`), a starter config
(`config.yaml`), and artifact templates (`templates/`).

## Schemas

### `omnia`

- **URL**: `https://github.com/augentic/specify/schemas/omnia`
- **Purpose**: Greenfield development (JIRA -> Rust WASM)
- **Source**: JIRA Epic (`/plan:epic-analyzer`) or Manual
- **Target**: Rust WASM (Omnia SDK)
- **Workflow**: `propose` -> `specs` (from Epic) -> `design` (from Epic) -> `tasks` -> `apply` (crate-writer)

### `realtime`

- **URL**: `https://github.com/augentic/specify/schemas/realtime`
- **Purpose**: Migration (TypeScript -> Rust WASM)
- **Source**: Git Repository (`/rt:code-analyzer`) or Manual
- **Target**: Rust WASM (Omnia SDK)
- **Workflow**: `propose` -> `specs` (from Code) -> `design` (from Code) -> `tasks` -> `apply` (crate-writer)

## Schema Directory Structure

Each schema directory contains:

```text
schemas/<name>/
├── schema.yaml      # Artifact definitions, instructions, apply instruction
├── config.yaml      # Starter config installed by /spec:init
└── templates/       # Artifact templates
    ├── proposal.md
    ├── spec.md
    ├── design.md
    └── tasks.md
```

- **`schema.yaml`**: Defines the artifacts (id, template filename,
  instruction, dependencies) and the apply instruction. Skills read this to
  know how to generate artifacts and implement tasks.
- **`config.yaml`**: Installed into `.specify/config.yaml` by `/spec:init`.
  Contains the `schema` URL, default `context`, and per-artifact `rules`.
- **`templates/`**: Markdown templates for each artifact. Referenced by
  filename in `schema.yaml`.

## Schema Resolution

Skills resolve the `schema` field from `.specify/config.yaml` to locate
schema files. The `schema` value can be a name or a URL.

**Name resolution** (e.g., `schema: omnia`):
- Look for `schemas/<name>/` in the plugin directory.

**URL resolution** (e.g., `schema: https://github.com/augentic/specify/schemas/omnia`):
1. Extract the schema name from the last path segment of the URL.
2. Check if `schemas/<name>/` exists locally in the plugin directory.
3. If found locally, use the local directory.
4. If not found locally, fetch files via WebFetch (for GitHub URLs, convert
   to raw content URLs:
   `https://raw.githubusercontent.com/<owner>/<repo>/main/<path>`).

## Templates

The `spec.md`, `design.md`, and `tasks.md` templates share the same structure
across schemas. The `proposal.md` templates differ:

- **Omnia**: uses "Crates" (New Crates / Modified Crates); Source supports
  Repository, Epic, and Manual.
- **Realtime**: uses "Capabilities" (New Capabilities / Modified
  Capabilities); Source supports Repository and Manual.

Schema instructions reference `references/specify.md` for artifact guidance.
This path resolves to `plugins/references/specify.md` in the skill execution
context (where symlinks map `references/` to the correct location).

## Configuration

The active schema is defined in `.specify/config.yaml` as a URL:

```yaml
schema: https://github.com/augentic/specify/schemas/omnia
```

The `/spec:init` skill installs the schema's `config.yaml` into
`.specify/config.yaml`. Users customize `context` and `rules` after
initialization.
