# Run: `execute-pause-resume` — **pass**

## Context

- **Scenario:** `execute-pause-resume`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the in-tree source)
- **Sandbox:** `quality/.sandbox/execute-pause-resume/` (recreated fresh 2026-06-15)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `breakout-state-consistent` | pass | |
| `execute-resumes-without-flags` | pass | |
| `execute-loop-all-done` | pass | |

Probe transcript highlights: two-slice `dashboard` plan (`metrics-summary`, `user-activity-feed`); first slice completed and merged before pause; `/spec:execute` paused on second slice at `build user-activity-feed` (`action: build`, entry `in-progress`, slice lifecycle `refined`, `resume: /spec:build user-activity-feed`); breakout `/spec:build user-activity-feed` from standalone session with plan lock held left `specify plan validate` exit 0 (`critical: 0`) and exactly one `status: in-progress` during pause (0 after merge); resumed `/spec:execute` merged second slice without extra flags (`action: merge` then `drained`); `grep -c 'status: done' plan.yaml` = 2; `specify journal show --filter plan.entry.advanced` shows one advance per slice (`metrics-summary`: 1, `user-activity-feed`: 1) without duplicate advance across cancel/resume.

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- Offline init via local omnia adapter path; `intent` source adapter symlinked per setup prerequisites.
- Gate 1 stamped with `--actor agent`.
- Pause simulated by stopping `/spec:execute` after `build --phase prepare` on second slice while entry remained `in-progress` at `refined`; breakout completed build+finalize for `user-activity-feed`.
- Plan lock held for the session via `specify plan lock -- <cmd>` (same posture as other eval runs).
- Minimal serde-only library crates (`metrics_summary`, `user_activity_feed`) with `cargo test` — no wasm32 guest pre-merge gate in this sandbox.
- Driver: `quality/profiles/workflow/execute-pause-resume.sh` over shared helpers `quality/profiles/workflow/single-repo.sh` + `quality/profiles/workflow/lib.sh`.

## Notes

- Resume path after breakout: `specify plan status` named `merge user-activity-feed` directly — no `--continue` or other flags.
- Subagent driver `quality/profiles/workflow/execute-pause-resume.sh` authored the multi-slice plan and full pause/breakout/resume sequence.

## Evidence

- **Retained at:** `quality/.sandbox/execute-pause-resume/`
- **Key paths:** `plan.yaml`, `crates/`, `.specify/specs/`, `.specify/archive/`, `.specify/journal.jsonl`, `quality/profiles/workflow/execute-pause-resume.sh`
