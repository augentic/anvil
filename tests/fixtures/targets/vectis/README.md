# `tests/fixtures/targets/vectis/`

Fixtures for the Vectis target adapter (RFC-25 W2.6, acceptance scenario #5h: "Target `shape` injection — synthesis consumes a non-empty `target.shape` brief"). Vectis is the source/target-split poster child: the **source** half (screenshots → spatial Evidence) belongs to W2.4; the **target** half (core synthesis → `spec.md` + `design.md` → `build` regenerates `composition.yaml` alongside code) is exercised here.

## Fixtures

- [`task-list/`](task-list/) — a single Vectis slice that exercises `screenshots`-sourced spatial Evidence → `spec.md` + `design.md` → `composition.yaml` regeneration. The fixture mirrors the structure of the `tests/fixtures/sources/screenshots/task-list-two-screen/` source-side fixture so a future end-to-end harness can chain the two.

## What these fixtures demonstrate

The Vectis target's `shape` brief carries idiom guidance — Crux app structure (`App`, `Model`, `Event`, `ViewModel`, `Page`, `Route`, `Effect`), platform-shell expectations (iOS SwiftUI, Android Compose), operator-curated build inputs (`tokens.yaml` / `assets.yaml`) — that core synthesis (W3.1, `/spec:refine`) folds into a slice's `spec.md` and `design.md`. The `build` brief then regenerates `composition.yaml` from those canonical artifacts on every run, alongside the implementation code.

These fixtures pin the *output* shape: what synthesised `spec.md` / `design.md` / `tasks.md` look like once the `shape` brief has been consumed, plus the `composition.yaml` that `build` reconstructs from them.

## How the harness consumes these

- **`/spec:refine` synthesis** — `tests/cross_repo/targets_test.ts` parses each fixture's `input/spec.md` with the W1.3 provenance parser, asserts every requirement carries `ID:` / `Sources:` / a closed `Status:` value, and structurally checks `expected/shape-evidence.md` for bullet content.
- **`/spec:build` regeneration** — the harness parses each `expected/composition.yaml`, asserts it is a YAML mapping with top-level `version` and `screens` keys, and (when `SPECIFY_BIN` is on PATH) the targets-side test can be extended to call `specify tool run vectis -- validate composition` against the expected file.
- **End-to-end source ↔ target chain** — the source-side fixture under [`tests/fixtures/sources/screenshots/task-list-two-screen/`](../../sources/screenshots/task-list-two-screen/) is intentionally shape-aligned with each fixture's `input/evidence/screens.yaml` so a future executable harness can chain the two without reshaping.

## Status

The deterministic boundary the harness covers runs green on every `make test`. Byte-exact synthesis-replay against the LLM-driven `/spec:refine` and `/spec:build` skill bodies is out of scope and tracked separately.

## See also

- [`targets/vectis/briefs/shape.md`](../../../../targets/vectis/briefs/shape.md) — the idiom guidance each fixture's `input/` reflects.
- [`targets/vectis/briefs/build.md`](../../../../targets/vectis/briefs/build.md) — the orchestration each fixture's `expected/composition.yaml` reflects.
- [`tests/fixtures/sources/screenshots/task-list-two-screen/`](../../sources/screenshots/task-list-two-screen/) — the matching source-side fixture (W2.4) whose `expected/evidence/` feeds these fixtures' `input/evidence/`.
- [`rfcs/rfc-25-workflow.md`](../../../../rfcs/rfc-25-workflow.md) §Acceptance scenarios #5h.
