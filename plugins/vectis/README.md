# Vectis reference material

Reference documentation for the Vectis target adapter at [`targets/vectis/`](../../targets/vectis/). In Specify 2.0 (RFC-25) Vectis is a **target adapter** — `shape`, `build`, `merge` — not a slash-command plugin.

The orchestration of the retired `vectis-core-writer`, `vectis-test-writer`, `vectis-ios-writer`, `vectis-android-writer`, `vectis-core-reviewer`, `vectis-ios-reviewer`, `vectis-android-reviewer`, and `vectis-template-updater` skills now lives in [`targets/vectis/briefs/build.md`](../../targets/vectis/briefs/build.md) and eight phase sub-briefs under [`targets/vectis/briefs/build/`](../../targets/vectis/briefs/build/). The depth (Crux idioms, SwiftUI patterns, Compose patterns, hard rules, review check libraries, token templates, design-system integration) and worked examples live in this folder.

## Briefs

| Brief | Purpose |
|-------|---------|
| [`shape.md`](../../targets/vectis/briefs/shape.md) | Idiom guidance for core synthesis. |
| [`build.md`](../../targets/vectis/briefs/build.md) | Orchestrator: phase order, sub-agent contract, verify-serial / review-parallel rule, consolidation, template-drift signal, phase outcome. |
| [`build/composition.md`](../../targets/vectis/briefs/build/composition.md) | Regenerate `composition.yaml` from `spec.md` + `design.md`; run the deterministic validator gate. |
| [`build/core/write.md`](../../targets/vectis/briefs/build/core/write.md) | Generate / update the Crux shared core. |
| [`build/core/review.md`](../../targets/vectis/briefs/build/core/review.md) | Agent-team review of the Rust `shared` crate. |
| [`build/test.md`](../../targets/vectis/briefs/build/test.md) | Generate / update Crux tests; run the core verify-repair loop. |
| [`build/ios/write.md`](../../targets/vectis/briefs/build/ios/write.md) | Generate / update the SwiftUI iOS shell + verify. |
| [`build/ios/review.md`](../../targets/vectis/briefs/build/ios/review.md) | Agent-team review of the iOS shell. |
| [`build/android/write.md`](../../targets/vectis/briefs/build/android/write.md) | Generate / update the Compose Android shell + verify. |
| [`build/android/review.md`](../../targets/vectis/briefs/build/android/review.md) | Agent-team review of the Android shell. |
| [`merge.md`](../../targets/vectis/briefs/merge.md) | Pre-merge gate run by `/spec:merge`. |

## References

### Hard rules

- [`hard-rules-core.md`](references/hard-rules-core.md) — Crux core hard rules.
- [`hard-rules-android.md`](references/hard-rules-android.md) — Android shell hard rules.

### Crux core depth

- [`crux/app-pattern.md`](references/crux/app-pattern.md) — `App` trait, `update()` / `view()`, `Model` / `Event` / `Effect` shapes.
- [`crux/capabilities.md`](references/crux/capabilities.md) — built-in capabilities (HTTP, KV, Time, Render).
- [`crux/command-api.md`](references/crux/command-api.md) — `Command<Effect, Event>` builder methods.
- [`crux/custom-capabilities.md`](references/crux/custom-capabilities.md) — when and how to author a custom effect.
- [`crux/testing-patterns.md`](references/crux/testing-patterns.md) — synchronous test API, `expect_*` chains, `resolve()`.
- [`crux/artifact-to-code-mapping.md`](references/crux/artifact-to-code-mapping.md) — `spec.md` + `design.md` → Rust types / methods.
- [`crux/update-change-patterns.md`](references/crux/update-change-patterns.md) — diff-driven editing patterns.
- [`crux/generated-type-conventions.md`](references/crux/generated-type-conventions.md) — `#[repr(C)]`, `#[derive(Facet)]`, kebab/PascalCase rules.

### iOS shell depth

- [`ios/shell-pattern.md`](references/ios/shell-pattern.md) — Core.swift / ContentView.swift anatomy.
- [`ios/view-patterns.md`](references/ios/view-patterns.md) — SwiftUI view patterns and hazards.
- [`ios/token-templates.md`](references/ios/token-templates.md) — Swift theme code derived from `tokens.yaml`.
- [`ios/design-system-integration.md`](references/ios/design-system-integration.md) — Theme + Assets.xcassets integration.

### Android shell depth

- [`android/shell-pattern.md`](references/android/shell-pattern.md) — Core.kt / Application.kt / root composable anatomy.
- [`android/view-patterns.md`](references/android/view-patterns.md) — Compose view patterns and hazards.
- [`android/token-templates.md`](references/android/token-templates.md) — Kotlin theme code derived from `tokens.yaml`.
- [`android/design-system-integration.md`](references/android/design-system-integration.md) — Theme + drawable integration.

### Test writer depth

- [`test-runbook.md`](references/test-runbook.md) — operational runbook for create / update / repair modes.
- [`test-spec-mapping.md`](references/test-spec-mapping.md) — scenario → test function mapping rules.

### Review depth

- [`agent-teams.md`](references/agent-teams.md) — shared specialists + antagonist + lead synthesis pattern.
- [`review/team-protocol-core.md`](references/review/team-protocol-core.md), [`review/team-protocol-ios.md`](references/review/team-protocol-ios.md), [`review/team-protocol-android.md`](references/review/team-protocol-android.md) — per-platform team-spawn prompts.
- [`review/crux-checks.md`](references/review/crux-checks.md), [`review/logic-checks.md`](references/review/logic-checks.md), [`review/general-checks.md`](references/review/general-checks.md), [`review/ios-checks.md`](references/review/ios-checks.md), [`review/swift-quality-checks.md`](references/review/swift-quality-checks.md), [`review/android-checks.md`](references/review/android-checks.md), [`review/kotlin-quality-checks.md`](references/review/kotlin-quality-checks.md), [`review/universal-checks.md`](references/review/universal-checks.md) — check libraries.
- [`review/iteration-report.md`](references/review/iteration-report.md) — iteration-report template and finding-ID conventions.

### Layout inferer contract (legacy)

- [`layout-inferer-contract.md`](references/layout-inferer-contract.md) — historic contract preserved for the [`sources/screenshots/`](../../sources/screenshots/) adapter.

### Worked examples

- [`examples/core/`](references/examples/core/) — simple counter, HTTP counter, KV notes.
- [`examples/ios/`](references/examples/ios/) — simple counter (iOS), HTTP counter (iOS).
- [`examples/android/`](references/examples/android/) — simple counter (Android), HTTP counter (Android).
