---
name: vectis-android-reviewer
description: Review generated Android shell (Kotlin/Jetpack Compose) code for structural issues, integration correctness, and quality problems. Use when `android-writer` has just produced or updated an Android shell and the slice is ready for review; not for the core (`core-reviewer`) or iOS shell (`ios-reviewer`).
argument-hint: <target-dir>
---

# Crux Android Shell Reviewer

> **The reviewer drives an agent team — three specialists and one antagonist — through a bounded review-fix loop, then either returns classified `design_findings` to the orchestrator or scaffolds a `review-…` Specify change via `/spec:define`. The lead never edits beyond mechanical auto-fixes and reverts the entire batch if verification regresses.**

## Critical Path

1. Gather context — read `shared/src/app.rs`, every `.kt` file under `Android/app/src/main/java/`, Gradle/manifest config, and the wired UI input set (`composition.yaml`, `tokens.yaml`, `assets.yaml` — change-local then baseline / project paths); if `reference-dir` is provided, read its counterparts too.
2. Spawn team — Structural + Quality (always); Integration only on the first iteration when `scope = full`. Each specialist applies its own check set (AND-, KTL-, INT-).
3. Lead applies universal codex checks (UNI-001..021) with Android/Compose heuristics, attaches `rule_id` on mapped findings, and skips checks already covered by the specialists.
4. Antagonist (see [`team-protocol.md`](team-protocol.md)) challenges every finding with evidence and counter-scans for Android blind spots; lead synthesises into a single iteration report and assigns a confidence level.
5. Auto-fix mechanical issues (a11y `contentDescription`, design-token swaps, missing `@Preview`, generated-FFI-type imports `import com.example.app.*`, `CancellationException` rethrow, replacing stale `import com.vectis.design.*` with `import com.vectis.<appname>.ui.theme.*`); revert all auto-fixes if the build breaks.
6. Loop control — re-spawn Structural + Quality on the changed files until `iteration == 3` or no mechanical fixes were applied.
7. Express accumulated design-level findings — when `orchestrated: true` return classified `design_findings`; otherwise delegate to `/spec:define` to scaffold a `review-…` change.

## Orientation

The Android reviewer systematically inspects a generated Kotlin/Jetpack Compose shell for structural issues, integration correctness, and Kotlin-level quality problems. It catches what the Kotlin compiler and linter miss: missing screen-composable / root-branch correspondence, incomplete effect handlers, hardcoded design tokens, missing accessibility descriptions, coroutine safety violations, missing UniFFI library override, and incorrect generated-type import patterns.

The skill drives an **agent team** — three specialist reviewers (Structural, Quality, Integration) plus an antagonist — through a bounded **review-fix loop** (max 3 iterations). The lead runs universal codex checks (UNI-001..021) with Kotlin/Android heuristics (via [`references/universal-checks.md`](references/universal-checks.md)), synthesises every report into one severity-graded output, applies only **mechanical** auto-fixes, and reverts the entire batch if the Gradle build regresses.

When the orchestrator passes `orchestrated: true` (Vectis build phase), the reviewer returns classified `design_findings` for cross-platform consolidation. When invoked standalone (default), it delegates accumulated design-level findings to `/spec:define` as a `review-{app}-android-{date}` slice. Cross-artifact checks against `composition.yaml` / `tokens.yaml` / `assets.yaml` degrade gracefully when those inputs are absent — but the no-hardcoded-literal portions of AND-005..007 stay enforced even when `tokens.yaml` is missing.

This skill consumes the writer's output, not its inputs — `android-writer` owns generation, `android-reviewer` owns the audit. Codex rule prose is read from the resolved project codex (`adapters/default/codex/`, `adapters/vectis/codex/`); the reviewer never copies that prose into reports.

See [`references/runbook.md`](references/runbook.md) for arguments, per-step spawn prompts, the universal-checks skip table, synthesis / auto-fix / loop-control rules, severity definitions, the verification checklist, and the Specify-workflow integration diagram.

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Arguments, spawn prompts, synthesis/auto-fix/loop rules, severity table, verification checklist, workflow integration |
| [`references/android-review-checks.md`](references/android-review-checks.md) | AND-001..027 structural checks (screen/ViewModel correspondence, effect handlers, token usage, UniFFI library override, generated-type imports, coroutine safety, recurring-group candidates) |
| [`references/kotlin-quality-checks.md`](references/kotlin-quality-checks.md) | KTL-001..010 Kotlin/Jetpack Compose quality checks (force-unwraps, debug output, coroutine cancellation, Compose state, previews, a11y) |
| [`references/universal-checks.md`](references/universal-checks.md) | UNI-001..021 application with Kotlin/Android heuristics + skip table for rules already covered by AND/KTL |
| [`references/agent-teams.md`](references/agent-teams.md) | Shared team protocol: roles, antagonist protocol, synthesis rules, file ownership, confidence scoring |
| [`references/iteration-report.md`](references/iteration-report.md) | Iteration report template used for every review-fix cycle |
| [`team-protocol.md`](team-protocol.md) | Verbatim antagonist spawn prompt + Android/Compose-specific blind-spot list |

## Guardrails

- **Mechanical auto-fixes only.** Never auto-fix structural or design-level findings (missing composables, missing effect handlers, AND-027 component promotion). Revert the full batch if the Gradle build regresses.
