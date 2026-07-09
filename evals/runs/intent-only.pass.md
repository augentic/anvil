# Run: `intent-only` — **pass**

## Context

- **Scenario:** `intent-only`
- **Operator:** Cursor agent (agent-as-operator, per the agent runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0`
- **Sandbox:** `evals/.sandbox/intent-only/` (recreated fresh 2026-06-15)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `plan-validates` | pass | |
| `intent-single-lead` | pass | |
| `gate-1-not-auto-stamped` | pass | |
| `sources-intent-only` | pass | |
| `refine-reaches-refined` | pass | |

Probe transcript highlights: fresh sandbox recreated; `plan.yaml` existed with `lifecycle: pending` before Gate 1; `discovery.md` carries exactly one `- lead:` block; `plan.reconcile.completed` payload reads `"slice-count":1`; exactly one `plan.transition.approved` event (`"actor":"agent"`); every `Sources:` line names `intent` only; `specify slice validate fix-typo` exits 0 and `slice.transition.refined` names `fix-typo`; `model.yaml` carries one requirement with a WHEN/THEN scenario string.

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- `specify init omnia@1.0.0` substituted with the documented offline fallback `specify init <framework>/adapters/targets/omnia` (local adapter path).
- Symlinked the `intent` source adapter into the sandbox (`adapters/sources/intent`) per setup prerequisites.
- Gate 1 stamped with `specify plan transition fix-typo approved --actor agent`.
- Plan lock held for refine via `specify plan lock -- <cmd>`.

## Notes

- `specify slice validate` returned three non-blocking `kind: review` suggestions (imperative proposal language, SHALL/MUST phrasing, thin lead synopsis) — inherent to the degenerate one-line intent; judged acceptable.
- Kernel renders single-source provenance as `Sources: intent` (unbracketed); provenance parser accepts both forms.

## Evidence

- **Retained at:** `evals/.sandbox/intent-only/`
- **Key paths:** `plan.yaml`, `discovery.md`, `.specify/slices/fix-typo/`, `.specify/journal.jsonl`
