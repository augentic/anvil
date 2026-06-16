# Scenario Run Summary

> Reusable template for capturing one manual run of a contracts scenario.

Fill in the fields below for one run. Keep this document next to the run's
evidence (or paste it into the operator's notes for a fully manual run). On
failure, preserve the evidence directory or notes that explain the result.

---

## Run Header

- **Scenario ID:** `<contracts-describe | contracts-design | contracts-update-boundary | contracts-import | contracts-source>`
- **Scenario file:** `<relative path, e.g. adapters/targets/contracts/tests/describe.md>`
- **Adapter:** `contracts@1.0.0`
- **Backend:** `<manual | agent | recorded | fixture>`
- **Operator / agent:** `<name or model identifier>`
- **Run id:** `<timestamp or uuid>`
- **Started at:** `<ISO 8601 timestamp>`
- **Finished at:** `<ISO 8601 timestamp>`
- **Workspace:** `<temp project root used for the run>`

## Inputs Created

List every file the operator created before invocation, with a one-line
description. These come from the scenario's **Inputs** section.

- `<path>` — `<one-line description>`

(For `describe.md` this list is typically empty: the prose is in the prompt.)

## Invocation

Record the exact slash-command(s) actually run, in order. These should match
the scenario's **Invocation** block verbatim; record any deviation explicitly.

```text
<paste the /spec:refine ... prompt that was run>
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

Record what cleanup the operator actually performed, per the scenario's
**Cleanup** section.

- **Slice action:** `<dropped | archived | preserved>`
- **Baseline action:** `<unchanged | promoted via /spec:merge | reverted>`
- **Workspace action:** `<retained | discarded>`

## Verdict

- **Result:** `pass | fail`
- **Fault domain (on failure):** one of `cli-substrate`,
  `skill-orchestration`, `adapter-brief`, `specialist-generation`, or
  `unknown`.
- **Notes:** free-form prose for context the structured fields above can't
  capture. Keep this short.
