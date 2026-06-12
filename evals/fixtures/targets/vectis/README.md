# `evals/fixtures/targets/vectis/`

Fixtures for the Vectis target adapter (eval scenario #5h: "Target `shape` injection — synthesis consumes a non-empty `target.shape` brief"). Vectis is the source/target-split poster child: the **source** half (screenshots → spatial Evidence) lives in the `screenshots` source adapter; the **target** half (core synthesis → `spec.md` + `design.md` → `build` regenerates `composition.yaml` alongside code) is exercised here.

## Fixtures

- [`task-list/`](task-list/) — a single Vectis slice that exercises `screenshots`-sourced spatial Evidence → `spec.md` + `design.md` → `composition.yaml` regeneration. The fixture mirrors the structure of the `evals/fixtures/sources/screenshots/task-list-two-screen/` source-side fixture so a future end-to-end harness can chain the two.

## What these fixtures demonstrate

The Vectis target's `shape` brief carries idiom guidance — Crux app structure (`App`, `Model`, `Event`, `ViewModel`, `Page`, `Route`, `Effect`), platform-shell expectations (iOS SwiftUI, Android Compose), operator-curated build inputs (`tokens.yaml` / `assets.yaml`) — that core synthesis (`/spec:refine`) folds into a slice's `spec.md` and `design.md`. The `build` brief then regenerates `composition.yaml` from those canonical artifacts on every run, alongside the implementation code.

These fixtures pin the *output* shape: what synthesised `spec.md` / `design.md` / `tasks.md` look like once the `shape` brief has been consumed, plus the `composition.yaml` that `build` reconstructs from them.

## Status

These fixtures are documentation pins for the target adapter. No automated harness walks target fixtures; executable replay of `/spec:refine`, `/spec:build`, and the end-to-end source ↔ target chain remains deferred to a future agent/CLI harness.

## See also

- [`adapters/targets/vectis/briefs/shape.md`](../../../../adapters/targets/vectis/briefs/shape.md) — the idiom guidance each fixture's `input/` reflects.
- [`adapters/targets/vectis/briefs/build.md`](../../../../adapters/targets/vectis/briefs/build.md) — the orchestration each fixture's `expected/composition.yaml` reflects.
- [`evals/fixtures/sources/screenshots/task-list-two-screen/`](../../sources/screenshots/task-list-two-screen/) — the matching source-side fixture whose `expected/evidence/` feeds these fixtures' `input/evidence/`.
- [`target-shape` scenario](../../../scenarios/target-shape.md) and the [evals entry point](../../../../docs/contributing/evals.md).
