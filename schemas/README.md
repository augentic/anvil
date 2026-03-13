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
schema files. The resolution algorithm is defined in
`plugins/spec/references/schema-resolution.md`. The `schema` value can be a
name or a URL.

### URL Format

Schema URLs support an optional `@ref` suffix to pin a specific git ref:

```
https://github.com/{owner}/{repo}/schemas/{name}
https://github.com/{owner}/{repo}/schemas/{name}@{ref}
```

When no `@ref` is present, `main` is used as the default ref. Examples:

```yaml
schema: https://github.com/augentic/specify/schemas/omnia          # defaults to main
schema: https://github.com/augentic/specify/schemas/omnia@v1       # pinned to tag
schema: https://github.com/augentic/specify/schemas/omnia@abc123   # pinned to commit
```

### Resolution Order

**Name resolution** (e.g., `schema: omnia`):
- Look for `schemas/<name>/` in the plugin directory.

**URL resolution** (e.g., `schema: https://github.com/augentic/specify/schemas/omnia@v1`):
1. Split on `@` to extract the schema name (last path segment) and ref
   (default `main`).
2. Check if `schemas/<name>/` exists locally in the plugin directory.
3. If found locally, use the local directory.
4. If not found locally, check the project-level cache at
   `.specify/.cache/` (see Caching below).
5. If no valid cache, fetch files via WebFetch (for GitHub URLs, convert to
   raw content URLs using the extracted ref:
   `https://raw.githubusercontent.com/<owner>/<repo>/<ref>/<path>`).

## Caching

When a schema is resolved remotely, fetched files are cached at the project
level in `.specify/.cache/`:

```text
.specify/.cache/
├── .cache-meta.yaml     # schema_url + fetched_at
├── schema.yaml
├── config.yaml          (if fetched)
└── templates/           (if fetched)
    ├── proposal.md
    ├── spec.md
    ├── design.md
    └── tasks.md
```

The cache is valid as long as `schema_url` in `.cache-meta.yaml` matches the
`schema` field in `.specify/config.yaml`. When the schema URL changes (e.g.,
bumping from `@v1` to `@v2`), the cache is automatically invalidated and
refetched on the next skill invocation.

The `/spec:init` skill creates `.specify/.cache/` and adds it to
`.specify/.gitignore`. To force a refetch, delete `.specify/.cache/`.

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
