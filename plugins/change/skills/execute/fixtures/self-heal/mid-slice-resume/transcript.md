# mid-slice-resume — self-heal picks up between phases

The prior `/change:execute` run finished the on-disk work of `/spec:define product-catalog` (LifecycleStatus advanced to `defined`) and then crashed before any phase stamped an `outcome` field on `.metadata.yaml`. No merge outcome, no failure, no deferral — just a phase boundary caught mid-stride.

This is NOT an ambiguity: per RFC-2 §"Plan Mutation and Crash Safety", "if an active slice directory exists with no terminal outcome yet, resumption is per §Context Threading → Resumption Within a Change using `LifecycleStatus` — no plan transition is needed". Self-heal's job is to resume, not to fail.

Self-heal:

1. Scans `plan.yaml`, finds `product-catalog` with `status: in-progress`.
2. Reads `metadata.yaml`. Sees `outcome` is absent and `LifecycleStatus == defined` (non-terminal) → fall through to the mid-change resume branch.
3. Reads `LifecycleStatus == defined`. Per RFC-2 §"Context Threading → Resumption Within a Change", the next phase is `/spec:build`.
4. Does NOT call `specify change plan transition` — the plan entry stays `in-progress` for the remainder of the phase sequence.
5. Appends one `type: recovery` entry to `journal.yaml` (see `journal.yaml.after`) documenting the resume.
6. Emits one diagnostic line, jumps directly to step 7 of the supervised run (invoke `/spec:build product-catalog`), bypassing the normal step-4 `specify change plan next` and step-5 `pending → in-progress` transition (both of which are unnecessary — the entry is already the active in-progress one).

```text
Self-heal: product-catalog — resuming build (LifecycleStatus=defined)
```

The remainder of the run is the normal supervised flow: `/spec:build` runs, stamps its outcome, the driver reads it at step 9, and the sequence proceeds through `/spec:merge` to a terminal plan transition exactly as though the driver had never crashed. If the resumed `/spec:build` succeeds, `plan.yaml` ends the run with `product-catalog` at `done`; if it fails or defers, the normal step-11 / step-12 drop-and-transition paths apply.

Other mid-change resumption branches (`LifecycleStatus` of `None`, `defining`, `building`, `complete`) follow the same shape with a different action / diagnostic per the resumption table in SKILL.md. This fixture pins the `defined → invoke /spec:build` row because it is the canonical inter-phase boundary crash and exercises the no-plan-transition rule most directly.
