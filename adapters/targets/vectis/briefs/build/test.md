# Vectis build — tests + core verify-repair

Loaded by [../build.md](../build.md) Step 5 (write tests) and Step 6 (verify-repair the shared core). Both phases run in their own sub-agents with clean context windows.

Carries the cross-cutting Rust verify-repair loop. The spec-to-test mapping rules live in [`test-spec-mapping.md`](../../references/test-spec-mapping.md) and the operational runbook lives in [`test-runbook.md`](../../references/test-runbook.md).

## Step 5 — Crux tests (test-writer body)

Run after [core/write.md](core/write.md) in the same slice. Detect mode from the existing `#[cfg(test)] mod tests` block in `app.rs`:

- No tests yet → **create mode**.
- Tests exist, spec changed → **update mode** (drift detection: diff spec scenarios against existing tests, add tests for new scenarios, update assertions for modified scenarios, flag stale tests for removed scenarios with `// STALE: scenario removed from spec`).
- Verify-repair failure → **repair mode** (sub-agent invoked with `mode: repair` plus failing test output).

### Inline writer steps

1. **Read inputs.** `${SPEC_PATH}`, `${DESIGN_PATH}`, `${APP_RS}`. Use spec-to-test mapping rules: one synchronous `#[test]` per scenario, named after the scenario, with a `/// Spec: <domain> > REQ-XXX > Scenario: <scenario>` traceability comment. Full mapping rules: [`test-spec-mapping.md`](../../references/test-spec-mapping.md).
2. **Map scenarios deterministically.** Each `#### Scenario:` block produces exactly one test function. The `**WHEN**` clause becomes the test setup (model state, dispatched events). The `**THEN**` clause becomes assertions over `Command` effects and `view()` output. Stable `REQ-XXX` ID + scenario title is the drift-detection key.
3. **Write tests inside `#[cfg(test)] mod tests`** in `app.rs` (Crux convention — not a separate `tests/` directory). Preserve existing helpers, factory functions, and test style.
4. **Coverage requirements.** Every scenario; every shell-facing `Event` variant; every page transition (Loading → Main, Error → retry); every validation rule; every adapter's happy and error path; factory helpers for repeated setup.
5. **Crux test API.** Synchronous only — never `#[tokio::test]` or any async runtime. Call `update()` directly; inspect `Command` effects; resolve effects with simulated responses (`expect_one_effect()`, `expect_http()`, `resolve()`); assert on model and view-model state (`expect_one_event()`). Patterns: [`crux/testing-patterns.md`](../../references/crux/testing-patterns.md).
6. **Do not run `cargo test` in create or update mode** — orchestration owns it. In repair mode, run `cargo test` to get fresh errors and verify the fix before returning. Preserve test names, `/// Spec:` traceability comments, and assertion intent — only adjust the syntax used to express them.

## Step 6 — Core verify-repair loop (max 3 iterations)

Spawn in its own sub-agent with `PROJECT_DIR`, the spec path, and (in update mode) a baseline test log captured before the writers ran. The sub-agent returns `status`, `iterations_used`, and any unresolved errors.

Capture the baseline before the writers (update mode only):

```bash
cd "$PROJECT_DIR" && cargo test 2>&1 | tee "/tmp/${SLICE_ID}-${DOMAIN_NAME}-baseline.txt"
```

Each iteration runs all four checks; if any fail, apply the targeted fix and start a new iteration.

```bash
cd "$PROJECT_DIR" && cargo fmt --check         # 1. Formatting (auto-fix with `cargo fmt`).
cd "$PROJECT_DIR" && cargo check               # 2. Compilation.
cd "$PROJECT_DIR" && cargo clippy --all-targets # 3. Lint.
cd "$PROJECT_DIR" && cargo test                # 4. Tests.
```

### Failure classification → repair sub-agent routing

| Failure signal | Classification | Fix action |
|---|---|---|
| Error in `#[cfg(test)] mod tests`, test helpers, or factories | Test issue | Spawn `test-writer` repair sub-agent with the error output. |
| Error in production code (`app.rs` outside `#[cfg(test)]`), missing types or methods | Code issue | Spawn `core-writer` repair sub-agent — see [`core/write.md`](core/write.md). |
| Assertion mismatch where *actual* looks correct per spec | Test issue | Spawn `test-writer` repair sub-agent — the expected value is wrong. |
| Assertion mismatch where *expected* matches spec | Code issue | Spawn `core-writer` repair sub-agent — the handler returns the wrong result. |
| Type mismatch between handler output and assertion | Per spec | Classify per spec, spawn the appropriate repair sub-agent. |
| API surface mismatch: wrong method on `Command`, incorrect `expect_*` chain, stale builder, wrong `resolve()` argument shape | Test issue | Spawn `test-writer` repair sub-agent (the Crux 0.17 API surface is non-trivial; the sub-agent reads the relevant Crux docs / template before fixing). |
| Unresolved import or missing crate in `Cargo.toml` | Workspace issue | Edit `Cargo.toml` directly (no sub-agent needed). |

### Repair discipline

- **Minimum change only.** Fix the reported error and nothing else.
- **Scope the diff.** Before committing a repair, verify the change is limited to files and functions identified in the error output.
- **One failure class per sub-agent.** When multiple failures are present, group them by classification (code vs test) and spawn one repair sub-agent per class.

### Regression check (update mode only)

After tests pass, compare results against the baseline from before the writers ran. For each test that passed before and now fails:

- If the test asserts behaviour the updated spec **explicitly changes** → expected behavioural change, not a regression.
- If the test asserts behaviour the spec does **not** change → true regression. Surface as a failure and route to the appropriate repair sub-agent.

### Loop control

Repeat until all four checks pass or 3 iterations are exhausted. If still failing after 3 iterations: **stop**. Do not mark the task complete. Report the remaining failures with full error output and escalate for guidance (the parent brief reads this as a `build` failure outcome).
