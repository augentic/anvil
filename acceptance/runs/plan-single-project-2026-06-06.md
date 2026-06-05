# Scenario run summary

## Run header

- **Scenario id:** `plan-single-project`
- **Scenario file:** `acceptance/lifecycle/plan-single-project.md`
- **Backend:** `manual`
- **Operator / agent:** Cursor agent (Claude Opus 4.8)
- **Run id:** `plan-single-project-2026-06-06`
- **Started at / finished at:** `2026-06-05T18:59Z` / `2026-06-05T19:00Z`
- **`specify` build:** `/Users/andrewweston/.local/bin/specify` → `specify 0.3.0`
- **Workspace / project roots:** `/private/tmp/specify-acceptance-plan-single.3yFWYr/inventory` (disposable temp)

## Inputs created

- `<root>/.specify/...` — created (`specify init omnia@v1`)
- `<root>/docs/inventory-adjustments.md` — created (scenario brief)
- `<root>/.specify/.cache/extractions/documentation/survey/scratch/lead-set.md` — created (agent stand-in for `documentation.survey`)
- `<root>/proposal-response.json` — created

## Invocation

### Plan

```text
/spec:plan inventory-adjustments from docs/inventory-adjustments.md
  → specify plan create inventory-adjustments --source brief=documentation:docs/inventory-adjustments.md
  → specify source survey brief --phase prepare / (write lead-set.md) / --phase finalize   # 1 lead
  → specify plan propose --dry-run ; specify plan propose --from proposal-response.json --reconcile-platforms   # 1 slice
```

### Review (operator pause)

```bash
specify plan validate --format json      # 0 findings
cat plan.yaml                            # lifecycle: pending; 1 local slice; sources [brief=documentation]
```

- **Gate 1 stamp:** n/a — scenario ends at plan validation (`stages: [plan]`).
- **specify plan amend invocations:** none

### Execute / Finalize

n/a — plan-only scenario.

## Plan structure

| Role | Slice | Project | Depends on | Sources | Status |
| --- | --- | --- | --- | --- | --- |
| local | `inventory-adjustments` | `project` (auto-bound sole project) | none | `brief` (documentation) | pending |

## Expected artifacts and state

| Item | Status | Notes |
| --- | --- | --- |
| `plan.yaml` | `present` | written at project root |
| `.specify/plans/inventory-adjustments/discovery.md` | `absent (path drift)` | `discovery.md` written at project root instead |
| `.specify/plans/inventory-adjustments/proposal.md` | `absent` | no propose-time `proposal.md` written (plan-only; proposal authored at refine) |

## Assertions

| Assertion id | Verdict | Evidence pointer |
| --- | --- | --- |
| `plan-exists` | `pass` | `<root>/plan.yaml` present |
| `plan-validates` | `pass` | `specify plan validate` → 0 findings, exit 0 |
| `slices-match-expected-shape` | `pass` | one named slice `inventory-adjustments`; tightly-coupled goals → single cohesive slice; no spurious deps |
| `no-project-routing-required` | `pass` | slice bound to auto `project`; no registry-derived routing fields (single-project init, no registry) |

## Negative expectations

| Negative expectation | Verdict | Notes |
| --- | --- | --- |
| `automated-runner-added` | `held` | real CLI only |
| `fake-forge-added` | `held` | n/a |
| `transcript-replay-added` | `held` | |
| `ci-target-added` | `held` | |
| `golden-output-required` | `held` | structural grading only |

## Command output

- **Plan validation:** `{"summary":{"critical":0,"important":0,"suggestion":0,"optional":0},"findings":[]}`

## Cleanup

- **Workspaces / projects:** retained (disposable temp dir)
- **Run evidence:** this file

## Verdict

- **Result:** `pass` — all 4 assertions pass.
- **Notes:** Single-project plan generation is clean on `specify 0.3.0`. Lead grouping judgment: the brief is a monolithic single-feature doc (H1 title + meta `## Goals` / `## Scope`), so the four goals were treated as facets of one stock-adjustment behaviour → one slice (defensible per `documentation.survey` "skip non-behavioural sections"). `expected-artifacts` lists `.specify/plans/<name>/discovery.md` and `proposal.md`; `discovery.md` lands at project root (path drift, same as pure-intent), and no `proposal.md` is written at plan time (proposal is a refine-time artifact) — both are scenario/doc-vs-binary drifts, not plan defects.
