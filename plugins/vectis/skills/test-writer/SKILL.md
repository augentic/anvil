---
name: vectis-test-writer
description: "Generate or update test suites for Crux shared crates from Specify artifacts -- spec-to-test mapping, traceability, drift detection, and synchronous Crux testing patterns. Use when a Vectis slice has pending Crux test tasks, or when an existing test suite needs to be regenerated after a core update; not for the core itself (`core-writer`) or platform-shell tests."
argument-hint: "[feature-name]"
---

# Crux Test Writer

## Arguments

```text
$FEATURE_NAME   = $ARGUMENTS[0]
$PROJECT_DIR    = $(pwd)
$SLICE_DIR      = .specify/slices/<active-change>
$SPECS_DIR      = $SLICE_DIR/specs
$SPEC_PATH      = $SPECS_DIR/$FEATURE_NAME/spec.md
$DESIGN_PATH    = $SLICE_DIR/design.md
$APP_RS         = $PROJECT_DIR/shared/src/app.rs
```

`$FEATURE_NAME` is optional; the orchestrator passes it explicitly when invoking the writer for a specific feature. `<active-change>` is the in-flight slice name; the runbook explains how to discover it.

## Critical Path

1. **Resolve mode and inputs** — derive `$FEATURE_NAME`, `$SPEC_PATH`, `$DESIGN_PATH`, `$APP_RS`, then choose create, update, or repair mode from traceability comments or caller input.
2. **Read references and artifacts** — load spec-to-test mapping, Crux testing patterns, the feature spec, design.md, and `app.rs` before writing tests.
3. **Map scenarios deterministically** — generate one synchronous test per scenario with stable `REQ-XXX` traceability, effect-chain assertions, validation coverage, and page-transition checks.
4. **Create or update the test module** — write tests inside `#[cfg(test)] mod tests`, preserve existing helpers/style, add new scenarios, update changed assertions, and mark stale tests instead of deleting them.
5. **Enforce coverage structure** — require scenario coverage, shell-facing Event coverage, page transitions, validation rules, capability happy/error paths, and factory helpers for repeated setup.
6. **Repair with minimum edits** — in repair mode, run `cargo test`, read fresh errors and Crux API references, fix only test syntax/setup needed to preserve intent, then verify once.
7. **Report verification ownership** — do not run `cargo test` in create/update mode; return structural checklist status for the outer verify-repair loop.

## Orientation

The test writer generates or updates tests for a Crux shared crate from Specify artifacts (specs + `design.md`) and existing crate code. Tests use Crux's synchronous testing model: call `update()` directly, inspect `Command` effects, resolve effects with simulated responses, and assert on model and view-model state.

The mapping from spec scenarios to tests is deterministic — the same spec always produces the same test structure. Each BDD scenario maps to exactly one test function, traced by the stable `REQ-XXX` ID plus the scenario title. This enables drift detection: regenerated test structure from a baseline spec can be diffed against committed tests to surface missing, stale, or assertion-drift cases.

**Relationship to other skills.** `core-writer` generates production code only (no tests); test-writer owns all test generation, spec-to-test traceability, scenario coverage, and test updates. The build orchestration layer runs a verify-repair loop after both writers complete. In **create** and **update** modes, test-writer generates tests but does **not** run them; compilation and test execution happen at the orchestration level. In **repair** mode (invoked as a sub-agent with `mode: repair` plus the failing error output), test-writer runs `cargo test` itself to get fresh errors and verify fixes, applying the minimum change to preserve test intent — names, `/// Spec:` traceability, and what each assertion checks are not changed; only the syntax used to express them. `core-reviewer` checks spec-to-test coverage (LOG-008) and stale tests (LOG-009) during review using the traceability comments produced here.

See [`references/runbook.md`](references/runbook.md) for arguments, required references, authority hierarchy, mode detection, the Create / Update / Repair step bodies, the spec-to-test mapping rules, drift detection workflow, test conventions, troubleshooting, and the full verification checklist.

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Arguments, authority hierarchy, mode detection (Create/Update/Repair), step bodies, drift detection, test conventions, troubleshooting, verification checklist |
| [`references/spec-to-test-mapping.md`](references/spec-to-test-mapping.md) | How spec scenarios map to test functions, traceability format, WHEN-to-setup and THEN-to-assertion translation, drift detection rules |
| [`references/crux-testing-patterns.md`](references/crux-testing-patterns.md) | Crux test API: `update()`, `Command`, effect assertions (`expect_one_effect()`, `expect_http()`), `resolve()`, `expect_one_event()` |
| [`references/crux-command-api.md`](references/crux-command-api.md) | Command creation, chaining, combining, async context — canonical surface for repair-mode API fixes |

## Guardrails

- **NEVER write tests outside `#[cfg(test)] mod tests` in `app.rs`** (Crux convention — not a separate `tests/` directory) and **NEVER use `#[tokio::test]` or any async runtime** — Crux's testing model is fully synchronous.
- **NEVER silently delete tests** for removed spec scenarios; flag them with `// STALE: scenario removed from spec`. In repair mode, **NEVER change test names, `/// Spec:` traceability comments, or assertion intent** — only the syntax used to express them.
- **NEVER run `cargo test` in create or update mode** (orchestration owns it); **ALWAYS** run `cargo test` in repair mode against fresh errors, and **ALWAYS** emit one synchronous `#[test]` per scenario with a `/// Spec: ... > REQ-XXX > Scenario: ...` traceability comment.
