# Specify Schemas

This directory contains the published, selectable schema definitions for the Specify workflow. Each schema provides a pipeline of brief references and default domain context within `schema.yaml` and `briefs/`.

## Schemas

| Schema | Purpose | Details |
|--------|---------|---------|
| [`omnia`](omnia/README.md) | Rust WASM development (Omnia SDK) | Greenfield or migration via Git Repository or Manual |
| [`vectis`](vectis/README.md) | Cross-platform Crux app development | Rust core, iOS/Android shells, design system |
| [`contracts`](contracts/README.md) | API contract generation and validation | JSON Schema, OpenAPI 3.1, AsyncAPI 3.0 |

## Schema Directory Structure

A base schema directory contains all files. A child schema that uses `extends` may omit files that are inherited from the parent (the resolution algorithm falls back to the parent directory for missing files).

Child schemas that use `extends` may omit the entire `briefs/` directory or individual files within it. Missing files are resolved from the parent schema via fallback.

- **`schema.yaml`**: Declares the pipeline (define, build, merge phases), each referencing a brief by file path, plus a `domain` string describing the default project context (tech stack, architecture, testing approach). Child schemas may use `extends` to inherit from a parent and only override what differs. Skills read this to know how to generate artifacts and implement tasks.
- **`briefs/`**: One markdown file per pipeline entry. Each brief has YAML frontmatter declaring its `id`, `description`, `generates` pattern, and `needs` dependencies. The body contains detailed generation or implementation instructions. Referenced by file path from `schema.yaml`'s pipeline entries.

## Supporting JSON Schemas

The JSON Schemas that validate `schema.yaml`, brief frontmatter, `.specify/plan.yaml`, and CLI JSON output live with their owners:

- `specify-cli/schemas/` owns CLI and workflow metadata schemas such as `schema.schema.json`, `brief/schema.json`, `plan/plan.schema.json`, and `plan-validate-output/schema.json`.
- `.cursor/schemas/` owns this repository's local authoring checks, including `skill.schema.json`.

## Schema File Reference

### `schema.yaml`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Schema identifier (e.g., `omnia`) |
| `version` | integer | yes | Schema version number |
| `description` | string | yes | Human-readable description |
| `extends` | string | no | URL of parent schema for composition (see Schema Composition) |
| `domain` | string | no | Default project context (tech stack, architecture, testing approach) |
| `pipeline` | object | yes | Pipeline phases with brief references (see Pipeline below) |

The `pipeline` object has three keys — `define`, `build`, and `merge` — each containing an ordered array of pipeline entries.

**Pipeline entry fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | yes | Brief identifier (e.g., `proposal`, `specs`, `design`, `tasks`, `build`, `merge`) |
| `brief` | string | yes | Relative path to the brief markdown file with YAML frontmatter |

**Brief frontmatter fields:**

Each brief markdown file begins with YAML frontmatter containing metadata that was previously declared inline in `schema.yaml`. The body of the brief contains the detailed instructions.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | yes | Brief identifier, matching the pipeline entry `id` |
| `description` | string | yes | What this brief produces or does |
| `generates` | string | no | Output filename or glob pattern (e.g., `proposal.md`, `specs/**/*.md`) |
| `needs` | array of strings | no | Brief IDs that must be complete before this one can run |
| `tracks` | string | no | Brief ID whose output file tracks build progress (build briefs only) |

### Project Config (`.specify/project.yaml`)

Created by `/spec:init` in the project directory. This is the project-level configuration file — it does not exist in the schema directory.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Project name |
| `domain` | string | no | Project-specific domain context (tech stack, architecture, etc.) |
| `schema` | string | yes | Schema URL or bare name (see Schema Resolution) |
| `rules` | object | no | Per-brief rule overrides as file paths keyed by brief `id` |

The project config is a thin overlay. When `domain` is left empty it falls back to the schema's `domain` automatically. Rules are file-path references to markdown files containing additional guidance for a specific brief.

## Schema Resolution

Skills resolve the `schema` field from `.specify/project.yaml` to locate schema files. The resolution algorithm is defined in `plugins/spec/references/schema-resolution.md`. The `schema` value can be a name or a URL.

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

1. Split on `@` to extract the schema name (last path segment) and ref (default `main`).
2. Check the project-level cache at `.specify/.cache/` (see Caching below).
3. If no valid cache, fetch files via WebFetch (for GitHub URLs, convert to raw content URLs using the extracted ref: `https://raw.githubusercontent.com/<owner>/<repo>/<ref>/<path>`).

URL schemas skip local resolution entirely to guarantee that a pinned URL produces the same schema across machines and branches.

### Schema Composition

Schemas can extend other schemas using the `extends` field in `schema.yaml`. See `plugins/spec/references/schema-resolution.md` for the full composition rules, including pipeline merging, field-level overrides, and file fallback behavior.

## Caching

When a schema is resolved remotely, fetched files are cached at the project level in `.specify/.cache/`:

```text
.specify/.cache/
├── .cache-meta.yaml     # schema_url + fetched_at
├── schema.yaml
└── briefs/              (if fetched)
    ├── proposal.md
    ├── specs.md
    ├── composition.md   # Vectis only
    ├── design.md
    ├── tasks.md
    ├── build.md
    └── merge.md
```

The cache is valid as long as `schema_url` in `.cache-meta.yaml` matches the `schema` field in `.specify/project.yaml`. When the schema URL changes (e.g., bumping from `@v1` to `@v2`), the cache is automatically invalidated and refetched on the next skill invocation.

The `/spec:init` skill creates `.specify/.cache/` and adds it to `.specify/.gitignore`. To force a refetch, delete `.specify/.cache/`.

## Configuration

The active schema is defined in `.specify/project.yaml` as a URL:

```yaml
schema: https://github.com/augentic/specify/schemas/omnia
```

The `/spec:init` skill creates `.specify/project.yaml` with the `name`, `schema`, and scaffolded `domain` and `rules` keys. Users customize these after initialization to provide project-specific context and rule overrides.

## Rules

The schema's brief files contain default guidance in their body text. Projects can supplement or replace this guidance on a per-brief basis using file-path rules in `.specify/project.yaml`.

The override granularity is **per-brief key**. If the project's `.specify/project.yaml` defines a non-empty value for `rules.<brief-id>`, that value is a relative file path to a markdown file containing additional rules for that brief. Brief IDs that are absent or empty in the project config use the schema brief's body text as-is.

For example, to add project-specific rules for the `proposal` brief:

```yaml
rules:
  proposal: rules/proposal.md
```

The file at `rules/proposal.md` (relative to `.specify/`) contains the additional guidance. Only `proposal` gets supplemental rules; all other briefs continue to use their schema defaults.

Skills that consume rules (define, build) read the brief body text at runtime and merge any project-level rule file for the corresponding brief.
