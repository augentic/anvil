# scoped — `/spec:execute` forwards scope globs as per-glob flags

The plan entry `ingest-pipeline` declares `sources: [monolith]` and a
`scope.monolith` map with two `include` globs and one `exclude` glob.
`/spec:execute` resolves the source key as usual and walks the
`scope.monolith` map, emitting one `--scope-include` flag per
`include` glob (in declaration order) and one `--scope-exclude` flag
per `exclude` glob. The driver forwards the globs verbatim — no glob
expansion, no filesystem access, no cross-check against the source
tree.

## Resolution trace

```text
plan entry: ingest-pipeline
  sources: [monolith]
  affects: (absent)
  scope:
    monolith:
      include: [src/ingest/**, src/kafka/**]
      exclude: [src/ingest/_deprecated/**]
plan's top-level sources map:
  monolith: ./legacy

resolve "monolith":
  → found in top-level map
  → value is a local filesystem path
  → emit --source monolith=./legacy

scope.monolith:
  include:
    → emit --scope-include monolith=src/ingest/**
    → emit --scope-include monolith=src/kafka/**
  exclude:
    → emit --scope-exclude monolith=src/ingest/_deprecated/**
  manifest: (absent) — no --scope-manifest flag emitted

affects: (absent) — no --affects flags emitted

final argument order:
  --source, then --scope-include (in declaration order), then
  --scope-exclude (in declaration order). /spec:define does not
  depend on order but the fixture pins a canonical rendering.
```

## Pinned invocation

Contents of `invocation.txt`:

```text
/spec:define ingest-pipeline --source monolith=./legacy --scope-include monolith=src/ingest/** --scope-include monolith=src/kafka/** --scope-exclude monolith=src/ingest/_deprecated/**
```

## Rendered define step

When the driver emits the per-change output block for this entry,
the `Processing:` header suffix carries the `sources` list only —
scope is a per-source modifier, not a top-level signal in the
transcript header. The define step body shows the extract sub-step
with the resolved path on the `Source:` line; the scope filters
surface inside `/spec:define`'s per-source brief loop, which
translates the `--scope-*` flags into `/spec:extract`'s native
`--include` / `--exclude` filters:

```text
### Processing: ingest-pipeline (sources: [monolith])

Step 1/3: define
  - extract sub-step (via /spec:extract)
      Source: ./legacy
      Filter: include src/ingest/**, src/kafka/**; exclude src/ingest/_deprecated/**
      Artifacts: specs/ingest-pipeline/spec.md, design.md ✓
  Artifacts: proposal.md, specs, design.md, tasks.md ✓
```

The `Filter:` line is define's rendering of the translated extract
flags, not the driver's. The driver only forwards `--scope-*`
values.

## Invariants pinned

1. **One flag per glob.** Two `include` globs produce two separate
   `--scope-include monolith=<glob>` flags — never a comma-joined
   value, never a single flag with a list payload. Same for
   `--scope-exclude`.
2. **Key (`monolith`) travels verbatim** on every scope flag. Define
   uses the key to group scope flags per source during its
   per-source extract loop.
3. **Declaration order is preserved.** The two `include` globs
   appear in the invocation in the order they appeared on the plan
   entry. Reordering would drift the fixture.
4. **Absent `manifest`, no flag.** `scope.monolith.manifest` is
   unset, so no `--scope-manifest` flag is emitted. The driver does
   not synthesize an empty manifest value.
5. **Globs are opaque strings to the driver.** `src/ingest/**` is
   forwarded unchanged; the driver does not walk the source tree,
   does not stat `./legacy`, and does not verify that any file
   matches. A zero-match extract is a hard error surfaced by
   `/spec:extract` (RFC-3a C06), not by the driver.
