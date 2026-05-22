# Vectis

Cross-platform Crux application generation: Rust shared core, SwiftUI iOS shell, and Kotlin/Jetpack Compose Android shell.

The Vectis adapter manifest and its `shape` / `build` / `merge` briefs moved to [`targets/vectis/`](../../targets/vectis/) in RFC-25 W2.6. The eight prior generation skills (`core-writer`, `test-writer`, `core-reviewer`, `ios-writer`, `ios-reviewer`, `android-writer`, `android-reviewer`, `template-updater`) were collapsed into those briefs; the `image-layout-inferer` skill moved to the [`screenshots` source adapter](../../sources/screenshots/adapter.yaml) per RFC-25 W2.4. This plugin now hosts references only.

## Briefs

| Brief | Location | Role |
|-------|----------|------|
| `shape` | [`targets/vectis/briefs/shape.md`](../../targets/vectis/briefs/shape.md) | Idiom guidance core synthesis folds into `spec.md` + `design.md`. |
| `build` | [`targets/vectis/briefs/build.md`](../../targets/vectis/briefs/build.md) | Regenerates `composition.yaml` from `spec.md` + `design.md`, then drives core / test / iOS / Android generation and reviewer passes. |
| `merge` | [`targets/vectis/briefs/merge.md`](../../targets/vectis/briefs/merge.md) | Lands the slice and re-runs the host cap matrix (`cargo`, `make build`, `gradlew`) against the merged baseline. |

## References

- [Vectis target adapter manifest](../../targets/vectis/adapter.yaml)
- [Vectis codex rules](../../targets/vectis/codex/)
- [Vectis shared schemas](../../targets/vectis/schemas/)
- [Crux patterns and design-system docs](references/)
- [Layout-inferer contract](references/layout-inferer-contract.md)
