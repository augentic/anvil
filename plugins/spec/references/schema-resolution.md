# Schema Resolution

Skills resolve the `schema` field from `.specify/config.yaml` (or `.metadata.yaml`) to locate schema files. This document defines the resolution algorithm used by all spec skills.

## Inputs

- **`$SCHEMA_VALUE`**: the `schema` field value (a name or URL)
- **`$FILES_NEEDED`**: which files the calling skill requires (e.g., `schema.yaml`, `config.yaml`, `instructions/*`)

## URL Format

Schema URLs support an optional `@ref` suffix to pin a specific git ref
(branch, tag, or commit):

```text
https://github.com/{owner}/{repo}/schemas/{name}
https://github.com/{owner}/{repo}/schemas/{name}@{ref}
```

Examples:

```yaml
schema: https://github.com/augentic/specify/schemas/omnia          # defaults to main
schema: https://github.com/augentic/specify/schemas/omnia@main     # explicit branch
schema: https://github.com/augentic/specify/schemas/omnia@v1       # pinned to tag
schema: https://github.com/augentic/specify/schemas/omnia@abc123   # pinned to commit
```

When no `@ref` is present, `main` is used as the default ref.

## Algorithm

1. **Parse the schema value**

   - If `$SCHEMA_VALUE` contains no `/` (bare name like `omnia`):
     set `$NAME = $SCHEMA_VALUE`, `$REF = main` → local resolution only
     (step 2).
   - If `$SCHEMA_VALUE` contains `/` (URL):
     - Split on `@` — the part before `@` is the URL path, the part after
       is `$REF` (default `main` if no `@` present).
     - Extract `$NAME` from the last path segment of the URL path.
     - Skip local resolution — go directly to step 3 (cache check).

2. **Local resolution** (bare name only)

   This step only runs when `$SCHEMA_VALUE` is a bare name (no `/`).
   URL-based schemas skip this step to ensure deterministic pinning.

   Check if `schemas/$NAME/` exists relative to the workspace root
   (i.e., the root of the repository that contains the schema definitions).
   If found, use the local directory for all `$FILES_NEEDED`. Done.

   If not found and `$SCHEMA_VALUE` is a bare name, stop and report an
   error — bare names cannot fall through to remote resolution.

   > **Note**: Bare-name resolution is a development convenience for
   > working within the specify repository itself. Downstream projects
   > should use URL-based schemas (with optional `@ref` pinning) to
   > ensure reproducible resolution across machines.

3. **Cache check** (skip for init)

   If `.specify/.cache/.cache-meta.yaml` exists, read it:

   ```yaml
   schema_url: https://github.com/augentic/specify/schemas/omnia@v1
   fetched_at: 2026-03-13T10:30:00Z
   ```

   If `schema_url` matches `$SCHEMA_VALUE` exactly, use the cached files
   from `.specify/.cache/` for all `$FILES_NEEDED`. Done.

   If `schema_url` does not match (schema URL changed in config), the
   cache is stale — proceed to step 4 to refetch.

4. **Remote resolution** (URL, no valid cache)

   Construct raw content URLs using `$REF`:

   ```text
   https://raw.githubusercontent.com/<owner>/<repo>/$REF/<path>/<file>
   ```

   Fetch each file in `$FILES_NEEDED` via **WebFetch**.

   If any fetch fails, stop and report the error — do not fall back to
   defaults or inline content.

   **Populate the cache**: write fetched files to `.specify/.cache/`
   mirroring the schema directory structure:

   ```text
   .specify/.cache/
   ├── .cache-meta.yaml
   ├── schema.yaml
   ├── config.yaml          (if fetched)
   └── instructions/        (if fetched)
       ├── proposal.md
       ├── specs.md
       ├── design.md
       ├── tasks.md
       └── apply.md
   ```

   Write `.cache-meta.yaml` with:
   - `schema_url`: the full `$SCHEMA_VALUE` (including `@ref` if present)
   - `fetched_at`: current ISO-8601 timestamp

## Schema Composition

Schemas can extend other schemas using the `extends` field:

```yaml
name: omnia-secure
extends: https://github.com/augentic/specify/schemas/omnia
```

When `extends` is present, the resolution algorithm first resolves the
parent schema using the same algorithm (local → cache → remote), then
merges the child on top.

### Merge Rules

- **`artifacts`**: child artifacts with the same `id` override the parent
  entirely; new `id`s are appended to the parent's list. Dependency order
  is recomputed from the merged `requires` graph.
- **`terminology`**: child replaces parent. If omitted, inherits the
  parent's `terminology`. Contains only `unit` (skills infer plural and
  heading forms).
- **`cross-artifact-checks`**: child replaces parent entirely (simple
  string list). If omitted, inherits the parent's checks.
- **`apply`**: child `requires` replaces parent `requires`; child
  `instruction` replaces parent `instruction`; child `tracks` replaces
  parent `tracks`. Omitted fields inherit from parent.
- **`instructions/`**: resolve from the child schema directory first;
  fall back to the parent schema directory for any files not present in
  the child.
- **All other top-level fields** (`name`, `version`, `description`): child
  replaces parent. These are identity fields and should always be declared
  in the child.
- **Circular `extends` chains** are an error — stop and report.

### Resolution Example

Given `omnia-secure` extends `omnia`:

1. Resolve `omnia` (parent) → yields base `schema.yaml`, `instructions/*`
2. Resolve `omnia-secure` (child) → yields override `schema.yaml`
3. Merge: parent artifacts + child artifacts (override by `id`, append new)
4. For file reads: check child directory first, fall back to parent

## Resolution Modes

The schema value format determines the resolution path:

| Format              | Example                                      | Resolution                                       |
|---------------------|----------------------------------------------|--------------------------------------------------|
| Bare name           | `schema: omnia`                              | Local `schemas/omnia/` only. For development.    |
| URL (default ref)   | `schema: https://github.com/.../omnia`       | Cache, then remote fetch at `main`.              |
| URL with pinned ref | `schema: https://github.com/.../omnia@v1`    | Cache, then remote fetch at `v1`.                |

Bare names always resolve locally and never reach the network. URLs always
resolve via cache or remote and never use a local `schemas/` directory,
even if one exists with the same name. This guarantees that a pinned URL
produces the same schema across machines and branches.

## Cache Notes

- The `.specify/.cache/` directory should be gitignored. The `init` skill
  creates this directory and adds it to `.gitignore` if needed.
- Cache invalidation is automatic: when the `schema` value in
  `.specify/config.yaml` changes, the cached `schema_url` no longer
  matches, triggering a refetch.
- To force a refetch, delete `.specify/.cache/` and run any skill that
  resolves the schema.
- The `init` skill does **not** use the cache (it creates the project
  structure from scratch and only needs `config.yaml`).

## What Each Skill Needs

| Skill   | Files needed                                          |
|---------|-------------------------------------------------------|
| init    | `config.yaml`                                         |
| propose | `schema.yaml`, `config.yaml`, `instructions/*`        |
| apply   | `schema.yaml`, `config.yaml`, `instructions/apply.md` |
| archive | `schema.yaml`                                         |
| abandon | `schema.yaml`                                         |
| verify  | `schema.yaml`                                         |
| explore | `schema.yaml`                                         |
| status  | `schema.yaml`                                         |
