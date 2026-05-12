---
name: omnia-test-writer
description: "Generate or update test suites for Omnia Rust WASM crates from Specify artifacts -- MockProvider setup, integration tests, spec-to-test mapping, and drift detection. Use when an Omnia slice has pending crate-test tasks, or when an existing test suite needs to be regenerated after a crate update; not for the crate itself (`crate-writer`) or guest wiring (`guest-writer`)."
argument-hint: "[crate-name]"
---

# Test Writer

> **Generate or update test suites for Omnia Rust WASM crates from Specify artifacts (specs + design.md) and existing crate code.** Tests use `MockProvider` implementations and the `Client` typestate builder to invoke handlers; specs are ground truth, not the generated code.

## Critical Path

1. **Load artifacts and references** — read `spec.md`, `design.md`, `mock-provider.md`, `spec-to-test-mapping.md`, and the closest example before generating tests.
2. **Inventory crate and tests** — inspect handlers, provider trait bounds, input/output types, existing `tests/`, fixtures, and assertion style.
3. **Map specs to tests** — create one deterministic test per scenario, trace each to the stable `REQ-XXX` ID, and derive validation/error/happy-path coverage from specs.
4. **Assert side effects from design** — enumerate every provider interaction in design.md and generate assertions for publishes, writes, cache changes, transactions, and rollback behavior.
5. **Generate MockProvider and fixtures** — implement only required provider traits, load JSON fixtures from `tests/data/`, and preserve existing test style.
6. **Handle drift without deletion** — report missing, extra, and assertion-drift cases; update tests to match changed specs while preserving manual tests unless clearly obsolete.
7. **Leave execution to orchestration** — verify structural checklist here; compilation and test execution happen in the build verify-repair loop.

## Orientation

The skill consumes the same Specify artifacts as `crate-writer` plus the existing crate inventory, then emits one deterministic test function per spec scenario (`test_<crate>_<scenario_snake_case>`). Spec requirements drive happy-path, error, and validation coverage; design.md Business Logic drives side-effect assertions.

The split with sibling skills is strict: `crate-writer` owns code only; `replay-writer` adds regression tests from captured production fixtures; `test-writer` owns all spec-driven test generation. The build orchestration layer runs a unified verify-repair loop after both crate-writer and test-writer complete — this skill never compiles or runs tests itself.

Specs are ground truth. Generate the side-effect assertions implied by the design even when the current crate code does not yet implement them; a failing test is the right signal back to `crate-writer`. Manual tests in the existing suite are flagged as drift, never silently deleted.

See [`references/runbook.md`](references/runbook.md) for the operational detail (arguments, required references list, authority hierarchy, six-step generation process, conventions, directory structure, drift detection, and verification checklist).

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Arguments, required references, authority hierarchy, full Test Generation Process (steps 1–6), conventions, directory structure, drift detection, verification checklist |
| [`references/mock-provider.md`](references/mock-provider.md) | Static and Replay MockProvider patterns for each provider trait |
| [`references/spec-to-test-mapping.md`](references/spec-to-test-mapping.md) | How spec scenarios map to test functions, traceability via `REQ-XXX` IDs |
| [`references/guardrails.md`](references/guardrails.md) | Test-writer-specific forbidden patterns and assertion rules |
| [`references/providers/`](references/providers/) | Per-provider mock-implementation deep-dives |
| [`examples/testing.md`](examples/testing.md) | Core test patterns: layout, MockProvider, test structures, fixtures |
| [`examples/testing-http.md`](examples/testing-http.md) | Simple HTTP handler testing with Config-only MockProvider |
| [`examples/testing-statestore.md`](examples/testing-statestore.md) | Multi-trait MockProvider with StateStore and cache-aside |
| [`examples/testing-publisher.md`](examples/testing-publisher.md) | Publish, event capture, request-reply, topic checks |

## Guardrails

- **NEVER compile or execute tests.** Compilation and `cargo test` belong to the orchestration layer's verify-repair loop; this skill verifies structural checklist items only.
- **NEVER skip side-effect assertions for design-specified provider interactions.** Specs and design.md are ground truth — generate the assertion even if the current handler code does not yet satisfy it; a failing test routes back to `crate-writer`.
- **NEVER delete manual tests during drift handling.** Flag extras and assertion drift; leave operator-authored tests in place unless they are clearly obsolete.
