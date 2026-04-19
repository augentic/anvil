# affects-only — `/spec:execute` forwards a delta-targeting name

The plan entry `registration-duplicate-email-crash` declares
`affects: [user-registration]` and no `sources`.
`/spec:execute` passes the name through to `/spec:define` as a
single `--affects` flag. No source resolution happens because there
are no `sources` keys to look up; the plan's top-level `sources` map
is absent entirely from this fixture, matching the common
`affects`-only case in which no legacy source is involved.

## Resolution trace

```text
plan entry: registration-duplicate-email-crash
  sources: (absent)
  affects: [user-registration]

sources: (absent) — no --source flags emitted

affects:
  pass "user-registration" as --affects user-registration
```

## Pinned invocation

Contents of `invocation.txt`:

```text
/spec:define registration-duplicate-email-crash --affects user-registration
```

## Rendered define step

When the driver emits the per-change output block for this entry,
the `Processing:` header suffix carries the `affects` list only,
and the define step body has **no** extract sub-step — the plan
entry has no sources to extract from:

```text
### Processing: registration-duplicate-email-crash (affects: [user-registration])

Step 1/3: define
  Artifacts: proposal.md, specs, design.md, tasks.md ✓
```

Delta targeting — define locating `.specify/specs/user-registration/spec.md`
and preparing to emit delta specs under
`.specify/changes/registration-duplicate-email-crash/specs/user-registration/spec.md`
— happens inside define. The driver-level transcript does not call
it out as its own sub-step; the delta-vs-new classification surfaces
later, after `specify change touched-specs --scan` runs in step 8
of the define skill.

## Invariants pinned

1. **No extract sub-step** appears in the define step when the plan
   entry has no `sources`. The sub-step is gated on `--source`, not
   on `--affects`.
2. **`--affects` name is verbatim** — no kebab-case normalization,
   no path expansion. Define receives the string as it appears in
   the plan entry.
3. **Absent top-level `sources` map is not an error.** The
   `plan.yaml` in this fixture omits the top-level `sources` key
   entirely. Validation (RFC-2 Change L1.F) treats this as valid;
   the driver never looks the map up because the plan entry's
   `sources` list is itself empty. An absent map combined with an
   entry that *does* declare `sources` would be an `unknown-source`
   diagnostic at validation time and an `Error::Config` halt in the
   driver.
4. **Prior-done entry as `affects` target.** The
   `user-registration` entry that this change `affects` is already
   `done`, so its baseline specs live under `.specify/specs/`.
   Define's delta-targeting machinery needs that baseline to
   exist; a plan that `affects` a `pending` entry would be caught
   by `specify initiative validate` (the `unknown-affects` diagnostic
   checks for entry existence, but the *baseline spec existence*
   check is deferred to define's own phase-level error handling).
