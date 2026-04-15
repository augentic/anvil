---
id: build
description: Implement the tasks in tasks.md by delegating to the skills below
needs: [specs, design, tasks]
tracks: tasks
---

Arguments (used by all skills):
- CHANGE_ID: the name of this change (from specify status)
- CRATE_NAME: the spec folder name (specs/<crate>/spec.md)
- CRATE_PATH: crates/CRATE_NAME

## Mode detection

Check whether CRATE_PATH/Cargo.toml exists:

- If Cargo.toml does not exist, use create mode.
- If Cargo.toml exists, use update mode.

---

## Create mode (Cargo.toml does NOT exist -- new crate)

### Phase 1: Generate

1. /omnia:guest-writer -- generate the WASM guest project
2. /omnia:crate-writer -- generate the domain crate (code only, no tests)
3. /omnia:test-writer -- generate comprehensive test suites from spec scenarios

Run each skill from start to finish. Complete every step in each skill's
process and ensure its verification checklist is satisfied before moving
to the next skill.

### Phase 2: Verify and repair

Run the verify-repair loop described below.

### Phase 3: Review

4. /omnia:code-reviewer $CRATE_PATH --fix

### Phase 4: Remediate review findings

Run the remediation process described below.

---

## Update mode (Cargo.toml exists -- incremental change)

### Step 0: Capture baseline

Before any changes, record the current test state:

```bash
cd $CRATE_PATH && cargo test 2>&1 | tee /tmp/${CHANGE_ID}-${CRATE_NAME}-baseline.txt
```

Record which tests pass and which fail. This baseline is used in
Phase 2 to detect regressions.

### Phase 1: Generate

1. /omnia:crate-writer -- update the domain crate (code only)
2. /omnia:test-writer -- update tests to match changed specs

Note: guest wiring updates are handled internally by crate-writer
(Step 5 in its process) when the change affects routes, topics, or
WebSocket handlers. Guest-writer is not re-invoked separately in
update mode.

Run each skill from start to finish. Complete every step in each skill's
process and ensure its verification checklist is satisfied before moving
to the next skill.

### Phase 2: Verify and repair

Run the verify-repair loop described below. In update mode, step 3
includes a regression check: compare post-test results against the
baseline captured in Step 0. See "Repair discipline" for how to
distinguish true regressions from expected behavioral changes.

### Phase 3: Review

3. /omnia:code-reviewer $CRATE_PATH --fix

### Phase 4: Remediate review findings

Run the remediation process described below.

---

## Remediation process

Parse `$CRATE_PATH/REVIEW.md`. Process findings by severity:

**CRITICAL and HIGH findings**:

1. If the finding is marked auto-fixable and was not disputed by the
   antagonist: apply the fix directly
2. If the finding is not auto-fixable: classify and route to the
   appropriate skill using the same logic as the verify-repair loop:
   - **Test issue** (finding in `tests/` files, `MockProvider`, or
     `provider.rs`; assertion or fixture problems): re-enter
     /omnia:test-writer with the finding description, file:line
     reference, and suggested fix as context.
   - **Code issue** (finding in `src/` files, handler logic, type
     definitions, production code): re-enter /omnia:crate-writer in
     update mode with the finding description, file:line reference,
     and suggested fix as context.
   Apply minimum-change repair discipline.
3. After all CRITICAL/HIGH fixes: run the verify-repair loop (max 2
   iterations -- tighter than the standard 3 since these are targeted
   repairs to already-reviewed code)

**MEDIUM findings**:

1. Apply auto-fixable fixes
2. For non-auto-fixable: document as accepted technical debt in
   REVIEW.md with rationale for deferral

**LOW findings**: No action required. Document in REVIEW.md.

After remediation, re-run /omnia:code-reviewer $CRATE_PATH (without
--fix) to verify fix quality. If new CRITICAL or HIGH findings are
introduced by the fixes, repeat the remediation cycle once.

---

## Verify-repair loop (max 3 iterations)

After both crate-writer and test-writer have completed, run this loop
to converge on a clean build. Each iteration runs all three checks; if
any fail, apply the targeted fix and start a new iteration.

### 1. Formatting

```bash
cd $CRATE_PATH && cargo fmt --check
```

If fails: run `cargo fmt` to fix. Formatting is mechanical; one pass
suffices.

### 2. Compilation and lint

```bash
cd $CRATE_PATH && cargo check
cd $CRATE_PATH && cargo clippy -- -D warnings
```

If fails: fix each error or warning. Reference
[repair-patterns.md](../../../plugins/omnia/skills/crate-writer/references/repair-patterns.md)
for canonical Omnia SDK patterns (Handler structure, error handling,
serde conventions, clippy fixes).

### 3. Test suite

```bash
cd $CRATE_PATH && cargo test
```

If failures are detected, classify each failure and route the fix to
the appropriate skill:

| Failure signal | Classification | Fix action |
| --- | --- | --- |
| Error in `tests/` file paths, `MockProvider`, or `provider.rs` | **Test issue** | Re-enter test-writer with the error output |
| Error in `src/` file paths, missing trait impls in production code | **Code issue** | Re-enter crate-writer with the error output |
| Assertion mismatch where the *actual* value looks correct per spec | **Test issue** | Re-enter test-writer -- the expected value is wrong |
| Assertion mismatch where the *expected* value matches spec | **Code issue** | Re-enter crate-writer -- the handler returns the wrong result |
| MockProvider missing a trait impl the handler now requires | **Test issue** | Re-enter test-writer to update MockProvider |
| Type mismatch between handler output and test assertion | **Code issue** if handler type is wrong per spec; **test issue** if assertion type is stale | Classify per spec, fix accordingly |
| Unresolved import or missing crate in `Cargo.toml` dependencies | **Workspace issue** | Fix `Cargo.toml` dependency paths or workspace member list directly -- this is a structural issue outside both skills |

When re-entering a skill for repair, pass the full error output as
context so the skill can make a targeted fix. Reference
[mock-provider.md](../../../plugins/omnia/skills/test-writer/references/mock-provider.md)
for test-side repair strategies.

### Repair discipline

When re-entering a skill for repair:

- **Minimum change only** -- fix the reported error and nothing else.
  Do not refactor, restructure, or "improve" adjacent code. A repair
  that touches more than the failing code path is likely to introduce
  new failures elsewhere, causing the loop to oscillate.
- **Scope the diff** -- before committing a repair, verify the change
  is limited to files and functions identified in the error output.
  If the fix requires changes outside that scope, classify it as a
  new failure and route it through the classification table separately.
- **One failure class per re-entry** -- if multiple failures are
  present, group them by classification (code issue vs test issue) and
  re-enter each skill once with all same-class errors. Do not
  interleave code and test fixes in a single re-entry.

**Update mode only -- regression check**: compare post-test results
against the baseline from Step 0. For each test that passed before
and now fails:

- If the test asserts behavior that the **updated spec explicitly
  changes**, the failure is an **expected behavioral change**, not a
  regression. The test expectation should have been updated by
  test-writer in Phase 1. If it was not, re-enter test-writer to
  align the test with the new spec.
- If the test asserts behavior that the spec does **not** change, the
  failure is a **true regression**. Route the fix through the
  classification table above.

The spec is the arbiter: a previously-passing test is only a
regression if the behavior it validates is still specified.

### Loop control

Repeat from step 1 until all three checks pass or 3 iterations are
exhausted. If still failing after 3 iterations: **STOP**. Do not mark
the task complete. Report the remaining failures with full error output
and escalate for guidance.
