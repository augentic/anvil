# Run: `workspace-stale-recovery` — **pass**

## Context

- **Scenario:** `workspace-stale-recovery`
- **Operator:** Cursor agent (agent-as-operator, per the agent runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0`
- **Sandbox:** `evals/.sandbox/workspace-stale-recovery/` (`platform/`, `backend/`, `mobile/`, `contracts/`)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `dirty-slot-detected-at-sync` | pass | |
| `slice-state-preserved` | pass | |
| `resume-continues-from-in-progress` | pass | |
| `execute-loop-all-done` | pass | |

Probe transcript highlights: interrupted `/spec:execute` mid `oauth-backend` build left backend slot dirty; `specify workspace sync` completed; triage committed resume-safe slice-tree dirtiness; resumed loop drained with four `status: done` entries; `specify plan status` reports `"action":"drained"`.

**Negative expectations:** held.

## Deviations

- Offline local adapter paths; bare-repo `file://` origins.
- Plan authored headlessly via `evals/drivers/workspace.sh workspace-stale-recovery`; Gate 1 stamped `--actor agent`.
- Pre-merge git staging fix in `merge_slice` to avoid `dirty-unrelated-tracked` on contracts slot.

## Notes

- Multi-step invocation followed: interrupt → workspace sync → resume to all-done.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/workspace-stale-recovery`
- **Retained at:** `evals/.sandbox/workspace-stale-recovery/`
- **Key paths:** `platform/plan.yaml`, `platform/workspace/{backend,mobile,contracts}/`, `platform/.specify/journal.jsonl`
