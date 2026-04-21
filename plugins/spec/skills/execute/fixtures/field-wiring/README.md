# `/spec:execute` — `sources` / `affects` / `scope` argument-wiring fixtures

These fixtures pin the five argument-shape variants `/spec:execute`
builds for `/spec:define` from a plan entry's `sources`, `affects`,
and `scope` fields. They correspond to RFC-2 Change L2.I (the Layer
2 exit gate) and RFC-3a (scope plumbing); the algorithm they
illustrate lives in
[`../../SKILL.md` → §Argument resolution (`sources`, `affects`, and `scope`)](../../SKILL.md).

There is no automated harness that runs these fixtures. They are
prose artefacts: a human reviewing a change to `/spec:execute`'s
argument-resolution code should be able to diff a new invocation
rendering against the `invocation.txt` files here, and a change to
the rendered transcript format against the `transcript.md` files.

## Layout

```text
field-wiring/
├── sources-only/
│   ├── plan.yaml          # one-entry plan, sources: [monolith], no affects
│   ├── invocation.txt     # pinned command line built from the entry
│   └── transcript.md      # rendered define step (extract sub-step present)
├── affects-only/
│   ├── plan.yaml          # one-entry plan, affects: [user-registration], no sources
│   ├── invocation.txt     # pinned command line
│   └── transcript.md      # rendered define step (no extract sub-step; delta targeting)
├── combined/
│   ├── plan.yaml          # one-entry plan, sources + affects on the same entry
│   ├── invocation.txt     # pinned command line
│   └── transcript.md      # rendered define step (both extract and delta targeting)
├── scoped/
│   ├── plan.yaml          # one-entry plan, sources + scope.include/exclude
│   ├── invocation.txt     # pinned command line (per-glob --scope-* flags)
│   └── transcript.md      # rendered define step (filter visible in extract sub-step)
└── scoped-with-affects/
    ├── plan.yaml          # three-signal plan (sources + affects + scope.include)
    ├── invocation.txt     # pinned command line (all three flag families)
    └── transcript.md      # canonical extract-shared-validation rendering
```

## Argument-shape matrix

| Fixture | `sources` | `affects` | `scope` | `/spec:define` extra flags |
|---|---|---|---|---|
| `sources-only/` | `[monolith]` | (empty) | (empty) | `--source monolith=/path/to/legacy` |
| `affects-only/` | (empty) | `[user-registration]` | (empty) | `--affects user-registration` |
| `combined/` | `[monolith]` | `[user-registration]` | (empty) | `--source monolith=/path/to/legacy --affects user-registration` |
| `scoped/` | `[monolith]` | (empty) | `monolith: {include: [src/ingest/**, src/kafka/**], exclude: [src/ingest/_deprecated/**]}` | `--source monolith=./legacy --scope-include monolith=src/ingest/** --scope-include monolith=src/kafka/** --scope-exclude monolith=src/ingest/_deprecated/**` |
| `scoped-with-affects/` | `[monolith]` | `[user-registration, email-verification]` | `monolith: {include: [src/common/validation/**]}` | `--source monolith=./legacy/monolith --affects user-registration --affects email-verification --scope-include monolith=src/common/validation/**` |

## Invariants every fixture asserts

1. **`--source` passes through the key verbatim.** The key
   (`monolith`) travels unchanged from the plan entry's `sources`
   list through the driver into the flag, so `/spec:define`'s brief
   pipeline can retain provenance when it hands the value to
   `/spec:extract`.
2. **`--source` value is the top-level `sources` map value,
   unchanged.** The driver neither stats local paths nor clones git
   URLs; it forwards the string as stored in `plan.yaml`. A missing
   path surfaces as a phase-level error from `/spec:extract`, not a
   driver-level one.
3. **`--affects` names travel verbatim** in the order they appear
   on the plan entry. `/spec:define` is responsible for locating
   `.specify/specs/<name>/spec.md` for each.
4. **Greenfield invocation has neither flag.** When a plan entry
   has neither `sources` nor `affects`, the invocation is simply
   `/spec:define <name>` — no empty flag strings, no placeholder
   values. None of the three fixtures here demonstrate this case;
   the pre-existing greenfield fixtures under
   `../single-change/success/` and `../loop/all-done/` cover it.
5. **All three signals are independent.** The `combined/` fixture
   shows that a single entry can carry both `sources` and `affects`;
   `scoped-with-affects/` shows all three — `sources`, `affects`,
   AND `scope` — on one entry. Define handles each independently.
   The driver does not coordinate between them.
6. **One flag per glob, one flag per manifest.** The `scoped/`
   fixture asserts that two `scope.<key>.include` globs become two
   separate `--scope-include <key>=<glob>` flags, never a
   comma-joined value. Same for `--scope-exclude`. `--scope-manifest`
   is emitted at most once per key and is mutually exclusive with
   `--scope-include` / `--scope-exclude` for that key (enforced by
   `specify initiative validate`).
7. **Scope strings are opaque to the driver.** Globs are forwarded
   verbatim; the driver does not walk the source tree, stat the
   source path, or cross-check that any file matches. Zero-match
   outcomes are `/spec:extract`'s concern (RFC-3a C06).

## Using these fixtures

- Before changing the argument-resolution logic in `SKILL.md`, diff
  the `invocation.txt` files here against what the new algorithm
  would emit. Drift between the algorithm and these pins is a
  regression — update the fixtures in the same commit as the
  SKILL.md change.
- Before changing the rendered define-step output format, re-read
  the `transcript.md` files and confirm the new format still maps
  cleanly. The `Processing:` header suffix (`(sources: [...],
  affects: [...])` vs `(greenfield)` vs one-sided forms) is the
  load-bearing variant surface.
- End-to-end coverage of these signals across a multi-change plan
  lives in `../e2e-platform-v2/`; that meta-fixture exercises the
  same argument-resolution code path on every entry of RFC-2
  §"The Plan".
