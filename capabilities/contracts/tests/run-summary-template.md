# Scenario Run Summary

> Reusable template for capturing one acceptance run of a contracts scenario.
> Mirrors the future runner's `summary.md` shape from
> [`acceptance/runner/README.md` §Run Directories And Evidence](../../../acceptance/runner/README.md#run-directories-and-evidence)
> so a human-driven run today produces output the runner can replicate
> mechanically tomorrow.

Fill in the fields below for one run. Keep this document next to the run's
evidence (or paste it into the operator's notes for a fully manual run). On
failure, preserve the run-evidence directory per the
[Run Evidence Policy](../../../acceptance/README.md#run-evidence-policy).

---

## Run Header

- **Scenario ID:** `<contracts-describe | contracts-design | contracts-update-boundary | contracts-import | contracts-source>`
- **Scenario file:** `<relative path, e.g. capabilities/contracts/tests/describe.md>`
- **Capability:** `contracts@v1`
- **Backend:** `<manual | stub | agent | recorded>`
- **Operator / agent:** `<name or model identifier>`
- **Run id:** `<timestamp or uuid the runner would use>`
- **Started at:** `<ISO 8601 timestamp>`
- **Finished at:** `<ISO 8601 timestamp>`
- **Workspace root:** `<temp project root used for the run>`

## Inputs Created

List every file the operator/runner created before invocation, with a one-line
description. These come from the scenario's **Inputs** section.

- `<path>` — `<one-line description>`

(For `describe.md` this list is typically empty: the prose is in the prompt.)

## Invocation

Record the exact slash-command(s) actually run, in order. These should match
the scenario's **Invocation** block verbatim; record any deviation explicitly.

```text
<paste the /spec:define ... prompt that was run>
```

```text
<paste the /spec:build <slice-name> command that was run>
```

```text
<paste the /spec:merge <slice-name> command if run>
```

## Expected Artifacts

For each path in the scenario's **Expected Artifacts** list, record one of
`present` (created in the slice working tree), `present-after-merge` (only in
the baseline `contracts/` after merge), `absent` (expected but missing), or
`not-expected` (boundary scenario, the path must not appear).

| Path                                       | Status                                    | Notes |
| ------------------------------------------ | ----------------------------------------- | ----- |
| `contracts/...`                            | `present | present-after-merge | absent | not-expected` | |

## Assertions

For each assertion id from the scenario's **Assertions** list, record
`pass` / `fail` / `skipped`, plus an evidence pointer on failure (a missing
file, a verifier finding line, a JSON field whose value did not match).

| Assertion id                  | Verdict                  | Evidence pointer                              |
| ----------------------------- | ------------------------ | --------------------------------------------- |
| `files-exist`                 | `pass | fail | skipped`  | `<path or stdout line on fail>`               |
| `contract-validator-clean`    | `pass | fail | skipped`  | `<verifier finding or stdout line on fail>`   |
| `<scenario-specific-id>`      | `pass | fail | skipped`  | `<evidence>`                                  |

## Negative Expectations

For each item in the scenario's **Negative Expectations** list, confirm the
forbidden condition did not occur. Record `held` (the boundary held),
`violated` (the forbidden condition occurred — this is a failure), or
`untested`.

| Negative expectation                                | Verdict                      | Notes |
| --------------------------------------------------- | ---------------------------- | ----- |
| `<negative-expectation-id from scenario>`           | `held | violated | untested` |       |

## Verifier Output

Capture the relevant verifier output (the `contract` WASI tool result, or the
build phase's verifier summary):

- **Exit code:** `<0 clean | 1 findings | 2 tool/invocation error>`
- **Findings:** `<count>`; list any unresolved `$ref` failures, missing schema
  metadata, binding coverage failures, or manual-review warnings.
- **Manual-review warnings:** `<count and one-liner per warning>`.

## Cleanup

Record what cleanup the operator/runner actually performed, per the scenario's
**Cleanup** section.

- **Slice action:** `<dropped | archived | preserved>`
- **Baseline action:** `<unchanged | promoted via /spec:merge | reverted>`
- **Workspace action:** `<retained | discarded>`

## Verdict

- **Result:** `pass | fail`
- **Fault domain (on failure):** one of `cli-substrate`,
  `skill-orchestration`, `capability-brief`, `specialist-generation`,
  `runner-setup`, `external-fake-boundary`, `live-agent-nondeterminism`, or
  `unknown`. Taxonomy from
  [`acceptance/runner/README.md` §Failure Reporting](../../../acceptance/runner/README.md#failure-reporting).
- **Notes:** free-form prose for context the structured fields above can't
  capture. Keep this short.
