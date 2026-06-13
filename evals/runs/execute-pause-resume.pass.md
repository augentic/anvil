# Run: `execute-pause-resume` — **pass**

## Context

- **Scenario:** `execute-pause-resume`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source via `make install-cli`)
- **Sandbox:** `evals/.sandbox/execute-pause-resume/`

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `breakout-state-consistent` | pass | |
| `execute-resumes-without-flags` | pass | |
| `execute-loop-all-done` | pass | |

Probe transcript highlights: two-slice `dashboard` plan (`metrics-summary`, `user-activity-feed`); first slice completed and merged before pause; `/spec:execute` paused on second slice at `build user-activity-feed` (`action: build`, entry `in-progress`, slice lifecycle `refined`); breakout `/spec:build user-activity-feed` from standalone session with plan lock held left `specify plan validate` exit 0 and exactly one `status: in-progress`; resumed `/spec:execute` (via driver) merged second slice without extra flags; `specify plan status` reports `"action":"drained"` with two `status: done` entries; `specify journal show --filter plan.entry.advanced` shows one advance per slice without duplicate advance for `user-activity-feed` across cancel/resume.

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- Offline init via local omnia adapter path; `intent` source adapter symlinked per setup prerequisites.
- Gate 1 stamped with `--actor agent`.
- Pause simulated by stopping `/spec:execute` after first slice merge while second slice remained `in-progress` at build; breakout completed build+merge for `user-activity-feed`.
- macOS plan lock via Python `fcntl` fallback (same posture as other eval runs).
- Minimal omnia crates (`metrics_summary`, `user_activity_feed`) with `cargo test` — no wasm32 guest pre-merge gate in this sandbox.

## Notes

- Subagent driver `_drive.zsh` authored the multi-slice plan and first-slice completion; parent agent verified drained terminal state and assertion probes.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/execute-pause-resume`
- **Retained at:** `evals/.sandbox/execute-pause-resume/`
- **Key paths:** `plan.yaml`, `crates/`, `.specify/specs/`, `.specify/archive/`, `.specify/journal.jsonl`
