# Run: `single-project-plan` — **pass**

## Context

- **Scenario:** `single-project-plan`
- **Operator:** Cursor agent (Fable)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from local `../specify-cli` via `make install-cli`)
- **Sandbox:** `acceptance/.sandbox/single-project-plan/`

## Assertions

| Assertion | Verdict |
| --- | --- |
| `plan-exists` | pass |
| `plan-validates` | pass |
| `slices-match-expected-shape` | pass |
| `no-project-routing-required` | pass |

**Negative expectations:** held (manual-by-design posture unchanged).

## Deviations

- Used local `specify init <framework>/adapters/targets/omnia` (`omnia@v1` remote fetch failed: `Remote branch v1 not found in upstream origin` — same failure as prior runs).
- Symlinked `adapters/sources/documentation` into the project (source adapters are not vendored by `specify init`, per `shared/setup.md`).
- Drove the `/spec:plan` lifecycle via the CLI verbs the skill orchestrates (`plan create --source docs=documentation:./docs/inventory-adjustments.md`, two-phase `source survey docs`, `plan propose --dry-run` / `--from .specify/cache/propose-response.json --reconcile-platforms`, `plan validate`) rather than the slash command in Cursor chat.

## Notes

- Survey emitted one lead (`docs:inventory-adjustments`): the brief is a single monolithic file whose H1 carries the opening behavioural paragraph, so the one-concept rule applied; `discovery.md` written in the three-section form.
- Propose wrote exactly one slice `inventory-adjustments` bound `{ source: docs, lead: inventory-adjustments }` with empty `depends-on` — consistent with the brief's "keep the first release small" scope; no dependency was invented between local slices.
- `no-project-routing-required`: no `registry.yaml` exists; the slice's `project: project` is the auto-bound sole project from `propose --from`, not a registry-derived routing assignment. No discriminator or routing fields appear in `plan.yaml`.
- `specify plan validate --format json` exited 0 with zero findings.
- Plan stopped at `lifecycle: pending`; Gate 1 was **not** stamped (scenario ends after plan validation). Closing hint printed verbatim: ``Plan `inventory-adjustments` is at `pending`. Run `specify plan transition inventory-adjustments approved` to stamp Gate 1, then `/spec:execute` to drive the slices.``
- Journal timeline: `source.execution.agent` → `source.survey.cache-miss` (adapter-opt-out) → `plan.reconcile.completed` (slice-count 1).

## Evidence

- **Reproduce:** `scripts/snapshot.sh acceptance/.sandbox/single-project-plan`
- **Retained at:** `acceptance/.sandbox/single-project-plan/`
- **Key paths:** `plan.yaml`, `change.md`, `discovery.md`, `docs/inventory-adjustments.md`, `.specify/journal.jsonl`, `.specify/cache/propose-response.json`
