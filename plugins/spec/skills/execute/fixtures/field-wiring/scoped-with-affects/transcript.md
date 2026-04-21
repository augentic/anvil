# scoped-with-affects — `/spec:execute` forwards all three signals

The plan entry `extract-shared-validation` is the canonical
three-signal case from RFC-3a §*`--affects` composition with scope*:
a refactor that re-reads a legacy source (`sources: [monolith]`),
delta-targets two already-merged baselines (`affects:
[user-registration, email-verification]`), AND narrows the extracted
slice to a shared sub-tree (`scope.monolith.include:
[src/common/validation/**]`). `/spec:execute` resolves the source
key, forwards each affects name, and forwards the scope include
glob — all three as independent sets of flags on a single
`/spec:define` invocation.

## Resolution trace

```text
plan entry: extract-shared-validation
  sources: [monolith]
  affects: [user-registration, email-verification]
  scope:
    monolith:
      include: [src/common/validation/**]
plan's top-level sources map:
  monolith: ./legacy/monolith

resolve "monolith":
  → found in top-level map
  → value is a local filesystem path
  → emit --source monolith=./legacy/monolith

affects:
  emit --affects user-registration
  emit --affects email-verification

scope.monolith:
  include:
    → emit --scope-include monolith=src/common/validation/**
  exclude:  (absent)
  manifest: (absent)

final argument order:
  --source, then --affects (in declaration order), then
  --scope-include. Stable ordering keeps invocation.txt diffs
  readable; /spec:define does not depend on it.
```

## Pinned invocation

Contents of `invocation.txt`:

```text
/spec:define extract-shared-validation --source monolith=./legacy/monolith --affects user-registration --affects email-verification --scope-include monolith=src/common/validation/**
```

## Rendered define step

When the driver emits the per-change output block for this entry,
the `Processing:` header suffix carries both the `sources` list and
the `affects` list. Scope is a per-source modifier rendered inside
the define step body, alongside the delta-targeting annotations
(from `affects`):

```text
### Processing: extract-shared-validation (sources: [monolith], affects: [user-registration, email-verification])

Step 1/3: define
  - extract sub-step (via /spec:extract)
      Source: ./legacy/monolith
      Filter: include src/common/validation/**
      Artifacts:
        specs/user-registration/spec.md (delta),
        specs/email-verification/spec.md (delta),
        design.md ✓
  Artifacts: proposal.md, specs, design.md, tasks.md ✓
```

The `(delta)` annotations on the two already-merged baselines come
from `--affects`; the `Filter:` line comes from `--scope-include`;
both are rendered by `/spec:define`, not by the driver. The driver's
only job was to forward all three flag families untouched.

## Invariants pinned

1. **All three flag families appear** when all three signals are
   present on the plan entry. The driver does not choose between
   them, drop any, or synthesize a hybrid.
2. **Signals are fully independent.** Scope narrows which *source*
   files extract reads; affects redirects which *baseline* specs the
   brief writes against. The driver does not coordinate between
   them; composition happens inside `/spec:define`'s brief pipeline
   (RFC-3a §*`--affects` composition with scope*).
3. **Same key across families.** The `monolith` key appears on both
   `--source` and `--scope-include` — define's per-source brief loop
   uses the key to group scope flags with their source during the
   per-source extract invocation.
4. **Both `affects` targets keep their baseline names.** Define's
   delta-targeting machinery locates `.specify/specs/user-registration/`
   and `.specify/specs/email-verification/`; the driver only forwards
   the names.
5. **Scope does not widen or narrow `affects`.** The fact that only
   one glob is in scope does not prune the `--affects` list; each
   family is emitted independently. Whether the extracted slice
   actually produces content matching every `--affects` target is a
   brief-level concern surfaced after extract runs.
