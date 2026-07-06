---
name: specify-build
description: Build the active in-progress slice by driving the guest-routed `specify slice build` verb and running its target adapter's build brief. Use when `/spec:execute` parks on a build failure, when running build standalone after `/spec:refine`, or to retry the brief after fixing a failing task; not when the slice has yet to be refined (use `/spec:refine`) or has already merged.
argument-hint: "[slice-name]"
---

# Build skill

Build the active `in-progress` slice. The skill body is shared with the `/spec:execute` loop — when the loop runs build, it loads this same body. Both the loop and standalone breakouts resolve the active slice from `specify plan next` and drive the guest-routed `specify slice build` orchestration (request assembly → brief → finalize tail). Deterministic work — slice resolution, lifecycle reads, target resolution, build-request assembly, report schema-validation, the `target-build-*` aborts, the `slice.build.*` events, and the `Refined → Built` transition gate — flows through the `specify` CLI; this body owns ONLY running the target build brief against the prepared request.

The skill refuses only on slice lifecycle. Synthesis review tags (`[unknown]`, `[conflict]`, `[divergence]`) carried in `spec.md` are review signals, not build blockers — operators may have reconciled the tags after `/spec:refine` (via `specify plan amend --authority-override` + re-refine, or prose edits outside the kernel-rendered provenance lines), and the brief proceeds against whatever spec is on disk.

## Bindings

```text
$SLICE                  = active in-progress plan entry's slice name (from `specify plan next`)
$TARGET                 = active slice's target adapter (from the prepare handoff)
$PROJECT                = active slice's workspace project (workspace mode only)
$SLICE_DIR              = .specify/slices/$SLICE/
$REQUEST                = $SLICE_DIR/build/request.yaml (written by the orchestration's request-assembly leg)
$REPORT                 = $SLICE_DIR/build/report.yaml (brief-written, validated by the finalize tail)
$LOG_PATH               = brief-captured stdout/stderr on failure (target-specific path)
```

`$SLICE` defaults to the active `in-progress` entry. When `[slice-name]` is supplied it MUST equal that active entry; mismatches refuse to preserve the single-active-slice invariant.

## Critical Path

1. **Mutual exclusion is guest-owned.** The guest-routed verbs hold the `.specify/guest.lock` marker for the run's lifetime (see [plan-lock.md](../../references/plan-lock.md)); a concurrent driver fails loudly host-side. No explicit lock step remains.
2. **Resolve the active slice.** Run `specify plan next --format json`. If `[slice-name]` was passed, validate it matches the returned `in-progress` entry; refuse on mismatch. Read `$PROJECT` (workspace mode) from the same response.
3. **Workspace routing.** When `.specify/project.yaml` carries `workspace: true`, route per [`../execute/references/workspace-routing.md`](../execute/references/workspace-routing.md) (sync, `chdir` into the slot, `SPECIFY_PLAN_DIR` export; restore on exit). Single-repo mode is a no-op.
4. **Refuse on slice lifecycle.** Read `$SLICE_DIR/metadata.yaml`. Proceed only when `status: refined`. Pre-`refined` (e.g. `refining`) → halt with hint pointing at `/spec:refine`. Post-`refined` (`built`, `merged`, `dropped`) → halt with "no rebuild needed" / "already merged".
5. **Run the build orchestration.** Run `specify slice build $SLICE --format json`. The orchestration resolves `$TARGET` from the slice's bound `metadata.yaml`, assembles + schema-validates the request to `$REQUEST`, emits `target.execution.agent`, and surfaces the handoff envelope (`slice`, `target`, `request`, `report`, `briefs-dir`, `build-brief`, `execution: agent`). Read `target`, `request`, `report`, and `build-brief` / `briefs-dir` from it.
6. **Run the target build brief.** Read the handoff's `build-brief` (`adapters/targets/$TARGET/prose/briefs/build.md`) and execute it against the prepared `$REQUEST` — agent codegen plus target-local validation. The brief carries the orchestration (omnia: crate / test / guest / review; vectis: core / iOS / Android / `composition.yaml`; contracts: format-dispatched author-import-verify). Follow it linearly; do not invoke standalone writer or reviewer skills directly. Write the build report to the handoff's `report` path (`$REPORT`) — `status: success` on a clean run, `status: failure` on a brief-side failure (see § Stop hint contract).
7. **Finalize and gate the transition.** The orchestration's finalize tail frames entry with `slice.build.started`, reads + schema-validates `$REPORT`, rejects a `status: success` report carrying any blocking finding (`target-build-success-with-blocking-finding`) and any `status: failure` report (`target-build-failed`), and on a clean success report GATES the `Refined → Built` transition, emitting `slice.build.succeeded`. On any failure it emits `slice.build.failed`, exits non-zero, and leaves the slice at `refined` (see § Stop hint contract). Return control to the caller; `/spec:execute` advances to merge, otherwise the operator runs `/spec:merge $SLICE`.

