# `tests/fixtures/targets/vectis/`

Fixtures for the Vectis target adapter (RFC-25 W2.6, acceptance scenario #5h: "Target `shape` injection — synthesis consumes a non-empty `target.shape` brief"). Vectis is the source/target-split poster child: the **source** half (screenshots → spatial Evidence) belongs to W2.4; the **target** half (core synthesis → `spec.md` + `design.md` → `build` regenerates `composition.yaml` alongside code) is exercised here.

## Fixtures

- [`task-list/`](task-list/) — a single Vectis slice that exercises `screenshots`-sourced spatial Evidence → `spec.md` + `design.md` → `composition.yaml` regeneration. The fixture mirrors the structure of the `tests/fixtures/sources/screenshots/task-list-two-screen/` source-side fixture so a future end-to-end harness can chain the two.

## What these fixtures demonstrate

The Vectis target's `shape` brief carries idiom guidance — Crux app structure (`App`, `Model`, `Event`, `ViewModel`, `Page`, `Route`, `Effect`), platform-shell expectations (iOS SwiftUI, Android Compose), operator-curated build inputs (`tokens.yaml` / `assets.yaml`) — that core synthesis (W3.1, `/spec:refine`) folds into a slice's `spec.md` and `design.md`. The `build` brief then regenerates `composition.yaml` from those canonical artifacts on every run, alongside the implementation code.

These fixtures pin the *output* shape: what synthesised `spec.md` / `design.md` / `tasks.md` look like once the `shape` brief has been consumed, plus the `composition.yaml` that `build` reconstructs from them.

## How W3.1 / W3.4 / W5.3 consume these

- **W3.1 (`/spec:refine`)** — point a synthesis golden test at each fixture's `input/`. The harness verifies that synthesised `spec.md` / `design.md` include every section listed in `expected/shape-evidence.md` (named screen requirements, ViewModel variants, per-page view structs, capability matrix, etc.). The `shape` brief is what makes those sections appear regardless of whether the upstream source is `intent`, `documentation`, or `screenshots`.
- **W3.4 (`/spec:build`)** — point a build golden test at each fixture's `input/` and assert the regenerated `composition.yaml` matches `expected/composition.yaml` modulo formatter passes. `specify tool run vectis -- validate composition` against the expected file must exit clean.
- **W5.3 (acceptance sweep)** — chain `tests/fixtures/sources/screenshots/task-list-two-screen/` (W2.4) → `/spec:refine` (W3.1) → `/spec:build` (W3.4) → `expected/composition.yaml` here. The chain exercises the source/target split end-to-end against scenario #5h.

## Status

These fixtures document the contract; they are **not yet executable end-to-end** because W3.1 (the synthesis library) and W3.4 (`/spec:build` skill body) have not landed. The `input/evidence/screens.yaml` in each fixture is a hand-authored stand-in for what `sources/screenshots/extract` would emit; W5.3 should replace it with the live extractor output once both chunks are in.

## See also

- [`targets/vectis/briefs/shape.md`](../../../../targets/vectis/briefs/shape.md) — the idiom guidance each fixture's `input/` reflects.
- [`targets/vectis/briefs/build.md`](../../../../targets/vectis/briefs/build.md) — the orchestration each fixture's `expected/composition.yaml` reflects.
- [`tests/fixtures/sources/screenshots/task-list-two-screen/`](../../sources/screenshots/task-list-two-screen/) — the matching source-side fixture (W2.4) whose `expected/evidence/` feeds these fixtures' `input/evidence/`.
- [`rfcs/rfc-25-workflow.md`](../../../../rfcs/rfc-25-workflow.md) §Acceptance scenarios #5h.
