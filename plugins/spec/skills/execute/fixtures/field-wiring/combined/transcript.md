# combined — `/spec:execute` forwards sources; delta targeting is description-driven

The plan entry `registration-hardening` declares `sources:
[monolith]` and a description that references `user-registration`.
This is the canonical shape for a refactor that re-reads a legacy
source while delta-targeting an already-merged baseline.
`/spec:execute` resolves the source key and passes it through as
`--source`. Delta targeting is inferred by the define skill from
the entry's description — no explicit flag is needed.

## Resolution trace

```text
plan entry: registration-hardening
  sources: [monolith]
  description: "Tighten email-parser validation; same legacy source
                as the original extraction, delta-targeting the
                merged user-registration baseline."
plan's top-level sources map:
  monolith: /path/to/legacy-codebase

resolve "monolith":
  → found in top-level map
  → value is a local filesystem path (starts with "/")
  → emit --source monolith=/path/to/legacy-codebase

delta targeting:
  /spec:define reads the description and infers that
  user-registration is the delta target
```

## Pinned invocation

Contents of `invocation.txt`:

```text
/spec:define registration-hardening --source monolith=/path/to/legacy-codebase
```

No `--affects` flag — the define skill infers delta targets from the
change's description.

## Rendered define step

When the driver emits the per-change output block for this entry,
the `Processing:` header suffix carries the `sources` list, and the
define step body shows the extract sub-step with the resolved path.
Delta targeting (from the description) is handled inside define:

```text
### Processing: registration-hardening (sources: [monolith])

Step 1/3: define
  - extract sub-step (via /spec:extract)
      Source: /path/to/legacy-codebase
      Artifacts: specs/user-registration/spec.md (delta), design.md ✓
  Artifacts: proposal.md, specs, design.md, tasks.md ✓
```

The extract sub-step's `Artifacts:` line notes `(delta)` next to
`specs/user-registration/spec.md` because the description indicates
this change targets the `user-registration` baseline — the extracted
spec is emitted as a delta against the already-merged
`.specify/specs/user-registration/spec.md`, not as a fresh baseline.
This `(delta)` annotation is define's doing, not the driver's; the
driver only forwards source values.

## Invariants pinned

1. **`--source` appears** in the emitted invocation. The driver
   resolves the source key and forwards it as before.
2. **No `--affects` flag is emitted.** The `affects` field has been
   removed from the plan schema. Delta targeting is inferred by
   the define skill from the change's description.
3. **Description carries delta intent.** The description mentions
   `user-registration` by name and says "delta-targeting", giving
   the define skill enough context to locate the baseline spec and
   produce delta artifacts.
4. **Signals are independent.** Define runs the extract sub-step
   (from `sources`) and infers delta targeting (from description)
   independently. The driver does not coordinate between them.
5. **Same key, different change.** The `monolith` key resolves to
   `/path/to/legacy-codebase` here just as it did in `sources-only/`
   — the driver's resolution is purely a function of the plan's
   top-level map and the entry's key list, not of the change name.
