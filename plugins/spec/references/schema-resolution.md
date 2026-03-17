# Schema Resolution

Skills resolve the `schema` field from `.specify/config.yaml` (or `.metadata.yaml`) to locate schema files. This document defines the resolution algorithm used by all spec skills.

## Inputs

- **`$SCHEMA_VALUE`**: the `schema` field value (a name or URL)
- **`$FILES_NEEDED`**: which files the calling skill requires (e.g., `schema.yaml`, `instructions/*`)

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
     set `$NAME = $SCHEMA_VALUE`, `$REF = main` → local resolution
     (step 2), then cache (step 3) if local not found.
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

   If not found, fall through to step 3 (cache check). The `init` skill
   populates the cache for bare-name schemas using `local:<name>` as the
   `schema_url`, so downstream skills can resolve from cache even without
   a local `schemas/` directory. If neither local resolution nor cache
   produces a match, stop and report an error — bare names cannot fall
   through to remote resolution.

   > **Note**: Bare-name resolution against `schemas/` is a development
   > convenience for working within the specify repository itself.
   > Downstream projects either use URL-based schemas (with optional
   > `@ref` pinning) or rely on the cache populated by `init`.

3. **Cache check**

   If `.specify/.cache/.cache-meta.yaml` exists, read it:

   ```yaml
   schema_url: https://github.com/augentic/specify/schemas/omnia@v1
   fetched_at: 2026-03-13T10:30:00Z
   ```

   Match `schema_url` against `$SCHEMA_VALUE`:
   - For URL-based schemas: `schema_url` must match `$SCHEMA_VALUE` exactly.
   - For bare-name schemas: `schema_url` must equal `local:$NAME`.

   If matched, use the cached files from `.specify/.cache/` for all
   `$FILES_NEEDED`. Done.

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
   └── instructions/        (if fetched)
       ├── proposal.md
       ├── specs.md
       ├── design.md
       ├── tasks.md
       └── build.md
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

- **`blueprints`**: child blueprints with the same `id` override the parent
  entirely; new `id`s are appended to the parent's list. Dependency order
  is recomputed from the merged `requires` graph.
- **`terminology`**: child replaces parent. If omitted, inherits the
  parent's `terminology`. Contains only `deliverable` (skills infer plural
  and heading forms).
- **`validation`**: child replaces parent entirely (boolean flag map).
  If omitted, inherits the parent's validation flags.
- **`build`**: child `requires` replaces parent `requires`; child
  `instructions` replaces parent `instructions`; child `tracks` replaces
  parent `tracks`. Omitted fields inherit from parent.
- **`defaults`**: child `defaults.context` replaces parent if present;
  child `defaults.rules` merges per blueprint key (child overrides parent
  for matching keys, parent provides the rest). If `defaults` is omitted
  entirely, inherits the parent's `defaults`.
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
3. Merge: parent blueprints + child blueprints (override by `id`, append new)
4. For file reads: check child directory first, fall back to parent

## Resolution Modes

The schema value format determines the resolution path:

| Format              | Example                                      | Resolution                                       |
|---------------------|----------------------------------------------|--------------------------------------------------|
| Bare name           | `schema: omnia`                              | Local `schemas/omnia/`, then cache.              |
| URL (default ref)   | `schema: https://github.com/.../omnia`       | Cache, then remote fetch at `main`.              |
| URL with pinned ref | `schema: https://github.com/.../omnia@v1`    | Cache, then remote fetch at `v1`.                |

Bare names resolve locally first, then fall back to the cache populated by
`init`; they never reach the network. URLs always resolve via cache or
remote and never use a local `schemas/` directory, even if one exists with
the same name. This guarantees that a pinned URL produces the same schema
across machines and branches.

## Cache Notes

- The `.specify/.cache/` directory should be gitignored. The `init` skill
  creates this directory and adds it to `.gitignore` if needed.
- Cache invalidation is automatic: when the `schema` value in
  `.specify/config.yaml` changes, the cached `schema_url` no longer
  matches, triggering a refetch.
- To force a refetch, delete `.specify/.cache/` and run any skill that
  resolves the schema.
- The `init` skill populates the cache with the full schema during
  initialization. Subsequent skills resolve from cache.
- For locally resolved schemas (bare name), init writes `.cache-meta.yaml`
  with `schema_url: local:<name>` so downstream skills can match against it.

## What Each Skill Needs

| Skill   | Files needed                              |
|---------|-------------------------------------------|
| init    | `schema.yaml`, `instructions/*`           |
| define  | `schema.yaml`, `instructions/*`           |
| build   | `schema.yaml`, `instructions/build.md`    |
| merge   | `schema.yaml`                             |
| drop    | `schema.yaml`                             |
| verify  | `schema.yaml`                             |
| explore | `schema.yaml`                             |
| status  | `schema.yaml`                             |
