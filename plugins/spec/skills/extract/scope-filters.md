# Scope filters and manifest shape

Scope filters restrict **which source files are read for business-logic extraction** (Step 2 onward). They never touch Step 1 — language detection and dependency version pinning always run against the full set of sentinel files listed below.

## Filter rules

- Globs are gitignore-style, with `**` for recursive match.
- Globs resolve relative to `$SOURCE_PATH`.
- Empty `$INCLUDE`, `$EXCLUDE`, and `$MANIFEST` ≡ today's behaviour: extract reads the full source tree. Small-legacy and greenfield callers see no change.
- With `$INCLUDE` non-empty: the read set for business-logic extraction is the union of `$INCLUDE` glob matches, minus any paths that also match a glob in `$EXCLUDE`.
- With `$MANIFEST` set: the read set is the verbatim file list from the manifest (see §Manifest shape below). `$INCLUDE` and `$EXCLUDE` are absent in this mode.
- A filter set that matches zero files under `$SOURCE_PATH` is a hard error — extract fails fast rather than emitting empty artifacts.

## Sentinels always read

Extract reads a fixed set of files regardless of the filter, for language / dependency detection:

- `package.json`, `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`
- `Cargo.toml`, `Cargo.lock`
- `go.mod`, `go.sum`
- `pyproject.toml`, `poetry.lock`, `requirements.txt`
- `pom.xml`, `build.gradle[.kts]`, `gradle.lockfile`
- `*.csproj`, `packages.lock.json`
- top-level `README*`

`$INCLUDE` cannot subtract sentinels; `$EXCLUDE` cannot hide them. Scope filters *business-logic extraction* (Step 2), not *manifest / language discovery* (Step 1).

## Manifest shape

v1 ships a minimal YAML manifest with `include` only:

```yaml
version: 1
include:
  - relative/path/to/file.ts
  - another/file.rs
```

- Paths are **literal file paths**, resolved relative to `$SOURCE_PATH`. No globs inside a manifest — globbing lives in `$INCLUDE` / `$EXCLUDE`.
- v1 is exactly `version` + `include` — no other top-level keys (`deny_unknown_fields`); `specify change plan validate` rejects unknown keys, wrong `version`, empty `include`, `..` segments, and absolute paths in `include` (see `specify-change` `Plan::validate` / `manifest-invalid`, `manifest-empty`, `manifest-path-escape`).
- v1 ships `include` only. Line-range subsets per file, `exclude`, and per-file symbol filters are out of scope for v1 and are the natural v2 extensions.
- A `$MANIFEST` that is missing, malformed, or references a file that does not exist under `$SOURCE_PATH` is a hard error — fail early with a clear message.
- Manifests are authored at plan time (by the propose brief) and referenced from the Plan's `scope.<src>.manifest` field. Extract consumes manifests; it does not author them. On disk they typically live under `.specify/plans/<initiative>/slices/` — see [`/change:plan` working directory](../../../change/skills/plan/SKILL.md) (§*Working directory*).

For a walk-through, see `fixtures/scoped-monolith/`.
