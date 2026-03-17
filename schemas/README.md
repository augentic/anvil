# Specify Schemas

This directory contains the schema definitions for the Specify workflow. Each
schema provides artifact declarations, default context and rules, and artifact
instructions — all within `schema.yaml` and `instructions/`.

## Schemas

| Schema | Purpose | Details |
|--------|---------|---------|
| [`omnia`](omnia/README.md) | Rust WASM development (Omnia SDK) | Greenfield or migration via JIRA Epic, Git Repository, or Manual |

## Schema Directory Structure

A base schema directory contains all files. A child schema that uses `extends`
may omit files that are inherited from the parent (the resolution algorithm
falls back to the parent directory for missing files).

```text
schemas/<name>/
├── schema.yaml      # Blueprint declarations, terminology, build config, defaults
└── instructions/    # Detailed instructions for each blueprint and build
    ├── proposal.md
    ├── specs.md
    ├── design.md
    ├── tasks.md
    └── build.md
```

Child schemas that use `extends` may omit the entire `instructions/` directory
or individual files within it. Missing files are resolved from the parent schema
via fallback.

- `**schema.yaml**`: Declares blueprints (id, instructions file path,
dependencies, validation rules), `terminology` (the `deliverable` name,
e.g., "crate"), the `build` configuration, and `defaults` (default
`context` and per-blueprint `rules`). Child schemas may use `extends` to
inherit from a parent and only override what differs. Skills read this to
know how to generate blueprints and implement tasks.
- `**instructions/**`: One markdown file per blueprint plus `build.md`.
Contains the detailed generation or implementation instructions including
output structure. Referenced by file path from `schema.yaml`'s
`instructions` field.

## Schema File Reference

### `schema.yaml`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Schema identifier (e.g., `omnia`) |
| `version` | integer | yes | Schema version number |
| `description` | string | yes | Human-readable description |
| `extends` | string | no | URL of parent schema for composition (see Schema Composition) |
| `terminology` | object | yes | Vocabulary used by skills |
| `terminology.deliverable` | string | yes | Deliverable noun (e.g., `crate`); skills infer plural and heading forms |
| `blueprints` | array | yes | Ordered list of blueprint declarations |
| `validation` | object | no | Cross-blueprint boolean validation flags (keys are rule names, values are booleans) |
| `build` | object | yes | Build-phase configuration |
| `defaults` | object | no | Default context and per-blueprint rules (see Defaults below) |

**Blueprint object fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | yes | Blueprint identifier (e.g., `proposal`, `specs`, `design`, `tasks`) |
| `generates` | string | yes | Output filename or glob pattern (e.g., `proposal.md`, `specs/**/*.md`) |
| `description` | string | yes | What this blueprint produces |
| `instructions` | string | yes | Relative path to the instructions markdown file |
| `requires` | array of strings | yes | Blueprint IDs that must exist before this one can be generated |
| `validate` | array of strings | no | Human-readable validation rules checked after generation |

**Build object fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `requires` | array of strings | yes | Blueprint IDs that must be complete before build runs |
| `tracks` | string | yes | File that tracks build progress (e.g., `tasks.md`) |
| `instructions` | string | yes | Relative path to the build instructions markdown file |

**Defaults object fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `defaults.context` | string | no | Default project context (tech stack, architecture, testing approach) |
| `defaults.rules` | object | no | Default per-blueprint generation rules keyed by blueprint `id` |

Each key under `defaults.rules` is a blueprint `id` whose value is a
multi-line string of generation rules. See Rules Override below for
merge semantics.

### Project Config (`.specify/config.yaml`)

Created by `/spec:init` in the project directory. This is the project-level
override file — it does not exist in the schema directory.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `schema` | string | yes | Schema URL or bare name (see Schema Resolution) |
| `context` | string | no | Project-specific context override (tech stack, architecture, etc.) |
| `rules` | object | no | Per-blueprint rule overrides keyed by blueprint `id` |

The project config is a thin overlay. Keys left empty or as placeholders
fall back to the schema's `defaults` automatically.

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

- Look for `schemas/<name>/` relative to the workspace root.

**URL resolution** (e.g., `schema: https://github.com/augentic/specify/schemas/omnia@v1`):

1. Split on `@` to extract the schema name (last path segment) and ref
  (default `main`).
2. Check the project-level cache at `.specify/.cache/` (see Caching below).
3. If no valid cache, fetch files via WebFetch (for GitHub URLs, convert to
  raw content URLs using the extracted ref:
   `https://raw.githubusercontent.com/<owner>/<repo>/<ref>/<path>`).

URL schemas skip local resolution entirely to guarantee that a pinned URL
produces the same schema across machines and branches.

### Schema Composition

Schemas can extend other schemas using the `extends` field in `schema.yaml`.
See `plugins/spec/references/schema-resolution.md` for the full composition
rules, including blueprint merging, field-level overrides, and file fallback
behavior.

## Caching

When a schema is resolved remotely, fetched files are cached at the project
level in `.specify/.cache/`:

```text
.specify/.cache/
├── .cache-meta.yaml     # schema_url + fetched_at
├── schema.yaml
└── instructions/        (if fetched)
    ├── proposal.md
    ├── specs.md
    ├── design.md
    ├── tasks.md
    └── build.md
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

The `/spec:init` skill creates `.specify/config.yaml` with the `schema`
value and scaffolded `context` and `rules` keys. Users customize these
after initialization to override schema defaults.

## Rules Override

The schema's `defaults.rules` section in `schema.yaml` provides default
rules for each blueprint (e.g., `proposal`, `specs`, `design`, `tasks`).

The override granularity is **per-blueprint key**. If the project's
`.specify/config.yaml` defines a non-empty value for `rules.<blueprint-id>`,
that value replaces the schema default for that blueprint. Blueprint keys
that are absent or empty in the project config fall back to the schema
default automatically.

For example, to override the `specs` rules while keeping the schema
defaults for all other blueprints:

```yaml
rules:
  specs: |
    - Use GIVEN/WHEN/THEN format for scenarios
    - Include performance benchmarks in every scenario
```

Only `specs` is overridden; `proposal`, `design`, and `tasks` continue to
use the schema defaults.

Skills that consume rules (define, build) read the schema's
`defaults.rules` at runtime and apply this fallback per blueprint.
