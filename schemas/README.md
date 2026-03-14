# Specify Schemas

This directory contains the schema definitions for the Specify workflow. Each
schema provides artifact declarations (`schema.yaml`), artifact instructions
(`instructions/`), and a starter config (`config.yaml`).

## Schemas

### `omnia`

- **URL**: `https://github.com/augentic/specify/schemas/omnia`
- **Purpose**: Rust WASM development (greenfield or migration)
- **Source**: JIRA Epic (`/plan:epic-analyzer`), Git Repository (`/rt:code-analyzer`), or Manual
- **Target**: Rust WASM (Omnia SDK)
- **Workflow**: `propose` -> `specs` (from Epic, Code, or Manual) -> `design` -> `tasks` -> `apply` (crate-writer)

## Schema Directory Structure

A base schema directory contains all files. A child schema that uses `extends`
may omit files that are inherited from the parent (the resolution algorithm
falls back to the parent directory for missing files).

```text
schemas/<name>/
├── schema.yaml      # Artifact declarations, terminology, apply config
├── config.yaml      # Starter config installed by /spec:init
└── instructions/    # Detailed instructions for each artifact and apply
    ├── proposal.md
    ├── specs.md
    ├── design.md
    ├── tasks.md
    └── apply.md
```

Child schemas that use `extends` may omit the entire `instructions/` directory
or individual files within it. Missing files are resolved from the parent schema
via fallback.

- `**schema.yaml**`: Declares artifacts (id, instruction file path,
dependencies, validation rules), `terminology` (the `unit` name,
e.g., "crate"), and the `apply` configuration.
Child schemas may use `extends` to inherit from a parent and only override
what differs. Skills read this to know how to generate artifacts and
implement tasks.
- `**instructions/**`: One markdown file per artifact plus `apply.md`.
Contains the detailed generation or implementation instructions including
output structure. Referenced by file path from `schema.yaml`'s
`instruction` field.
- `**config.yaml**`: Installed into `.specify/config.yaml` by `/spec:init`.
Contains the `schema` URL, default `context`, and per-artifact `rules`.

## Schema Resolution

Skills resolve the `schema` field from `.specify/config.yaml` to locate
schema files. The resolution algorithm is defined in
`plugins/spec/references/schema-resolution.md`. The `schema` value can be a
name or a URL.

### URL Format

Schema URLs support an optional `@ref` suffix to pin a specific git ref:

```text
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

### Schema Composition

Schemas can extend other schemas using the `extends` field in `schema.yaml`.
See `plugins/spec/references/schema-resolution.md` for the full composition
rules, including artifact merging, field-level overrides, and file fallback
behavior.

## Caching

When a schema is resolved remotely, fetched files are cached at the project
level in `.specify/.cache/`:

```text
.specify/.cache/
├── .cache-meta.yaml     # schema_url + fetched_at
├── schema.yaml
├── config.yaml          (if fetched)
└── instructions/        (if fetched)
    ├── proposal.md
    ├── specs.md
    ├── design.md
    ├── tasks.md
    └── apply.md
```

The cache is valid as long as `schema_url` in `.cache-meta.yaml` matches the
`schema` field in `.specify/config.yaml`. When the schema URL changes (e.g.,
bumping from `@v1` to `@v2`), the cache is automatically invalidated and
refetched on the next skill invocation.

The `/spec:init` skill creates `.specify/.cache/` and adds it to
`.specify/.gitignore`. To force a refetch, delete `.specify/.cache/`.

## Configuration

The active schema is defined in `.specify/config.yaml` as a URL:

```yaml
schema: https://github.com/augentic/specify/schemas/omnia
```

The `/spec:init` skill installs the schema's `config.yaml` into
`.specify/config.yaml`. Users customize `context` and `rules` after
initialization.