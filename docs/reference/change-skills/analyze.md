# /spec:analyze

Plan-time capability inference for legacy code and documentation inputs.

## Synopsis

```text
/spec:analyze <input-path> <output-dir> <legacy-code|documentation> [source-key <key>]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `input-path` | Yes | Path to the input (code directory or documentation file) |
| `output-dir` | Yes | Directory for output artifacts |
| `--kind` | Yes | Input type: `legacy-code` or `documentation` |
| `--source-key` | No | Key name for the source (used in metadata paths) |

## When to use

Typically invoked by the discovery phase during `/change:plan`, not directly. Reads one input and appends capability summaries to `discovery.md`.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| Capability summaries | `<output-dir>/discovery.md` (appended) | Per-capability: name, summary, source files, dependencies, confidence |
| Structural metadata | `<output-dir>/analyze/<key>/metadata.json` | Language, LOC, module count (legacy-code only) |

## Behavior

Branches on `--kind`:

### `legacy-code`

1. Reads the source tree at `input-path`.
2. Identifies modules, entry points, and dependency structure.
3. Produces structural metadata (`metadata.json`): language, lines of code, module count.
4. Extracts capability summaries: name, one-line summary, `sources:` file-hint list, `depends-on:` edges, `confidence:` marker.
5. Appends summaries to `discovery.md`.

### `documentation`

1. Reads the documentation file(s) at `input-path`.
2. Identifies capabilities described in the documentation.
3. Extracts capability summaries in the same format as legacy-code.
4. Appends summaries to `discovery.md`.

### Key principle

Analyze produces **capability summaries**, not full specs. It is deliberately cheap -- it scans the whole source to build an inventory without deep extraction. Deep per-capability extraction happens later at `/spec:define` time via `/spec:extract`.

This two-skill split is a scaling strategy:

| Skill | When | Depth | Scope |
|-------|------|-------|-------|
| `/spec:analyze` | Plan time | Shallow (summaries) | Entire source |
| `/spec:extract` | Define time | Deep (full specs) | Per-change slice |

## Lifecycle transitions

None. Analyze is a supporting skill invoked during planning.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| Unknown kind | `--kind` is not `legacy-code` or `documentation` | Use one of the two supported values |
| Input not found | `input-path` does not exist | Check the path |
| Empty input | No analysable content found | Check the input file or directory |

## Examples

```text
# Analyse legacy code
/spec:analyze ./src/legacy .specify/plans/migrate/ legacy-code monolith

# Analyse documentation
/spec:analyze ./docs/prd.md .specify/plans/new-platform/ documentation
```

## See also

- [/change:plan](plan.md) -- the primary consumer of analyze
- [/spec:extract](../slice-skills/extract.md) -- the deep counterpart at define time
