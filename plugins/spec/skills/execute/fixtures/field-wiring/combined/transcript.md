# combined — `/spec:execute` forwards both signals on a single entry

The plan entry `registration-hardening` declares BOTH `sources:
[monolith]` AND `affects: [user-registration]`. This is the canonical
shape for a refactor that re-reads a legacy source while delta-
targeting an already-merged baseline. `/spec:execute` resolves the
source key, passes it through as `--source`, and passes the affects
name through as `--affects`. Define handles the two independently —
a source-aware extract sub-step AND delta targeting — and the
driver does not coordinate between them.

## Resolution trace

```text
plan entry: registration-hardening
  sources: [monolith]
  affects: [user-registration]
plan's top-level sources map:
  monolith: /path/to/legacy-codebase

resolve "monolith":
  → found in top-level map
  → value is a local filesystem path (starts with "/")
  → emit --source monolith=/path/to/legacy-codebase

affects:
  emit --affects user-registration

final argument order:
  --source before --affects (sources first, then affects — fixed by
  this skill's conventions; /spec:define does not depend on order
  but the fixture pins a canonical rendering).
```

## Pinned invocation

Contents of `invocation.txt`:

```text
/spec:define registration-hardening --source monolith=/path/to/legacy-codebase --affects user-registration
```

## Rendered define step

When the driver emits the per-change output block for this entry,
both signals appear in the `Processing:` header suffix and both
the extract sub-step (from `sources`) and the delta-targeted
artifacts (from `affects`) are visible in the define step body:

```text
### Processing: registration-hardening (sources: [monolith], affects: [user-registration])

Step 1/3: define
  - extract sub-step (via /spec:extract)
      Source: /path/to/legacy-codebase
      Artifacts: specs/user-registration/spec.md (delta), design.md ✓
  Artifacts: proposal.md, specs, design.md, tasks.md ✓
```

The extract sub-step's `Artifacts:` line notes `(delta)` next to
`specs/user-registration/spec.md` because `affects` targets that
baseline — the extracted spec is emitted as a delta against the
already-merged `.specify/specs/user-registration/spec.md`, not as a
fresh baseline. This `(delta)` annotation is define's doing, not
the driver's; the driver only forwards the flag values.

## Invariants pinned

1. **Both flags appear** in the emitted invocation when both signals
   are present on the plan entry. The driver does not choose between
   them, drop one, or synthesize a hybrid.
2. **`--source` precedes `--affects`** in the canonical rendering.
   The skill's Argument resolution section documents this ordering;
   /spec:define does not depend on it, but stable ordering makes
   invocation.txt diffs easier to read.
3. **Signals are independent.** Define is free to run the extract
   sub-step and the delta-targeting step in either order inside its
   own brief pipeline; the driver does not impose a sequence.
4. **Same key, different change.** The `monolith` key resolves to
   `/path/to/legacy-codebase` here just as it did in `sources-only/`
   — the driver's resolution is purely a function of the plan's
   top-level map and the entry's key list, not of the change name.
