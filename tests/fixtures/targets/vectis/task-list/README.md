# `tests/fixtures/targets/vectis/task-list/`

End-to-end fixture covering a Vectis slice that exercises `screenshots`-sourced spatial Evidence → `spec.md` + `design.md` → `composition.yaml` regeneration (RFC-25 W2.6 / scenario #5h).

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

This fixture is **not yet executable end-to-end** because W3.1 (synthesis library) and W3.4 (`/spec:build` skill body) have not landed. `input/evidence/screens.yaml` is a hand-authored stand-in for what `sources/screenshots/extract` would emit; W5.3 should replace it with the live extractor output from the sibling source-side fixture once both chunks are in.

## See also

- [`tests/fixtures/targets/vectis/README.md`](../README.md) — the index for Vectis target fixtures.
- [`targets/vectis/briefs/shape.md`](../../../../../targets/vectis/briefs/shape.md) — the shape brief whose injection produced `input/spec.md` + `input/design.md`.
- [`targets/vectis/briefs/build.md`](../../../../../targets/vectis/briefs/build.md) — the build brief that regenerates `expected/composition.yaml` from `input/`.
