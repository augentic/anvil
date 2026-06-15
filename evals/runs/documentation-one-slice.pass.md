# Run: `documentation-one-slice` — **pass**

## Context

- **Scenario:** `documentation-one-slice`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0`
- **Sandbox:** `evals/.sandbox/documentation-one-slice/` (recreated fresh 2026-06-15)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `plan-validates` | pass | |
| `single-slice-from-doc` | pass | |
| `sources-documentation-only` | pass | |
| `execute-loop-all-done` | pass | |

Probe transcript highlights: fresh sandbox recreated; `plan.reconcile.completed` payload reads `"slice-count":1`; baseline `Sources: brief` on merged spec at `.specify/specs/health-check/spec.md`; `specify plan status --format json` reports `"action":"drained"` with one `status: done` entry; one `plan.entry.advanced` event names `health-check`; `specify slice merge run` created baseline with one requirement.

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- Offline init via local omnia adapter path instead of `omnia@v1` network fetch.
- Symlinked `adapters/sources/documentation` per setup prerequisites.
- Build used `omnia-sdk = "0.33"` on crates.io; workspace `wasip3` pinned to `0.6` to match `omnia-wasi-http`.
- Phase work driven by following `/spec:plan`, `/spec:refine`, `/spec:build`, and `/spec:merge` skill bodies via CLI verbs with the plan lock held via `specify plan lock -- <cmd>`.
- Gate 1 stamped with `specify plan transition feature-doc approved --actor agent`.
- Initial synthesis artifacts needed proposal `## Why` and tasks `X.Y` checkbox format fixes before `specify slice validate` passed cleanly (two `kind: review` suggestions on imperative/SHALL language judged acceptable).

## Notes

- `specify plan next` after lock release returns `plan-lock-not-held` (exit 2); drained state verified via `specify plan status`.
- `grep -c '^  - name: ' plan.yaml` returns 0 because this CLI writes slice rows as `- name:` (no leading indent); journal `slice-count:1` and `^- name:` count of 1 are the authoritative probes.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/documentation-one-slice`
- **Retained at:** `evals/.sandbox/documentation-one-slice/`
- **Key paths:** `plan.yaml`, `crates/health_check/`, `.specify/specs/health-check/spec.md`, `.specify/archive/`, `.specify/journal.jsonl`
