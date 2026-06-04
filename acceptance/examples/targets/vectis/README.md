# `acceptance/examples/targets/vectis/`

Fixtures for the Vectis target adapter (Wave 2.6, acceptance scenario #5h: "Target `shape` injection — synthesis consumes a non-empty `target.shape` brief"). Vectis is the source/target-split poster child: the **source** half (screenshots → spatial Evidence) belongs to W2.4; the **target** half (core synthesis → `spec.md` + `design.md` → `build` regenerates `composition.yaml` alongside code) is exercised here.

## Fixtures

- [`task-list/`](task-list/) — a single Vectis slice that exercises `screenshots`-sourced spatial Evidence → `spec.md` + `design.md` → `composition.yaml` regeneration. The fixture mirrors the structure of the `acceptance/examples/sources/screenshots/task-list-two-screen/` source-side fixture so a future end-to-end harness can chain the two.

## What these fixtures demonstrate

The Vectis target's `shape` brief carries idiom guidance — Crux app structure (`App`, `Model`, `Event`, `ViewModel`, `Page`, `Route`, `Effect`), platform-shell expectations (iOS SwiftUI, Android Compose), operator-curated build inputs (`tokens.yaml` / `assets.yaml`) — that core synthesis (W3.1, `/spec:refine`) folds into a slice's `spec.md` and `design.md`. The `build` brief then regenerates `composition.yaml` from those canonical artifacts on every run, alongside the implementation code.

These fixtures pin the *output* shape: what synthesised `spec.md` / `design.md` / `tasks.md` look like once the `shape` brief has been consumed, plus the `composition.yaml` that `build` reconstructs from them.

## Status

These fixtures are documentation pins for the target adapter. No automated harness walks target fixtures; executable replay of `/spec:refine`, `/spec:build`, and the end-to-end source ↔ target chain remains deferred to a future agent/CLI harness.

## See also

- [`adapters/targets/vectis/briefs/shape.md`](../../../../adapters/targets/vectis/briefs/shape.md) — the idiom guidance each fixture's `input/` reflects.
- [`adapters/targets/vectis/briefs/build.md`](../../../../adapters/targets/vectis/briefs/build.md) — the orchestration each fixture's `expected/composition.yaml` reflects.
- [`acceptance/examples/sources/screenshots/task-list-two-screen/`](../../sources/screenshots/task-list-two-screen/) — the matching source-side fixture (W2.4) whose `expected/evidence/` feeds these fixtures' `input/evidence/`.
- [`target-shape-injection` scenario](../../../suites/lifecycle/05h-target-shape-injection/scenario.md) and the [acceptance entry point](../../../../docs/contributing/acceptance.md).
