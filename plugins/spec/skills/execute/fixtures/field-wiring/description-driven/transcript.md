# description-driven — `/spec:execute` forwards a description-driven delta target

The plan entry `registration-duplicate-email-crash` carries a
`description` that mentions `user-registration` but declares no
`sources`. `/spec:execute` passes the change name to `/spec:define`
without any extra flags. The define skill reads the description and
infers that the change targets the `user-registration` baseline —
delta targeting is description-driven, not flag-driven.

## Resolution trace

```text
plan entry: registration-duplicate-email-crash
  sources: (absent)
  description: "Duplicate email submission returns 500 instead of 409.
                Modifies user-registration."

sources: (absent) — no --source flags emitted
affects: (removed from schema) — no --affects flags emitted

delta targeting:
  /spec:define reads the description and infers that
  user-registration is the delta target
```

## Pinned invocation

Contents of `invocation.txt`:

```text
/spec:define registration-duplicate-email-crash
```

No `--affects` flag — the define skill infers delta targets from the
change's description.

## Rendered define step

When the driver emits the per-change output block for this entry,
the `Processing:` header carries the change name only (no signal
annotations), and the define step body has **no** extract sub-step —
the plan entry has no sources to extract from:

```text
### Processing: registration-duplicate-email-crash

Step 1/3: define
  Artifacts: proposal.md, specs, design.md, tasks.md ✓
```

Delta targeting — define locating `.specify/specs/user-registration/spec.md`
and preparing to emit delta specs under
`.specify/changes/registration-duplicate-email-crash/specs/user-registration/spec.md`
— happens inside define. The define skill reads the entry's description,
identifies `user-registration` as an existing baseline spec, and
automatically treats the change as a delta against it. The driver-level
transcript does not call it out as its own sub-step; the delta-vs-new
classification surfaces later, after `specify change touched-specs --scan`
runs in step 8 of the define skill.

## Invariants pinned

1. **No extract sub-step** appears in the define step when the plan
   entry has no `sources`. The sub-step is gated on `--source`, not
   on description content.
2. **No `--affects` flag is emitted.** The `affects` field has been
   removed from the plan schema. Delta targeting is inferred by the
   define skill from the change's description.
3. **Description carries intent.** The description mentions
   `user-registration` by name, giving the define skill enough
   context to locate the baseline spec and produce delta artifacts.
4. **Prior-done entry as delta target.** The `user-registration`
   entry that this change modifies is already `done`, so its
   baseline specs live under `.specify/specs/`. Define's
   delta-targeting machinery needs that baseline to exist; a
   description referencing a `pending` entry's specs would produce
   a phase-level error inside define, not a driver-level one.
