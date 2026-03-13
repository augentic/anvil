# Schema Resolution

Skills resolve the `schema` field from `.specify/config.yaml` (or `.metadata.yaml`) to locate schema files. This document defines the resolution algorithm used by all spec skills.

## Inputs

- **`$SCHEMA_VALUE`**: the `schema` field value (a name or URL)
- **`$FILES_NEEDED`**: which files the calling skill requires (e.g., `schema.yaml`, `config.yaml`, `instructions/*`, `templates/*`)

## URL Format

Schema URLs support an optional `@ref` suffix to pin a specific git ref
(branch, tag, or commit):

```
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
     set `$NAME = $SCHEMA_VALUE`, `$REF = main` → local resolution only.
   - If `$SCHEMA_VALUE` contains `/` (URL):
     - Split on `@` — the part before `@` is the URL path, the part after
       is `$REF` (default `main` if no `@` present).
     - Extract `$NAME` from the last path segment of the URL path.

2. **Local resolution**

   Check if `schemas/$NAME/` exists in this plugin directory.
   If found, use the local directory for all `$FILES_NEEDED`. Done.

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

4. **Remote resolution** (URL, no local match, no valid cache)

   Construct raw content URLs using `$REF`:
   ```
   https://raw.githubusercontent.com/<owner>/<repo>/$REF/<path>/<file>
   ```

   Fetch each file in `$FILES_NEEDED` via **WebFetch**.

   If any fetch fails, stop and report the error — do not fall back to
   defaults or inline content.

   **Populate the cache**: write fetched files to `.specify/.cache/`
   mirroring the schema directory structure:

   ```
   .specify/.cache/
   ├── .cache-meta.yaml
   ├── schema.yaml
   ├── config.yaml          (if fetched)
   ├── instructions/        (if fetched)
   │   ├── proposal.md
   │   ├── specs.md
   │   ├── design.md
   │   ├── tasks.md
   │   └── apply.md
   └── templates/
       ├── proposal.md      (if fetched)
       ├── spec-new.md      (if fetched)
       ├── spec-delta.md    (if fetched)
       ├── design.md        (if fetched)
       └── tasks.md         (if fetched)
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
- **`spec_format`**: child overrides parent field-by-field (e.g., child
  can override `section_heading` without restating `delta_operations`).
- **`apply`**: child `requires` replaces parent `requires`; child
  `instruction` replaces parent `instruction`; child `tracks` replaces
  parent `tracks`. Omitted fields inherit from parent.
- **`instructions/` and `templates/`**: resolve from the child schema
  directory first; fall back to the parent schema directory for any files
  not present in the child.
- **Circular `extends` chains** are an error — stop and report.

### Resolution Example

Given `omnia-secure` extends `omnia`:

1. Resolve `omnia` (parent) → yields base `schema.yaml`, `instructions/*`, `templates/*`
2. Resolve `omnia-secure` (child) → yields override `schema.yaml`
3. Merge: parent artifacts + child artifacts (override by `id`, append new)
4. For file reads: check child directory first, fall back to parent

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
| propose | `schema.yaml`, `instructions/*`, `templates/*`        |
| apply   | `schema.yaml`, `instructions/apply.md`                |
| archive | `schema.yaml`                                         |
| explore | `schema.yaml`                                         |
| status  | `schema.yaml`                                         |
