# /spec:extract

Extract Specify artifacts from existing source code.

## Synopsis

```text
/spec:extract <source-path> <slice-dir> [--include <glob>...] [--exclude <glob>...] [--manifest <path>]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `source-path` | Yes | Path to the existing codebase to extract from |
| `slice-dir` | Yes | Target slice directory for generated artifacts |
| `--include` | No | Glob patterns to include (narrows scope) |
| `--exclude` | No | Glob patterns to exclude |
| `--manifest` | No | Path to a manifest file listing specific source files |

## When to use

- You have an existing codebase and want to produce reconstruction-grade specs and design from it.
- During `/spec:define` for migration initiatives when the plan entry has `sources`.
- Standalone after `/spec:init` for brownfield onboarding.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| `spec.md` (per capability) | `<slice-dir>/specs/<capability>/spec.md` | Requirements with BDD scenarios extracted from source |
| `design.md` | `<slice-dir>/design.md` | Domain model, APIs, dependencies, business logic with tags |

## Behavior

1. Reads the source tree at `source-path` (optionally filtered by `--include`/`--exclude`/`--manifest`).
2. Identifies the structure: modules, entry points, dependencies.
3. Extracts business logic and classifies it with tags (`[domain]`, `[infrastructure]`, `[mechanical]`, `[unknown]`).
4. Produces behavioral specs: one spec file per discovered capability, with requirements, scenarios, and error conditions.
5. Produces a design document capturing the technical shape.

## Key principle

Artifacts are **language-agnostic**. They describe *what* the code does, not *how* it should be reimplemented. Extracted specs should read identically whether the source was TypeScript, Python, or Go.

## Lifecycle transitions

None directly. Extract is typically invoked as part of `/spec:define`, which handles transitions.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| Source path not found | Invalid `source-path` | Check the path |
| Empty source tree | No files match after include/exclude filtering | Widen the filter |
| Unrecognised structure | Source code layout is unusual | Provide a `--manifest` with explicit file list |

## Examples

```text
# Extract from an entire codebase into a new change
/spec:extract ./src .specify/slices/initial-baseline/

# Extract only the auth module
/spec:extract ./src .specify/slices/migrate-auth/ --include "src/auth/**"

# Extract using a manifest
/spec:extract ./src .specify/slices/migrate-core/ --manifest ./migration-manifest.txt
```

**Brownfield onboarding flow:**

```text
/spec:init https://github.com/augentic/specify/capabilities/omnia
/spec:extract . .specify/slices/initial-baseline/
/spec:merge initial-baseline
```

## See also

- [/spec:define](define.md) -- invokes extract when `--source` is provided
- [/spec:analyze](../change-skills/analyze.md) -- the plan-time counterpart (cheap capability summaries vs deep extraction)
- [Tutorial: Brownfield Onboarding](../../tutorials/brownfield-onboarding.md) -- full walkthrough
