# Scenario run summary

## Run header

- **Scenario id:** `pure-intent`
- **Scenario file:** `acceptance/lifecycle/01-pure-intent.md`
- **Backend:** `manual`
- **Operator / agent:** Cursor agent (Claude Opus 4.8)
- **Run id:** `pure-intent-2026-06-06`
- **Started at / finished at:** `2026-06-05T18:55Z` / `2026-06-05T18:58Z`
- **`specify` build:** `/Users/andrewweston/.local/bin/specify` → `specify 0.3.0` (symlink to `../specify-cli/target/release/specify`)
- **Workspace / project roots:** `/private/tmp/specify-acceptance-pure-intent.vsTHv8/pure-intent` (disposable temp)

## Inputs created

- `<root>/.specify/...` — created (`specify init omnia@v1`, `SPECIFY_FRAMEWORK_ROOT` → framework checkout)
- `<root>/.specify/.cache/extractions/intent/survey/scratch/lead-set.md` — created (agent stand-in for `intent.survey`)
- `<root>/.specify/.cache/extractions/intent/fix-typo/scratch/evidence.yaml` — created (agent stand-in for `intent.extract`)
- `<root>/proposal-response.json`, `<root>/synth-response.json` — created (propose + synthesize response envelopes)

## Invocation

### Plan

```text
/spec:plan fix-typo "fix typo in user.rs"
  → specify plan create fix-typo --source intent=intent:value:fix typo in user.rs
  → specify source survey intent --phase prepare / (write lead-set.md) / --phase finalize   # 1 lead
  → specify plan propose --dry-run ; specify plan propose --from proposal-response.json --reconcile-platforms   # 1 slice
```

### Review (operator pause)

```bash
specify plan validate --format json      # 0 findings
cat plan.yaml                            # lifecycle: pending, one slice fix-typo, sources [intent]
```

- **Gate 1 stamp:** `specify plan transition fix-typo approved` (pending → approved)
- **specify plan amend invocations:** none

### Execute (refine only — build/merge out of scope per scenario)

```text
specify plan next                                  # active: fix-typo, target omnia@v1
specify slice create fix-typo --target omnia@v1
specify source extract intent fix-typo --slice fix-typo --phase prepare / (write evidence.yaml) / --phase finalize
specify slice synthesize fix-typo --dry-run ; --from synth-response.json
specify slice validate fix-typo                    # 0 violation findings (after tasks-format fix); 3 review-only suggestions
specify slice transition fix-typo refined
```

## Plan structure

| Role | Slice | Project | Depends on | Sources | Status |
| --- | --- | --- | --- | --- | --- |
| local | `fix-typo` | `project` | none | `intent` | pending → approved → refined |

## Expected artifacts and state

| Item | Status | Notes |
| --- | --- | --- |
| `plan.yaml` | `present` | written at **project root** (`<root>/plan.yaml`), not under `.specify/` |
| `.specify/plans/fix-typo/discovery.md` | `absent (path drift)` | `discovery.md` written at **project root** instead; see Findings |

## Assertions

| Assertion id | Verdict | Evidence pointer |
| --- | --- | --- |
| `plan-exists` | `pass` | `<root>/plan.yaml` present after `/spec:plan` |
| `plan-validates` | `pass` | `specify plan validate` → 0 findings, exit 0 |
| `intent-single-lead` | `pass` | survey → 1 lead `fix-typo`; propose dry-run → 1 lead; propose → 1 slice |
| `gate-1-not-auto-stamped` | `pass` | plan stayed `lifecycle: pending`; operator ran the literal transition |
| `sources-intent-only` | `pass` | slice `sources: [{source: intent, lead: fix-typo}]`; rendered `spec.md` → `Sources: intent` |
| `refine-reaches-refined` | `pass` | slice validate exit 0 (3 review-only findings); `.metadata.yaml` status `refined` |

## Negative expectations

| Negative expectation | Verdict | Notes |
| --- | --- | --- |
| `automated-runner-added` | `held` | drove real `specify` CLI only |
| `fake-forge-added` | `held` | n/a — no forge step |
| `transcript-replay-added` | `held` | |
| `ci-target-added` | `held` | |
| `golden-output-required` | `held` | graded on durable structure only |

## Command output

- **Plan validation:** `{"summary":{"critical":0,"important":0,"suggestion":0,"optional":0},"findings":[]}`
- **Refine:** synthesize persisted `proposal.md / specs/fix-typo/spec.md / design.md / tasks.md / model.yaml`; slice validate exit 0; transition `refined`.
- **Finalize invocations:** n/a

## Cleanup

- **Workspaces / projects:** retained (disposable temp dir)
- **Branches:** n/a
- **Run evidence:** this file

## Verdict

- **Result:** `pass` — all 6 in-scope deterministic assertions pass.
- **Fault domain on failure:** n/a (no CLI fault observed).
- **Notes:** Wave-0 hard halt is GREEN on `specify 0.3.0`; the sweep may proceed. Two observations carried over and re-confirmed from the 2026-06-05 run: (1) `plan.yaml` and `discovery.md` land at the **project root**, not under `.specify/` / `.specify/plans/<name>/` as `AGENTS.md` and this scenario's `expected-artifacts` state (path drift). (3) A first synthesis attempt produced a **blocking** `tasks.use-checkbox-format` violation because the tasks body followed the `references/synthesis/substeps.md` §4 form (`- [ ] <description>`); the `tasks.use-checkbox-format` validator and `docs/reference/artifact-format.md` require the numbered `- [ ] X.Y` form. Re-authoring with `- [ ] 1.1 …` cleared it. This is a doc-vs-validator drift in the framework under test, not a CLI defect.
