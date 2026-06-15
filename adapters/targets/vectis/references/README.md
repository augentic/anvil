# Vectis reference material

Reference documentation for the Vectis target adapter at [`adapters/targets/vectis/`](..). In Specify, Vectis is a **target adapter** — `shape`, `build`, `merge` — not a slash-command plugin.

The Vectis core / test / iOS / Android writer and reviewer orchestration lives in [`../briefs/build.md`](../briefs/build.md) and eight phase sub-briefs under [`../briefs/build/`](../briefs/build/). The depth (Crux idioms, SwiftUI patterns, Compose patterns, hard rules, review check libraries, token templates, design-system integration) and worked examples live in this folder.

## Briefs

| Brief | Purpose |
|-------|---------|
| [`shape.md`](../briefs/shape.md) | Idiom guidance for core synthesis. |
| [`build.md`](../briefs/build.md) | Orchestrator: phase order, sub-agent contract, verify-serial / review-parallel rule, consolidation, template-drift signal, phase outcome. |
| [`build/composition.md`](../briefs/build/composition.md) | Regenerate `composition.yaml` from `spec.md` + `design.md`; run the deterministic validator gate. |
| [`build/core/write.md`](../briefs/build/core/write.md) | Generate / update the Crux shared core. |
| [`build/core/review.md`](../briefs/build/core/review.md) | Agent-team review of the Rust `shared` crate. |
| [`build/test.md`](../briefs/build/test.md) | Generate / update Crux tests; run the core verify-repair loop. |
| [`build/ios/write.md`](../briefs/build/ios/write.md) | Generate / update the SwiftUI iOS shell + verify. |
| [`build/ios/review.md`](../briefs/build/ios/review.md) | Agent-team review of the iOS shell. |
| [`build/android/write.md`](../briefs/build/android/write.md) | Generate / update the Compose Android shell + verify. |
| [`build/android/review.md`](../briefs/build/android/review.md) | Agent-team review of the Android shell. |
| [`merge.md`](../briefs/merge.md) | Pre-merge gate run by `/spec:merge`. |

## References

### Hard rules

- [`hard-rules-core.md`](hard-rules-core.md) — Crux core hard rules.
- [`hard-rules-android.md`](hard-rules-android.md) — Android shell hard rules.

### Crux core depth

- [`crux/app-pattern.md`](crux/app-pattern.md) — `App` trait, `update()` / `view()`, `Model` / `Event` / `Effect` shapes.
- [`crux/capabilities.md`](crux/capabilities.md) — built-in capabilities (HTTP, KV, Time, Render).
- [`crux/command-api.md`](crux/command-api.md) — `Command<Effect, Event>` builder methods.
- [`crux/custom-capabilities.md`](crux/custom-capabilities.md) — when and how to author a custom effect.
- [`crux/testing-patterns.md`](crux/testing-patterns.md) — synchronous test API, `expect_*` chains, `resolve()`.
- [`crux/artifact-to-code-mapping.md`](crux/artifact-to-code-mapping.md) — `spec.md` + `design.md` → Rust types / methods.
- [`crux/update-change-patterns.md`](crux/update-change-patterns.md) — diff-driven editing patterns.
- [`crux/generated-type-conventions.md`](crux/generated-type-conventions.md) — `#[repr(C)]`, `#[derive(Facet)]`, kebab/PascalCase rules.

### iOS shell depth

- [`ios/shell-pattern.md`](ios/shell-pattern.md) — Core.swift / ContentView.swift anatomy.
- [`ios/view-patterns.md`](ios/view-patterns.md) — SwiftUI view patterns and hazards.
- [`ios/token-templates.md`](ios/token-templates.md) — Swift theme code derived from `tokens.yaml`.
- [`ios/design-system-integration.md`](ios/design-system-integration.md) — Theme + Assets.xcassets integration.

### Android shell depth

- [`android/shell-pattern.md`](android/shell-pattern.md) — Core.kt / Application.kt / root composable anatomy.
- [`android/view-patterns.md`](android/view-patterns.md) — Compose view patterns and hazards.
- [`android/token-templates.md`](android/token-templates.md) — Kotlin theme code derived from `tokens.yaml`.
- [`android/design-system-integration.md`](android/design-system-integration.md) — Theme + drawable integration.

### Test writer depth

- [`test-runbook.md`](test-runbook.md) — operational runbook for create / update / repair modes.
- [`test-spec-mapping.md`](test-spec-mapping.md) — scenario → test function mapping rules.

### Review depth

- [`agent-teams.md`](agent-teams.md) — shared specialists + antagonist + lead synthesis pattern.
- [`review/team-protocol-core.md`](review/team-protocol-core.md), [`review/team-protocol-ios.md`](review/team-protocol-ios.md), [`review/team-protocol-android.md`](review/team-protocol-android.md) — per-platform team-spawn prompts.
- [`review/crux-checks.md`](review/crux-checks.md), [`review/logic-checks.md`](review/logic-checks.md), [`review/general-checks.md`](review/general-checks.md), [`review/ios-checks.md`](review/ios-checks.md), [`review/swift-quality-checks.md`](review/swift-quality-checks.md), [`review/android-checks.md`](review/android-checks.md), [`review/kotlin-quality-checks.md`](review/kotlin-quality-checks.md), [`review/universal-checks.md`](review/universal-checks.md) — check libraries.
- [`review/iteration-report.md`](review/iteration-report.md) — iteration-report template and finding-ID conventions.

### Asset materialization (implemented)

Canonical SVG → per-platform exports (`vectis materialize assets`), render-by-`kind` shell writers, bootstrap-only `plan-bootstrap-app-icon-missing` (shell-resident launcher icons satisfy incremental plans). Inference-only symbol fallback; see [`VECTIS-006`](../rules/VECTIS-006-asset-render-by-kind.md), [`ios/design-system-integration.md`](ios/design-system-integration.md), [`android/design-system-integration.md`](android/design-system-integration.md), and [`rfcs/roadmap.md`](../../../../rfcs/roadmap.md#recently-implemented) (**Recently implemented**).

### Layout inferer contract (legacy)

- [`layout-inferer-contract.md`](layout-inferer-contract.md) — historic contract preserved for the [`adapters/sources/screenshots/`](../../../sources/screenshots/) adapter.

### Worked examples

- [`examples/core/`](examples/core/) — simple counter, HTTP counter, KV notes.
- [`examples/ios/`](examples/ios/) — simple counter (iOS), HTTP counter (iOS).
- [`examples/android/`](examples/android/) — simple counter (Android), HTTP counter (Android).
