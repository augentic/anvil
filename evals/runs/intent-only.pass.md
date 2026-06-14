# Run: `intent-only` — **pass**

## Context

- **Scenario:** `intent-only`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0`
- **Sandbox:** `evals/.sandbox/intent-only/` (recreated fresh 2026-06-14)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `plan-validates` | pass | |
| `intent-single-lead` | pass | |
| `gate-1-not-auto-stamped` | pass | |
| `sources-intent-only` | pass | |
| `refine-reaches-refined` | pass | |

Probe transcript highlights: fresh sandbox recreated; `plan.yaml` existed with `lifecycle: pending` before Gate 1; `discovery.md` carries exactly one `- lead:` block; `plan.reconcile.completed` payload reads `"slice-count":1`; exactly one `plan.transition.approved` event (`"actor":"agent"`); every `Sources:` line names `intent` only; `specify slice validate fix-typo` exits 0 and `slice.transition.refined` names `fix-typo`; `model.yaml` carries `requirements[0].scenarios[]` with one WHEN/THEN scenario.

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- `specify init omnia@v1` substituted with the documented offline fallback `specify init <framework>/adapters/targets/omnia` (local adapter path).
- Symlinked the `intent` source adapter into the sandbox (`adapters/sources/intent`) per setup prerequisites.
- Gate 1 stamped with `specify plan transition fix-typo approved --actor agent`.
- Plan lock acquired via Python `fcntl` fallback (stock macOS lacks `flock(1)`).
- Pre-wrote `discovery.md` before survey finalize created a duplicate lead block; corrected to a single `intent:fix-typo` block before `propose --dry-run`.
- Synthesis response initially omitted `tasks[].id`; re-ran `specify slice synthesize --from` after adding `TASK-001` / `TASK-002` ids.

## Notes

- `specify slice validate` returned three non-blocking `kind: review` suggestions (imperative proposal language, SHALL/MUST phrasing, thin lead synopsis) — inherent to the degenerate one-line intent; judged acceptable.
- Kernel renders single-source provenance as `Sources: intent` (unbracketed); provenance parser accepts both forms.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/intent-only`
- **Retained at:** `evals/.sandbox/intent-only/`
- **Key paths:** `plan.yaml`, `discovery.md`, `.specify/slices/fix-typo/`, `.specify/journal.jsonl`
