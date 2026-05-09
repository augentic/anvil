# Cross-Repo Scenario Run Summary

> Reusable template for capturing one manual cross-repo scenario run.

Fill in the fields below for one run. Keep this document next to the run's
evidence, or paste it into the operator's notes for a fully manual run. On
failure, preserve enough evidence for another operator to reproduce the state.

---

## Run Header

- **Scenario ID:** `cross-repo-contract-flow`
- **Scenario file:** `tests/cross-repo/scenario.md`
- **Backend:** `manual`
- **Operator / agent:** `<name or model identifier>`
- **Run id:** `<timestamp or uuid>`
- **Started at:** `<ISO 8601 timestamp>`
- **Finished at:** `<ISO 8601 timestamp>`
- **Hub root:** `<temporary hub project root>`
- **Backend project root:** `<temporary Omnia project root>`
- **Mobile project root:** `<temporary Vectis project root>`

## Inputs Created

List every file the operator created before invocation.

- `docs/oauth-login.md` - `<created | reused | modified>`
- `<other path>` - `<one-line description>`

## Invocation

Record the exact slash-command and CLI commands actually run, in order. These
should match the scenario's **Invocation** section; record any deviation
explicitly.

```text
<paste the /change:plan ... prompt that was run>
```

```text
<paste the /change:execute loop command or prompt that was run>
```

```bash
<paste workspace push, merge, and finalize commands that were run>
```

## Plan Structure

Record the final planned slices before execution.

| Role | Slice name | Project | Depends on | Status |
| --- | --- | --- | --- | --- |
| Contract | `<slice>` | `<none or hub>` | `<none>` | `<status>` |
| Backend implementation | `<slice>` | `<backend project>` | `<contract slice>` | `<status>` |
| Mobile implementation | `<slice>` | `<mobile project>` | `<contract slice>` | `<status>` |

## Expected Artifacts And State

For each expected artifact or state transition, record one of `present`,
`absent`, `clean`, `dirty`, `created`, `merged`, `archived`, or `skipped`.

| Item | Status | Notes |
| --- | --- | --- |
| `plan.yaml` | `<status>` | |
| `registry.yaml` | `<status>` | |
| `.specify/workspace/<backend-project>/` | `<status>` | |
| `.specify/workspace/<mobile-project>/` | `<status>` | |
| `contracts/` baseline update | `<status>` | |
| Backend branch `specify/<change-name>` | `<status>` | |
| Mobile branch `specify/<change-name>` | `<status>` | |
| Workspace push PRs or merge requests | `<status>` | |
| Archived plan | `<status>` | |

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
| `execute-loop-all-done` | `<pass | fail | skipped>` | |
| `workspace-branches-prepared` | `<pass | fail | skipped>` | |
| `push-created-prs` | `<pass | fail | skipped>` | |
| `finalize-archives-plan` | `<pass | fail | skipped>` | |
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

- **Plan validation:** `<summary or path>`
- **Execute loop:** `<summary or path>`
- **Workspace push:** `<summary or path>`
- **Finalize:** `<summary or path>`
- **Second finalize:** `<summary or path>`

## Cleanup

Record what cleanup the operator actually performed.

- **Hub workspace:** `<retained | discarded>`
- **Backend workspace:** `<retained | discarded>`
- **Mobile workspace:** `<retained | discarded>`
- **Branches:** `<retained | deleted>`
- **Run evidence:** `<path>`

## Verdict

- **Result:** `<pass | fail>`
- **Fault domain on failure:** `<planning | registry-routing | execution-loop | capability-output | workspace-push | finalize | operator-error | unknown>`
- **Notes:** `<free-form context>`
