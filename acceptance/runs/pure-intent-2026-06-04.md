# Scenario run summary

## Run header

- **Scenario id:** `pure-intent`
- **Scenario file:** `acceptance/suites/lifecycle/01-pure-intent/scenario.md`
- **Backend:** `manual`
- **Operator / agent:** `Claude Opus 4.8 (Cursor agent)`
- **Run id:** `pure-intent-2026-06-04`
- **Started at / finished at:** `2026-06-04T06:38:00Z` / `2026-06-04T06:48:00Z`
- **`SPECIFY_BIN`:** `specify-cli/target/release/specify` (2.0 build; `slice` command present; `--version` reports 0.3.0)
- **Workspace / project roots:** disposable temp root under `/tmp/specify-acceptance.*/specify-acceptance-pure-intent` (discarded after the run)

## Inputs created

- `project root` — `created` via `specify init <omnia adapter path>` (the `omnia@v1` shorthand resolves as a relative path; pointed `init` at the in-repo `adapters/targets/omnia`).
- `adapters` symlink → repo `adapters/` — `created` (so the `intent` source adapter resolved from the disposable project).
- `.specify/.cache/extractions/intent/survey/scratch/lead-set.md` — `created` (agent-produced survey output).
- `.specify/.cache/extractions/intent/fix-typo/scratch/evidence.yaml` — `created` (agent-produced extract Evidence).

## Invocation

### Plan

```text
/spec:plan fix-typo "fix typo in user.rs"
```

Driven via the CLI choreography the plan skill owns:
`specify plan create fix-typo --source intent=intent:value:fix typo in user.rs`
→ `specify source survey intent --phase prepare|finalize`
→ `specify plan propose --dry-run` then `--from <response.json> --reconcile-platforms`.

### Review (operator pause)

```bash
specify plan validate --format json   # 0 findings
cat plan.yaml                          # lifecycle: pending; slices: [fix-typo], sources: [intent]
```

- **Gate 1 stamp:** `specify plan transition fix-typo approved` (pending → approved)
- **specify plan amend invocations:** `none`

### Execute

```text
specify plan next                                  # next: fix-typo (target omnia@v1), status in-progress
specify slice create fix-typo --target omnia@v1    # refining
specify source extract intent fix-typo --slice fix-typo --phase prepare|finalize
specify slice synthesize fix-typo --dry-run | --from <response.json>
specify slice validate fix-typo                    # BLOCKS — see Assertions
```

The loop never reached `all-done`: `specify slice validate` returns blocking violations, so the slice could not transition to `refined`.

### Finalize  `n/a — execute did not complete`

## Plan structure

| Role | Slice | Project | Depends on | Sources | Status |
| --- | --- | --- | --- | --- | --- |
| implementation | `fix-typo` | `project` | none | `intent` | `in-progress` (stuck at `refining`) |

## Expected artifacts and state

| Item | Status | Notes |
| --- | --- | --- |
| `plan.yaml` | `present` | Written at the **project root**, not under `.specify/` (drift vs. docs / scenario `expected-artifacts`). |
| `.specify/plans/fix-typo/discovery.md` | `absent` | `discovery.md` is written at the **project root** instead. |
| `.specify/slices/fix-typo/` (proposal/spec/design/tasks/model) | `present` | Synthesized, but the slice fails validation. |

## Assertions

| Assertion id | Verdict | Evidence pointer |
| --- | --- | --- |
| `plan-exists` | `pass` | `plan.yaml` written by `plan create`. |
| `plan-validates` | `pass` | `specify plan validate` → 0 findings. |
| `intent-single-lead` | `pass` | one lead `fix-typo` → one slice. |
| `gate-1-not-auto-stamped` | `pass` | plan exits at `lifecycle: pending`; stamp was operator-run. |
| `sources-intent-only` | `pass` | slice `sources: [intent]`. |
| `execute-loop-all-done` | `fail` | `specify slice validate fix-typo` blocks (`spec.requirement-sources-empty`, `specs.requirements-have-scenarios`); slice never reaches `refined`. See #149, #150. |

## Negative expectations

| Negative expectation | Verdict | Notes |
| --- | --- | --- |
| `automated-runner-added` | `held` | No runner added; driven by hand via real CLI. |
| `fake-forge-added` | `held` | No forge involved. |
| `transcript-replay-added` | `held` | None. |
| `ci-target-added` | `held` | None. |
| `golden-output-required` | `held` | Structural checks only. |

## Command output

- **Plan validation:** `specify plan validate --format json` → `{critical:0, important:0, suggestion:0, optional:0}`.
- **Execute loop:** `specify slice validate fix-typo` → blocking `proposal.units-listed`, `specs.requirements-have-scenarios` (and, on the verbatim-brief path, `spec.requirement-sources-empty`). Did not reach `all-done`.
- **Finalize invocations:** `n/a`.

## Cleanup

- **Workspaces / projects:** `discarded` (temp root removed).
- **Branches:** `n/a`.
- **Run evidence:** this file; follow-up issues augentic/specify#149, augentic/specify#150.

## Verdict

- **Result:** `fail`
- **Fault domain on failure:** `synthesis`
- **Notes:** Two reproducible blockers stop `/spec:refine` from transitioning the release-blocker slice to `refined`:
  1. `intent.extract` omits the claim `id`, but synthesis drops id-less claims from the anchor index → empty `Sources:` (`spec.requirement-sources-empty`). Follow-up: augentic/specify#149.
  2. `specify slice synthesize` renders scenarios as `- …` bullets, but `specify slice validate` requires `#### Scenario:` headings → `specs.requirements-have-scenarios` (general defect across all targets). Follow-up: augentic/specify#150.

  Per the Wave 0 hard-halt rule, the sweep stopped here; Waves 1–2 were not run. Automated surface (`make lint` + `tests/fan_in_fan_out.rs`) is green.
