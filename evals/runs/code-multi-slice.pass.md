# Run: `code-multi-slice` — **pass**

## Context

- **Scenario:** `code-multi-slice`
- **Operator:** Cursor agent (Fable 5)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0`
- **Sandbox:** `acceptance/.sandbox/code-multi-slice/`

## Assertions

| Assertion | Verdict |
| --- | --- |
| `plan-exists` | pass |
| `plan-validates` | pass |
| `multiple-slices-from-code` | pass |
| `sources-legacy-only` | pass |
| `no-under-slicing` | pass |

**Negative expectations:** held (manual-by-design posture unchanged).

## Deviations

- Used local `specify init <framework>/adapters/targets/omnia` (`omnia@v1` remote fetch failed: `Remote branch v1 not found in upstream origin` — same as the `documentation-multi-slice` run).
- Symlinked `adapters/sources/typescript` into the project (source adapters are not vendored by `specify init`).
- Authored the legacy TypeScript service in-sandbox at `vendor/legacy-monolith/` (Express + BullMQ + node-cron, 18 files, ~1244 production LOC). The checked-in `acceptance/fixtures/sources/typescript` fixture is single-lead by design (below the survey brief's 1000-LOC Decision-2 threshold), so it cannot exercise multi-slice decomposition. Binding command: `specify plan create legacy-port --source legacy=typescript:./vendor/legacy-monolith`.
- Drove the plan lifecycle via CLI equivalents of `/spec:plan` (`plan create`, `source survey --phase prepare/finalize` with agent-executed survey brief, `plan propose --dry-run` / `--from --reconcile-platforms`) rather than the slash command in Cursor chat.

## Notes

- Survey enumerated 13 surfaces (10 Express `http-route`, 1 BullMQ `message-pub`, 1 `message-sub`, 1 node-cron `scheduled-job`); union LOC 1244 ≥ 1000 → surface leads, clustered per the ≥ 50%-touches rule into 5 leads: `user-accounts`, `product-catalog`, `order-management`, `order-notifications`, `nightly-sales-report` (staged walk retained at `.specify/cache/extractions/typescript/scratch/survey/staged.json`).
- Propose mapped the 5 leads 1:1 into 5 slices with `depends-on` ordering (`order-management` → accounts + catalog; notifications and the nightly report downstream of orders). No enumerate/repair loop was needed — distinct behaviors were not collapsed.
- Every slice's provenance is the sole `legacy` source key; `specify plan validate --format json` exited 0 with zero findings; plan stayed `lifecycle: pending`; Gate-1 command: `specify plan transition legacy-port approved` (not stamped — scenario stops at Gate 1).
- Journal taxonomy as expected: `source.execution.agent` → `source.survey.cache-miss` (`adapter-opt-out`) → `plan.reconcile.completed` (slice-count 5).
- Doc/CLI drift observed (not an assertion failure): the plan skill states "The CLI writes `change.md` and `plan.yaml` atomically", but `specify plan create` 0.2.0 scaffolds only `plan.yaml` (its help says as much); the agent authored `change.md` for the Gate-1 review prose.

## Evidence

- **Reproduce:** `scripts/snapshot.sh acceptance/.sandbox/code-multi-slice`
- **Retained at:** `acceptance/.sandbox/code-multi-slice/`
- **Key paths:** `plan.yaml`, `discovery.md`, `change.md`, `vendor/legacy-monolith/`, `.specify/journal.jsonl`, `.specify/cache/extractions/typescript/scratch/survey/{staged.json,leads.md}`
