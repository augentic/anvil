# `/spec:execute` — self-heal behavioural fixtures

These fixtures pin the five startup paths the §Self-heal on startup step in [`../../SKILL.md`](../../SKILL.md) distinguishes: no-op on a clean plan, terminal reconciliation on a prior success, terminal reconciliation on a prior failure, halt-for-triage on ambiguity, and mid-change resume when the prior driver crashed between phases. They correspond to RFC-2 Change L2.G; the algorithm they illustrate lives in [`../../SKILL.md` → §Self-heal on startup](../../SKILL.md).

There is no automated harness that runs these fixtures. They are prose artefacts: a human reviewing a change to `/spec:execute`'s self-heal step should be able to trace each on-disk transition by reading the files in order, and any drift between the `.metadata.yaml` outcome and the `plan.yaml.after` `status-reason` is a regression.

## Layout

```text
self-heal/
├── clean-start/
│   ├── plan.yaml                  # no in-progress entries
│   └── transcript.md              # the "no-op" diagnostic line
├── resolves-to-done/
│   ├── plan.yaml.before           # email-verification: in-progress
│   ├── metadata.yaml              # LifecycleStatus: merged, outcome: merge success
│   ├── plan.yaml.after            # email-verification: done
│   ├── journal.yaml.after         # appended type: recovery entry
│   └── transcript.md              # the "→ done" diagnostic line
├── resolves-to-failed/
│   ├── plan.yaml.before           # checkout-api: in-progress
│   ├── metadata.yaml              # LifecycleStatus: building, outcome: build failure
│   ├── plan.yaml.after            # checkout-api: failed (status-reason verbatim)
│   ├── journal.yaml.after         # phase-authored failure entries + driver recovery entry
│   └── transcript.md              # the "→ failed" diagnostic line
├── ambiguous-halt/
│   ├── plan.yaml.before           # shopping-cart: in-progress
│   ├── metadata.yaml              # contradiction: phase=merge success, status=defining
│   ├── plan.yaml.after            # IDENTICAL to plan.yaml.before (halt, no transition)
│   └── transcript.md              # halt diagnostic + Exit 1
└── mid-change-resume/
    ├── plan.yaml.before           # product-catalog: in-progress
    ├── metadata.yaml              # LifecycleStatus: defined, NO outcome field
    ├── plan.yaml.after            # IDENTICAL to plan.yaml.before (plan stays in-progress)
    ├── journal.yaml.after         # appended type: recovery documenting the resume
    └── transcript.md              # the "resuming <phase>" diagnostic line
```

`ambiguous-halt/` intentionally has no `journal.yaml.after`: the halt path does not append a recovery entry (nothing was resolved). The other four fixtures all emit exactly one `type: recovery` entry, per the algorithm in SKILL.md step 4 of the self-heal subsection.

## Invariants every fixture asserts

1. **Sole signal channel is `.metadata.yaml.outcome`.** No fixture has the driver consulting `journal.yaml`, stderr, tempfiles, or any other side channel. Outcome-on-disk is the complete contract.
2. **Verbatim `outcome.summary` → `status-reason`.** The `resolves-to-failed/` fixture demonstrates this for the failure path; a `resolves-to-blocked/` variant would follow the exact same rule (omitted because the failure fixture already pins the equality and the deferred-outcome shape is identical).
3. **No plan transition on mid-change resume or halt.** The `mid-change-resume/` and `ambiguous-halt/` fixtures both have `plan.yaml.after` byte-identical to `plan.yaml.before`. Only the terminal-outcome fixtures (`resolves-to-done/`, `resolves-to-failed/`) carry a `plan.yaml` diff.
4. **Exactly one `type: recovery` entry per resolved or resumed entry.** Self-heal never emits more than one recovery entry for the same plan entry in the same invocation; the halt path emits none. The `journal.yaml.after` files pin the shape.
5. **Halt leaves plan and journal untouched.** `ambiguous-halt/` ships no `journal.yaml.after` and its `plan.yaml.after` mirrors `plan.yaml.before`. A human triaging the ambiguity sees the on-disk state the crashed driver left, not a driver-authored overlay.

## Using these fixtures

- Before changing the self-heal algorithm in `SKILL.md`, re-read the before / after pairs here and confirm the new algorithm still maps them cleanly. If it does not, update the fixtures in the same commit as the SKILL.md change.
- The `.metadata.yaml` files in these fixtures are illustrative snapshots shaped per `crates/change/src/lib.rs::ChangeMetadata` in `augentic/specify-cli`. They are prose, not a validated schema input — convenience fields like `updated-at` and a top-level `name` appear for readability and mirror the style of `../single-change/`; the real on-disk file is whatever `specify change phase-outcome` writes.
