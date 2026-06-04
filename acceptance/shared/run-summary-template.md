# Scenario run summary

> Reusable template for capturing one manual `lifecycle` scenario run. Parameterised for the full range of scenarios — fill the stages the scenario's `stages` frontmatter declares and mark the rest `n/a`.

Copy this into `acceptance/runs/<id>-<date>.md`, fill it against the live run, and update the scenario's status in the [catalog](../lifecycle/README.md). On failure, preserve enough evidence for another operator to reproduce the state.

---

## Run header

- **Scenario id:** `<id>`
- **Scenario file:** `acceptance/lifecycle/<id>.md`
- **Backend:** `manual`
- **Operator / agent:** `<name or model identifier>`
- **Run id:** `<timestamp or uuid>`
- **Started at / finished at:** `<ISO 8601>` / `<ISO 8601>`
- **`specify` build:** `<output of \`command -v specify\` + \`specify --version\`>`
- **Workspace / project roots:** `<temporary roots created for this run>`

## Inputs created

List every file created before invocation.

- `<path>` — `<created | reused | modified>`

## Invocation

Record the exact slash-command and CLI commands run, in order, for each declared stage. Match the scenario's **Invocation**; record any deviation explicitly. Mark stages the scenario does not declare as `n/a`.

### Plan

```text
<paste the /spec:plan ... prompt that was run>
```

### Review (operator pause)

```bash
<paste the specify plan validate / inspect plan.yaml commands>
```

- **Gate 1 stamp:** `<specify plan transition <name> approved | n/a — plan-only scenario>`
- **specify plan amend invocations:** `<none | list>`

### Execute  `<n/a for plan-only scenarios>`

```text
<paste the /spec:execute loop command/prompt and post-execute inspection>
```

### Finalize  `<n/a unless the scenario finalizes>`

```text
<first /spec:finalize — expect halt on pr-not-merged>
<external merge action>
<second /spec:finalize — expect archive>
<third /spec:finalize — expect "no active plan">
```

## Plan structure

| Role | Slice | Project | Depends on | Sources | Status |
| --- | --- | --- | --- | --- | --- |
| `<contract / implementation / local>` | `<slice>` | `<none or project>` | `<none or slices>` | `<source list>` | `<status>` |

## Expected artifacts and state

For each expected artifact/state record `present`, `absent`, `clean`, `dirty`, `created`, `merged`, `archived`, or `skipped`.

| Item | Status | Notes |
| --- | --- | --- |
| `plan.yaml` | `<status>` | |
| `<scenario expected-artifact>` | `<status>` | |

## Assertions

For each assertion id from the scenario's **Assertions**, record `pass` / `fail` / `skipped` / `needs-human`, plus an evidence pointer on failure.

| Assertion id | Verdict | Evidence pointer |
| --- | --- | --- |
| `<assertion>` | `<pass | fail | skipped | needs-human>` | |

## Negative expectations

Confirm the forbidden condition did not occur: `held`, `violated`, or `untested`.

| Negative expectation | Verdict | Notes |
| --- | --- | --- |
| `automated-runner-added` | `<held | violated | untested>` | |
| `fake-forge-added` | `<held | violated | untested>` | |
| `transcript-replay-added` | `<held | violated | untested>` | |
| `ci-target-added` | `<held | violated | untested>` | |
| `golden-output-required` | `<held | violated | untested>` | |

## Command output

Capture the important command output, or point to files that contain it.

- **Plan validation:** `<summary or path>`
- **Execute loop:** `<summary or path, or n/a>`
- **Finalize invocations:** `<summary or path, or n/a>`

## Cleanup

- **Workspaces / projects:** `<retained | discarded>`
- **Branches:** `<retained | deleted | n/a>`
- **Run evidence:** `<path>`

## Verdict

- **Result:** `<pass | fail | deferred>`
- **Fault domain on failure:** `<plan | review | execute | finalize-push | finalize-pr-observation | finalize-archive | synthesis | operator-error | unknown>`
- **Notes:** `<short free-form context>`
