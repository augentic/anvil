# single-source-no-scope — zero-flag back-compat path

Pins the back-compat / small-legacy case: a source-driven run with a
single `--source` and no `--scope-*` flags at all. The per-source loop
runs once, the scope-bundle collection step short-circuits (empty
bundle), and `/spec:extract` is invoked with no filter flags — same as
pre-RFC-3a behaviour.

## Per-source loop (one iteration)

The bundle for `legacy` is empty:

```text
key      include   exclude   manifest
-------------------------------------
legacy   —         —         —
```

Translation emits zero filter flags. Extract invocation:

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
