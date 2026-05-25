# `tests/fixtures/targets/vectis/task-list/`

End-to-end fixture covering a Vectis slice that exercises `screenshots`-sourced spatial Evidence → `spec.md` + `design.md` → `composition.yaml` regeneration (Wave 2.6 / scenario #5h).

The fixture is paired with [`tests/fixtures/sources/screenshots/task-list-two-screen/`](../../../sources/screenshots/task-list-two-screen/) — the source-side fixture (W2.4) whose `expected/evidence/` is the same shape as this fixture's `input/evidence/screens.yaml`. The two fixtures are intentionally aligned so the full plan-time → slice-time chain can be wired end-to-end once both chunks land.

## Layout

```text
tests/fixtures/targets/vectis/task-list/
├── README.md
├── input/                          # synthesised slice artifacts (post-shape-injection)
│   ├── proposal.md                 # Platforms: core, ios, android
│   ├── spec.md                     # core requirements + iOS / Android sections; one REQ-XXX namespace
│   ├── design.md                   # Domain Model, Adapters, shell details, implementation constraints
│   ├── tasks.md                    # core first, shells second
│   └── evidence/
│       └── screens.yaml            # hand-authored stand-in for `sources/screenshots/extract` output
└── expected/
    ├── shape-evidence.md           # checklist of shape-derived sections that MUST appear in input/
    └── composition.yaml            # what `targets/vectis/briefs/build.md` regenerates from input/
```

## Status

This fixture is a documentation pin for the Vectis target. `make test` no longer walks target fixtures. `input/evidence/screens.yaml` remains a hand-authored stand-in for what `sources/screenshots/extract` would emit; replacing it with live extractor output is a follow-up bound to the source ↔ target chain end-to-end story.

## See also

- [`tests/fixtures/targets/vectis/README.md`](../README.md) — the index for Vectis target fixtures.
- [`adapters/targets/vectis/briefs/shape.md`](../../../../../adapters/targets/vectis/briefs/shape.md) — the shape brief whose injection produced `input/spec.md` + `input/design.md`.
- [`adapters/targets/vectis/briefs/build.md`](../../../../../adapters/targets/vectis/briefs/build.md) — the build brief that regenerates `expected/composition.yaml` from `input/`.
