# `evals/fixtures/targets/vectis/task-list/`

End-to-end fixture covering a Vectis slice that exercises `screenshots`-sourced spatial Evidence → `spec.md` + `design.md` → `composition.yaml` regeneration (scenario #5h).

The fixture is paired with [`evals/fixtures/sources/screenshots/task-list-two-screen/`](../../../sources/screenshots/task-list-two-screen/) — the source-side fixture whose `expected/evidence/` is the same shape as this fixture's `input/evidence/screens.yaml`. The two fixtures are intentionally aligned so the full plan-time → slice-time chain can be wired end-to-end.

## Layout

```text
evals/fixtures/targets/vectis/task-list/
├── README.md
├── design-system/                  # operator-curated manifests + committed exports (RFC-46)
│   ├── assets.yaml                 # app-icon + vector illustration + symbol chrome
│   ├── assets/
│   │   ├── app-icon.svg            # canonical masters
│   │   ├── empty-tasks-hero.svg
│   │   └── exports/                # version-controlled materialize output (do not gitignore)
│   │       ├── ios/
│   │       └── android/
├── input/                          # synthesised slice artifacts (post-shape-injection)
│   ├── proposal.md                 # Platforms: core, ios, android
│   ├── specs/
│   │   └── task-list/
│   │       └── spec.md             # core requirements + iOS / Android sections; one REQ-XXX namespace
│   ├── design.md                   # Domain Model, Adapters, shell details, implementation constraints
│   ├── tasks.md                    # core first, shells second
│   └── evidence/
│       └── screens.yaml            # hand-authored stand-in for `sources/screenshots/extract` output
└── expected/
    ├── shape-evidence.md           # checklist of shape-derived sections that MUST appear in input/
    └── composition.yaml            # what `targets/vectis/briefs/build.md` regenerates from input/
```

## Status

This fixture is a documentation pin for the Vectis target. No automated harness walks target fixtures. `input/evidence/screens.yaml` remains a hand-authored stand-in for what `sources/screenshots/extract` would emit; replacing it with live extractor output is a follow-up bound to the source ↔ target chain end-to-end story.

### Committed exports (`design-system/`)

RFC-46 acceptance pin (R46-S25): `design-system/assets/exports/` is version-controlled so CI and shell builds can consume per-platform binaries without running `vectis materialize` on every job. The tree demonstrates:

- **`app-icon`** — vector master → iOS `AppIcon.appiconset` + Android adaptive/legacy mipmap tree; top-level `app-icon:` pointer set.
- **`empty-tasks-hero`** — vector illustration → iOS `@2x`/`@3x` imageset + Android density drawables; composition-referenced from `expected/composition.yaml`.
- **Symbol chrome** (`settings`, `chevron-right`, `plus`, `chevron-left`) — no exports; render-by-`kind` uses platform glyphs at call sites.

Regenerate after editing canonical masters:

```bash
vectis materialize assets design-system/assets.yaml
```

Re-commit `assets/exports/` and any auto-written `sources.<platform>` pins. Golden layout checks run in `wasi-tools/vectis` against a mirrored copy under `tests/fixtures/acceptance/task-list/`.

## See also

- [`evals/fixtures/targets/vectis/README.md`](../README.md) — the index for Vectis target fixtures.
- [`adapters/targets/vectis/briefs/shape.md`](../../../../../adapters/targets/vectis/briefs/shape.md) — the shape brief whose injection produced `input/specs/task-list/spec.md` + `input/design.md`.
- [`adapters/targets/vectis/briefs/build.md`](../../../../../adapters/targets/vectis/briefs/build.md) — the build brief that regenerates `expected/composition.yaml` from `input/`.