## Stop hint contract

A build failure surfaces a stop hint as the body's final output — a single structured message the operator (or the parent loop) can act on without re-deriving context:

- `slice` — slice name from `specify plan next`.
- `phase` — `build`.
- `failing-task` — the brief sub-step (or `tasks.md` checkbox) that exited non-zero (e.g. `cargo test`, `clippy`, `cargo build --target wasm32-wasip2`), or the failing CLI phase (`prepare` / `finalize`).
- `log-path` — absolute path to the captured stdout/stderr (`$SLICE_DIR/.build-log` is the default; targets MAY override).
- `next-action` — typically `re-run /spec:build $SLICE after fix`, optionally a target-specific suggestion (e.g. `inspect $LOG_PATH for clippy lints`).

A failure can surface from (a) **request assembly** (`target-build-input-missing` / `target-build-request-schema`) — the run stops before step 6 and never writes `$REPORT`; (b) the **brief's** own verify-repair loop — write a `status: failure` report and let the finalize tail convert it into the structured abort; or (c) the **finalize tail** (`target-build-report-schema`, `target-build-success-with-blocking-finding`, `target-build-failed`).

Render the hint as the final visible output of the run. Do not call `specify slice transition $SLICE built` by hand — the finalize tail owns the gate and stamps `built` only on a clean success report; on any failure report (or a prepare-side abort) the slice stays `refined` so the loop (or a re-invocation) re-enters cleanly. Do not write to `plan.yaml`; the per-entry status stays `in-progress`, which is the v1 wire signal that execution is parked rather than drained.

## Guardrails

- **Refuse only on slice lifecycle, never on synthesis tags.** `[unknown]` / `[conflict]` / `[divergence]` in `spec.md` are review signals; the build proceeds against whatever spec is on disk.
- **The `built` transition is owned by the `specify slice build` finalize tail.** `/spec:build` never calls `specify slice transition $SLICE built`; the `Refined → Built` gate fires inside the tail on a clean success report.
- **Never write `plan.yaml` from this body.** Per-entry transitions are owned by `specify plan next` (writes `in-progress`) and `specify slice merge` (writes `done`). `/spec:build` writes no lifecycle state by hand — the slice's `metadata.yaml` is stamped by the `specify slice build` finalize tail.
- **Lifecycle single-writer:** [shared guardrails](../../references/guardrails.md#single-writer-for-lifecycle-state).
- **Never invoke standalone writer or reviewer skills directly.** The target build brief carries those bodies inline; calling them out-of-band bypasses brief orchestration and breaks shape-injection guarantees.
- On prepare/finalize abort, exhausted verify-repair, or a `deferred` outcome: emit the stop hint and **exit**; never patch adapters, templates, or cache — [Consumer tooling boundary](../../references/guardrails.md#consumer-tooling-boundary).

## References

- [shared guardrails](../../references/guardrails.md#single-writer-for-lifecycle-state) — single-writer rules for `metadata.yaml`, `plan.yaml`, archive paths.
- [Consumer tooling boundary](../../references/guardrails.md#consumer-tooling-boundary) — stop on scaffold/verify/finalize/toolchain failure; never patch upstream tooling in-band.
- `adapters/targets/<target>/prose/briefs/build.md` — the orchestration this skill loads and executes (omnia, vectis, contracts); it also writes the `build/report.yaml` that the finalize tail validates.
