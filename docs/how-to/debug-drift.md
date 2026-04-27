# Debug Drift Detected by Verify

`/spec:verify` compares your code against the baseline specifications at `.specify/specs/` and classifies each requirement. When it reports something other than `COVERED`, here is how to investigate and resolve it.

## Understanding the classifications

| Classification | Meaning | Action |
|---------------|---------|--------|
| **COVERED** | Code implements the requirement as specified | None needed |
| **DRIFTED** | Code behavior diverges from the spec | Fix code or update the spec |
| **MISSING** | Requirement specified but not implemented | Implement the requirement |
| **UNSPECIFIED** | Code behavior exists with no spec | Add a spec or remove the code |

## Step 1: Run verify on a specific capability

Narrow the scope to reduce noise:

```text
/spec:verify <capability-name>
```

This produces a report listing each requirement and its classification.

## Step 2: Investigate DRIFTED requirements

Open the baseline spec at `.specify/specs/<capability>/spec.md` and find the `REQ-XXX` that is flagged. Compare the scenario expectations against the actual code behavior.

Common causes:
- Someone changed the code without going through Specify.
- A dependency update changed behavior.
- The spec was written against an older version of the domain model.

## Step 3: Resolve

**If the code is correct** (the spec is outdated), create a change to update the spec:

```text
/spec:define "Update notification spec to reflect current retry behavior"
```

The generated delta spec will `MODIFY` the drifted requirements. Build and merge to update the baseline.

**If the spec is correct** (the code regressed), fix the code to match the spec. Then re-run verify to confirm coverage.

## Step 4: Handle UNSPECIFIED behavior

`UNSPECIFIED` means the code does something that no spec describes. Decide whether to:

- **Spec it:** Create a change that adds requirements covering the behavior.
- **Remove it:** If the behavior is unintended, remove the code.

## See also

- [/spec:verify](../reference/change-skills/verify.md) -- full reference
- [Iterating on a Baseline](../tutorials/iterating-on-baseline.md) -- updating existing specs with deltas
