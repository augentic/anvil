# two-source-scope — multi-iteration loop with per-source design wrapping

Pins the two-source case: one glob-filtered source, one manifest-based
source. The per-source loop runs twice in `--source` declaration order;
the design merge wraps each contribution under a `## Source: <key>`
heading; the name-collision rule stands ready to fire if any capability
name appears under both source trees.

Mirrors the flag-to-bundle grouping pinned by
`plugins/spec/skills/define/fixtures/two-source-scope/collection.md`.
That fixture pins the define-side contract; this one pins what the
schema brief does with the bundles.

## Per-source loop (two iterations, in `--source` order)

```text
key          include                        exclude                      manifest
---------------------------------------------------------------------------------
monolith     src/ingest/**, src/kafka/**    src/ingest/_deprecated/**    —
shared-lib   —                              —                            ./slices/shared-lib.yaml
```

Manifest-XOR holds per key — `shared-lib` carries only a manifest, so no
defensive error fires.

### Iteration 1 — `monolith`

```text
/spec:extract ./legacy/monolith <change-dir>/.extract/monolith/ \
    --include 'src/ingest/**' \
    --include 'src/kafka/**' \
    --exclude 'src/ingest/_deprecated/**'
```

### Iteration 2 — `shared-lib`

```text
/spec:extract ./legacy/shared <change-dir>/.extract/shared-lib/ \
    --manifest ./slices/shared-lib.yaml
```

## After both extract runs

```text
<change-dir>/.extract/monolith/specs/ingest-pipeline/spec.md
<change-dir>/.extract/monolith/specs/kafka-adapter/spec.md
<change-dir>/.extract/monolith/design.md

<change-dir>/.extract/shared-lib/specs/validation-core/spec.md
<change-dir>/.extract/shared-lib/design.md
```

## Merge step

### `specs/` — no collision

Each capability name is unique across both source trees, so the merge
simply copies each directory into `<change-dir>/specs/`:

```text
<change-dir>/specs/ingest-pipeline/spec.md      ← from .extract/monolith/
<change-dir>/specs/kafka-adapter/spec.md        ← from .extract/monolith/
<change-dir>/specs/validation-core/spec.md      ← from .extract/shared-lib/
```

**Collision trigger (for reference).** If the `shared-lib` extract had
also emitted `specs/ingest-pipeline/spec.md`, the brief would halt with a
brief-level error surfacing both colliding paths. Resolution is upstream
— the propose brief must force distinct names or consolidate the
duplicates under one source.

### `design.md` — `## Source: <key>` wrapping

Two sources contributed, so each section is wrapped:

```markdown
## Source: monolith

<contents of .extract/monolith/design.md>

## Source: shared-lib

<contents of .extract/shared-lib/design.md>
```

Section order follows `--source` declaration order (`monolith` first,
`shared-lib` second).

## `.extract/` scratch tree after merge

```text
<change-dir>/.extract/monolith/      (kept for review)
<change-dir>/.extract/shared-lib/    (kept for review)
```

Operator cleans up manually once the change is merged. Never committed.
