# `screenshots` source adapter fixture — `task-list-two-screen`

Worked example for the [`screenshots` source adapter](../../../../../adapters/sources/screenshots/adapter.yaml). Exercises both operations of the contract: `survey` emits one lead per screen under `## Lead inventory` in `discovery.md`; `extract` returns one Evidence YAML per lead with `documentation` authority and the `region` / `container` / `leaf` claim kinds co-introduced for spatial Evidence.

This fixture preserves the regression input from the retired `vectis-image-layout-inferer` skill. The `input/` directory holds the synthetic screen image; the `design-system/` directory holds the sibling token and asset manifests downstream `targets/vectis/build` consumes when fusing the Evidence back into `composition.yaml`.

## Layout

```text
input/
  task-list-two-screen.png      # synthetic two-screen flow (task-list + archive)
design-system/
  tokens.yaml                   # gap / padding / color tokens the Evidence references
  assets.yaml                   # icon / image asset ids the Evidence references
expected/
  discovery.md                  # expected survey output (lead inventory section)
  evidence/
    task-list.yaml              # expected extract output for lead: task-list
    archive.yaml                # expected extract output for lead: archive
```

## Bindings assumed by the fixture

- `<source-key>` = `screens`
- `$SOURCE_DIR` = `input/`
- Two lead ids: `archive`, `task-list` (alphabetical order)

The single input image depicts two screens stacked vertically. Vision triage decomposes them into two leads; an operator binding multiple per-screen images would supply them as separate files. The `extract` outputs below model both screens as if they were extracted from one bound directory — `claim-id`s prefix each claim with its screen slug so the same Evidence shape works for either layout.

## Validation

The Evidence YAMLs under `expected/evidence/` validate against [`schemas/evidence.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/evidence.schema.json) in the CLI repo (the spatial `region` / `container` / `leaf` claim kinds land alongside the textual kinds in the closed `kind` enum). The lead blocks in `expected/discovery.md` follow the grammar in [`schemas/discovery/lead.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/discovery/lead.schema.json).

Downstream `targets/vectis/build` (W2.6) consumes the spatial Evidence — after core synthesis (W3.1) folds the claims into `spec.md` / `design.md`, the Vectis target rebuilds `composition.yaml` from the synthesised hierarchy and the sibling `tokens.yaml` / `assets.yaml` in `design-system/`.

## Component-detection note

The `task-row` group skeleton appears in both leads (`task-list.body.tasks.task-row` and `archive.body.tasks.task-row`). Under the `screenshots.extract` stage-6 ≥2-screens rule, the brief promotes the second occurrence to `component: task-row` on its container claim and back-fills the same `component:` slug on the first when the brief sees the second lead during a multi-lead extract pass. In the fixture YAMLs both claims carry `component: task-row` because the expected output models the post-promotion state.
