---
name: vectis-ios-reviewer
description: Review generated iOS shell (SwiftUI) code for structural issues, integration correctness, and quality problems. Use when `ios-writer` has just produced or updated an iOS shell and the slice is ready for review; not for reviewing the core (`core-reviewer`) or Android shell (`android-reviewer`).
argument-hint: <target-dir>
---

# Crux iOS Shell Reviewer

> **The reviewer drives an agent team — three specialists and one antagonist — through a bounded review-fix loop, then either returns classified `design_findings` to the orchestrator or scaffolds a `review-…` Specify change via `/spec:define`. The lead never edits beyond mechanical auto-fixes and reverts the entire batch if verification regresses.**

## Critical Path

1. **Gather review context** — read Crux core files, all iOS Swift/build files, optional reference app files, and available composition/tokens/assets inputs.
2. **Spawn the review team** — run Structural and Quality every iteration; add Integration on the first full-scope iteration only.
3. **Apply lead checks** — run Swift-specific universal codex checks, attach `rule_id` on mapped findings, and tag design-level/spec-change indicators for later consolidation.
4. **Challenge findings** — send all specialist and universal findings to the antagonist for evidence review, severity adjustment, and counter-scan.
5. **Synthesize and auto-fix** — merge findings into one report, classify mechanical vs design-level, apply safe mechanical fixes, and revert all fixes if they regress verification.
6. **Loop deliberately** — repeat changed-file review until no mechanical fixes remain or the three-iteration cap is reached.
7. **Express design-level findings** — return classified findings when orchestrated; otherwise delegate one `/spec:define` slice that captures code-fix and spec-change work.

## Orientation

The iOS reviewer systematically inspects a generated SwiftUI shell for structural issues, integration correctness, and Swift-level quality problems. It catches what the Swift compiler and `swiftformat` miss: missing ViewModel/screen correspondence, incomplete effect handlers, hardcoded design tokens, missing accessibility labels, and concurrency violations.

The skill drives an **agent team** — three specialist reviewers (Structural, Quality, Integration) plus an antagonist — through a bounded **review-fix loop** (max 3 iterations). The lead runs universal codex checks (UNI-001..021) with Swift heuristics, synthesises every report into one severity-graded output, applies only **mechanical** auto-fixes (a11y labels, design-token swaps, missing `#Preview`, Inject boilerplate), and reverts the entire batch if `swiftformat` or the build regresses.

When the orchestrator passes `orchestrated: true` (Vectis build phase), the reviewer returns classified `design_findings` for cross-platform consolidation. When invoked standalone (default), it delegates accumulated design-level findings to `/spec:define` as a `review-{app}-ios-{date}` slice. Cross-artifact checks against `composition.yaml` / `tokens.yaml` / `assets.yaml` degrade gracefully when those inputs are absent.

This skill consumes the writer's output, not its inputs — `ios-writer` owns generation, `ios-reviewer` owns the audit. Codex rule prose is read from the resolved project codex (`capabilities/default/codex/`, `capabilities/vectis/codex/`); the reviewer never copies that prose into reports.

See [`references/runbook.md`](references/runbook.md) for arguments, per-step spawn prompts, the full UNI-001..021 Swift heuristic list, the synthesis / auto-fix / loop-control rules, severity definitions, the verification checklist, and the Specify-workflow integration diagram.

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Arguments, spawn prompts, universal-check heuristics, synthesis/auto-fix/loop rules, severity table, verification checklist, workflow integration |
| [`references/ios-review-checks.md`](references/ios-review-checks.md) | IOS-001..019 structural checks (ViewModel/screen correspondence, effect handlers, token usage, ScrollView hazards, recurring-group candidates) |
| [`references/swift-quality-checks.md`](references/swift-quality-checks.md) | SWF-001..010 Swift/SwiftUI quality checks (concurrency, force unwraps, a11y, state management, previews, swiftformat) |
| [`references/agent-teams.md`](references/agent-teams.md) | Shared team protocol: roles, antagonist protocol, synthesis rules, file ownership, confidence scoring |
| [`references/iteration-report.md`](references/iteration-report.md) | Iteration report template used for every review-fix cycle |
| [`team-protocol.md`](team-protocol.md) | Verbatim antagonist spawn prompt + SwiftUI-specific blind-spot list |

## Guardrails

- **Mechanical auto-fixes only.** Never auto-fix structural or design-level findings (missing screens, missing effect handlers, IOS-019 component promotion). Revert the full batch if `swiftformat` or the build regresses.
