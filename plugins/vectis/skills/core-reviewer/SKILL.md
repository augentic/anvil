---
name: vectis-core-reviewer
description: Review generated Crux core (Rust shared crate) code for structural issues, logic bugs, and quality problems. Use when `core-writer` has just produced or updated a Crux core crate and the slice is ready for review; not for platform-shell reviews (`ios-reviewer` / `android-reviewer`).
argument-hint: <target-dir>
---

# Crux Core Reviewer

> **The reviewer drives an agent team — three specialists and one antagonist — through a bounded review-fix loop, then delegates accumulated design-level findings to `/spec:define` as a `review-…` slice. The lead never edits beyond mechanical auto-fixes and reverts the entire batch if `cargo check` / `clippy` / `test` regress.**

## Critical Path

1. Gather context — read `spec.md`, `shared/Cargo.toml`, every `.rs` file under `shared/src/`; if `reference-dir` is provided read its counterparts too.
2. Spawn team — Structural + Quality (always); Logic only on the first iteration when `scope = full`. Each specialist applies its own check set (CRX-, LOG-, GEN-).
3. Lead runs universal codex checks (UNI-001..021) with Rust-specific heuristics, attaches `rule_id` on mapped findings, and runs an optional comparative pass when a reference app is supplied.
4. Antagonist (see [`team-protocol.md`](team-protocol.md)) challenges every finding with evidence and counter-scans for Crux blind spots; lead synthesises into a single iteration report and assigns a confidence level.
5. Auto-fix mechanical issues (missing serde derives, `render().and(...)` wraps, `.trim()`/empty input checks, unused deps); re-run `cargo check` / `clippy` / `test` and revert all auto-fixes on regression.
6. Loop control — re-spawn Structural + Quality on changed files until `iteration == 3` or no mechanical fixes were applied.
7. Express accumulated design-level findings — classify each as `code-fix` or `spec-change` and delegate to `/spec:define` to scaffold a `review-…` change.

## Orientation

The core reviewer systematically inspects a generated Crux core (Rust `shared` crate) for structural issues, logic bugs, and Rust-level quality problems. It catches semantic issues that compilers, linters, and clippy miss: missing `render()` calls, conflict-resolution gaps, pending-op coalescing bugs, state-machine incompleteness, and interaction-sequence race conditions.

The skill drives an **agent team** — three specialist reviewers (Structural, Logic, Quality) plus an antagonist — through a bounded **review-fix loop** (max 3 iterations). The lead runs universal codex checks (UNI-001..021) with Rust/Crux heuristics (via [`references/universal-checks.md`](references/universal-checks.md)), an optional `CMP-` comparative pass when `reference-dir` is supplied, synthesises every report into one severity-graded output, applies only **mechanical** auto-fixes (serde derives, `render().and(...)` wraps, `.trim()` checks, unused deps), and reverts the entire batch if `cargo check` / `cargo test` / `cargo clippy` regress.

Unlike the platform reviewers, the core reviewer has no `orchestrated` mode — when design-level findings accumulate it always delegates to `/spec:define` to scaffold a `review-{app}-{date}` slice with proposal, design, specs, and tasks ready for `/spec:build`. The Logic Specialist requires `spec.md` (the Vectis artifact path, `.specify/slices/<change>/specs/<feature>/spec.md`, not the project root) so it can run LOG-008 spec-to-test coverage and LOG-009 stale-test checks.

This skill consumes the writer's output, not its inputs — `core-writer` owns generation, `core-reviewer` owns the audit. Codex rule prose is read from the resolved project codex (`adapters/default/codex/`, `adapters/vectis/codex/`); the reviewer never copies that prose into reports.

See [`references/runbook.md`](references/runbook.md) for arguments, per-step spawn prompts (including the full LOG-001..009 logic specialist prompt), the comparative-review heuristics, synthesis / auto-fix / loop-control rules, severity definitions, the verification checklist, and the Specify-workflow integration diagram.

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Arguments, spawn prompts, comparative-pass heuristics, synthesis/auto-fix/loop rules, severity table, verification checklist, workflow integration |
| [`references/crux-review-checks.md`](references/crux-review-checks.md) | CRX-001..011 structural checks (missing `render()`, serde derives, input validation, `PendingOp` timestamps, ViewModel typing, unused deps) |
| [`references/logic-review-checks.md`](references/logic-review-checks.md) | LOG-001..009 logic checks (state-machine completeness, op coalescing, concurrent conflicts, temporal ordering, rapid-action sequences, spec gaps, spec-to-test coverage, stale tests) |
| [`references/general-review-checks.md`](references/general-review-checks.md) | GEN-001..012 Rust quality checks (no `unwrap`/`expect`, no debug output, no hardcoded secrets, error propagation, match exhaustiveness, function length) |
| [`references/universal-checks.md`](references/universal-checks.md) | UNI-001..021 application with Rust/Crux heuristics + skip table for rules already covered by CRX/LOG/GEN |
| [`references/agent-teams.md`](references/agent-teams.md) | Shared team protocol: roles, antagonist protocol, synthesis rules, file ownership, confidence scoring |
| [`references/iteration-report.md`](references/iteration-report.md) | Iteration report template used for every review-fix cycle |
| [`team-protocol.md`](team-protocol.md) | Verbatim antagonist spawn prompt + Crux-specific blind-spot list |

## Guardrails

- **Mechanical auto-fixes only.** Never auto-fix logic bugs (LOG-001..008) or other design-level findings without explicit confirmation. Revert the full batch if `cargo check` / `cargo test` / `cargo clippy` regress.
