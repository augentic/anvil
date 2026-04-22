# cross-source-refactor — per-source scope-bundle collection

`/spec:execute` forwards seven flags to `/spec:define` in the canonical
order (sources first, then scope):

```text
/spec:define cross-source-refactor \
    --source monolith=./legacy/monolith \
    --source shared-lib=./legacy/shared \
    --scope-include monolith=src/ingest/** \
    --scope-include monolith=src/kafka/** \
    --scope-exclude monolith=src/ingest/_deprecated/** \
    --scope-manifest shared-lib=./slices/shared-lib.yaml
```

## Flag-to-bundle grouping

Define walks the `--scope-*` flag set once and groups entries by their
`<key>` prefix. The `--source` flags define the set of valid keys; every
scope key must appear in that set (otherwise the defensive
`scope-key-not-in-sources` check fires).

```text
key         include                        exclude                     manifest
-------------------------------------------------------------------------------
monolith    src/ingest/**, src/kafka/**    src/ingest/_deprecated/**   —
shared-lib  —                              —                           ./slices/shared-lib.yaml
```

The `monolith` bundle carries two includes and one exclude; the
`shared-lib` bundle carries a single manifest path. The manifest-XOR
invariant holds per key: `shared-lib` has no include or exclude, so no
defensive error fires.

## Resulting extract invocations

The schema's per-source define brief runs one `/spec:extract` per
`--source` key, translating the bundle at the call site. Globs and the
manifest path flow through verbatim — define does no expansion of its
own.

```text
/spec:extract ./legacy/monolith   <change-dir>/.extract/monolith/   \
    --include 'src/ingest/**' --include 'src/kafka/**' \
    --exclude 'src/ingest/_deprecated/**'
/spec:extract ./legacy/shared     <change-dir>/.extract/shared-lib/ \
    --manifest ./slices/shared-lib.yaml
```

## Merged outputs

After both extract runs complete, the brief merges the per-source
`.extract/<key>/` outputs into the change's top-level artifacts:

```text
<change-dir>/.extract/monolith/specs/     ┐
<change-dir>/.extract/shared-lib/specs/   ┼→ <change-dir>/specs/
                                          ┘
<change-dir>/.extract/monolith/design.md    ┐
<change-dir>/.extract/shared-lib/design.md  ┼→ <change-dir>/design.md
                                            ┘
```

The merge policy (conflict resolution between overlapping specs, the
design concatenation strategy) is the brief's concern, not define's.
This fixture pins only the flag-to-bundle grouping and the
bundle-to-extract-flag translation.

## Invariants pinned

1. **Per-key grouping.** `--scope-include monolith=…` flags collapse
   into the same bundle as `--scope-exclude monolith=…`; the
   `shared-lib` bundle is disjoint.
2. **Manifest XOR.** `shared-lib` carries only a manifest; no
   include/exclude entry appears for the same key, so no defensive
   error fires.
3. **Order preservation.** Include globs retain the order they arrived
   in (`src/ingest/**` before `src/kafka/**`), which is the order they
   appeared in `plan.yaml:scope.monolith.include`.
4. **Verbatim forwarding.** Define never stats a path, never expands a
   glob, never reads the manifest. Expansion is `/spec:extract`'s
   concern.
5. **Back-compat.** A source key without any `--scope-*` entry (not
   shown in this fixture; see `../../../execute/fixtures/field-wiring/combined/`
   for the sources-only case) produces an extract invocation with zero
   scope flags.
