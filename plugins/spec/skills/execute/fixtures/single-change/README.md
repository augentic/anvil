# `/spec:execute` — single-change behavioural fixtures

These fixtures pin the three terminal outcomes of a supervised single-change run (no `--loop`): `done`, `failed`, and `blocked`. They correspond to RFC-2 Change L2.F; the algorithm they illustrate lives in [`../../SKILL.md` → §Supervised single-change run](../../SKILL.md).

There is no automated harness that runs these fixtures. They are prose artefacts: a human reviewing a change to `/spec:execute` should be able to trace each on-disk transition by reading the files in order, and any drift between the `.metadata.yaml` outcome and the `plan.yaml.after` `status-reason` is a regression.

## Layout

```text
single-change/
├── success/
│   ├── plan.yaml                        # before: pending
│   ├── metadata-after-define.yaml       # /spec:define stamps outcome: success
│   ├── metadata-after-build.yaml        # /spec:build  stamps outcome: success
│   ├── metadata-after-merge.yaml        # /spec:merge  stamps outcome: success
│   ├── plan.yaml.after                  # after: done
│   └── transcript.md                    # rendered output
├── failure/
│   ├── plan.yaml                        # before: pending
│   ├── metadata-after-build-failure.yaml# /spec:build  stamps outcome: failure
│   ├── plan.yaml.after                  # after: failed  (status-reason == outcome.summary verbatim)
│   ├── journal.yaml                     # type: failure entries written by the phase
│   └── transcript.md                    # rendered output
└── deferred/
    ├── plan.yaml                        # before: pending
    ├── metadata-after-define-deferred.yaml  # /spec:define stamps outcome: deferred
    ├── plan.yaml.after                  # after: blocked (status-reason == outcome.summary verbatim)
    ├── journal.yaml                     # type: question entry written by the phase
    └── transcript.md                    # rendered output
```

## Invariants every fixture asserts

1. **State-machine validity.** Every transition is a legal edge in the `PlanStatus` state machine (`pending → in-progress` implied by the algorithm, then `in-progress → {done, failed, blocked}`).
2. **Verbatim `outcome.summary` → `status-reason`.** In the failure and deferred fixtures, `plan.yaml.after`'s `status-reason` is byte-identical to the `.metadata.yaml:outcome.summary` string. Paraphrasing, truncating, or re-rendering is forbidden by the algorithm and will fail hand review.
3. **No driver writes to `.metadata.yaml` or `journal.yaml`.** The metadata snapshots and `journal.yaml` contents are authored by the phase skills (via `specify change outcome set` and `specify change journal append`). `/spec:execute` only writes plan status transitions.
4. **Journal entries preserved.** The failure / deferred fixtures ship `journal.yaml` files written by the phase during its run. The driver neither rewrites nor extends them; the transcript's `Journal:` line points at the file so humans can read it for context.
5. **`outcome.context` stays on disk.** The metadata snapshots include a `context` block. It is preserved for human triage but NOT copied into the plan entry — only `summary` travels to `status-reason`.

## Using these fixtures

- Before changing the algorithm in `SKILL.md`, re-read the before/after pairs here and confirm the new algorithm still maps them cleanly. If it does not, update the fixtures in the same commit as the SKILL.md change.
- Future Changes (L2.G self-heal, L2.H `--loop`, L2.I sources/affects wiring) will ship their own fixtures alongside these; none of them should require editing the single-change fixtures to stay coherent.
