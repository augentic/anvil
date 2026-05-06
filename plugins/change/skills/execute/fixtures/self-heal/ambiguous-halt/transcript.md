# ambiguous-halt — self-heal refuses to speculate

The prior `/change:execute` run left `shopping-cart` as `in-progress` in `plan.yaml`, and its `.specify/slices/shopping-cart/.metadata.yaml` carries a contradictory pair:

- `status: defining` — /spec:define has not yet stamped `defined`; /spec:build and /spec:merge have never run.
- `outcome: { phase: merge, outcome: success, … }` — yet something claims /spec:merge finished.

This combination cannot be produced by the legitimate lifecycle transition graph (`Defining → Defined → Building → Complete → Merged`). It is either file corruption, a concurrent-writer bug, or a partial write from a crashed phase. Self-heal halts rather than speculate: picking either signal as authoritative risks silently flipping a never-implemented change to `done`, and that failure mode is strictly worse than the operator cost of one manual triage.

Self-heal:

1. Scans `plan.yaml`, finds `shopping-cart` with `status: in-progress`.
2. Reads `metadata.yaml`. Detects the contradiction (`outcome.phase == merge` but `LifecycleStatus == defining`).
3. Does NOT call `specify change plan transition`.
4. Does NOT call `/spec:drop`.
5. Does NOT append a `type: recovery` journal entry.
6. Emits one diagnostic line to stdout, releases the driver lock (step 13 of the supervised run — even halts run the release step), and exits with code 1.

```text
Self-heal halted: shopping-cart has outcome=success phase=merge but LifecycleStatus=defining. Manual triage required.
Exit 1
```

`plan.yaml.after` is byte-identical to `plan.yaml.before`. `metadata.yaml` is byte-identical to what the prior crashed run left. `journal.yaml` is untouched. The on-disk state the operator sees is exactly the state the crashed driver left, with no self-heal overlay.

Expected next action (by the human, not the driver): inspect `metadata.yaml` against git history, decide whether to believe `LifecycleStatus` (→ rerun /spec:define) or `outcome.phase` (→ manually transition the plan entry), repair the file, then re-run `/change:execute`.
