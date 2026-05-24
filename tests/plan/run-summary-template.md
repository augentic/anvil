# Plan Generation Scenario Run Summary

> Reusable template for capturing one manual `/spec:plan` scenario run.

Fill in the fields below for one run. Keep this document next to the run's evidence, or paste it into the operator's notes for a fully manual run. On failure, preserve enough evidence for another operator to reproduce the state.

---

## Run Header

- **Scenario ID:** `<plan-single-project | contract-routing>`
- **Scenario file:** `<relative path, e.g. tests/plan/single-project.md>`
- **Backend:** `manual`
- **Operator / agent:** `<name or model identifier>`
- **Run id:** `<timestamp or uuid>`
- **Started at:** `<ISO 8601 timestamp>`
- **Finished at:** `<ISO 8601 timestamp>`
- **Workspace root:** `<temporary project or hub root>`
- **Peer project roots:** `<none or relative/absolute paths used for registry>`

## Inputs Created

List every file the operator created before invocation.

- `<path>` - `<one-line description>`

## Invocation

Record the exact slash-command and CLI commands actually run, in order. These should match the scenario's **Invocation** section; record any deviation explicitly.

```text
<paste the /spec:plan ... prompt that was run>
```

```bash
<paste validation and inspection commands that were run>
```

## Generated Plan Entries

Record the generated plan entries after `/spec:plan` and validation.

| Role       | Slice name     | Project | Depends on | Sources   | Status              |
| ---------- | -------------- | ------- | ---------- | --------- | ------------------- |
| `<contract | implementation | local   | other>`    | `<slice>` | `<none or project>` | `<none or slice list>` | `<none or source list>` | `<status>` |

## Expected Artifacts And State

For each expected artifact or state, record one of `present`, `absent`, `clean`, `dirty`, `recorded`, or `skipped`.

| Item                                                          | Status     | Notes                         |
| ------------------------------------------------------------- | ---------- | ----------------------------- |
| `plan.yaml`                                                   | `<status>` |                               |
| `.specify/plans/<change-name>/discovery.md`                   | `<status>` |                               |
| `.specify/plans/<change-name>/proposal.md`                    | `<status>` |                               |
| `.specify/plans/<change-name>/workspace.md`                   | `<status>` | `<multi-repo scenarios only>` |
| `.specify/plans/<change-name>/analyze/<source>/metadata.json` | `<status>` | `<source scenarios only>`     |
| `registry.yaml`                                               | `<status>` | `<multi-repo scenarios only>` |

## Assertions

For each assertion id from the scenario's **Assertions** list, record `pass` / `fail` / `skipped`, plus an evidence pointer on failure.

| Assertion id             | Verdict | Evidence pointer |
| ------------------------ | ------- | ---------------- |
| `plan-exists`            | `<pass  | fail             | skipped>` |  |
| `plan-validates`         | `<pass  | fail             | skipped>` |  |
| `<scenario-specific-id>` | `<pass  | fail             | skipped>` |  |

## Negative Expectations

Confirm the forbidden condition did not occur. Record `held`, `violated`, or `untested`.

| Negative expectation      | Verdict | Notes    |
| ------------------------- | ------- | -------- |
| `automated-runner-added`  | `<held  | violated | untested>` |  |
| `fake-forge-added`        | `<held  | violated | untested>` |  |
| `transcript-replay-added` | `<held  | violated | untested>` |  |
| `ci-target-added`         | `<held  | violated | untested>` |  |
| `golden-output-required`  | `<held  | violated | untested>` |  |

## Command Output

Capture the important command output, or point to files that contain it.

- **Plan validation:** `<summary or path>`
- **Plan status:** `<summary or path>`
- **Registry validation:** `<summary or path, multi-repo scenarios only>`
- **Other inspection:** `<summary or path>`

## Cleanup

Record what cleanup the operator actually performed.

- **Workspace:** `<retained | discarded>`
- **Peer projects:** `<retained | discarded | not applicable>`
- **Run evidence:** `<path>`

## Verdict

- **Result:** `<pass | fail>`
- **Fault domain on failure:** `<planning-brief | plan-cli | registry-routing | scenario-input | operator-error | unknown>`
- **Notes:** free-form prose for context the structured fields above cannot capture. Keep this short.
