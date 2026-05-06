---
name: vectis-design-system-writer
description: "DEPRECATED no-op alias retained only so legacy tasks and plans still resolve while RFC-11 lands. Generates nothing. Use `/vectis:ios-writer` and `/vectis:android-writer` instead — both shell writers now read `tokens.yaml`, `assets.yaml`, and `composition.yaml` directly and emit shell-local theme + asset code under `iOS/<App>/Theme/` and `Android/app/src/main/java/com/vectis/<appname>/ui/theme/`. The alias and its directory are removed in the same change that bumps `schemas/vectis/schema.yaml` to `version: 3` (Phase 4.1 of the RFC-11 implementation plan)."
---

# Design System Writer (deprecated alias)

This skill is a **deprecated no-op**. It exists only so legacy tasks, plans, and operator muscle memory that still spell `/vectis:design-system-writer` resolve to a clear migration message instead of a missing skill while RFC-11's design-system dissolution lands across both repositories. **It generates no files and modifies no project state.**

If you arrived here because a brief, plan entry, or task list named this skill: stop, follow the redirect below, and prefer regenerating the calling artifact (run `/spec:define` again to refresh `tasks.md`) over invoking this alias.

## Replacement

The work this skill used to do — emitting iOS / Android design system code from `tokens.yaml` — now lives inside the shell writers themselves:

| Was | Now |
|---|---|
| `/vectis:design-system-writer` (this skill) | `/vectis:ios-writer` and `/vectis:android-writer` |
| `design-system/ios/Sources/VectisDesign/*.swift` (Swift Package) | `iOS/<App>/Theme/*.swift` (shell-local) |
| `design-system/android/src/main/.../com/vectis/design/*.kt` (`:vectis-design` Gradle module) | `Android/app/src/main/java/com/vectis/<appname>/ui/theme/*.kt` (shell-local) |
| `import VectisDesign` (Swift) | Same-target reference to `Theme/*.swift` (no import) |
| `implementation(project(":vectis-design"))` (Gradle) | Same-package reference to `ui/theme/*.kt` (no import) |
| `tokens.yaml` consumed only by `design-system-writer` | `tokens.yaml`, `assets.yaml`, `composition.yaml` consumed by each shell writer |

Both replacement skills follow the **fallback policy belongs to shell writers** rule from RFC-11 §F: when `tokens.yaml` is absent, the iOS writer falls back to SwiftUI semantic colors / `Font.system(.body)` / inline padding (HIG); the Android writer falls back to Material 3 dynamic / static color schemes, M3 typography slots, and `CardDefaults.*` (Material 3). Neither path produces a `VectisDesign` Swift Package or `:vectis-design` Gradle module.

The single-source-of-truth contract for what each shell writer reads, where it writes, and how it falls back lives in:

- [`plugins/vectis/skills/ios-writer/references/design-system-integration.md`](../ios-writer/references/design-system-integration.md)
- [`plugins/vectis/skills/android-writer/references/design-system-integration.md`](../android-writer/references/design-system-integration.md)

The token-emit templates that used to live under this skill's `references/` directory were migrated in lockstep with the shell-writer rewrites:

- Swift templates moved to [`plugins/vectis/skills/ios-writer/references/swift-token-templates.md`](../ios-writer/references/swift-token-templates.md) (Phase 3.1 of the RFC-11 plan).
- Kotlin templates moved to [`plugins/vectis/skills/android-writer/references/kotlin-token-templates.md`](../android-writer/references/kotlin-token-templates.md) (Phase 3.2).

Existing on-disk `design-system/ios/` and `design-system/android/` trees are not migrated automatically. The shell writers emit the new shell-local files alongside whatever already exists; reviewers flag any remaining `import VectisDesign` / `:vectis-design` references as stale-dependency migration debt (RFC-11 §I "Reviewer surface" + §L "Compatibility policy"). Operators MAY delete the legacy `design-system/ios/` and `design-system/android/` directories once every shell that referenced them has been regenerated.

## Behaviour when invoked

When this alias is invoked (manually, or by a stale plan / task entry), the agent MUST:

1. Print the redirect message above (the "Was → Now" mapping plus the two `design-system-integration.md` pointers).
2. Identify which shell platforms the calling change actually targets by reading the proposal's `## Platforms` enumeration (`ios`, `android`).
3. Suggest the operator re-run `/vectis:ios-writer` and / or `/vectis:android-writer` for each targeted platform.
4. Exit without writing, copying, or deleting any file.

The alias does not read `tokens.yaml`, does not read `assets.yaml`, does not touch `design-system/`, does not invoke `swift build` or `./gradlew`, and does not modify any tracked file. Treat this skill like a `404` page for a moved URL: it explains where the content went and stops.

## Why this alias still exists

RFC-11 §J pins the deprecation lifecycle: this alias is removed in the same change that bumps `schemas/vectis/schema.yaml:version` from `2` to `3` (the natural cliff for any breaking removal of a long-published skill). The RFC-11 implementation plan tracks that bump as Phase 4.1.

Until Phase 4.1 lands:

- New briefs, tasks, and plans MUST NOT name `/vectis:design-system-writer`. The alias is a one-way exit ramp for legacy artifacts, not a recommended skill.
- The CLI does not enforce this prohibition at write time; the plan briefs and tasks brief simply omit the skill from their available-skills tables (RFC-11 §K "Step 3c", landing in Phase 3.5 / 3.6).
- The `references/` directory no longer contains any tracked files — Phases 3.1 and 3.2 moved every template into the shell writers. Phase 4.1 removes the skill directory entirely.

## See also

- [RFC-11 §J — Skill surface](../../../../rfcs/rfc-11-ui-spec.md)
- [RFC-11 §L — Compatibility policy](../../../../rfcs/rfc-11-ui-spec.md)
- [RFC-11 implementation plan, Phases 3.1 / 3.2 / 3.3 / 4.1](../../../../docs/plans/rfc-11-implementation.md)
- [`/vectis:ios-writer` SKILL.md](../ios-writer/SKILL.md)
- [`/vectis:android-writer` SKILL.md](../android-writer/SKILL.md)
