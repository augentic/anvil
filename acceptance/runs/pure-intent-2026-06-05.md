# Scenario run summary

## Run header

- **Scenario id:** `pure-intent`
- **Scenario file:** `acceptance/lifecycle/01-pure-intent.md`
- **Backend:** `manual`
- **Operator / agent:** Cursor agent (Claude Opus 4.8)
- **Run id:** `pure-intent-2026-06-05`
- **Started at / finished at:** `2026-06-05T09:13Z` / `2026-06-05T09:18Z`
- **`specify` build:** `/Users/andrewweston/.local/bin/specify` → `specify 0.3.0` (symlink to `../specify-cli/target/release/specify`)
- **Workspace / project roots:** `/private/var/folders/.../specify-acceptance-pure-intent.BVn3MR/pure-intent` (disposable temp)

## Inputs created

- `<temp>/.specify/...` — created (`make acceptance-scenario ID=pure-intent` → `specify init omnia@v1`)
- `<temp>/.specify/.cache/extractions/intent/survey/scratch/lead-set.md` — created (agent stand-in for `intent.survey`)
- `<temp>/.specify/.cache/extractions/intent/fix-typo/scratch/evidence.yaml` — created (agent stand-in for `intent.extract`)
- `<temp>/proposal-response.json`, `<temp>/synth-response.json` — created (propose + synthesize response envelopes)

## Invocation

### Plan

```text
/spec:plan fix-typo "fix typo in user.rs"
  → specify plan create fix-typo --source intent=intent:value:fix typo in user.rs
  → specify source survey intent --phase prepare / (write lead-set.md) / --phase finalize
  → specify plan propose --dry-run ; specify plan propose --from proposal-response.json --reconcile-platforms
```

### Review (operator pause)

```bash
specify plan validate --format json      # 0 findings
cat plan.yaml                            # lifecycle: pending, one slice fix-typo, sources [intent]
```

- **Gate 1 stamp:** `specify plan transition fix-typo approved` (pending → approved)
- **specify plan amend invocations:** none

### Execute

```text
/spec:execute
  → specify plan next                         # active: fix-typo
  → /spec:refine: slice create / source extract intent / slice synthesize / slice validate / transition refined
  → /spec:build: specify slice build fix-typo --phase prepare  (PARKED — see Verdict)
```

## Plan structure

| Role | Slice | Project | Depends on | Sources | Status |
| --- | --- | --- | --- | --- | --- |
| local | `fix-typo` | `project` | none | `intent` | pending → approved → in-progress |

## Expected artifacts and state

| Item | Status | Notes |
| --- | --- | --- |
| `plan.yaml` | `present` | written at **project root** (`<root>/plan.yaml`), not under `.specify/` |
| `.specify/plans/fix-typo/discovery.md` | `absent (path drift)` | `discovery.md` was written at **project root** instead; see Findings |

## Assertions

| Assertion id | Verdict | Evidence pointer |
| --- | --- | --- |
| `plan-exists` | `pass` | `<root>/plan.yaml` present after `/spec:plan` |
| `plan-validates` | `pass` | `specify plan validate` → 0 findings, exit 0 |
| `intent-single-lead` | `pass` | survey → 1 lead `fix-typo`; propose → 1 slice |
| `gate-1-not-auto-stamped` | `pass` | plan stayed `lifecycle: pending`; operator ran the literal transition |
| `sources-intent-only` | `pass` | slice `sources: [{source: intent, lead: fix-typo}]` |
| `execute-loop-all-done` | `needs-human` | refine reached `refined`; build is the irreducible LLM-codegen + generated-output-correctness seam (omnia create-mode crate from a no-op intent). Not driven to `all-done`; not fabricated. |

## Negative expectations

| Negative expectation | Verdict | Notes |
| --- | --- | --- |
| `automated-runner-added` | `held` | drove real `specify` CLI only |
| `fake-forge-added` | `held` | n/a — plan-only had no forge step |
| `transcript-replay-added` | `held` | |
| `ci-target-added` | `held` | |
| `golden-output-required` | `held` | graded on durable structure only |

## Command output

- **Plan validation:** `{"summary":{"critical":0,"important":0,"suggestion":0,"optional":0},"findings":[]}`
- **Execute loop:** refine clean (journal: `slice.transition.refined`); slice validate exit 0 (3 non-blocking `kind: review` findings only); build prepared (`target.execution.agent`), not finalized.
- **Finalize invocations:** n/a

## Cleanup

- **Workspaces / projects:** retained (disposable temp dir)
- **Branches:** n/a
- **Run evidence:** this file + `<root>/.specify/journal.jsonl`

## Verdict

- **Result:** `pass` for plan + refine (5/6 deterministic assertions); `execute-loop-all-done` = `needs-human`
- **Fault domain on failure:** n/a (no CLI fault observed) — build is an LLM-codegen seam, not a CLI defect
- **Notes:** The plan/refine path is fully green on `specify 0.3.0`. The only un-confirmed assertion is `execute-loop-all-done`, which requires real Omnia WASM crate codegen from a degenerate "fix typo" intent against a project with no Rust workspace — the runbook's irreducible codegen/judgment seam. Two doc/scenario drifts observed (see chat report): plan.yaml + discovery.md land at the project root, not under `.specify/` / `.specify/plans/<name>/` as AGENTS.md and this scenario's `expected-artifacts` state.
