# Cross-Repo Scenario Run Summary

> Reusable template for capturing one manual cross-repo scenario run.

Fill in the fields below for one run. Keep this document next to the run's
evidence, or paste it into the operator's notes for a fully manual run. On
failure, preserve enough evidence for another operator to reproduce the state.

---

## Run Header

- **Scenario ID:** `cross-repo-contract-flow`
- **Scenario file:** `acceptance/suites/cross-repo-workflow/scenario.md`
- **Backend:** `manual`
- **Operator / agent:** `<name or model identifier>`
- **Run id:** `<timestamp or uuid>`
- **Started at:** `<ISO 8601 timestamp>`
- **Finished at:** `<ISO 8601 timestamp>`
- **Workspace:** `<temporary workspace project root>`
- **Backend project root:** `<temporary Omnia project root>`
- **Mobile project root:** `<temporary Vectis project root>`
- **`SPECIFY_BIN`:** `<path or "PATH default">`

## Inputs Created

List every file the operator created before invocation.

- `docs/oauth-login.md` - `<created | reused | modified>`
- `<other path>` - `<one-line description>`

## Invocation

Record the exact slash-command and CLI commands actually run, in order, across
the four lifecycle stages. These should match the scenario's **Invocation**
section; record any deviation explicitly.

### Stage 1 — Draft

```text
<paste the /spec:plan ... prompt that was run>
```

### Stage 2 — Review (operator pause)

```bash
<paste the specify plan validate / inspect plan.yaml commands actually run>
```

- **Operator action taken:** `<accepted-as-authored | edited-via-amend | aborted>`
- **`specify plan amend` invocations:** `<none | list>`

### Stage 3 — Execute

```text
<paste the /spec:execute loop command or prompt that was run>
```

```bash
<paste any post-execute inspect plan.yaml / inspect .specify/workspace/<project> with git status commands>
```

### Stage 4 — Finalize

Record each `/spec:finalize` invocation in order. Three invocations are
expected for the parity scenario: halts on unmerged PRs, archives, then
re-entry reports `plan-not-found`.

```text
<paste the first /spec:finalize oauth-login invocation>
```

```bash
<paste the operator's external merge commands or describe the forge UI action>
```

```text
<paste the second /spec:finalize oauth-login invocation>
```

```text
<paste the third /spec:finalize oauth-login re-entry>
```

## Plan Structure

Record the final planned slices before execution.

| Role | Slice name | Project | Depends on | Status |
| --- | --- | --- | --- | --- |
| Contract | `<slice>` | `<none or workspace>` | `<none>` | `<status>` |
| Backend implementation | `<slice>` | `<backend project>` | `<contract slice>` | `<status>` |
| Mobile implementation | `<slice>` | `<mobile project>` | `<contract slice>` | `<status>` |

## Expected Artifacts And State

For each expected artifact or state transition, record one of `present`,
`absent`, `clean`, `dirty`, `created`, `merged`, `archived`, or `skipped`.

| Item | Status | Notes |
| --- | --- | --- |
| `plan.yaml` (post-draft) | `<status>` | |
| `registry.yaml` | `<status>` | |
| `.specify/workspace/<backend-project>/` | `<status>` | |
| `.specify/workspace/<mobile-project>/` | `<status>` | |
| `contracts/` baseline update | `<status>` | |
| Backend branch `specify/<change-name>` | `<status>` | |
| Mobile branch `specify/<change-name>` | `<status>` | |
| Backend PR (created by first finalize push) | `<status>` | |
| Mobile PR (created by first finalize push) | `<status>` | |
| Archived plan path (`.specify/archive/plans/<change>-<date>.yaml`) | `<status>` | |
| Archived `change.md` next to archived plan | `<status>` | |

## Durable End-State Snapshot

Record the durable end-state outcomes the three-skill change lifecycle
(`/spec:plan → /spec:execute loop → /spec:finalize`) produces.
Each row is one parity check.

| Parity check | Observed | Notes |
| --- | --- | --- |
| Archive path under `.specify/archive/plans/` | `<path>` | Path shape matches the canonical archive layout. |
| Number of merged PRs | `<n>` | Expected: 2 (one per routed project). |
| Backend PR (number, URL, state) | `<#NN https://… MERGED>` | |
| Mobile PR (number, URL, state) | `<#NN https://… MERGED>` | |
| Archived `change.md` content | `<present | absent | divergent>` | Same brief content as the pre-finalize `change.md`. |

## Assertions

For each assertion id from the scenario's **Assertions** list, record
`pass` / `fail` / `skipped`, plus an evidence pointer on failure.

| Assertion id | Verdict | Evidence pointer |
| --- | --- | --- |
| `plan-exists` | `<pass | fail | skipped>` | |
| `plan-validates` | `<pass | fail | skipped>` | |
| `contract-slice-first` | `<pass | fail | skipped>` | |
| `implementation-slices-routed` | `<pass | fail | skipped>` | |
| `dependencies-contract-before-implementations` | `<pass | fail | skipped>` | |
| `draft-stops-at-handoff` | `<pass | fail | skipped>` | |
| `review-step-no-op` | `<pass | fail | skipped>` | |
| `execute-loop-all-done` | `<pass | fail | skipped>` | |
| `workspace-branches-prepared` | `<pass | fail | skipped>` | |
| `finalize-halts-on-unmerged-prs` | `<pass | fail | skipped>` | |
| `finalize-archives-plan` | `<pass | fail | skipped>` | |
| `archived-plan-path-recorded` | `<pass | fail | skipped>` | |
| `archived-change-md-present` | `<pass | fail | skipped>` | |
| `merged-pr-list-recorded` | `<pass | fail | skipped>` | |
| `rerun-finalize-plan-not-found` | `<pass | fail | skipped>` | |

## Negative Expectations

Confirm the forbidden condition did not occur. Record `held`, `violated`, or
`untested`.

| Negative expectation | Verdict | Notes |
| --- | --- | --- |
| `automated-runner-added` | `<held | violated | untested>` | |
| `fake-forge-added` | `<held | violated | untested>` | |
| `transcript-replay-added` | `<held | violated | untested>` | |
| `ci-target-added` | `<held | violated | untested>` | |
| `golden-output-required` | `<held | violated | untested>` | |

## Command Output

Capture the important command output, or point to files that contain it.

- **Plan validation (post-draft):** `<summary or path>`
- **Operator review (`inspect plan.yaml`):** `<summary or path>`
- **Execute loop:** `<summary or path>`
- **First `/spec:finalize` (halt on `pr-not-merged`):** `<summary or path>`
- **External PR merge action:** `<summary or path>`
- **Second `/spec:finalize` (archive):** `<summary or path>`
- **Third `/spec:finalize` (`plan-not-found`):** `<summary or path>`

## Cleanup

Record what cleanup the operator actually performed.

- **Hub workspace:** `<retained | discarded>`
- **Backend workspace:** `<retained | discarded>`
- **Mobile workspace:** `<retained | discarded>`
- **Branches:** `<retained | deleted>`
- **Run evidence:** `<path>`

## Verdict

- **Result:** `<pass | fail>`
- **Fault domain on failure:** `<draft | review | execute | finalize-push | finalize-pr-observation | finalize-archive | operator-error | unknown>`
- **Notes:** `<free-form context>`
