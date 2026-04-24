# single-source-no-scope — full-tree path

Pins the small-legacy case: a source-driven run with a single `--source` and a description that contains no file-path hints. The brief's scope inference finds nothing to narrow, so `/spec:extract` is invoked with no filter flags — the full source tree is extracted.

## Per-source loop (one iteration)

No path hints in description → no inferred filters. Extract invocation:

```text
/spec:extract ./legacy <change-dir>/.extract/legacy/
```

## After `/spec:extract` returns

```text
<change-dir>/.extract/legacy/specs/<...>/spec.md
<change-dir>/.extract/legacy/design.md
```

## After the merge step (single source, no `## Source:` wrapper)

```text
<change-dir>/specs/<...>/spec.md   ← from .extract/legacy/specs/
<change-dir>/design.md             ← from .extract/legacy/design.md
```

No name-collision check fires — only one source contributed.
