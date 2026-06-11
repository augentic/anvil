# Run: `pure-intent` — **pass**

## Context

- **Scenario:** `pure-intent`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source via `make install-cli`)
- **Sandbox:** `evals/.sandbox/pure-intent/`

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `plan-validates` | pass | |
| `intent-single-lead` | pass | |
| `gate-1-not-auto-stamped` | pass | |
| `sources-intent-only` | pass | |
| `refine-reaches-refined` | pass | |

Probe transcript highlights: `plan.yaml` read `lifecycle: pending` when `/spec:plan` printed the literal `specify plan transition fix-typo approved` hint and exited; `discovery.md` carries exactly one `- lead:` block; `plan.reconcile.completed` payload reads `"slice-count":1`; exactly one `plan.transition.approved` event (`"actor":"agent"`); every `Sources:` line and the `specify slice provenance` projection name `intent` only; `specify slice validate fix-typo` exits 0 and `slice.transition.refined` names `fix-typo`.

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- `specify init omnia@v1` failed (`adapter-git-failed: Remote branch v1 not found in upstream origin`); used the documented offline fallback `specify init ../../../adapters/targets/omnia` (local adapter path).
- Symlinked the `intent` source adapter into the sandbox (`adapters/sources/intent`) per the setup prerequisite — `specify init` caches only the target adapter.
- Gate 1 stamped with the literal transition command plus `--actor agent` (agent stamping at the operator's standing direction; the journal payload records who stamped).
- Phase work driven by the agent following the `/spec:plan`, `/spec:execute`, and `/spec:refine` skill bodies directly (plan lock held for the loop; stop after `refined` per the scenario's Scope).

## Notes

- New RFC-43 journal probes observed live: `plan.entry.advanced` fired exactly once (on the real `pending → in-progress` advance) and `plan.transition.approved` carries the `actor` field.
- `specify slice validate` returned three non-blocking `kind: review` suggestions (imperative proposal language, SHALL/MUST phrasing, thin lead synopsis) — all inherent to the degenerate one-line intent; judged acceptable.
- Renderer nit, no assertion impact: the kernel renders single-source provenance as `Sources: intent` (unbracketed) while `requirement-block.md`'s canonical template shows `Sources: [intent]`; the provenance parser accepts both.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/pure-intent`
- **Retained at:** `evals/.sandbox/pure-intent/`
- **Key paths:** `plan.yaml`, `change.md`, `discovery.md`, `.specify/slices/fix-typo/` (`proposal.md`, `specs/user/spec.md`, `design.md`, `tasks.md`, `model.yaml`, `evidence/intent.yaml`), `.specify/journal.jsonl`
